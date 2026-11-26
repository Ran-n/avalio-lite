// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:51:23.264699
//+ Revised: 	2026/11/25 11:43:19.088312
// -----------------------------------------------------------------------------

//! Typing the PIN in the pupil view: what shows meanwhile, and wrong PINs.

use crate::driver::{PIN, click, error_text, press_enter, type_text, with_control};
use crate::fixtures::type_pin_on_keypad;
use crate::pupil_screens::{
    assert_nothing_on_screen_has, grades, open, open_for, pupil_view, sample_with_pin, shown_name, stage, unlock_with,
};
use avalio_lite::error::AppError;
use avalio_lite::ui::PupilViewStage;

#[test]
fn nothing_but_the_wanted_name_is_on_screen_while_the_pin_is_typed() {
    let ui = sample_with_pin();
    open_for(&ui, "Pérez, Ana");
    click(&ui, "Back to the list");
    click(&ui, "Souto, Daniel");
    assert_nothing_on_screen_has(&ui, &["Ana", "Pérez", "Otero", "Exam 1", "Back to the sheet", "Forgot the PIN?"]);
    assert!(grades(&ui).is_empty());
}

#[test]
fn the_pin_can_be_typed_and_entered() {
    let ui = sample_with_pin();
    open(&ui);
    click(&ui, "Pérez, Ana");
    type_text(&ui, PIN);
    assert_eq!(pupil_view(&ui).get_pin(), PIN);
    press_enter(&ui);
    assert_eq!(shown_name(&ui), "Ana Pérez");
    assert!(pupil_view(&ui).get_pin().is_empty(), "the PIN isn't kept");
}

#[test]
fn a_wrong_pin_shows_nothing_and_is_cleared() {
    let ui = sample_with_pin();
    open(&ui);
    click(&ui, "Pérez, Ana");
    type_pin_on_keypad(&ui, "1112");
    click(&ui, "Erase");
    assert_eq!(pupil_view(&ui).get_pin(), "111");
    unlock_with(&ui, "1");
    assert_eq!(stage(&ui), PupilViewStage::Pin);
    assert_eq!(pupil_view(&ui).get_pin_error(), error_text(&AppError::WrongPin));
    assert!(pupil_view(&ui).get_pin().is_empty() && grades(&ui).is_empty());
    click(&ui, "Erase");
    assert_eq!(pupil_view(&ui).get_pin(), "", "erasing an empty PIN does nothing");
    unlock_with(&ui, PIN);
    assert_eq!(stage(&ui), PupilViewStage::Grades);
    assert!(pupil_view(&ui).get_pin_error().is_empty());
}

/// Guesses here and on the lock screen count together, so one can't be used to go on at the other,
/// and going back to the list doesn't start the count again.
#[test]
fn wrong_pins_also_hold_back_the_lock_screen() {
    let ui = sample_with_pin();
    open(&ui);
    for _ in 0..5 {
        click(&ui, "Pérez, Ana");
        unlock_with(&ui, "1111");
        click(&ui, "Back to the list");
    }
    let waits: Vec<String> = (1..=30).map(|seconds| error_text(&AppError::PinWait { seconds })).collect();
    click(&ui, "Back to the sheet");
    unlock_with(&ui, PIN);
    assert_eq!(stage(&ui), PupilViewStage::Pin, "even the right PIN waits");
    assert!(waits.contains(&pupil_view(&ui).get_pin_error().to_string()));
    ui.session.window.invoke_lock_now();
    unlock_with(&ui, PIN);
    assert!(ui.session.window.get_locked(), "the lock screen waits too");
    assert!(waits.contains(&ui.session.window.get_unlock_error().to_string()));
}

/// A PIN is never hijacked by a shortcut, here as on the lock screen.
#[test]
fn shortcuts_do_nothing_while_the_pin_is_typed() {
    let ui = sample_with_pin();
    open(&ui);
    click(&ui, "Pérez, Ana");
    with_control(&ui, "l");
    assert!(!ui.session.window.get_locked());
    assert_eq!(stage(&ui), PupilViewStage::Pin);
}
