// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:12:05.804233
//+ Revised: 	2026/11/25 11:43:09.356610
// -----------------------------------------------------------------------------

//! Wires the pupil view: the list of pupils, one pupil's grades over the whole window, and the PIN
//! between them. With a PIN, showing a pupil (picked or next) and leaving for the sheet ask for it;
//! going back to the list never does. What each page holds is in `pages`, what a pupil may see in
//! the parent. Every callback first checks the stage it belongs to, so nothing skips the PIN.

mod pages;

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use slint::{ComponentHandle, SharedString};

use crate::gradebook;
use crate::i18n::{self, Message, SheetMessage};
use crate::lock::AttemptLimiter;
use crate::ui::{AppWindow, PupilView, PupilViewStage, Update};
use crate::window::{SharedState, report_failure, view, with_window, with_window1};

/// `PupilView.wanted-pupil` while the PIN is for leaving.
const LEAVING: i32 = -1;

/// Wires the pupil view. `limiter` is the lock screen's, so both count the same wrong PINs.
pub fn wire(window: &AppWindow, state: &SharedState, limiter: &Rc<RefCell<AttemptLimiter>>) {
    let pupil_view = window.global::<PupilView>();
    let shared = state.clone();
    pupil_view.on_open(with_window(window, move |window| open(window, &shared)));
    let shared = state.clone();
    pupil_view.on_show_pupil(with_window1(window, move |window, pupil_id| {
        if is_at(window, PupilViewStage::Menu) {
            show_pupil_after_pin(window, &shared, pupil_id);
        }
    }));
    let shared = state.clone();
    pupil_view.on_next(with_window(window, move |window| next(window, &shared)));
    let shared = state.clone();
    pupil_view.on_back_to_list(with_window(window, move |window| {
        if is_at(window, PupilViewStage::Grades) || is_at(window, PupilViewStage::Pin) {
            show_menu(window, &shared);
        }
    }));
    let (shared, limiter) = (state.clone(), limiter.clone());
    pupil_view.on_unlock(with_window1(window, move |window, pin: SharedString| {
        unlock(window, &shared, &limiter, &pin);
    }));
    let shared = state.clone();
    pupil_view.on_leave(with_window(window, move |window| {
        if is_at(window, PupilViewStage::Menu) {
            leave_after_pin(window, &shared);
        }
    }));
}

/// Puts the pupil view away and forgets what it showed: leaving it, and locking the app.
pub fn end(window: &AppWindow) {
    pages::forget(window);
    if is_at(window, PupilViewStage::Off) {
        return;
    }
    view::clear_messages(window);
    go_to(window, PupilViewStage::Off);
    log::info!("pupil view closed");
}

fn is_at(window: &AppWindow, stage: PupilViewStage) -> bool {
    window.global::<PupilView>().get_stage() == stage
}

fn go_to(window: &AppWindow, stage: PupilViewStage) {
    window.global::<PupilView>().set_stage(stage);
}

/// Says `message` where errors are.
fn say(window: &AppWindow, message: SheetMessage) {
    view::show_error_text(window, i18n::text(i18n::current(), &Message::Sheet(message)));
}

/// Covers the sheet with the list of pupils; no PIN, since the teacher is at the sheet. Not while an
/// update downloads: it ends in a restart, which without a PIN would open the sheet in front of the
/// pupil.
fn open(window: &AppWindow, state: &SharedState) {
    if !is_at(window, PupilViewStage::Off) {
        return;
    }
    if window.global::<Update>().get_downloading() {
        say(window, SheetMessage::PupilViewWhileUpdating);
        return;
    }
    show_menu(window, state);
    log::info!("pupil view opened");
}

/// The list, with the class's pupils as they are now and nothing left of the last one. Pupils that
/// can't be read are said on it.
fn show_menu(window: &AppWindow, state: &SharedState) {
    view::clear_messages(window);
    pages::forget(window);
    go_to(window, PupilViewStage::Menu);
    if let Err(error) = pages::fill_menu(window, state) {
        report_failure!(window, "listing the pupils", &error);
    }
}

/// Asks for the PIN, or with none set goes straight to the pupil.
fn show_pupil_after_pin(window: &AppWindow, state: &SharedState, pupil_id: i32) {
    if !state.borrow().is_protected() {
        show_pupil(window, state, pupil_id);
        return;
    }
    match pages::full_name(window, state, pupil_id) {
        Ok(Some(name)) => ask_for_pin(window, pupil_id, &name),
        Ok(None) => pupil_is_gone(window, state),
        Err(error) => report_failure!(window, "showing a pupil's grades", &error),
    }
}

/// The pupil after the one on screen, by number. With none left (the list got shorter meanwhile),
/// the list.
fn next(window: &AppWindow, state: &SharedState) {
    if !is_at(window, PupilViewStage::Grades) {
        return;
    }
    let shown = window.global::<PupilView>().get_pupil().id;
    // first, so a failure is said on the list, never over the pupil's grades
    show_menu(window, state);
    match pages::next_pupil(window, state, shown) {
        Ok(Some(pupil_id)) => show_pupil_after_pin(window, state, pupil_id),
        Ok(None) => log::info!("no pupil after the one shown"),
        Err(error) => report_failure!(window, "showing the next pupil", &error),
    }
}

fn leave_after_pin(window: &AppWindow, state: &SharedState) {
    if state.borrow().is_protected() {
        ask_for_pin(window, LEAVING, "");
    } else {
        end(window);
    }
}

/// Nothing but the PIN on screen, and the name of the pupil it is for.
fn ask_for_pin(window: &AppWindow, wanted: i32, name: &str) {
    view::clear_messages(window);
    pages::forget(window);
    let pupil_view = window.global::<PupilView>();
    pupil_view.set_wanted_pupil(wanted);
    pupil_view.set_wanted_name(name.into());
    go_to(window, PupilViewStage::Pin);
}

/// Tries the PIN; the right one does what it was asked for.
fn unlock(window: &AppWindow, state: &SharedState, limiter: &RefCell<AttemptLimiter>, pin: &str) {
    if !is_at(window, PupilViewStage::Pin) {
        return;
    }
    let outcome = limiter.borrow_mut().attempt(Instant::now(), || state.borrow().check_current_pin(pin));
    let pupil_view = window.global::<PupilView>();
    pupil_view.set_pin(SharedString::new());
    match outcome {
        Ok(()) if pupil_view.get_wanted_pupil() == LEAVING => end(window),
        Ok(()) => show_pupil(window, state, pupil_view.get_wanted_pupil()),
        Err(error) => pupil_view.set_pin_error(i18n::error_message(i18n::current(), &error).into()),
    }
}

/// Shows the pupil's grades, read afresh. A failure is said on the list.
fn show_pupil(window: &AppWindow, state: &SharedState, pupil_id: i32) {
    let loaded = pages::load_grades(window, state, pupil_id);
    show_menu(window, state);
    match loaded {
        Ok(Some(shown)) => {
            pages::fill_grades(window, shown);
            go_to(window, PupilViewStage::Grades);
        }
        Ok(None) => pupil_is_gone(window, state),
        Err(error) => report_failure!(window, "showing a pupil's grades", &error),
    }
}

/// The pupil picked isn't in the class any more (another copy of the app removed them): the sheet
/// and the list catch up, and it is said.
fn pupil_is_gone(window: &AppWindow, state: &SharedState) {
    log::warn!("the pupil picked for the pupil view is no longer in the class");
    if gradebook::ui::redraw_selecting(window, state, None) {
        show_menu(window, state);
        say(window, SheetMessage::PupilNotInClass);
    }
}
