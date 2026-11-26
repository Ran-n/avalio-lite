// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.281265
//+ Revised: 	2026/11/23 13:00:32.281265
// -----------------------------------------------------------------------------

//! The lock screen (keypad, wrong PINs, a forgotten PIN), Settings › PIN, and the Ctrl+L
//! lock regressions.

use crate::common::{Fixture, Ui};
use crate::driver::{PIN, click, english, error_text, press_enter, shows, type_into, type_text, with_control};
use crate::fixtures::{encrypted, encrypted_fixture, open_pin_settings, seeded, type_pin_on_keypad, unlock};
use avalio_lite::error::AppError;
use avalio_lite::i18n::{DataMessage, Message};
use avalio_lite::platform::config::DATABASE_FILE_NAME;
use avalio_lite::platform::db;
use avalio_lite::ui::SetupNotice;
use std::fs;

// ---------------------------------------------------------------------------------------------
// Lock screen

#[test]
fn the_keypad_unlocks_with_the_right_pin() {
    let ui = encrypted();
    type_pin_on_keypad(&ui, PIN);
    assert_eq!(ui.session.window.get_unlock_pin(), PIN);
    click(&ui, "Unlock");
    assert!(!ui.session.window.get_locked());
    assert!(ui.session.state.borrow().connection().is_some());
    assert!(ui.session.window.get_unlock_pin().is_empty(), "the PIN isn't kept in the field");
}

#[test]
fn typing_the_pin_and_enter_unlocks() {
    let ui = encrypted();
    type_text(&ui, PIN);
    press_enter(&ui);
    assert!(!ui.session.window.get_locked());
}

#[test]
fn erase_removes_the_last_digit() {
    let ui = encrypted();
    type_pin_on_keypad(&ui, "123");
    click(&ui, "Erase");
    assert_eq!(ui.session.window.get_unlock_pin(), "12");
    for _ in 0..3 {
        click(&ui, "Erase");
    }
    assert_eq!(ui.session.window.get_unlock_pin(), "", "erasing an empty PIN does nothing");
}

#[test]
fn a_wrong_pin_is_reported_and_keeps_the_lock() {
    let ui = encrypted();
    type_pin_on_keypad(&ui, "1111");
    click(&ui, "Unlock");
    assert!(ui.session.window.get_locked());
    assert_eq!(ui.session.window.get_unlock_error(), error_text(&AppError::WrongPin));
    assert!(ui.session.window.get_unlock_pin().is_empty());
}

/// The leave and return callbacks only act when they apply: no database chosen, or none locked.
#[test]
fn leaving_and_returning_to_the_lock_screen_need_a_locked_database() {
    let ui = Ui::open(Fixture::new());
    ui.session.window.invoke_forgot_pin();
    assert_eq!(ui.session.window.get_setup_notice(), SetupNotice::FirstRun, "still the first-run screen");
    drop(ui);
    let ui = seeded();
    ui.session.window.invoke_back_to_lock();
    assert!(!ui.session.window.get_locked(), "nothing is locked, so there is nothing to go back to");
}

/// Was: the lock screen said the file doesn't exist on every try, with no way off it.
#[test]
fn regression_a_database_gone_while_locked_asks_where_it_went() {
    let ui = encrypted();
    fs::remove_file(ui.fixture.database()).unwrap();
    type_pin_on_keypad(&ui, PIN);
    click(&ui, "Unlock");
    let window = &ui.session.window;
    assert!(window.get_setup_required(), "the setup screen replaces the lock screen");
    assert_eq!(window.get_setup_notice(), SetupNotice::DatabaseMissing);
    assert_eq!(window.get_setup_notice_detail(), ui.fixture.database().display().to_string());
    assert!(!ui.fixture.database().exists(), "no empty database was made in its place");
    assert!(shows(&ui, "Create database"));
}

#[test]
fn forgot_the_pin_leaves_the_lock_screen_for_setup_and_can_come_back() {
    let ui = encrypted();
    click(&ui, "Forgot the PIN?");
    let window = &ui.session.window;
    assert!(window.get_setup_required());
    assert_eq!(window.get_setup_notice(), SetupNotice::LeftLocked);
    assert_eq!(window.get_setup_notice_detail(), ui.fixture.database().display().to_string());
    assert!(db::is_encrypted(&ui.fixture.database()).unwrap(), "the locked file is left as it is");
    click(&ui, "Back to the lock screen");
    assert!(!window.get_setup_required());
    assert!(window.get_locked());
    unlock(&ui);
}

#[test]
fn after_forgetting_the_pin_a_new_database_can_be_started_elsewhere() {
    let ui = encrypted();
    click(&ui, "Forgot the PIN?");
    let window = &ui.session.window;
    window.set_setup_database_folder(ui.fixture.path("fresh").display().to_string().into());
    window.set_setup_log_folder(ui.fixture.path("logs").display().to_string().into());
    click(&ui, "Create database");
    assert!(!window.get_setup_required());
    assert!(!window.get_locked(), "the new database has no PIN");
    assert!(!window.get_pin_set());
    let saved = ui.fixture.store().load().unwrap().unwrap();
    assert_eq!(saved.database_path, ui.fixture.path("fresh").join(DATABASE_FILE_NAME));
    assert!(db::is_encrypted(&ui.fixture.database()).unwrap(), "the old file is still there, still encrypted");
}

#[test]
fn back_on_the_lock_screen_does_not_reset_the_wait() {
    let ui = encrypted();
    for _ in 0..5 {
        type_pin_on_keypad(&ui, "1111");
        click(&ui, "Unlock");
    }
    click(&ui, "Forgot the PIN?");
    click(&ui, "Back to the lock screen");
    type_pin_on_keypad(&ui, PIN);
    click(&ui, "Unlock");
    assert!(ui.session.window.get_locked(), "still waiting");
    let error = ui.session.window.get_unlock_error().to_string();
    let waits: Vec<String> = (1..=30).map(|seconds| error_text(&AppError::PinWait { seconds })).collect();
    assert!(waits.contains(&error), "the wait goes on: {error}");
}

#[test]
fn after_five_wrong_pins_even_the_right_one_waits() {
    let ui = encrypted();
    for _ in 0..5 {
        type_pin_on_keypad(&ui, "1111");
        click(&ui, "Unlock");
    }
    let error = ui.session.window.get_unlock_error().to_string();
    assert_eq!(error, error_text(&AppError::PinWait { seconds: 30 }), "the fifth miss starts a wait");
    type_pin_on_keypad(&ui, PIN);
    click(&ui, "Unlock");
    assert!(ui.session.window.get_locked(), "no tries while waiting");
}

#[test]
fn the_sidebar_lock_and_ctrl_l_lock_an_encrypted_database() {
    let ui = encrypted();
    unlock(&ui);
    click(&ui, "Lock (Ctrl+L)");
    assert!(ui.session.window.get_locked());
    assert!(ui.session.state.borrow().connection().is_none());

    unlock(&ui);
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked(), "Ctrl+L locks too");
}

#[test]
fn ctrl_l_without_a_pin_explains_how_to_lock() {
    let ui = seeded();
    assert!(!shows(&ui, "Lock (Ctrl+L)"), "no lock button without a PIN");
    with_control(&ui, "l");
    assert!(!ui.session.window.get_locked());
    assert_eq!(ui.session.window.get_error_message(), english(&Message::Data(DataMessage::LockNeedsPin)));
}

// ---------------------------------------------------------------------------------------------
// Settings: PIN

#[test]
fn setting_a_pin_encrypts_the_database() {
    let ui = seeded();
    open_pin_settings(&ui);
    type_into(&ui, "PIN", PIN);
    type_into(&ui, "Repeat PIN", PIN);
    click(&ui, "Encrypt with this PIN");
    assert!(ui.session.window.get_pin_set());
    assert_eq!(ui.session.window.get_status_message(), english(&Message::Data(DataMessage::PinTurnedOn)));
    assert!(ui.session.window.get_pin_new().is_empty(), "the fields are cleared");
    assert!(ui.session.state.borrow().is_protected());
    assert!(shows(&ui, "Lock (Ctrl+L)"));
}

#[test]
fn a_password_with_letters_and_spaces_locks_and_unlocks() {
    const PASSWORD: &str = "corvo azul 7";
    let ui = seeded();
    open_pin_settings(&ui);
    type_into(&ui, "PIN", PASSWORD);
    type_into(&ui, "Repeat PIN", PASSWORD);
    click(&ui, "Encrypt with this PIN");
    assert!(ui.session.window.get_pin_set(), "{}", ui.session.window.get_error_message());
    click(&ui, "Lock (Ctrl+L)");
    assert!(ui.session.window.get_locked());
    type_text(&ui, "corvo azul 8");
    press_enter(&ui);
    assert!(ui.session.window.get_locked(), "a near miss doesn't open it");
    type_text(&ui, PASSWORD);
    press_enter(&ui);
    assert!(!ui.session.window.get_locked());
    assert!(ui.session.state.borrow().connection().is_some());
}

#[test]
fn a_password_can_be_clicked_on_the_letter_keys() {
    const PASSWORD: &str = "Ñu 7xa";
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin(PASSWORD, PASSWORD).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    click(&ui, "Letters");
    assert!(!shows(&ui, "1"), "the letter keys replace the number keys");
    click(&ui, "Capital letters");
    click(&ui, "Ñ");
    click(&ui, "Capital letters");
    click(&ui, "u");
    click(&ui, "Space");
    click(&ui, "Numbers");
    click(&ui, "7");
    click(&ui, "Letters");
    for letter in ["x", "q", "Erase", "a"] {
        click(&ui, letter);
    }
    assert_eq!(ui.session.window.get_unlock_pin(), PASSWORD);
    click(&ui, "Unlock");
    assert!(!ui.session.window.get_locked());
    assert!(ui.session.state.borrow().connection().is_some());
}

#[test]
fn a_password_with_symbols_can_be_clicked_on_the_symbol_keys() {
    const PASSWORD: &str = "a#\\€¿b";
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin(PASSWORD, PASSWORD).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    click(&ui, "Letters");
    click(&ui, "a");
    click(&ui, "Symbols");
    assert!(!shows(&ui, "q"), "the symbol keys replace the letter keys");
    click(&ui, "#");
    click(&ui, "\\");
    click(&ui, "More symbols");
    assert!(!shows(&ui, "#"), "the second page of symbols");
    click(&ui, "€");
    click(&ui, "¿");
    click(&ui, "Letters");
    click(&ui, "b");
    assert_eq!(ui.session.window.get_unlock_pin(), PASSWORD);
    click(&ui, "Unlock");
    assert!(!ui.session.window.get_locked());
}

#[test]
fn the_lock_screen_opens_on_the_number_keys_again() {
    let ui = encrypted();
    click(&ui, "Letters");
    assert!(shows(&ui, "q"));
    type_text(&ui, PIN);
    press_enter(&ui);
    assert!(!ui.session.window.get_locked(), "the keyboard still types into the PIN field");
    click(&ui, "Lock (Ctrl+L)");
    assert!(shows(&ui, "1") && !shows(&ui, "q"), "a new lock screen starts on the numbers");
}

#[test]
fn mismatched_pins_are_refused() {
    let ui = seeded();
    open_pin_settings(&ui);
    type_into(&ui, "PIN", PIN);
    type_into(&ui, "Repeat PIN", "1357");
    click(&ui, "Encrypt with this PIN");
    assert!(!ui.session.window.get_pin_set());
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::PinMismatch));
    assert!(!ui.session.state.borrow().is_protected());
}

#[test]
fn the_pin_can_be_changed_and_removed() {
    let ui = encrypted();
    unlock(&ui);
    open_pin_settings(&ui);

    type_into(&ui, "Current PIN", "0000");
    type_into(&ui, "New PIN", "13579");
    type_into(&ui, "Repeat PIN", "13579");
    click(&ui, "Change PIN");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::WrongPin));

    type_into(&ui, "Current PIN", PIN);
    type_into(&ui, "New PIN", "13579");
    type_into(&ui, "Repeat PIN", "13579");
    click(&ui, "Change PIN");
    assert_eq!(ui.session.window.get_status_message(), english(&Message::Data(DataMessage::PinChanged)));

    type_into(&ui, "Current PIN", "13579");
    click(&ui, "Remove PIN and encryption");
    assert_eq!(ui.session.window.get_status_message(), english(&Message::Data(DataMessage::PinTurnedOff)));
    assert!(!ui.session.window.get_pin_set());
    assert!(!ui.session.state.borrow().is_protected());
}

// ---------------------------------------------------------------------------------------------
// Regressions: each test failed before its fix. Run them alone with `cargo test regression`.

/// Ctrl+L only reaches the shortcut handler while something in the window has focus. After the
/// PIN field that unlocked the app went away, nothing had focus and Ctrl+L did nothing.
#[test]
fn regression_ctrl_l_locks_right_after_unlocking() {
    let ui = encrypted();
    unlock(&ui);
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked());
}

/// Same cause: a database without a PIN opened straight onto the sheet with nothing focused,
/// so Ctrl+L didn't even show the hint about setting a PIN.
#[test]
fn regression_ctrl_l_answers_right_after_startup() {
    let ui = seeded();
    with_control(&ui, "l");
    assert_eq!(ui.session.window.get_error_message(), english(&Message::Data(DataMessage::LockNeedsPin)));
}

/// Same cause: typing in a settings field and then going back to the sheet destroyed the
/// focused field, and Ctrl+L stopped working until something else was clicked.
#[test]
fn regression_ctrl_l_locks_after_leaving_a_focused_settings_field() {
    let fixture = encrypted_fixture();
    let ui = Ui::open(fixture);
    unlock(&ui);
    type_into(&ui, "New class", "1A");
    press_enter(&ui);
    open_pin_settings(&ui);
    type_into(&ui, "Current PIN", "1");
    click(&ui, "1A");
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked());
}
