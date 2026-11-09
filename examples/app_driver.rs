// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.921715
//+ Revised: 	2026/11/09 10:42:02.530093
// -----------------------------------------------------------------------------

//! Runs the real app, in its real window, for the desktop tests (`tests/desktop.rs`), against a
//! scratch settings folder so the user's own data is never touched.
//! Usage: cargo run --example app_driver -- <settings folder> [en|gl] [unlicensed|offline-soon]
//!
//! The licence comes from a fake Clavren in memory (tests/common/licence.rs): activated, never
//! activated with `unlicensed` (the licence screen), or activated 26 days ago with `offline-soon`
//! (the sheet's "connect to the internet" warning).
//!
//! It reads one JSON command per line on stdin and answers each with one line on stdout,
//! `@@{"ok":true,...}` or `@@{"ok":false,"error":"..."}`. Commands only set things up and read
//! state; clicks and keys come from the test, through the operating system, like a user's. Typed
//! text is the one exception: the driver posts it to its own window, so each character is queued
//! together with its key press (see `common::win32::post_text`).
//!
//!   {"cmd":"ping"}                                   the window's scale factor
//!   {"cmd":"size","width":1000,"height":600}         logical size of the window
//!   {"cmd":"minimum","width":1000,"height":582}      lowers the window's minimum size, as the app does
//!                                                    on a screen too small for it
//!   {"cmd":"theme","dark":true}                      light or dark look, without saving the choice
//!   {"cmd":"locate","label":"Create database"}       where the controls with that label are (and a
//!                                                    plain text showing it, with "text":true; and a
//!                                                    field's text), and where an open popup menu
//!                                                    sits (its controls' positions are relative to it)
//!   {"cmd":"dialogs","folders":[...],"files":[...],"saves":[...],"answers":[true]}  answers for the
//!                                                    next dialogs (each list may be left out; null cancels)
//!   {"cmd":"licence","refuse":"already_activated","status":409}  how the fake Clavren answers
//!                                                    from now on; {"cmd":"licence","offline":true}
//!                                                    stops answering, {"cmd":"licence"} answers normally
//!   {"cmd":"release","version":"0.6.0"}             what the next look for a new version finds
//!   {"cmd":"activate","active":true}                 whether the window counts as the active one (its
//!                                                    focused field draws focused), as when the user
//!                                                    brings it to the front or leaves it, without
//!                                                    touching the real foreground; answers what it was
//!   {"cmd":"type","text":"1A"}                       types the text into the focused control, as
//!                                                    Windows messages to the window
//!   {"cmd":"state"}                                  what the window shows, read-only
//!   {"cmd":"quit"}                                   closes the window; the app shuts down

#[path = "../tests/common/mod.rs"]
mod common;

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;

use avalio_lite::app::{self, Ports, Session, placement};
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::ConfigStore;
use avalio_lite::platform::logging;
use avalio_lite::ui::{Gradebook, Licence, Theme, Tone, Update};
use avalio_lite::update::Release;
use avalio_lite::update::source::FixedRelease;
use avalio_lite::update::ui::Updates;
use avalio_lite::window::worker::InlineWorker;
use common::FakeDialogs;
use common::controls;
use common::licence::{FakeLicence, Mode};
use i_slint_core::window::{PopupWindowLocation, WindowInner};
use serde_json::{Value, json};
use slint::{ComponentHandle, Model};

thread_local! {
    static DRIVEN: RefCell<Option<(Session, Rc<FakeDialogs>)>> = const { RefCell::new(None) };
    static LICENCE: RefCell<Option<FakeLicence>> = const { RefCell::new(None) };
    /// what "the latest release" answers; set with the `release` command
    static RELEASES: Arc<FixedRelease> = Arc::new(FixedRelease::default());
}

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let Some(config_dir) = args.next().map(PathBuf::from) else {
        eprintln!("usage: app_driver <settings folder> [en|gl] [unlicensed|offline-soon]");
        return ExitCode::FAILURE;
    };
    let language = args.next().and_then(|code| Language::from_code(&code)).unwrap_or(Language::English);
    let licence = match args.next().as_deref() {
        Some("unlicensed") => FakeLicence::new(),
        Some("offline-soon") => FakeLicence::at(chrono::Utc::now() - chrono::TimeDelta::days(26)).activate(),
        _ => FakeLicence::activated(),
    };
    let logger = logging::install();
    let dialogs = Rc::new(FakeDialogs::default());
    // the fake answers at once, so its work runs inline and every reply sees its result
    let (client, worker) = (licence.client(), Rc::new(InlineWorker));
    LICENCE.with(|cell| *cell.borrow_mut() = Some(licence));
    let updates = Updates {
        source: RELEASES.with(Clone::clone),
        key: common::release_key().verifying_key(),
        // never the driver's own exe: Update now opens the release page instead
        installation: None,
        downloads: config_dir.join("downloads"),
    };
    let ports = Ports { dialogs: dialogs.clone(), licence: client, worker, updates };
    let session = match app::open(ConfigStore::in_dir(&config_dir), logger, ports, language) {
        Ok(session) => session,
        Err(reason) => {
            eprintln!("the window failed: {reason}");
            return ExitCode::FAILURE;
        }
    };
    let window = session.window.clone_strong();
    // the app opens on the monitor under the mouse; the tests pin it to the primary one, so the display
    // scale under test is the one set there, wherever the user's mouse is
    placement::apply_with(&window, placement::work_area_at(placement::PRIMARY_SCREEN));
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
                // a window the app opened maximised (on a small screen) takes a size only once restored
                window.window().set_maximized(false);
                window.window().set_size(slint::LogicalSize::new(width, height));
                Ok(json!({}))
            }
            "minimum" => {
                window.set_fit_min_width(command["width"].as_f64().ok_or("missing width")? as f32);
                window.set_fit_min_height(command["height"].as_f64().ok_or("missing height")? as f32);
                Ok(json!({}))
            }
            "theme" => {
                window.invoke_apply_theme(command["dark"].as_bool().ok_or("missing dark")?);
                Ok(json!({}))
            }
            "locate" => {
                let label = command["label"].as_str().ok_or("missing label")?;
                let elements = if command["text"].as_bool() == Some(true) {
                    controls::texts(window, label)
                } else {
                    controls::labelled(window, label)
                };
                let found: Vec<Value> = elements
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
                for save in command["saves"].as_array().into_iter().flatten() {
                    dialogs.save_file_next(save.as_str().map(std::path::Path::new));
                }
                for answer in command["answers"].as_array().into_iter().flatten() {
                    dialogs.answer_next(answer.as_bool().ok_or("answers are true or false")?);
                }
                Ok(json!({}))
            }
            "licence" => {
                let mode = if command["offline"].as_bool() == Some(true) {
                    Mode::Offline
                } else if let Some(code) = command["refuse"].as_str() {
                    let status = command["status"].as_u64().ok_or("missing status")?;
                    // the modes keep `'static` codes; a driver run makes only a handful
                    Mode::Refuse(Box::leak(code.to_owned().into_boxed_str()), status as u16)
                } else {
                    Mode::Normal
                };
                LICENCE.with(|cell| cell.borrow().as_ref().map(|licence| licence.server.set_mode(mode)));
                Ok(json!({}))
            }
            "release" => {
                let version = command["version"].as_str().ok_or("missing version")?;
                let version = semver::Version::parse(version).map_err(|error| error.to_string())?;
                let page = format!("https://github.com/Ran-n/avalio-lite/releases/tag/v{version}");
                RELEASES.with(|releases| releases.set(Ok(Release { version, page, assets: Vec::new() })));
                Ok(json!({}))
            }
            "activate" => {
                let active = command["active"].as_bool().ok_or("missing active")?;
                let was = WindowInner::from_pub(window.window()).active();
                // straight to Slint: a posted WM_SETFOCUS would make winit replay the keys the user
                // holds down in another window (modifiers reach the app, so a held Ctrl could stick).
                // Only on a change: Slint sends the focused control a focus event on every one.
                if active != was {
                    window.window().dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(active));
                }
                Ok(json!({ "was": was }))
            }
            "type" => {
                let text = command["text"].as_str().ok_or("missing text")?;
                let hwnd = common::win32::app_window_of(std::process::id()).ok_or("the window isn't showing")?;
                common::win32::post_text(hwnd, text)?;
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
        "active": WindowInner::from_pub(window.window()).active(),
        "locked": window.get_locked(),
        "licensed": window.global::<Licence>().get_licensed(),
        "update": window.global::<Update>().get_notice().as_str(),
        "pin_set": window.get_pin_set(),
        "language": window.get_language_code().as_str(),
        "dark": window.global::<Theme>().get_dark(),
        "error": window.get_error_message().as_str(),
        "status": window.get_status_message().as_str(),
        "classes": classes,
        "class_name": gradebook.get_class_name().as_str(),
        "columns": columns,
        "rows": rows,
        "panel_index": gradebook.get_panel_index(),
        "sidebar_collapsed": gradebook.get_sidebar_collapsed(),
        "database_open": session.state.borrow().connection().is_some(),
    })
}
