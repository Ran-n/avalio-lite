// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.666070
//+ Revised: 	2026/11/23 13:00:31.666070
// -----------------------------------------------------------------------------

//! Fields from the keyboard: Enter in the PIN and name fields, and refused values put back.

use crate::driver::{
    PIN, click, click_element, english, error_text, gradebook, press_enter, replace_text, shows_text, type_into,
};
use crate::fixtures::{name_fields, open_pin_settings, open_settings, seeded, with_second_class};
use crate::screens::{error_on_screen, field_text};
use avalio_lite::error::AppError;
use avalio_lite::i18n::{DataMessage, Message};
use slint::Model;

#[test]
fn regression_enter_in_the_repeat_pin_field_sets_the_pin() {
    let ui = seeded();
    open_pin_settings(&ui);
    type_into(&ui, "PIN", PIN);
    type_into(&ui, "Repeat PIN", PIN);
    press_enter(&ui);
    assert!(ui.session.window.get_pin_set(), "{}", ui.session.window.get_error_message());
    assert_eq!(ui.session.window.get_status_message(), english(&Message::Data(DataMessage::PinTurnedOn)));
}

/// Saving as you type leaves a refused value (empty, a taken name) in the field; the field kept
/// it when left, out of step with what was saved.
#[test]
fn regression_a_refused_class_name_is_put_back_when_the_field_is_left() {
    let ui = with_second_class();
    click(&ui, "Class name");
    replace_text(&ui, "2B");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::DuplicateClassName("2B".into())));
    assert_eq!(field_text(&ui, "Class name"), "2B", "the refused text stays while typing");
    click(&ui, "First name…");
    // "2" was saved on the way to "2B" (saving as you type), and that is what the field goes back to
    assert_eq!(field_text(&ui, "Class name"), "2");
    assert_eq!(gradebook(&ui).get_class_name(), "2");
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn regression_enter_puts_back_a_refused_pupil_name() {
    let ui = seeded();
    let ana = || name_fields(&ui, "First name").into_iter().find(|e| e.accessible_value().as_deref() == Some("Ana"));
    click_element(&ana().unwrap());
    replace_text(&ui, "");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyName));
    press_enter(&ui);
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(gradebook(&ui).get_rows().row_data(0).unwrap().full_name, "Ana");
    assert!(ana().is_some(), "the field shows the saved name again");
}

#[test]
fn regression_a_refused_column_value_is_put_back_when_the_panel_field_is_left() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "lots");
    error_on_screen(&ui);
    assert_eq!(field_text(&ui, "Weight"), "lots");
    type_into(&ui, "Out of", "10");
    assert_eq!(field_text(&ui, "Weight"), "100");
    assert!(ui.session.window.get_error_message().is_empty());
    type_into(&ui, "Title", "");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyName));
    press_enter(&ui);
    assert_eq!(field_text(&ui, "Title"), "Exam");
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn a_value_that_was_saved_is_kept_when_the_field_is_left() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Title", "Quiz");
    type_into(&ui, "Weight", "60");
    assert_eq!(field_text(&ui, "Title"), "Quiz");
    assert_eq!(field_text(&ui, "Weight"), "60");
}

#[test]
fn regression_a_refused_new_class_or_pupil_stays_in_its_field() {
    let ui = seeded();
    type_into(&ui, "New class", "1A");
    press_enter(&ui);
    error_on_screen(&ui);
    assert_eq!(field_text(&ui, "New class"), "1A", "a refused name is not lost");
    replace_text(&ui, "3C");
    press_enter(&ui);
    assert_eq!(field_text(&ui, "New class"), "", "an accepted one empties the field");

    click(&ui, "1A");
    type_into(&ui, "First name…", "   ");
    press_enter(&ui);
    press_enter(&ui);
    error_on_screen(&ui);
    assert_eq!(field_text(&ui, "First name…"), "   ");
    replace_text(&ui, "Carla");
    press_enter(&ui);
    press_enter(&ui);
    assert_eq!(field_text(&ui, "First name…"), "");
}

#[test]
fn regression_the_weight_tag_follows_each_languages_percent_style() {
    let ui = seeded();
    assert!(shows_text(&ui, "100%"));
    open_settings(&ui);
    click(&ui, "Galego");
    click(&ui, "1A");
    assert!(shows_text(&ui, "100 %"));
    assert!(!shows_text(&ui, "100%"));
}
