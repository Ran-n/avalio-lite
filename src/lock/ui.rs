// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:42.253612
//+ Revised: 	2026/11/25 11:43:08.207154
// -----------------------------------------------------------------------------

//! Wires the PIN to the window: the lock screen, forgetting the PIN, and setting, changing or removing it in
//! Settings. The rules for a PIN are in `crate::lock`; re-encrypting the database in `state::protection`.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use slint::{ComponentHandle, SharedString};

use crate::error::{AppError, Result};
use crate::gradebook;
use crate::i18n::{self, DataMessage, Message};
use crate::lock::AttemptLimiter;
use crate::pupil_view;
use crate::state::SetupReason;
use crate::ui::{AppWindow, Backup};
use crate::window::report_failure;
use crate::window::{SharedState, view, with_window, with_window1};

/// Wires the lock screen, the leaving of it and the PIN settings, in the order they were always wired.
/// `limiter` counts wrong PINs, here and on the pupil view's cover.
pub fn wire(window: &AppWindow, state: &SharedState, limiter: &Rc<RefCell<AttemptLimiter>>) {
    wire_lock_now(window, state);
    wire_unlock(window, state, limiter);
    wire_forgot_pin(window, state);
    wire_back_to_lock(window, state);
    wire_set_pin(window, state);
    wire_change_pin(window, state);
    wire_remove_pin(window, state);
    wire_erase_last(window);
}

/// Shows the lock screen; the database is already closed (or was never opened).
pub fn show_locked(window: &AppWindow) {
    // a pupil's grades aren't there to come back to once the app is unlocked
    pupil_view::ui::end(window);
    // a backup waiting for its PIN is dropped, so nothing of it stays open behind the lock
    window.global::<Backup>().invoke_pin_cancelled();
    window.set_unlock_pin(SharedString::new());
    window.set_unlock_error(SharedString::new());
    clear_pin_fields(window);
    window.set_pin_set(true);
    window.set_locked(true);
}

/// After a database was opened without a PIN: the lock button and Settings show "no PIN".
pub fn show_unprotected(window: &AppWindow) {
    window.set_pin_set(false);
    window.set_locked(false);
}

/// Locks the open database now. With no PIN set there is nothing to lock it with, and that is said.
fn wire_lock_now(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.on_lock_now(with_window(window, move |window| {
        if shared.borrow_mut().lock() {
            show_locked(window);
        } else {
            view::show_error_text(window, i18n::text(i18n::current(), &Message::Data(DataMessage::LockNeedsPin)));
        }
    }));
}

/// Tries the PIN typed on the lock screen. A wrong PIN is counted by the guess limiter, which
/// holds the next attempts back for a while.
fn wire_unlock(window: &AppWindow, state: &SharedState, limiter: &Rc<RefCell<AttemptLimiter>>) {
    let (shared, limiter) = (state.clone(), limiter.clone());
    window.on_unlock(with_window1(window, move |window, pin: SharedString| {
        let outcome = limiter.borrow_mut().attempt(Instant::now(), || shared.borrow_mut().unlock(&pin));
        window.set_unlock_pin(SharedString::new());
        match outcome {
            Ok(()) => {
                window.set_unlock_error(SharedString::new());
                window.set_locked(false);
                gradebook::ui::show_database(window, &shared);
            }
            // the file went away while locked (e.g. its drive was unplugged): no PIN can open it, so
            // offer what a restart would, the setup screen saying it is missing
            Err(AppError::NotFound(path)) => {
                log::error!("the locked database is gone: {}", path.display());
                view::show_setup(window, &SetupReason::DatabaseMissing(path));
            }
            Err(error) => {
                // a wrong PIN is already logged by the limiter, as a warning
                if !matches!(error, AppError::WrongPin | AppError::PinWait { .. }) {
                    log::error!("unlocking failed: {error}");
                }
                window.set_unlock_error(i18n::error_message(i18n::current(), &error).into());
            }
        }
    }));
}

/// "Forgot the PIN?" leaves the lock screen for setup, where another database can be opened or a
/// new one created; the locked file is left exactly as it is. The guess limiter outlives
/// both screens, so leaving and coming back doesn't reset a wait.
fn wire_forgot_pin(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.on_forgot_pin(with_window(window, move |window| {
        let Some(path) = shared.borrow().config().map(|config| config.database_path.clone()) else {
            log::error!("\"Forgot the PIN?\" with no database chosen");
            return;
        };
        log::info!("left the lock screen to choose another database");
        view::show_setup(window, &SetupReason::LeftLocked(path));
    }));
}

/// "Back to the lock screen" returns from setup to the lock screen, the database still locked.
fn wire_back_to_lock(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.on_back_to_lock(with_window(window, move |window| {
        if !shared.borrow().is_locked() {
            log::error!("back to the lock screen, but no database is locked");
            return;
        }
        show_locked(window);
        window.set_error_message(SharedString::new());
        window.set_setup_required(false);
    }));
}

/// Turns the PIN on with the new PIN typed twice.
fn wire_set_pin(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.on_set_pin(with_window(window, move |window| {
        let result = shared.borrow_mut().set_pin(&window.get_pin_new(), &window.get_pin_repeat());
        after_pin_change(window, &shared, result, Message::Data(DataMessage::PinTurnedOn));
    }));
}

/// Changes the PIN: the current one, then the new one typed twice.
fn wire_change_pin(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.on_change_pin(with_window(window, move |window| {
        let result =
            shared.borrow_mut().change_pin(&window.get_pin_current(), &window.get_pin_new(), &window.get_pin_repeat());
        after_pin_change(window, &shared, result, Message::Data(DataMessage::PinChanged));
    }));
}

/// Turns the PIN off, given the current one.
fn wire_remove_pin(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.on_remove_pin(with_window(window, move |window| {
        let result = shared.borrow_mut().remove_pin(&window.get_pin_current());
        after_pin_change(window, &shared, result, Message::Data(DataMessage::PinTurnedOff));
    }));
}

/// Removes the last digit typed on the lock screen's keypad.
fn wire_erase_last(window: &AppWindow) {
    window.on_erase_last(|text| {
        let mut text = text.to_string();
        text.pop();
        text.into()
    });
}

fn after_pin_change(window: &AppWindow, state: &SharedState, result: Result<()>, done: Message) {
    clear_pin_fields(window);
    let (protected, locked) = {
        let state = state.borrow();
        (state.is_protected(), state.is_locked())
    };
    window.set_pin_set(protected);
    match result {
        Ok(()) => view::show_status(window, i18n::text(i18n::current(), &done)),
        Err(error) => {
            report_failure!(window, "PIN change", &error);
        }
    }
    // a failed re-key that couldn't reopen the file leaves it closed: ask for the PIN again
    if locked {
        show_locked(window);
    }
}

fn clear_pin_fields(window: &AppWindow) {
    window.set_pin_current(SharedString::new());
    window.set_pin_new(SharedString::new());
    window.set_pin_repeat(SharedString::new());
}
