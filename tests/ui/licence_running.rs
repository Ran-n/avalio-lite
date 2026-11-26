// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.027769
//+ Revised: 	2026/11/23 13:00:32.027769
// -----------------------------------------------------------------------------

//! Licence while the app runs: revocation, the warnings before it ends, and Check now.

use crate::common::Ui;
use crate::common::licence::{FakeLicence, Mode};
use crate::driver::{click, count, english, gradebook, shows, shows_text, type_into};
use crate::fixtures::{cell_text, scroll_settings_to};
use crate::screens::{error_on_screen, licence, licence_calls, licence_text, open_licence_settings, seeded_with};
use avalio_lite::i18n::{self, Language, LicenceMessage, Message};
use avalio_lite::licence::{Notice, SUPPORT_EMAIL};
use clavren_client::Outcome;
use clavren_client::clock::Clock;

/// What the daily check would hand the window.
fn deliver(ui: &Ui, outcome: Outcome) {
    avalio_lite::licence::ui::show(&ui.session.window, &ui.licence.client(), &Ok(outcome));
}

fn valid() -> Outcome {
    Outcome::Licensed { licence_expires_at: None, offline_until: chrono::Utc::now() + chrono::TimeDelta::days(30) }
}

#[test]
fn a_revocation_while_open_shows_the_licence_page_and_a_renewal_brings_the_sheet_back() {
    let ui = seeded_with(FakeLicence::activated(), Language::English);
    assert_eq!(gradebook(&ui).get_class_name(), "1A");
    deliver(&ui, Outcome::Revoked);
    assert!(!licence(&ui).get_licensed());
    assert!(shows_text(&ui, &licence_text(&Notice::Revoked)));
    assert!(!shows(&ui, "Settings"));

    deliver(&ui, valid());
    assert!(licence(&ui).get_licensed());
    assert!(shows(&ui, "Settings"));
    assert_eq!(gradebook(&ui).get_class_name(), "1A", "back on the same class");
}

#[test]
fn a_mark_being_typed_is_kept_when_the_licence_screen_appears() {
    let ui = seeded_with(FakeLicence::activated(), Language::English);
    type_into(&ui, "Ana, Exam", "7");
    deliver(&ui, Outcome::Expired { at: None });
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 1, "the mark was saved as it was typed");
    deliver(&ui, valid());
    assert_eq!(cell_text(&ui, 0, 0), "7");
}

#[test]
fn the_sheet_warns_five_days_before_the_licence_needs_the_internet() {
    let computer = FakeLicence::at(chrono::Utc::now() - chrono::TimeDelta::days(26)).activate();
    let ui = seeded_with(computer, Language::English);
    let status = ui.session.window.get_status_message().to_string();
    assert!(status.starts_with("Connect to the internet by "), "{status}");
    assert!(shows_text(&ui, &status), "on the sheet");
}

#[test]
fn the_sheet_warns_two_weeks_before_the_licence_ends() {
    let now = chrono::Utc::now();
    let computer = FakeLicence::at(now);
    computer.ends_at(Some(now + chrono::TimeDelta::days(10)));
    let ui = seeded_with(computer.activate(), Language::English);
    let status = ui.session.window.get_status_message().to_string();
    assert!(status.starts_with("Your licence ends on ") && status.contains(SUPPORT_EMAIL), "{status}");
}

#[test]
fn a_licence_with_time_to_spare_shows_no_warning() {
    let ui = seeded_with(FakeLicence::at(chrono::Utc::now()).activate(), Language::English);
    assert!(ui.session.window.get_status_message().is_empty());
}

#[test]
fn the_licence_card_shows_permanent_and_until_when_it_works_offline() {
    let computer = FakeLicence::activated();
    let offline = computer.clock.now() + chrono::TimeDelta::days(30);
    let ui = seeded_with(computer, Language::English);
    open_licence_settings(&ui);
    assert!(shows(&ui, "Check now"), "scrolled to the Licence card");
    assert!(shows_text(&ui, "Licensed"));
    assert_eq!(licence(&ui).get_term(), "Permanent");
    let day = i18n::licence_day(offline.with_timezone(&chrono::Local).date_naive());
    assert_eq!(licence(&ui).get_offline_until(), format!("Works offline until {day}"));
}

#[test]
fn the_licence_card_shows_the_day_a_licence_ends() {
    let computer = FakeLicence::new();
    let end = computer.clock.now() + chrono::TimeDelta::days(200);
    computer.ends_at(Some(end));
    let ui = seeded_with(computer.activate(), Language::English);
    let day = i18n::licence_day(end.with_timezone(&chrono::Local).date_naive());
    assert_eq!(licence(&ui).get_term(), format!("Until {day}"));
}

#[test]
fn check_now_confirms_the_licence() {
    let ui = seeded_with(FakeLicence::activated(), Language::English);
    open_licence_settings(&ui);
    let before = licence_calls(&ui);
    click(&ui, "Check now");
    assert!(licence_calls(&ui) > before, "it went online");
    assert_eq!(ui.session.window.get_status_message(), english(&Message::Licence(LicenceMessage::Confirmed)));
    assert!(licence(&ui).get_licensed());
    assert!(!licence(&ui).get_busy());
}

#[test]
fn check_now_without_internet_says_so_and_keeps_the_app_running() {
    let ui = seeded_with(FakeLicence::activated(), Language::English);
    open_licence_settings(&ui);
    ui.licence.server.set_mode(Mode::Offline);
    click(&ui, "Check now");
    assert_eq!(error_on_screen(&ui), licence_text(&Notice::Unreachable));
    assert!(licence(&ui).get_licensed(), "the cached token still holds");
}

#[test]
fn check_now_finding_the_licence_revoked_locks_the_app() {
    let ui = seeded_with(FakeLicence::activated(), Language::English);
    open_licence_settings(&ui);
    ui.licence.server.set_mode(Mode::Refuse("licence_revoked", 403));
    click(&ui, "Check now");
    assert!(!licence(&ui).get_licensed());
    assert!(shows(&ui, "Licence key"), "a revoked activation is cleared, so a new key can go in");
}

#[test]
fn the_licence_card_and_warning_are_in_galician() {
    let computer = FakeLicence::at(chrono::Utc::now() - chrono::TimeDelta::days(26)).activate();
    let ui = seeded_with(computer, Language::Galician);
    let status = ui.session.window.get_status_message().to_string();
    assert!(status.starts_with("Conéctate a internet antes do "), "{status}");
    click(&ui, &crate::common::galician("Settings"));
    scroll_settings_to(&ui, &crate::common::galician("Check now"));
    assert!(shows_text(&ui, &crate::common::galician("Licensed")));
    assert_eq!(licence(&ui).get_term(), "Permanente");
    assert!(licence(&ui).get_offline_until().starts_with("Funciona sen conexión ata o "));
}
