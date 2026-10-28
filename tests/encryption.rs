// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:46.184372
//+ Revised: 	2026/10/28 15:53:09.126231
// -----------------------------------------------------------------------------

//! The PIN encrypts the database file (SQLCipher). These tests look at the real file on disk.

use std::fs;
use std::path::Path;

mod common;
use common::Fixture;

use avalio_lite::app::state::{AppState, Opened, Startup};
use avalio_lite::config::DATABASE_FILE_NAME;
use avalio_lite::db;
use avalio_lite::error::AppError;

const SECRET: &str = "Ana Pérez secret note";

/// A database with one recognisable value in it.
fn set_up(fixture: &Fixture) -> AppState {
    let mut state = fixture.state();
    assert_eq!(state.create_database(&fixture.path("data"), &fixture.path("logs")).unwrap(), Opened::Ready);
    state.connection().unwrap().execute("INSERT INTO pupils (name) VALUES (?1)", [SECRET]).unwrap();
    state
}

fn file_contains(path: &Path, text: &str) -> bool {
    let bytes = fs::read(path).unwrap();
    bytes.windows(text.len()).any(|window| window == text.as_bytes())
}

fn pupil_names(state: &AppState) -> Vec<String> {
    let mut statement = state.connection().unwrap().prepare("SELECT name FROM pupils").unwrap();
    statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
}

#[test]
fn without_a_pin_the_file_is_plain_sqlite() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.shutdown();
    assert!(!db::is_encrypted(&fixture.database()).unwrap());
    assert!(file_contains(&fixture.database(), SECRET), "plain SQLite keeps text readable");
}

#[test]
fn setting_a_pin_encrypts_the_whole_file() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.set_pin("246810", "246810").unwrap();
    assert!(state.is_protected());
    assert_eq!(pupil_names(&state), [SECRET], "the open database still works");
    state.shutdown();

    assert!(db::is_encrypted(&fixture.database()).unwrap());
    assert!(!file_contains(&fixture.database(), SECRET), "the text must not be readable in the file");
    assert!(!file_contains(&fixture.database(), "SQLite format 3"));
    assert!(matches!(db::open(&fixture.database(), None), Err(AppError::WrongPin)));
}

#[test]
fn an_encrypted_database_starts_locked_and_only_the_right_pin_opens_it() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.set_pin("246810", "246810").unwrap();
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Locked);
    assert!(restarted.connection().is_none(), "nothing is opened before the PIN");
    assert!(matches!(restarted.unlock("111111"), Err(AppError::WrongPin)));
    assert!(restarted.connection().is_none());
    restarted.unlock("246810").unwrap();
    assert_eq!(pupil_names(&restarted), [SECRET]);
    restarted.shutdown();
}

#[test]
fn locking_closes_the_database_and_forgets_the_pin() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    assert!(!state.lock(), "without a PIN there is nothing to lock with");
    assert!(state.connection().is_some());

    state.set_pin("246810", "246810").unwrap();
    assert!(state.lock());
    assert!(state.is_locked());
    assert!(state.connection().is_none());
    assert!(matches!(state.change_pin("246810", "135790", "135790"), Err(AppError::NotEditable)), "key forgotten");
    state.unlock("246810").unwrap();
    assert_eq!(pupil_names(&state), [SECRET]);
}

#[test]
fn changing_the_pin_re_encrypts_with_the_new_one() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.set_pin("246810", "246810").unwrap();
    assert!(matches!(state.change_pin("000000", "135790", "135790"), Err(AppError::WrongPin)));
    assert!(matches!(state.change_pin("246810", "135790", "135791"), Err(AppError::PinMismatch)));
    state.change_pin("246810", "135790", "135790").unwrap();
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Locked);
    assert!(matches!(restarted.unlock("246810"), Err(AppError::WrongPin)), "the old PIN no longer works");
    restarted.unlock("135790").unwrap();
    assert_eq!(pupil_names(&restarted), [SECRET]);
}

#[test]
fn removing_the_pin_decrypts_the_file() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.set_pin("246810", "246810").unwrap();
    assert!(matches!(state.remove_pin("000000"), Err(AppError::WrongPin)));
    state.remove_pin("246810").unwrap();
    assert!(!state.is_protected());
    state.shutdown();
    assert!(!db::is_encrypted(&fixture.database()).unwrap());
    assert!(file_contains(&fixture.database(), SECRET));
}

#[test]
fn re_keying_leaves_no_temporary_files_behind() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.set_pin("246810", "246810").unwrap();
    state.change_pin("246810", "135790", "135790").unwrap();
    state.shutdown();
    let mut names: Vec<String> = fs::read_dir(fixture.path("data"))
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    assert_eq!(names, [DATABASE_FILE_NAME]);
}

#[test]
fn the_schema_version_survives_re_keying() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    let before = db::migrations::schema_version(state.connection().unwrap()).unwrap();
    state.set_pin("246810", "246810").unwrap();
    assert_eq!(db::migrations::schema_version(state.connection().unwrap()).unwrap(), before);
}

#[test]
fn a_moved_encrypted_database_stays_encrypted() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.set_pin("246810", "246810").unwrap();
    let moved = state.move_database(&fixture.path("elsewhere")).unwrap();
    assert_eq!(pupil_names(&state), [SECRET]);
    state.shutdown();
    assert!(db::is_encrypted(&moved).unwrap());
    assert!(!file_contains(&moved, SECRET));
}

#[test]
fn opening_an_encrypted_file_from_settings_asks_for_its_pin() {
    let fixture = Fixture::new();
    let mut first = set_up(&fixture);
    first.set_pin("246810", "246810").unwrap();
    first.shutdown();

    let other = Fixture::new();
    let mut state = other.set_up();
    assert_eq!(state.open_existing(&fixture.database(), &other.path("logs")).unwrap(), Opened::Locked);
    assert!(state.is_locked());
    state.unlock("246810").unwrap();
    assert_eq!(pupil_names(&state), [SECRET]);
}

#[test]
fn a_pin_can_only_be_set_once_without_the_current_one() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.set_pin("246810", "246810").unwrap();
    assert!(matches!(state.set_pin("135790", "135790"), Err(AppError::NotEditable)));
}

#[test]
fn a_pin_change_that_cannot_swap_the_files_keeps_the_old_pin_working() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    state.set_pin("246810", "246810").unwrap();
    // another program has the database open, so it can't be renamed aside
    let _held = common::hold_against_removal(&fixture.database());
    let result = state.change_pin("246810", "135790", "135790");
    assert!(matches!(result, Err(AppError::Io { .. })), "{result:?}");
    assert_eq!(pupil_names(&state), [SECRET], "the database is open again with the old PIN");
    assert!(!fixture.path("data").join(format!("{DATABASE_FILE_NAME}.rekey")).exists(), "the new copy is removed");
    drop(_held);
    state.shutdown();
    let mut reopened = fixture.state();
    assert_eq!(reopened.start(), Startup::Locked);
    reopened.unlock("246810").unwrap();
}

#[test]
fn a_leftover_rekey_file_that_cannot_be_removed_stops_the_pin_change() {
    let fixture = Fixture::new();
    let mut state = set_up(&fixture);
    let leftover = fixture.path("data").join(format!("{DATABASE_FILE_NAME}.rekey"));
    fs::write(&leftover, b"from a crash").unwrap();
    let _held = common::hold_exclusively(&leftover);
    assert!(matches!(state.set_pin("246810", "246810"), Err(AppError::Io { .. })));
    assert!(!state.is_protected());
    assert_eq!(pupil_names(&state), [SECRET]);
}
