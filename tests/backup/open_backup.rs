// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.854801
//+ Revised: 	2026/11/23 13:00:36.854801
// -----------------------------------------------------------------------------

//! Opening a backup file: its PIN, older and newer versions, and files that are not backups.

use crate::common::Fixture;
use crate::support::{PIN, backup_of_another, class_names, filled, restore_copy};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::{classes, pupils, sheet};
use avalio_lite::platform::db::migrations;
use rusqlite::Connection;
use std::fs;
use std::path::Path;

#[test]
fn a_backup_of_this_database_opens_with_its_pin_without_asking() {
    let fixture = Fixture::new();
    let mut state = filled(&fixture);
    state.set_pin(PIN, PIN).unwrap();
    let file = fixture.path("mine.avalio");
    state.export_backup(&file).unwrap();
    let opened = state.open_backup(&file, None).unwrap();
    assert_eq!(opened.contents().classes, 2);
}

#[test]
fn regression_the_open_database_is_refused_as_its_own_backup() {
    // the file was copied without its write-ahead log, so restoring it put the data back as it was
    // when the app opened it and dropped everything typed since
    let fixture = Fixture::new();
    filled(&fixture).shutdown();
    let mut state = fixture.state();
    state.start();
    classes::create_class(state.connection().unwrap(), "Typed since").unwrap();
    let own = fixture.database();
    let result = state.open_backup(&own, None);
    assert!(matches!(&result, Err(AppError::OpenDatabaseAsBackup(path)) if *path == own), "{:?}", result.err());
    assert!(!restore_copy(&fixture).exists());
    // the same file spelled another way, and with a PIN typed for it
    let spelled = own.parent().unwrap().join(".").join(own.file_name().unwrap());
    assert!(matches!(state.open_backup(&spelled, Some(PIN)), Err(AppError::OpenDatabaseAsBackup(_))));
    assert!(class_names(&state).contains(&"Typed since".to_owned()));
    // a copy of it elsewhere is a backup like any other
    let copy = fixture.path("copy.db");
    state.shutdown();
    fs::copy(&own, &copy).unwrap();
    state.start();
    assert_eq!(state.open_backup(&copy, None).unwrap().contents().classes, 3);
}

#[test]
fn a_wrong_backup_pin_is_refused() {
    let (_other, secret) = backup_of_another(Some("2222"));
    let fixture = Fixture::new();
    let state = fixture.set_up();
    assert!(matches!(state.open_backup(&secret, Some("3333")), Err(AppError::WrongPin)));
    assert!(!restore_copy(&fixture).exists());
}

#[test]
fn an_older_backup_is_upgraded_and_the_file_is_untouched() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/db-v1.db");
    let before = fs::read(&golden).unwrap();
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let opened = state.open_backup(&golden, None).unwrap();
    state.restore_backup(opened).unwrap();
    assert_eq!(fs::read(&golden).unwrap(), before, "the chosen file is never changed");

    let connection = state.connection().unwrap();
    assert_eq!(migrations::schema_version(connection).unwrap(), migrations::latest_version());
    let class = classes::list_classes(connection).unwrap()[0].id;
    let loaded = sheet::load_sheet(connection, class).unwrap();
    let titles: Vec<&str> =
        loaded.columns.iter().filter(|column| !column.kind.is_name()).map(|column| column.title.as_str()).collect();
    assert_eq!(titles, ["Exam", "Final grade"]);
    assert_eq!(loaded.rows[0].pupil.first_name, "Ana");
    assert_eq!(loaded.rows[0].values, ["Ana", "", "7,5", ""], "the names first, then the marks");
}

#[test]
fn a_newer_backup_is_refused() {
    let (other, file) = backup_of_another(None);
    let newer = other.path("newer.avalio");
    fs::copy(&file, &newer).unwrap();
    let future = migrations::latest_version() + 1;
    Connection::open(&newer).unwrap().pragma_update(None, "user_version", future as i64).unwrap();

    let fixture = Fixture::new();
    let state = fixture.set_up();
    let result = state.open_backup(&newer, None);
    assert!(matches!(result, Err(AppError::NewerSchema { found, .. }) if found == future), "{:?}", result.err());
    assert!(!restore_copy(&fixture).exists());
}

#[test]
fn a_file_that_is_not_a_database_is_refused() {
    let fixture = Fixture::new();
    let state = fixture.set_up();
    for (name, contents) in [
        ("empty.avalio", Vec::new()),
        ("short.avalio", b"hello".to_vec()),
        ("notes.avalio", "Notes for June: nothing here is a database. ".repeat(40).into_bytes()),
    ] {
        let file = fixture.path(name);
        fs::write(&file, contents).unwrap();
        let result = state.open_backup(&file, None);
        assert!(matches!(&result, Err(AppError::NotADatabase(path)) if *path == file), "{name}: {:?}", result.err());
        assert!(!restore_copy(&fixture).exists(), "{name}");
    }
}

#[test]
fn a_damaged_backup_is_refused() {
    let (other, file) = backup_of_another(None);
    let damaged = other.path("damaged.avalio");
    let mut bytes = fs::read(&file).unwrap();
    // scribble over the second page, past the header SQLite needs to open the file at all
    for byte in &mut bytes[4096..4096 + 200] {
        *byte = 0x5A;
    }
    fs::write(&damaged, bytes).unwrap();

    let fixture = Fixture::new();
    let state = fixture.set_up();
    let result = state.open_backup(&damaged, None);
    assert!(
        matches!(&result, Err(AppError::IntegrityCheckFailed(path) | AppError::NotADatabase(path)) if *path == damaged)
            || matches!(&result, Err(AppError::Database(_))),
        "a damaged backup is reported as damaged, never as a wrong PIN: {:?}",
        result.err()
    );
    assert!(!restore_copy(&fixture).exists());
}

#[test]
fn a_missing_backup_file_is_reported() {
    let fixture = Fixture::new();
    let state = fixture.set_up();
    let gone = fixture.path("gone.avalio");
    assert!(matches!(state.open_backup(&gone, None), Err(AppError::Io { path, .. }) if path == gone));
}

#[test]
fn opening_a_backup_needs_a_chosen_database() {
    let (_other, file) = backup_of_another(None);
    let fixture = Fixture::new();
    assert!(matches!(fixture.state().open_backup(&file, None), Err(AppError::NoDatabaseOpen)));
}

#[test]
fn a_version_5_backup_opens_with_its_names_as_first_names() {
    let golden = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/db-v5.db");
    let fixture = Fixture::new();
    let state = fixture.set_up();
    let opened = state.open_backup(&golden, None).unwrap();
    let summary = state.add_backup_classes(opened).unwrap();
    let connection = state.connection().unwrap();
    let names: Vec<(String, String)> = pupils::list_class_pupils(connection, summary.class_ids[0])
        .unwrap()
        .into_iter()
        .map(|pupil| (pupil.first_name, pupil.surname))
        .collect();
    assert_eq!(names, [("Ana".to_owned(), String::new()), ("Bea".to_owned(), String::new())]);
    assert_eq!(crate::common::name_widths(connection, summary.class_ids[0]), (Some(260), None), "its width too");
}
