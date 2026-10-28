// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:42.253612
//+ Revised: 	2026/10/28 15:53:08.589167
// -----------------------------------------------------------------------------

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use slint::{ComponentHandle, SharedString};

use crate::app::{SharedState, gradebook, view};
use crate::error::{AppError, Result};
use crate::i18n::{self, Message};
use crate::lock::AttemptLimiter;
use crate::ui::{AppWindow, Gradebook};

pub fn wire(window: &AppWindow, state: &SharedState) {
    wire_locking(window, state);
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
            Err(error) => {
                log::error!("unlocking failed: {error}");
                window.set_unlock_error(i18n::error_message(i18n::current(), &error).into());
            }
        }
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
