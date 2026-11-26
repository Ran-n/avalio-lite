// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.465518
//+ Revised: 	2026/11/23 13:00:32.465518
// -----------------------------------------------------------------------------

//! Messages on the page: errors and notices shown on the screen that is open.

use crate::common::{Fixture, Ui};
use crate::driver::{click, english, with_control};
use crate::fixtures::{open_settings, seeded, with_shortcut_config};
use crate::screens::{break_saving_the_settings, error_on_screen};
use avalio_lite::i18n::{DataMessage, Message};

/// The sheet showed `Gradebook.error-message` and Settings the window's, so a failure reported
/// from the other page (the theme button in the sidebar, Ctrl+L) was set but never seen.
#[test]
fn regression_a_theme_failure_from_the_sheet_shows_on_the_sheet() {
    let ui = seeded();
    break_saving_the_settings(&ui);
    click(&ui, "Dark mode");
    error_on_screen(&ui);
}

#[test]
fn regression_the_lock_hint_shows_on_the_sheet_and_on_settings() {
    let ui = seeded();
    with_control(&ui, "l");
    assert_eq!(error_on_screen(&ui), english(&Message::Data(DataMessage::LockNeedsPin)));
    open_settings(&ui);
    assert!(ui.session.window.get_error_message().is_empty(), "leaving the sheet drops its message");
    with_control(&ui, "l");
    assert_eq!(error_on_screen(&ui), english(&Message::Data(DataMessage::LockNeedsPin)));
}

#[test]
fn regression_messages_are_dropped_when_the_page_changes() {
    let ui = seeded();
    break_saving_the_settings(&ui);
    click(&ui, "Dark mode");
    error_on_screen(&ui);
    open_settings(&ui);
    assert!(ui.session.window.get_error_message().is_empty());
    click(&ui, "Dark");
    error_on_screen(&ui);
    click(&ui, "1A");
    assert!(ui.session.window.get_error_message().is_empty());
    assert!(ui.session.window.get_status_message().is_empty());
}

#[test]
fn regression_ignored_shortcut_entries_show_on_screen() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    with_shortcut_config(&fixture, &[("lock", "Ctrl+")]);
    let ui = Ui::open(fixture);
    assert!(error_on_screen(&ui).contains("lock = \"Ctrl+\""));
}
