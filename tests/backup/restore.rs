// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.933290
//+ Revised: 	2026/11/23 17:29:55.913848
// -----------------------------------------------------------------------------

//! Restoring a backup: it replaces the database, keeps the PIN, and never leaves a copy behind.

use crate::common::Fixture;
use crate::support::{backup_of_another, class_names, everything, restore_copy, version_ten_backup_across_classes};
use avalio_lite::backup;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::model::ColumnKind;
use avalio_lite::gradebook::{classes, pupils, sheet};
use avalio_lite::platform::db::{self, migrations};
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;

/// The kind of the rounded column in class `name`, after a restore: its source and decimals.
fn rounded_kind_of(connection: &Connection, name: &str) -> ColumnKind {
    let class = classes::list_classes(connection).unwrap().into_iter().find(|class| class.name == name).unwrap();
    let loaded = sheet::load_sheet(connection, class.id).unwrap();
    loaded.columns.into_iter().find(|column| matches!(column.kind, ColumnKind::Round { .. })).unwrap().kind
}

/// Each rounding of a backup still rounds the same column of the same class after a restore.
#[test]
fn restoring_a_backup_keeps_each_rounding_link() {
    let (_other, file) = backup_of_another(None);
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let opened = state.open_backup(&file, None).unwrap();
    state.restore_backup(opened).unwrap();
    let connection = state.connection().unwrap();
    let class = classes::list_classes(connection).unwrap().into_iter().find(|class| class.name == "2º ESO A").unwrap();
    let loaded = sheet::load_sheet(connection, class.id).unwrap();
    let final_grade = loaded.columns.iter().find(|column| column.title == "Final grade").unwrap().id;
    assert_eq!(rounded_kind_of(connection, "2º ESO A"), ColumnKind::Round { source: Some(final_grade), decimals: 0 });
}

/// An older backup is upgraded when restored, and its rounding link across classes is dropped.
#[test]
fn restoring_an_older_backup_drops_a_link_across_classes() {
    let fixture = Fixture::new();
    let file = version_ten_backup_across_classes(&fixture);
    let mut state = fixture.set_up();
    let opened = state.open_backup(&file, None).unwrap();
    state.restore_backup(opened).unwrap();
    let kind = rounded_kind_of(state.connection().unwrap(), "1A");
    assert_eq!(kind, ColumnKind::Round { source: None, decimals: 2 }, "no link across classes, decimals kept");
}

#[test]
fn restoring_replaces_everything() {
    let (_other, file) = backup_of_another(None);
    let expected = everything(&db::open_existing(&file, None).unwrap());
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    classes::create_class(state.connection().unwrap(), "Only here").unwrap();

    let opened = state.open_backup(&file, None).unwrap();
    assert_eq!(opened.contents(), backup::BackupContents { classes: 2, pupils: 5 });
    assert_eq!(opened.file_name(), "theirs.avalio");
    state.restore_backup(opened).unwrap();

    assert_eq!(everything(state.connection().unwrap()), expected);
    assert!(!restore_copy(&fixture).exists());
    state.shutdown();
    let mut restarted = fixture.state();
    restarted.start();
    assert_eq!(class_names(&restarted), ["1º ESO B", "2º ESO A"], "the restore is on disk");
}

#[test]
fn restoring_keeps_the_current_pin() {
    // an encrypted database and a plain backup: still encrypted with its own PIN afterwards
    let (_other, plain) = backup_of_another(None);
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin("1111", "1111").unwrap();
    let opened = state.open_backup(&plain, None).unwrap();
    state.restore_backup(opened).unwrap();
    assert!(state.is_protected());
    state.shutdown();
    assert!(matches!(db::open_existing(&fixture.database(), None), Err(AppError::WrongPin)));
    assert_eq!(everything(&db::open_existing(&fixture.database(), Some("1111")).unwrap()).len(), 2);

    // a plain database and a backup made with another PIN: plain afterwards
    let (_other, secret) = backup_of_another(Some("2222"));
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    assert!(matches!(state.open_backup(&secret, None), Err(AppError::WrongPin)));
    let opened = state.open_backup(&secret, Some("2222")).unwrap();
    state.restore_backup(opened).unwrap();
    assert!(!state.is_protected());
    state.shutdown();
    assert!(!db::is_encrypted(&fixture.database()).unwrap());
    assert_eq!(everything(&db::open_existing(&fixture.database(), None).unwrap()).len(), 2);
}

#[test]
fn a_failed_swap_keeps_the_current_database() {
    let (_other, file) = backup_of_another(None);
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    classes::create_class(state.connection().unwrap(), "Only here").unwrap();
    let opened = state.open_backup(&file, None).unwrap();
    // another program has the database open, so it can't be renamed aside
    let held = crate::common::hold_against_removal(&fixture.database());
    let result = state.restore_backup(opened);
    assert!(matches!(result, Err(AppError::Io { .. })), "{result:?}");
    assert_eq!(class_names(&state), ["Only here"], "the current database is open again, unchanged");
    assert!(!restore_copy(&fixture).exists());
    drop(held);
    state.shutdown();
    let mut restarted = fixture.state();
    restarted.start();
    assert_eq!(class_names(&restarted), ["Only here"]);
}

#[test]
fn the_restore_copy_never_stays_behind() {
    let (_other, secret) = backup_of_another(Some("2222"));
    let fixture = Fixture::new();
    let state = fixture.set_up();
    // asked for a PIN, then given a wrong one, then cancelled (the opened backup dropped)
    assert!(matches!(state.open_backup(&secret, None), Err(AppError::WrongPin)));
    assert!(!restore_copy(&fixture).exists());
    assert!(matches!(state.open_backup(&secret, Some("0000")), Err(AppError::WrongPin)));
    assert!(!restore_copy(&fixture).exists());
    let opened = state.open_backup(&secret, Some("2222")).unwrap();
    assert!(restore_copy(&fixture).exists(), "the copy is open while the question is asked");
    drop(opened);
    assert!(!restore_copy(&fixture).exists());
    // a copy left by a crash is replaced
    fs::write(restore_copy(&fixture), b"from a crash").unwrap();
    drop(state.open_backup(&secret, Some("2222")).unwrap());
    assert!(!restore_copy(&fixture).exists());
}

/// A backup made before surnames had a column (schema 5), holding class "1A" with one pupil typed
/// surname first and one with a single name.
fn comma_name_backup(fixture: &Fixture) -> PathBuf {
    let file = fixture.path("old.avalio");
    let mut old = Connection::open(&file).unwrap();
    migrations::migrate_to(&mut old, 5).unwrap();
    old.execute_batch(
        "INSERT INTO classes (id, name) VALUES (1, '1A');
         INSERT INTO pupils (id, name) VALUES (1, 'Pérez López, Ana'), (2, 'Bea');
         INSERT INTO class_pupils VALUES (1, 1, 1), (1, 2, 2);",
    )
    .unwrap();
    drop(old);
    file
}

fn class_pupil_names(connection: &Connection, class: i64) -> Vec<(String, String)> {
    pupils::list_class_pupils(connection, class)
        .unwrap()
        .into_iter()
        .map(|pupil| (pupil.first_name, pupil.surname))
        .collect()
}

/// Old backups go through the migrations too, so their comma names arrive split, restored or added.
#[test]
fn restoring_an_old_backup_splits_comma_names() {
    let split = [("Ana".to_owned(), "Pérez López".to_owned()), ("Bea".to_owned(), String::new())];
    let fixture = Fixture::new();
    let file = comma_name_backup(&fixture);
    let mut state = fixture.set_up();
    let opened = state.open_backup(&file, None).unwrap();
    state.restore_backup(opened).unwrap();
    let connection = state.connection().unwrap();
    let class = classes::list_classes(connection).unwrap()[0].id;
    assert_eq!(class_pupil_names(connection, class), split, "restored");

    let other = Fixture::new();
    let file = comma_name_backup(&other);
    let state = other.set_up();
    let opened = state.open_backup(&file, None).unwrap();
    let summary = state.add_backup_classes(opened).unwrap();
    assert_eq!(class_pupil_names(state.connection().unwrap(), summary.class_ids[0]), split, "added");
}
