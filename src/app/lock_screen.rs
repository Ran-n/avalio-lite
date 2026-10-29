// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:42.253612
//+ Revised: 	2026/10/29 00:32:01.968554
// -----------------------------------------------------------------------------

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use slint::{ComponentHandle, SharedString};

use crate::app::state::SetupReason;
use crate::app::{SharedState, gradebook, view};
use crate::error::{AppError, Result};
use crate::i18n::{self, Message};
use crate::lock::AttemptLimiter;
use crate::ui::{AppWindow, Gradebook};

pub fn wire(window: &AppWindow, state: &SharedState) {
    wire_locking(window, state);
    wire_leaving(window, state);
    wire_pin_settings(window, state);
    window.on_erase_last(|text| {
        let mut text = text.to_string();
        text.pop();
        text.into()
    });
}

/// Shows the lock screen; the database is already closed (or was never opened).
pub fn show_locked(window: &AppWindow) {
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

fn wire_locking(window: &AppWindow, state: &SharedState) {
    let (weak, shared) = (window.as_weak(), state.clone());
    window.on_lock_now(move || {
        let Some(window) = weak.upgrade() else { return };
        if shared.borrow_mut().lock() {
            show_locked(&window);
        } else {
            let hint = i18n::text(i18n::current(), &Message::LockNeedsPin);
            window.global::<Gradebook>().set_error_message(hint.into());
        }
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    let limiter = Rc::new(RefCell::new(AttemptLimiter::default()));
    window.on_unlock(move |pin| {
        let Some(window) = weak.upgrade() else { return };
        let now = Instant::now();
        let waiting = limiter.borrow().wait_remaining(now);
        let outcome = match waiting {
            Some(wait) => Err(AppError::PinWait { seconds: wait.as_secs().max(1) }),
            None => shared.borrow_mut().unlock(&pin),
        };
        window.set_unlock_pin(SharedString::new());
        match outcome {
            Ok(()) => {
                limiter.borrow_mut().record_success();
                window.set_unlock_error(SharedString::new());
                window.set_locked(false);
                gradebook::show_database(&window, &shared);
            }
            Err(AppError::WrongPin) => {
                limiter.borrow_mut().record_failure(now);
                let error = match limiter.borrow().wait_remaining(now) {
                    Some(wait) => AppError::PinWait { seconds: wait.as_secs().max(1) },
                    None => AppError::WrongPin,
                };
                window.set_unlock_error(i18n::error_message(i18n::current(), &error).into());
            }
            // the file went away while locked (e.g. its drive was unplugged): no PIN can open it, so
            // offer what a restart would, the setup screen saying it is missing
            Err(AppError::NotFound(path)) => {
                log::error!("the locked database is gone: {}", path.display());
                view::show_setup(&window, &SetupReason::DatabaseMissing(path));
            }
            Err(error) => {
                log::error!("unlocking failed: {error}");
                window.set_unlock_error(i18n::error_message(i18n::current(), &error).into());
            }
        }
    });
}

/// "Forgot the PIN?" leaves the lock screen for setup, where another database can be opened or a
/// new one created; the locked file is left exactly as it is. "Back to the lock screen" returns.
/// The guess limiter lives in the unlock callback, so leaving and coming back doesn't reset a wait.
fn wire_leaving(window: &AppWindow, state: &SharedState) {
    let (weak, shared) = (window.as_weak(), state.clone());
    window.on_forgot_pin(move || {
        let Some(window) = weak.upgrade() else { return };
        let Some(path) = shared.borrow().config().map(|config| config.database_path.clone()) else {
            log::error!("\"Forgot the PIN?\" with no database chosen");
            return;
        };
        log::info!("left the lock screen to choose another database");
        view::show_setup(&window, &SetupReason::LeftLocked(path));
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    window.on_back_to_lock(move || {
        let Some(window) = weak.upgrade() else { return };
        if !shared.borrow().is_locked() {
            log::error!("back to the lock screen, but no database is locked");
            return;
        }
        show_locked(&window);
        window.set_error_message(SharedString::new());
        window.set_setup_required(false);
    });
}

fn wire_pin_settings(window: &AppWindow, state: &SharedState) {
    let (weak, shared) = (window.as_weak(), state.clone());
    window.on_set_pin(move || {
        let Some(window) = weak.upgrade() else { return };
        let result = shared.borrow_mut().set_pin(&window.get_pin_new(), &window.get_pin_repeat());
        after_pin_change(&window, &shared, result, Message::PinTurnedOn);
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    window.on_change_pin(move || {
        let Some(window) = weak.upgrade() else { return };
        let result =
            shared.borrow_mut().change_pin(&window.get_pin_current(), &window.get_pin_new(), &window.get_pin_repeat());
        after_pin_change(&window, &shared, result, Message::PinChanged);
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    window.on_remove_pin(move || {
        let Some(window) = weak.upgrade() else { return };
        let result = shared.borrow_mut().remove_pin(&window.get_pin_current());
        after_pin_change(&window, &shared, result, Message::PinTurnedOff);
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
            log::warn!("PIN change failed: {error}");
            view::show_error(window, &error);
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
