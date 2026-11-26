// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.368753
//+ Revised: 	2026/11/24 05:03:43.394849
// -----------------------------------------------------------------------------

//! Update and restart: the download, replacing the program, relaunching it and tidying up.

use crate::common::releases::Published;
use crate::common::{Fixture, Ui};
use crate::driver::{click, english};
use crate::fixtures::{saved_preferences, seeded};
use crate::screens::{
    automatic_check, break_saving_the_settings, error_on_screen, next_version, open_update_settings, update,
};
use avalio_lite::i18n::{Message, UpdateMessage};
use avalio_lite::ui::UpdateBanner;
use avalio_lite::update::release::UpdateError;
use avalio_lite::update::signature::{self, SigError};
use slint::ComponentHandle;
use std::fs;
use std::path::PathBuf;

// ---------------------------------------------------------------------------------------------
// Updates: Update and restart

/// A database, and the app as a release build at `<scratch>/app/avalio-lite.exe`.
fn release_copy() -> (Fixture, PathBuf) {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let folder = fixture.path("app");
    fs::create_dir_all(&folder).unwrap();
    let exe = folder.join("avalio-lite.exe");
    fs::write(&exe, b"old version").unwrap();
    (fixture, exe)
}

/// The release copy, offered the next version published (signed by the tests' key) on a fixture
/// server; `spoil` changes what it serves first.
pub(crate) fn offered(content: &[u8], spoil: impl FnOnce(&Published)) -> (Ui, PathBuf, Published) {
    let (fixture, exe) = release_copy();
    let ui = Ui::open_release(fixture, exe.clone());
    let published = Published::new(&next_version(), content, &crate::common::release_key());
    spoil(&published);
    ui.releases.set(Ok(published.release.clone()));
    automatic_check(&ui);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
    (ui, exe, published)
}

pub(crate) fn downloads_left(ui: &Ui) -> Vec<PathBuf> {
    fs::read_dir(ui.fixture.path("downloads"))
        .map_or_else(|_| Vec::new(), |entries| entries.map(|entry| entry.unwrap().path()).collect())
}

#[test]
fn update_and_restart_replaces_the_program_and_closes_for_the_new_one() {
    let (ui, exe, _published) = offered(b"the new version", |_| {});
    click(&ui, "Update and restart");
    assert_eq!(fs::read(&exe).unwrap(), b"the new version");
    assert_eq!(ui.session.updater.restart(), Some(exe));
    assert!(!ui.session.window.window().is_visible(), "the window closed as every quit does");
    assert!(downloads_left(&ui).is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn a_download_with_a_strangers_signature_is_refused_before_it_is_fetched() {
    let (ui, exe, published) = offered(b"the new version", |published| {
        let stranger = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
        let text = signature::minisign::sign(&stranger, &published.release.version, &published.name, &published.hashes)
            .unwrap();
        published.serve(&published.signature_path(), crate::common::http::file(text.as_bytes()));
    });
    click(&ui, "Update and restart");
    let version = next_version();
    let expected = english(&Message::Update(UpdateMessage::Failed {
        version: &version,
        error: &UpdateError::NotSigned(SigError::NotSigned),
        page: &published.release.page,
    }));
    assert_eq!(error_on_screen(&ui), expected);
    assert!(expected.contains("isn't signed with avalio lite's release key"));
    assert_eq!(published.server.count(&published.asset_path()), 0, "the binary was never requested");
    assert_eq!(fs::read(&exe).unwrap(), b"old version");
    assert_eq!(ui.session.updater.restart(), None);
    assert!(downloads_left(&ui).is_empty());
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer, "it can be tried again");
}

#[test]
fn a_download_that_does_not_match_its_checksum_is_refused_and_deleted() {
    let (ui, exe, published) = offered(b"the new version", |published| {
        published.serve(&published.asset_path(), crate::common::http::file(b"tampered"));
    });
    click(&ui, "Update and restart");
    let version = next_version();
    let expected = english(&Message::Update(UpdateMessage::Failed {
        version: &version,
        error: &UpdateError::ChecksumMismatch,
        page: &published.release.page,
    }));
    assert_eq!(error_on_screen(&ui), expected);
    assert_eq!(fs::read(&exe).unwrap(), b"old version");
    assert!(downloads_left(&ui).is_empty());
}

#[test]
fn a_download_that_fails_says_why_and_where_to_get_it() {
    let (ui, exe, published) = offered(b"the new version", |published| {
        published.serve(&published.asset_path(), crate::common::http::answer("503 Service Unavailable", b""));
    });
    click(&ui, "Update and restart");
    let shown = error_on_screen(&ui);
    assert!(shown.contains("the server answered 503") && shown.contains(&published.release.page), "{shown}");
    assert_eq!(fs::read(&exe).unwrap(), b"old version");
    assert!(downloads_left(&ui).is_empty());
}

#[test]
fn a_program_that_cannot_be_replaced_says_so_and_stays_the_old_one() {
    let (ui, exe, published) = offered(b"the new version", |_| {});
    // another program holds the exe so it can't be replaced
    let held = crate::common::hold_exclusively(&exe);
    click(&ui, "Update and restart");
    drop(held);
    let shown = error_on_screen(&ui);
    assert!(shown.starts_with("avalio lite couldn't replace itself at "), "{shown}");
    assert!(shown.contains(&published.release.page));
    assert_eq!(fs::read(&exe).unwrap(), b"old version");
    assert_eq!(ui.session.updater.restart(), None);
    assert!(downloads_left(&ui).is_empty(), "the download is removed either way");
}

#[test]
fn a_download_that_cannot_be_started_says_so() {
    let (ui, exe, _published) = offered(b"the new version", |_| {});
    ui.worker.refusing.set(true);
    click(&ui, "Update and restart");
    assert!(error_on_screen(&ui).contains("the download broke off"), "{}", error_on_screen(&ui));
    assert!(!update(&ui).get_downloading());
    assert_eq!(fs::read(&exe).unwrap(), b"old version");
}

#[test]
fn settings_offers_the_same_update_and_restart() {
    let (ui, exe, _published) = offered(b"the new version", |_| {});
    click(&ui, "Hide");
    open_update_settings(&ui);
    click(&ui, "Update and restart");
    assert_eq!(fs::read(&exe).unwrap(), b"the new version");
}

#[test]
fn after_an_update_the_next_start_says_so_once() {
    let ui = seeded();
    ui.session.state.borrow_mut().note_version("0.0.1").unwrap();
    avalio_lite::update::ui::after_start(&ui.session.window, &ui.session.updater);
    let version = avalio_lite::update::target::current_version().to_string();
    assert_eq!(
        ui.session.window.get_status_message(),
        english(&Message::Update(UpdateMessage::Updated { version: &version }))
    );
    let ui = ui.restart();
    avalio_lite::update::ui::after_start(&ui.session.window, &ui.session.updater);
    assert!(ui.session.window.get_status_message().is_empty(), "once");
}

#[test]
fn a_restart_that_failed_is_said_at_the_next_start() {
    let ui = seeded();
    ui.session.updater.not_restarted("the program couldn't be started");
    assert!(saved_preferences(&ui.fixture).update_failed.is_some());
    let ui = ui.restart();
    avalio_lite::update::ui::after_start(&ui.session.window, &ui.session.updater);
    assert_eq!(error_on_screen(&ui), english(&Message::Update(UpdateMessage::NotRestarted)));
    assert_eq!(saved_preferences(&ui.fixture).update_failed, None, "said once");
}

#[test]
fn after_start_says_what_happened_even_when_it_cannot_note_it() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_update_failed(Some("access denied".into())).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    break_saving_the_settings(&ui);
    avalio_lite::update::ui::after_start(&ui.session.window, &ui.session.updater);
    assert_eq!(error_on_screen(&ui), english(&Message::Update(UpdateMessage::NotRestarted)));
}

#[test]
fn after_start_removes_old_downloads() {
    let ui = seeded();
    let folder = ui.fixture.path("downloads");
    fs::create_dir_all(&folder).unwrap();
    let old = folder.join(format!("{}old", avalio_lite::update::download::PREFIX));
    fs::write(&old, b"x").unwrap();
    fs::File::options()
        .write(true)
        .open(&old)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600))
        .unwrap();
    avalio_lite::update::ui::after_start(&ui.session.window, &ui.session.updater);
    assert!(!old.exists());
}

#[test]
fn quitting_with_nothing_downloading_returns_at_once() {
    let ui = seeded();
    let started = std::time::Instant::now();
    ui.session.updater.quit();
    // well under the 5 s a download is waited for, and wide enough for a busy machine
    assert!(started.elapsed() < std::time::Duration::from_secs(3));
}

#[test]
fn a_copy_that_cannot_tell_where_it_runs_says_it_cannot_replace_itself() {
    let ui = seeded();
    let published = Published::new(&next_version(), b"the new version", &crate::common::release_key());
    ui.releases.set(Ok(published.release.clone()));
    automatic_check(&ui);
    click(&ui, "Update and restart");
    let shown = error_on_screen(&ui);
    assert!(shown.starts_with("avalio lite couldn't replace itself at "), "{shown}");
    assert_eq!(published.server.count(&published.asset_path()), 0, "nothing is downloaded");
}

#[test]
fn an_update_goes_ahead_even_when_its_version_cannot_be_noted() {
    let (ui, exe, _published) = offered(b"the new version", |_| {});
    break_saving_the_settings(&ui);
    click(&ui, "Update and restart");
    assert_eq!(fs::read(&exe).unwrap(), b"the new version");
    assert_eq!(ui.session.updater.restart(), Some(exe));
}

#[test]
fn a_failed_restart_that_cannot_be_kept_is_only_logged() {
    let ui = seeded();
    break_saving_the_settings(&ui);
    ui.session.updater.not_restarted("the file is gone");
    assert!(ui.session.window.get_error_message().is_empty(), "the window is already closing");
}
