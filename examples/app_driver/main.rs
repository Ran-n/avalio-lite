// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:01.613950
//+ Revised: 	2026/11/24 17:54:03.522658
// -----------------------------------------------------------------------------

//! Runs the real app, in its real window, for the desktop tests (`tests/desktop/`), against a
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
//!   {"cmd":"release","version":"0.6.0"}             what the next check finds: that version with its
//!                                                    files; with "serve":"good" (or "bad-signature",
//!                                                    "bad-checksum", "slow") published on a local
//!                                                    fixture server, so Update and restart downloads it
//!                                                    and replaces the scratch exe in the settings folder;
//!                                                    with "hang":true the checks never answer
//!   {"cmd":"activate","active":true}                 whether the window counts as the active one (its
//!                                                    focused field draws focused), as when the user
//!                                                    brings it to the front or leaves it, without
//!                                                    touching the real foreground; answers what it was
//!   {"cmd":"type","text":"1A"}                       types the text into the focused control, as
//!                                                    Windows messages to the window
//!   {"cmd":"state"}                                  what the window shows, read-only
//!   {"cmd":"quit"}                                   closes the window; the app shuts down
//!
//! The modules: `startup` (the options and the session the window runs on), `commands` (one function
//! per command above), `releases` (the driver's forge), `state` (the `state` reply, checked for
//! personal data). This file holds the reader of stdin, the dispatch and the one reply line.

mod commands;
#[path = "../../tests/common/mod.rs"]
mod common;
mod releases;
mod startup;
mod state;

use std::cell::RefCell;
use std::io::{BufRead, Write};
use std::process::ExitCode;
use std::rc::Rc;

use avalio_lite::app::Session;
use common::FakeDialogs;
use common::licence::FakeLicence;
use serde_json::{Value, json};
use slint::ComponentHandle;

thread_local! {
    /// The session the window runs on, and the fake dialogs its commands answer through.
    pub(crate) static DRIVEN: RefCell<Option<(Session, Rc<FakeDialogs>)>> = const { RefCell::new(None) };
    /// The fake Clavren the `licence` command answers through.
    pub(crate) static LICENCE: RefCell<Option<FakeLicence>> = const { RefCell::new(None) };
}

fn main() -> ExitCode {
    let options = match startup::Options::from_args(std::env::args().skip(1)) {
        Ok(options) => options,
        Err(usage) => {
            eprintln!("{usage}");
            return ExitCode::FAILURE;
        }
    };
    let window = match startup::open(options) {
        Ok(window) => window,
        Err(reason) => {
            eprintln!("{reason}");
            return ExitCode::FAILURE;
        }
    };
    read_commands();
    let result = window.run();
    let restart = shut_down();
    reply(Ok(json!({ "quit": true, "restart": restart })));
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("the event loop failed: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Reads the commands from stdin on a thread of its own, and runs each on the window's event loop.
fn read_commands() {
    std::thread::spawn(|| {
        for line in std::io::stdin().lock().lines() {
            let Ok(line) = line else { break };
            let posted = slint::invoke_from_event_loop(move || reply(handle(&line)));
            if posted.is_err() {
                break;
            }
        }
    });
}

/// As the app quits: a download is cancelled and waited for, then the state shuts down. Returns the
/// program the update restarts, if one was replaced.
fn shut_down() -> Option<String> {
    let restart = DRIVEN.with(|driven| {
        driven.borrow_mut().take().map(|(session, _)| {
            session.updater.quit();
            session.state.borrow_mut().shutdown();
            session.updater.restart()
        })
    });
    restart.flatten().map(|program| program.display().to_string())
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

/// Runs one line of the protocol: parses it, then dispatches it to its command by name.
fn handle(line: &str) -> Result<Value, String> {
    let command: Value = serde_json::from_str(line).map_err(|error| format!("not JSON: {error}"))?;
    let name = command["cmd"].as_str().ok_or("missing \"cmd\"")?.to_owned();
    DRIVEN.with(|driven| {
        let driven = driven.borrow();
        let (session, dialogs) = driven.as_ref().ok_or("the app has shut down")?;
        dispatch(&name, &command, session, dialogs)
    })
}

fn dispatch(name: &str, command: &Value, session: &Session, dialogs: &FakeDialogs) -> Result<Value, String> {
    let window = &session.window;
    match name {
        "ping" => commands::ping(window),
        "size" => commands::size(window, command),
        "minimum" => commands::minimum(window, command),
        "theme" => commands::theme(window, command),
        "locate" => commands::locate(window, command),
        "dialogs" => commands::dialogs(dialogs, command),
        "licence" => commands::licence(command),
        "release" => commands::release(command),
        "activate" => commands::activate(window, command),
        "type" => commands::type_text(command),
        "state" => state::reply(session, dialogs),
        "quit" => commands::quit(window),
        other => Err(format!("unknown command {other:?}")),
    }
}
