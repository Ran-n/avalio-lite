// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.921715
//+ Revised: 	2026/10/28 21:18:53.041199
// -----------------------------------------------------------------------------

//! Runs the real app, in its real window, for the desktop tests (`tests/desktop.rs`), against a
//! scratch settings folder so the user's own data is never touched.
//! Usage: cargo run --example app_driver -- <settings folder> [en|gl]
//!
//! It reads one JSON command per line on stdin and answers each with one line on stdout,
//! `@@{"ok":true,...}` or `@@{"ok":false,"error":"..."}`. Commands only set things up and read
//! state; clicks and typing come from the test, through the operating system, like a user's.
//!
//!   {"cmd":"ping"}                                   the window's scale factor
//!   {"cmd":"size","width":1000,"height":600}         logical size of the window
//!   {"cmd":"theme","dark":true}                      light or dark look, without saving the choice
//!   {"cmd":"locate","label":"Create database"}       where the controls with that label are (and a
//!                                                    field's text), and where an open popup menu
//!                                                    sits (its controls' positions are relative to it)
//!   {"cmd":"dialogs","folders":[...],"files":[...],"answers":[true]}  answers for the next dialogs
//!   {"cmd":"state"}                                  what the window shows, read-only
//!   {"cmd":"quit"}                                   closes the window; the app shuts down

#[path = "../tests/common/mod.rs"]
mod common;

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;

use avalio_lite::app::{self, Session};
use avalio_lite::config::ConfigStore;
use avalio_lite::i18n::Language;
use avalio_lite::logging;
use avalio_lite::ui::{Gradebook, Theme, Tone};
use common::FakeDialogs;
use common::controls;
use i_slint_core::window::{PopupWindowLocation, WindowInner};
use serde_json::{Value, json};
use slint::{ComponentHandle, Model};

thread_local! {
    static DRIVEN: RefCell<Option<(Session, Rc<FakeDialogs>)>> = const { RefCell::new(None) };
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(config_dir) = args.next().map(PathBuf::from) else {
        eprintln!("usage: app_driver <settings folder> [en|gl]");
        return ExitCode::FAILURE;
    };
    let language = args.next().and_then(|code| Language::from_code(&code)).unwrap_or(Language::English);
    let logger = logging::install();
    let dialogs = Rc::new(FakeDialogs::default());
    let session = match app::open(ConfigStore::in_dir(&config_dir), logger, dialogs.clone(), language) {
        Ok(session) => session,
        Err(reason) => {
            eprintln!("the window failed: {reason}");
            return ExitCode::FAILURE;
        }
    };
    let window = session.window.clone_strong();
    DRIVEN.with(|driven| *driven.borrow_mut() = Some((session, dialogs)));

    std::thread::spawn(|| {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let posted = slint::invoke_from_event_loop(move || reply(handle(&line)));
            if posted.is_err() {
                break;
            }
        }
    });

    let result = window.run();
    DRIVEN.with(|driven| {
        if let Some((session, _)) = driven.borrow_mut().take() {
            session.state.borrow_mut().shutdown();
        }
    });
    reply(Ok(json!({ "quit": true })));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("the event loop failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn reply(result: Result<Value, String>) {
    let mut answer = match result {
        Ok(Value::Object(fields)) => Value::Object(fields),
        Ok(other) => json!({ "value": other }),
        Err(error) => json!({ "ok": false, "error": error }),
    };
    if answer.get("ok").is_none() {
        answer["ok"] = json!(true);
    }
    let mut stdout = std::io::stdout().lock();
    // the test reads these lines; if it is gone there is nobody left to tell
    let _ = writeln!(stdout, "@@{answer}");
    let _ = stdout.flush();
}

fn handle(line: &str) -> Result<Value, String> {
    let command: Value = serde_json::from_str(line).map_err(|error| format!("not JSON: {error}"))?;
    let name = command["cmd"].as_str().ok_or("missing \"cmd\"")?.to_owned();
    DRIVEN.with(|driven| {
        let driven = driven.borrow();
        let (session, dialogs) = driven.as_ref().ok_or("the app has shut down")?;
        let window = &session.window;
        match name.as_str() {
            "ping" => Ok(json!({ "scale": window.window().scale_factor() })),
            "size" => {
                let width = command["width"].as_f64().ok_or("missing width")? as f32;
                let height = command["height"].as_f64().ok_or("missing height")? as f32;
                window.window().set_size(slint::LogicalSize::new(width, height));
                Ok(json!({}))
            }
            "theme" => {
                window.invoke_apply_theme(command["dark"].as_bool().ok_or("missing dark")?);
                Ok(json!({}))
            }
            "locate" => {
                let label = command["label"].as_str().ok_or("missing label")?;
                let found: Vec<Value> = controls::labelled(window, label)
                    .iter()
                    .map(|element| {
                        let (position, size) = (element.absolute_position(), element.size());
                        let description = element.accessible_description().unwrap_or_default();
                        let value = element.accessible_value().unwrap_or_default();
                        json!({
                            "x": position.x, "y": position.y, "width": size.width, "height": size.height,
                            "description": description.as_str(), "value": value.as_str(),
                        })
                    })
                    .collect();
                let popup =
                    WindowInner::from_pub(window.window()).active_popups().iter().find_map(|popup| {
                        match &popup.location {
                            PopupWindowLocation::ChildWindow(at) => Some(json!({ "x": at.x, "y": at.y })),
                            PopupWindowLocation::TopLevel(_) => None,
                        }
                    });
                Ok(json!({ "found": found, "popup": popup }))
            }
            "dialogs" => {
                for folder in command["folders"].as_array().into_iter().flatten() {
                    dialogs.pick_folder_next(folder.as_str().map(std::path::Path::new));
                }
                for file in command["files"].as_array().into_iter().flatten() {
                    dialogs.pick_file_next(file.as_str().map(std::path::Path::new));
                }
                for answer in command["answers"].as_array().into_iter().flatten() {
                    dialogs.answer_next(answer.as_bool().ok_or("answers are true or false")?);
                }
                Ok(json!({}))
            }
            "state" => Ok(state(session)),
            "quit" => {
                window.hide().map_err(|error| error.to_string())?;
                slint::quit_event_loop().map_err(|error| error.to_string())?;
                Ok(json!({}))
            }
            other => Err(format!("unknown command {other:?}")),
        }
    })
}

fn state(session: &Session) -> Value {
    let window = &session.window;
    let gradebook = window.global::<Gradebook>();
    let rows: Vec<Value> = gradebook
        .get_rows()
        .iter()
        .map(|row| {
            let cells: Vec<Value> = row
                .cells
                .iter()
                .map(|cell| {
                    let tone = match cell.tone {
                        Tone::Pass => "pass",
                        Tone::Fail => "fail",
                        Tone::Neutral => "neutral",
                    };
                    json!({ "text": cell.text.as_str(), "tone": tone, "provisional": cell.provisional, "invalid": cell.invalid })
                })
                .collect();
            json!({ "name": row.name.as_str(), "cells": cells })
        })
        .collect();
    let columns: Vec<String> = gradebook.get_columns().iter().map(|column| column.title.to_string()).collect();
    let classes: Vec<String> = gradebook.get_classes().iter().map(|class| class.name.to_string()).collect();
    json!({
        "setup_required": window.get_setup_required(),
        "locked": window.get_locked(),
        "pin_set": window.get_pin_set(),
        "language": window.get_language_code().as_str(),
        "dark": window.global::<Theme>().get_dark(),
        "error": window.get_error_message().as_str(),
        "status": window.get_status_message().as_str(),
        "sheet_error": gradebook.get_error_message().as_str(),
        "classes": classes,
        "class_name": gradebook.get_class_name().as_str(),
        "columns": columns,
        "rows": rows,
        "panel_index": gradebook.get_panel_index(),
        "database_open": session.state.borrow().connection().is_some(),
    })
}
