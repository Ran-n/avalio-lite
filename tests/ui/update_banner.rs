// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.279437
//+ Revised: 	2026/11/24 17:54:11.542294
// -----------------------------------------------------------------------------

//! The update banner and Settings › Updates: what each check result shows.

use crate::common::licence::FakeLicence;
use crate::common::{Fixture, Ui};
use crate::driver::{click, english, shows, shows_text};
use crate::fixtures::{open_settings, saved_preferences, scroll_settings_until, seeded};
use crate::screens::{
    automatic_check, break_saving_the_settings, error_on_screen, next_version, open_update_settings, release,
    seeded_with, update, with_a_worker_that_cannot_start,
};
use avalio_lite::i18n::{self, Language, Message, UpdateMessage};
use avalio_lite::ui::{Update, UpdateBanner};
use avalio_lite::update::release::{CheckError, NetworkError, Release};

fn now_hh_mm() -> String {
    chrono::Local::now().format("%H:%M").to_string()
}

#[test]
fn a_newer_release_shows_the_banner_and_the_settings_line() {
    let ui = seeded();
    let version = next_version();
    ui.releases.set(Ok(release(&version)));
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
    assert_eq!(update(&ui).get_banner_text(), english(&Message::Update(UpdateMessage::Offer { version: &version })));
    assert!(shows_text(&ui, &update(&ui).get_banner_text()));
    assert!(shows(&ui, "Update and restart") && shows(&ui, "Skip this version") && shows(&ui, "Hide"));
    let time = now_hh_mm();
    assert_eq!(
        update(&ui).get_status_line(),
        english(&Message::Update(UpdateMessage::Available { version: &version, time: &time }))
    );
    assert!(update(&ui).get_available());
    assert!(saved_preferences(&ui.fixture).last_check.is_some(), "a successful check is noted");
    assert_eq!((ui.releases.calls(), ui.gitlab.calls()), (1, 1), "both forges were asked");
}

#[test]
fn the_banner_is_offered_from_gitlab_when_github_is_down() {
    let ui = seeded();
    let version = next_version();
    ui.gitlab.set(Ok(release(&version)));
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
}

#[test]
fn up_to_date_and_no_build_show_no_banner_only_their_line() {
    let ui = seeded();
    ui.releases.set(Ok(release(env!("CARGO_PKG_VERSION"))));
    update(&ui).invoke_check_now();
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden);
    assert_eq!(
        update(&ui).get_status_line(),
        english(&Message::Update(UpdateMessage::UpToDate { time: &now_hh_mm() }))
    );
    let version = next_version();
    ui.releases.set(Ok(Release { assets: Vec::new(), ..release(&version) }));
    update(&ui).invoke_check_now();
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden);
    assert_eq!(update(&ui).get_status_line(), english(&Message::Update(UpdateMessage::NoBuild { version: &version })));
    assert!(!update(&ui).get_available());
}

#[test]
fn before_any_check_settings_says_so() {
    let ui = seeded();
    assert_eq!(update(&ui).get_status_line(), english(&Message::Update(UpdateMessage::NotChecked)));
    let version = avalio_lite::update::target::current_version().to_string();
    let line = english(&Message::Update(UpdateMessage::Version { version: &version, target: Some("windows-x86_64") }));
    assert_eq!(update(&ui).get_version_line(), line);
    open_update_settings(&ui);
    assert!(shows_text(&ui, &line));
}

#[test]
fn a_failed_check_by_hand_is_shown_but_an_automatic_one_only_changes_the_line() {
    let ui = seeded();
    ui.releases.set(Err(CheckError::Network(NetworkError::Status(429))));
    automatic_check(&ui);
    assert!(ui.session.window.get_error_message().is_empty(), "only the line says it");
    let reasons = vec![
        ("GitHub".to_owned(), CheckError::Network(NetworkError::Status(429))),
        ("GitLab".to_owned(), CheckError::Network(NetworkError::ConnectionFailed)),
    ];
    let failed = english(&Message::Update(UpdateMessage::CheckFailed { reasons: &reasons }));
    assert_eq!(update(&ui).get_status_line(), failed);
    assert!(failed.contains("GitHub: the server answered 429; GitLab: the connection failed"), "{failed}");
    assert_eq!(saved_preferences(&ui.fixture).last_check, None, "a failed check isn't noted");
    open_update_settings(&ui);
    click(&ui, "Check for updates now");
    assert_eq!(error_on_screen(&ui), failed);
    assert!(!update(&ui).get_checking());
}

#[test]
fn a_failed_check_keeps_an_offer_already_known() {
    let ui = seeded();
    ui.releases.set(Ok(release(&next_version())));
    automatic_check(&ui);
    ui.releases.set(Err(CheckError::Network(NetworkError::TimedOut)));
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
    assert!(update(&ui).get_available());
}

#[test]
fn skipping_a_version_hides_it_until_a_newer_one() {
    let ui = seeded();
    let version = next_version();
    ui.releases.set(Ok(release(&version)));
    automatic_check(&ui);
    click(&ui, "Skip this version");
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden);
    assert_eq!(saved_preferences(&ui.fixture).skipped_version.as_deref(), Some(version.as_str()));
    assert_eq!(update(&ui).get_skipped(), version.as_str());
    let time = now_hh_mm();
    assert_eq!(
        update(&ui).get_status_line(),
        english(&Message::Update(UpdateMessage::Skipped { version: &version, time: &time }))
    );
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden, "the skipped version stays quiet");
    let mut later = semver::Version::parse(&version).unwrap();
    later.patch += 1;
    ui.releases.set(Ok(release(&later.to_string())));
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer, "a newer one is offered");
}

#[test]
fn stop_skipping_offers_the_version_again_at_once() {
    let ui = seeded();
    let version = next_version();
    ui.releases.set(Ok(release(&version)));
    automatic_check(&ui);
    click(&ui, "Skip this version");
    open_update_settings(&ui);
    let label = format!("Stop skipping {version}");
    assert!(shows(&ui, &label));
    let asked = ui.releases.calls();
    click(&ui, &label);
    assert_eq!(ui.releases.calls(), asked + 1, "and checks at once");
    assert_eq!(saved_preferences(&ui.fixture).skipped_version, None);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
    assert!(!shows(&ui, &label));
}

#[test]
fn the_close_button_hides_the_banner_until_the_next_start_or_another_version() {
    let ui = seeded();
    let version = next_version();
    ui.releases.set(Ok(release(&version)));
    automatic_check(&ui);
    click(&ui, "Hide");
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden);
    assert_eq!(saved_preferences(&ui.fixture).skipped_version, None, "not saved");
    assert!(update(&ui).get_available(), "Settings still offers it");
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden, "the same version stays hidden");
    let mut later = semver::Version::parse(&version).unwrap();
    later.patch += 1;
    ui.releases.set(Ok(release(&later.to_string())));
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer, "another version shows again");
    let ui = ui.restart();
    ui.releases.set(Ok(release(&version)));
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer, "and every version after a restart");
}

#[test]
fn a_build_from_source_shows_no_update_controls_and_asks_nobody() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let ui = Ui::open_from_source(fixture);
    ui.releases.set(Ok(release(&next_version())));
    automatic_check(&ui);
    update(&ui).invoke_check_now();
    assert_eq!((ui.releases.calls(), ui.gitlab.calls()), (0, 0));
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden);
    open_settings(&ui);
    let source_line = english(&Message::Update(UpdateMessage::SourceBuild));
    scroll_settings_until(&ui, &source_line, |ui| shows_text(ui, &source_line));
    for control in ["Check for updates now", "Check for updates automatically", "Update and restart"] {
        assert!(!shows(&ui, control), "{control}");
    }
    let version = avalio_lite::update::target::current_version().to_string();
    assert!(shows_text(&ui, &english(&Message::Update(UpdateMessage::Version { version: &version, target: None }))));
}

/// The banner and Settings are worded by the Rust side; a language chosen afterwards rewords them.
#[test]
fn choosing_a_language_rewords_the_banner_and_the_settings_line() {
    let ui = seeded_with(FakeLicence::activated(), Language::English);
    let version = next_version();
    ui.releases.set(Ok(release(&version)));
    automatic_check(&ui);
    ui.session.window.invoke_choose_language("gl".into());
    assert_eq!(
        update(&ui).get_banner_text(),
        i18n::text(Language::Galician, &Message::Update(UpdateMessage::Offer { version: &version }))
    );
    let time = now_hh_mm();
    let line =
        i18n::text(Language::Galician, &Message::Update(UpdateMessage::Available { version: &version, time: &time }));
    assert_eq!(update(&ui).get_status_line(), line);
}

#[test]
fn checking_automatically_is_saved() {
    let ui = seeded();
    open_update_settings(&ui);
    update(&ui).invoke_set_automatic(false);
    assert!(!saved_preferences(&ui.fixture).check_automatically);
    assert!(!update(&ui).get_automatic());
    update(&ui).invoke_set_automatic(true);
    assert!(saved_preferences(&ui.fixture).check_automatically);
}

#[test]
fn an_update_setting_that_cannot_be_saved_says_so_and_stays_as_it_was() {
    let ui = seeded();
    break_saving_the_settings(&ui);
    update(&ui).invoke_set_automatic(false);
    assert!(!ui.session.window.get_error_message().is_empty());
    assert!(update(&ui).get_automatic(), "the toggle shows what is saved");
}

#[test]
fn a_skip_that_cannot_be_saved_says_so_and_keeps_the_banner() {
    let ui = seeded();
    ui.releases.set(Ok(release(&next_version())));
    automatic_check(&ui);
    break_saving_the_settings(&ui);
    click(&ui, "Skip this version");
    assert!(!ui.session.window.get_error_message().is_empty());
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
}

#[test]
fn a_check_whose_time_cannot_be_noted_still_shows_the_new_version() {
    let ui = seeded();
    ui.releases.set(Ok(release(&next_version())));
    break_saving_the_settings(&ui);
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
    assert!(ui.session.window.get_error_message().is_empty(), "that is only logged");
}

#[test]
fn the_update_buttons_do_nothing_with_no_offer() {
    let ui = seeded();
    for invoke in [Update::invoke_install, Update::invoke_skip, Update::invoke_dismiss, Update::invoke_cancel] {
        invoke(&update(&ui));
    }
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(saved_preferences(&ui.fixture).skipped_version, None);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden);
    assert_eq!(ui.session.updater.restart(), None);
}

#[test]
fn a_check_that_cannot_be_started_ends_the_wait_and_says_so() {
    let ui = with_a_worker_that_cannot_start();
    ui.releases.set(Ok(release(&next_version())));
    open_update_settings(&ui);
    click(&ui, "Check for updates now");
    assert_eq!(ui.releases.calls(), 0, "nothing ran");
    assert!(!update(&ui).get_checking());
    assert!(error_on_screen(&ui).starts_with("Couldn't check for updates:"), "{}", error_on_screen(&ui));
}

#[test]
fn a_version_that_cannot_be_unskipped_says_so_and_stays_skipped() {
    let ui = seeded();
    let version = next_version();
    ui.releases.set(Ok(release(&version)));
    automatic_check(&ui);
    click(&ui, "Skip this version");
    let asked = ui.releases.calls();
    break_saving_the_settings(&ui);
    update(&ui).invoke_stop_skipping();
    assert!(!ui.session.window.get_error_message().is_empty());
    assert_eq!(ui.releases.calls(), asked, "no check");
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Hidden);
}
