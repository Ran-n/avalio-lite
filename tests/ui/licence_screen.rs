// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.113846
//+ Revised: 	2026/11/23 13:00:32.113846
// -----------------------------------------------------------------------------

//! The licence screen: the first activation, key errors, and the licence page at start.

use crate::common::licence::{FakeLicence, KEY, Mode};
use crate::common::{Fixture, Ui};
use crate::driver::{click, english, find, press_enter, shows, shows_text, type_into, with_control};
use crate::fixtures::{encrypted_fixture, unlock};
use crate::screens::{licence, licence_calls, licence_text, unlicensed};
use avalio_lite::i18n::{self, DataMessage, Language, LicenceMessage, Message};
use avalio_lite::licence::{Notice, SUPPORT_EMAIL};
use avalio_lite::platform::db;
use clavren_client::Outcome;
use std::fs;

/// A database, on a computer with this licence.
fn with_licence(licence: FakeLicence) -> Ui {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let client = licence.client();
    Ui::open_licensed(fixture, Language::English, licence, client)
}

fn activate(ui: &Ui, key: &str) {
    type_into(ui, "Licence key", key);
    click(ui, "Activate");
}

#[test]
fn an_unlicensed_start_shows_the_licence_page() {
    let ui = unlicensed();
    assert!(!licence(&ui).get_licensed());
    assert!(shows(&ui, "Licence key"));
    assert!(shows_text(&ui, &licence_text(&Notice::AskForKey)));
    assert!(!shows(&ui, "Settings"), "the app itself stays hidden");
}

#[test]
fn an_activated_computer_opens_on_the_sheet() {
    let ui = Ui::with_database();
    assert!(licence(&ui).get_licensed());
    assert!(shows(&ui, "Settings"));
    assert!(!shows(&ui, "Licence key"));
}

#[test]
fn activating_a_valid_key_opens_the_sheet() {
    let ui = unlicensed();
    activate(&ui, KEY);
    assert!(licence(&ui).get_licensed());
    assert!(shows(&ui, "Settings"));
    assert!(licence(&ui).get_key().is_empty(), "the key isn't kept on screen");
}

#[test]
fn enter_in_the_key_field_activates() {
    let ui = unlicensed();
    type_into(&ui, "Licence key", KEY);
    press_enter(&ui);
    assert!(licence(&ui).get_licensed());
}

#[test]
fn an_invalid_key_keeps_the_page_and_the_text() {
    let ui = unlicensed();
    ui.licence.server.set_mode(Mode::Refuse("invalid_key", 404));
    activate(&ui, KEY);
    assert!(!licence(&ui).get_licensed());
    assert!(shows_text(&ui, &licence_text(&Notice::InvalidKey)));
    assert_eq!(licence(&ui).get_key(), KEY, "the key stays, to be corrected");
}

#[test]
fn a_mistyped_key_is_refused_without_going_online() {
    let ui = unlicensed();
    activate(&ui, "BRN-79YN-44XQ");
    assert!(shows_text(&ui, &licence_text(&Notice::InvalidKey)));
    assert_eq!(licence_calls(&ui), 0);
}

#[test]
fn a_key_in_use_elsewhere_names_the_support_email() {
    let ui = unlicensed();
    ui.licence.server.set_mode(Mode::Refuse("already_activated", 409));
    activate(&ui, KEY);
    let notice = licence(&ui).get_notice().to_string();
    assert_eq!(notice, licence_text(&Notice::InUseElsewhere));
    assert!(notice.contains(SUPPORT_EMAIL));
}

#[test]
fn an_unreachable_server_says_so_and_allows_another_try() {
    let ui = unlicensed();
    ui.licence.server.set_mode(Mode::Offline);
    activate(&ui, KEY);
    assert!(shows_text(&ui, &licence_text(&Notice::Unreachable)));
    assert!(!licence(&ui).get_busy());
    assert_eq!(find(&ui, "Activate").accessible_enabled(), Some(true));

    ui.licence.server.set_mode(Mode::Normal);
    click(&ui, "Activate");
    assert!(licence(&ui).get_licensed());
}

#[test]
fn too_many_attempts_say_how_long_to_wait() {
    let ui = unlicensed();
    ui.licence.server.set_mode(Mode::Refuse("rate_limited", 429));
    activate(&ui, KEY);
    assert!(shows_text(&ui, &licence_text(&Notice::Wait { minutes: 2 })));
}

#[test]
fn activate_is_disabled_while_the_key_is_empty() {
    let ui = unlicensed();
    assert_eq!(find(&ui, "Activate").accessible_enabled(), Some(false));
    click(&ui, "Activate");
    assert_eq!(licence_calls(&ui), 0);
    type_into(&ui, "Licence key", "B");
    assert_eq!(find(&ui, "Activate").accessible_enabled(), Some(true));
}

#[test]
fn a_licence_that_cannot_be_saved_says_so() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let computer = FakeLicence::new();
    let client = computer.failing_client();
    let ui = Ui::open_licensed(fixture, Language::English, computer, client);
    activate(&ui, KEY);
    assert!(!licence(&ui).get_licensed());
    assert_eq!(
        licence(&ui).get_notice(),
        english(&Message::Licence(LicenceMessage::NotSaved)),
        "the system's reason stays in the log"
    );
}

#[test]
fn a_revoked_licence_at_start_asks_for_a_new_key() {
    let computer = FakeLicence::activated();
    computer.server.set_mode(Mode::Refuse("licence_revoked", 403));
    assert_eq!(computer.client().check().unwrap(), Outcome::Revoked);
    computer.server.set_mode(Mode::Normal);

    let ui = with_licence(computer);
    assert!(!licence(&ui).get_licensed());
    assert!(shows_text(&ui, &licence_text(&Notice::Revoked)));
    activate(&ui, KEY);
    assert!(licence(&ui).get_licensed(), "a new key activates");
}

#[test]
fn thirty_days_offline_offers_to_check_again_and_a_check_opens_the_sheet() {
    let computer = FakeLicence::activated();
    computer.advance(chrono::TimeDelta::days(31));
    let ui = with_licence(computer);
    assert!(shows_text(&ui, &licence_text(&Notice::NeedsInternet)));
    assert!(!shows(&ui, "Licence key"), "this computer has its activation; it needs a check, not a key");

    ui.licence.server.set_mode(Mode::Offline);
    click(&ui, "Check again");
    assert!(shows_text(&ui, &licence_text(&Notice::Unreachable)));
    assert!(!shows(&ui, "Licence key"));

    ui.licence.server.set_mode(Mode::Normal);
    click(&ui, "Check again");
    assert!(licence(&ui).get_licensed());
}

#[test]
fn an_ended_licence_says_so_and_offers_to_check_again() {
    let computer = FakeLicence::activated();
    computer.server.set_mode(Mode::Refuse("licence_expired", 403));
    assert!(matches!(computer.client().check().unwrap(), Outcome::Expired { .. }));
    let ui = with_licence(computer);
    let notice = licence(&ui).get_notice().to_string();
    assert!(notice.starts_with("Your licence") && notice.contains(SUPPORT_EMAIL), "{notice}");
    assert!(shows(&ui, "Check again"));
}

#[test]
fn back_up_my_data_saves_a_backup_from_the_licence_page() {
    let ui = unlicensed();
    let destination = ui.fixture.path("backups").join("mine.avalio");
    fs::create_dir_all(destination.parent().unwrap()).unwrap();
    ui.dialogs.save_file_next(Some(&destination));
    click(&ui, "Back up my data");
    assert!(db::open_existing(&destination, None).is_ok(), "a backup that opens");
    let path = destination.display().to_string();
    let saved = english(&Message::Data(DataMessage::BackupExported { path: &path }));
    assert_eq!(ui.session.window.get_status_message(), saved);
    assert!(shows_text(&ui, &saved), "said on the licence page");

    activate(&ui, KEY);
    assert!(ui.session.window.get_status_message().is_empty(), "the sheet starts without it");
}

#[test]
fn a_pin_database_asks_for_the_pin_before_the_licence() {
    let ui = Ui::open_unlicensed(encrypted_fixture());
    assert!(ui.session.window.get_locked());
    assert!(!shows(&ui, "Licence key"));
    unlock(&ui);
    assert!(shows(&ui, "Licence key"));

    with_control(&ui, "l");
    assert!(ui.session.window.get_locked(), "Ctrl+L still locks from the licence page");
}

#[test]
fn the_licence_page_is_in_galician() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let computer = FakeLicence::new();
    let client = computer.client();
    let ui = Ui::open_licensed(fixture, Language::Galician, computer, client);
    assert!(shows_text(&ui, &crate::common::galician("Enter your licence key")));
    assert!(shows(&ui, &crate::common::galician("Activate")));
    assert!(shows(&ui, &crate::common::galician("Back up my data")));
    assert_eq!(licence(&ui).get_notice(), i18n::licence_notice(Language::Galician, &Notice::AskForKey));
}
