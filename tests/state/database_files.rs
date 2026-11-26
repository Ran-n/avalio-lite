// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:37.096257
//+ Revised: 	2026/11/23 13:00:37.096257
// -----------------------------------------------------------------------------

//! The database file: a missing, unreadable or foreign file, and copies kept by an update.

use crate::common::Fixture;
use crate::support::{add_pupil, pupil_count};
use avalio_lite::error::AppError;
use avalio_lite::platform::config::DATABASE_FILE_NAME;
use avalio_lite::state::{AppState, SetupReason, Startup};
use std::fs;

/// A crash mid PIN change can leave only `.previous`, the database gone; making a new database in
/// that folder must not delete it, since it may be the only good copy.
#[test]
fn a_new_database_keeps_the_previous_copy_of_one_that_went_missing() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.path("data")).unwrap();
    let previous = std::path::PathBuf::from(format!("{}.previous", fixture.database().display()));
    std::fs::write(&previous, b"the only good copy").unwrap();
    let mut state = fixture.state();
    state.create_database(&fixture.path("data"), &fixture.path("logs")).unwrap();
    assert!(previous.exists());
}

/// The same for a database without a PIN: the copies go as soon as it opens at start.
#[test]
fn copies_left_by_an_interrupted_change_go_when_the_database_opens() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let leftover = std::path::PathBuf::from(format!("{}.previous", fixture.database().display()));
    std::fs::write(&leftover, b"old copy").unwrap();
    let mut state = fixture.state();
    assert_eq!(state.start(), Startup::Ready);
    assert!(!leftover.exists());
}

#[test]
fn missing_database_is_reported_and_not_recreated() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let database_path = state.config().unwrap().database_path.clone();
    state.shutdown();
    avalio_lite::platform::db::remove_files(&database_path).unwrap();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::NeedsSetup(SetupReason::DatabaseMissing(database_path.clone())));
    assert!(!database_path.exists());
}

#[test]
fn unreadable_settings_ask_for_setup() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.path("config")).unwrap();
    fs::write(fixture.store().path(), "not = = toml").unwrap();
    assert!(matches!(fixture.state().start(), Startup::NeedsSetup(SetupReason::ConfigUnreadable(_))));
}

#[test]
fn corrupt_database_asks_for_setup() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let database_path = state.config().unwrap().database_path.clone();
    state.shutdown();
    avalio_lite::platform::db::remove_files(&database_path).unwrap();
    fs::write(&database_path, b"not a database").unwrap();
    assert!(matches!(fixture.state().start(), Startup::NeedsSetup(SetupReason::DatabaseUnusable(_))));
}

#[test]
fn open_existing_switches_database_and_refuses_missing_file() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let missing = fixture.path("nowhere.db");
    assert!(matches!(state.open_existing(&missing, &fixture.path("logs")), Err(AppError::NotFound(_))));

    let other = fixture.path("other.db");
    avalio_lite::platform::db::close(avalio_lite::platform::db::open(&other, None).unwrap()).unwrap();
    state.open_existing(&other, &fixture.path("logs")).unwrap();
    assert_eq!(fixture.store().load().unwrap().unwrap().database_path, other);
}

#[test]
fn a_database_held_by_another_program_asks_for_setup() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let _held = crate::common::hold_exclusively(&fixture.database());
    let startup = fixture.state().start();
    assert!(matches!(startup, Startup::NeedsSetup(SetupReason::DatabaseUnusable(_))), "{startup:?}");
}

fn keeps_working_as_before(state: &AppState, fixture: &Fixture) {
    assert_eq!(state.config().unwrap().database_path, fixture.database(), "the choice was not taken");
    assert_eq!(fixture.store().load().unwrap().unwrap().database_path, fixture.database(), "nothing was saved");
    assert!(state.connection().is_some(), "the current database stays open");
    assert!(!state.is_locked());
}

#[test]
fn regression_an_empty_file_is_not_initialised_as_a_database_when_chosen() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let stray = fixture.path("stray.db");
    fs::write(&stray, b"").unwrap();
    let result = state.open_existing(&stray, &fixture.path("logs"));
    assert!(matches!(result, Err(AppError::NotADatabase(path)) if path == stray));
    assert_eq!(fs::metadata(&stray).unwrap().len(), 0, "the file was not touched");
    keeps_working_as_before(&state, &fixture);
}

#[test]
fn regression_a_stray_file_does_not_make_the_app_ask_for_a_pin() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let stray = fixture.path("stray.db");
    // looks encrypted, but is not whole pages
    fs::write(&stray, vec![7u8; 700]).unwrap();
    assert!(matches!(state.open_existing(&stray, &fixture.path("logs")), Err(AppError::NotADatabase(_))));
    keeps_working_as_before(&state, &fixture);
    fs::write(&stray, b"tiny").unwrap();
    assert!(matches!(state.open_existing(&stray, &fixture.path("logs")), Err(AppError::NotADatabase(_))));
    keeps_working_as_before(&state, &fixture);
}

#[test]
fn a_sqlite_file_that_is_not_avalio_lite_is_refused_untouched() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let foreign = fixture.path("foreign.db");
    let connection = rusqlite::Connection::open(&foreign).unwrap();
    connection.execute("CREATE TABLE notes (text)", []).unwrap();
    connection.close().unwrap();
    let before = fs::read(&foreign).unwrap();
    assert!(matches!(state.open_existing(&foreign, &fixture.path("logs")), Err(AppError::NotADatabase(_))));
    assert_eq!(fs::read(&foreign).unwrap(), before, "no migration ran on it");
    keeps_working_as_before(&state, &fixture);
}

#[test]
fn a_whole_page_file_that_looks_encrypted_is_chosen_locked() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let locked = fixture.path("locked.db");
    fs::write(&locked, vec![0xA5u8; 4096]).unwrap();
    assert_eq!(state.open_existing(&locked, &fixture.path("logs")).unwrap(), avalio_lite::state::Opened::Locked);
    assert!(state.is_locked());
}

#[test]
fn regression_start_does_not_initialise_an_empty_or_foreign_database_file() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let path = state.config().unwrap().database_path.clone();
    state.shutdown();
    avalio_lite::platform::db::remove_files(&path).unwrap();
    for contents in [&b""[..], b"short", b"SQLite format 3\0 but then garbage garbage"] {
        fs::write(&path, contents).unwrap();
        let startup = fixture.state().start();
        assert!(matches!(startup, Startup::NeedsSetup(SetupReason::DatabaseUnusable(_))), "{contents:?}");
        assert_eq!(fs::read(&path).unwrap(), contents, "the file was left as it was");
    }
}

#[test]
fn a_database_from_a_newer_version_needs_setup_at_startup() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let connection = rusqlite::Connection::open(fixture.database()).unwrap();
    connection.pragma_update(None, "user_version", 999).unwrap();
    drop(connection);
    let mut state = fixture.state();
    let startup = state.start();
    assert!(
        matches!(&startup, Startup::NeedsSetup(SetupReason::DatabaseUnusable(text)) if text.contains("newer version")),
        "{startup:?}"
    );
}

#[test]
fn a_database_another_program_is_writing_to_is_reported_and_not_opened() {
    let fixture = Fixture::new();
    let other_place = fixture.path("other").join(DATABASE_FILE_NAME);
    avalio_lite::platform::db::close(avalio_lite::platform::db::open(&other_place, None).unwrap()).unwrap();
    let mut state = fixture.set_up();
    let writer = rusqlite::Connection::open(&other_place).unwrap();
    writer.execute_batch("PRAGMA journal_mode = DELETE; PRAGMA locking_mode = EXCLUSIVE; BEGIN EXCLUSIVE;").unwrap();
    let result = state.open_existing(&other_place, &fixture.path("logs"));
    assert!(matches!(result, Err(AppError::Database(_))), "{result:?}");
    assert_eq!(state.config().unwrap().database_path, fixture.database(), "still on the database it had");
    writer.execute_batch("ROLLBACK").unwrap();
}

#[test]
fn the_copy_kept_before_an_update_moves_with_the_database() {
    use avalio_lite::platform::{db, storage};
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let original = state.config().unwrap().database_path.clone();
    let kept = storage::with_suffix(&original, db::BEFORE_UPDATE_SUFFIX);
    fs::write(&kept, b"the copy").unwrap();

    let moved = state.move_database(&fixture.path("moved")).unwrap().path;

    assert!(!kept.exists());
    assert_eq!(fs::read(storage::with_suffix(&moved, db::BEFORE_UPDATE_SUFFIX)).unwrap(), b"the copy");
}

#[test]
fn a_kept_copy_that_cannot_be_moved_stays_and_the_database_still_moves() {
    use avalio_lite::platform::{db, storage};
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    add_pupil(&state);
    let original = state.config().unwrap().database_path.clone();
    let kept = storage::with_suffix(&original, db::BEFORE_UPDATE_SUFFIX);
    fs::write(&kept, b"the copy").unwrap();
    let held = crate::common::hold_exclusively(&kept);

    let moved = state.move_database(&fixture.path("moved")).unwrap();

    drop(held);
    assert_eq!(moved.left_behind, None);
    assert_eq!(pupil_count(&state), 1);
    assert_eq!(fs::read(&kept).unwrap(), b"the copy", "it stays where it was");
    assert!(!storage::with_suffix(&moved.path, db::BEFORE_UPDATE_SUFFIX).exists());
}
