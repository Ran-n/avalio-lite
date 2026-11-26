// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.908595
//+ Revised: 	2026/11/23 13:00:35.908595
// -----------------------------------------------------------------------------

//! Export, verification and removing files: a verified copy, its key, the names Windows refuses,
//! and the reports when a folder or file cannot be used.

use avalio_lite::error::AppError;
use avalio_lite::platform::db::{self};
use std::fs;

#[test]
fn export_produces_a_verified_copy_and_refuses_overwrite() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("source.db"), None).unwrap();
    connection.execute("INSERT INTO pupils (first_name) VALUES ('Ana Pérez')", []).unwrap();
    // Regression: copying into a folder that doesn't exist yet used to fail when moving the database.
    let copy = dir.path().join("not-yet-created").join("copy.db");
    db::export(&connection, &copy, None).unwrap();
    db::verify(&copy, None).unwrap();
    assert!(matches!(db::export(&connection, &copy, None), Err(AppError::AlreadyExists(_))));
    let copied = db::open(&copy, None).unwrap();
    assert_eq!(db::migrations::schema_version(&copied).unwrap(), db::migrations::latest_version());
}

#[test]
fn an_encrypted_export_needs_its_key() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("source.db"), None).unwrap();
    let copy = dir.path().join("encrypted.db");
    db::export(&connection, &copy, Some("246810")).unwrap();
    assert!(db::is_encrypted(&copy).unwrap());
    assert!(matches!(db::verify(&copy, None), Err(AppError::WrongPin)));
    assert!(matches!(db::verify(&copy, Some("111111")), Err(AppError::WrongPin)));
    db::verify(&copy, Some("246810")).unwrap();
}

#[test]
fn an_export_to_a_name_windows_refuses_is_reported_and_writes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    let destination = dir.path().join("copies").join("bad|name.db");
    let result = db::export(&connection, &destination, None);
    assert!(matches!(result, Err(AppError::Database(_))), "{result:?}");
    assert!(!destination.exists());
    let attached: Vec<String> = connection
        .prepare("SELECT name FROM pragma_database_list")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert!(!attached.iter().any(|name| name == "export"), "nothing is left attached: {attached:?}");
}

#[test]
fn verify_rejects_a_corrupt_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("corrupt.db");
    fs::write(&path, b"definitely not sqlite").unwrap();
    assert!(db::verify(&path, None).is_err());
}

#[test]
fn remove_files_deletes_database_and_sidecars_and_tolerates_missing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    for name in ["test.db", "test.db-wal", "test.db-shm"] {
        fs::write(dir.path().join(name), b"x").unwrap();
    }
    db::remove_files(&path).unwrap();
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 0);
    db::remove_files(&path).unwrap();
}

#[test]
fn export_reports_a_folder_it_cannot_create() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    let in_the_way = dir.path().join("copy");
    fs::write(&in_the_way, b"a file where the folder should be").unwrap();
    assert!(matches!(db::export(&connection, &in_the_way.join("x.db"), None), Err(AppError::Io { .. })));
}

#[test]
fn verify_rejects_a_file_with_damaged_pages() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.db");
    let connection = db::open(&path, None).unwrap();
    connection.execute_batch("PRAGMA journal_mode = DELETE").unwrap();
    for index in 0..400 {
        connection.execute("INSERT INTO pupils (first_name) VALUES (?1)", [format!("Pupil number {index}")]).unwrap();
    }
    let page_size: i64 = connection.query_row("PRAGMA page_size", [], |row| row.get(0)).unwrap();
    let root: i64 =
        connection.query_row("SELECT rootpage FROM sqlite_master WHERE name = 'pupils'", [], |row| row.get(0)).unwrap();
    db::close(connection).unwrap();
    let mut bytes = fs::read(&path).unwrap();
    // scribble over the pupils table's first page, leaving the file header readable
    let (root, page_size) = (usize::try_from(root).unwrap(), usize::try_from(page_size).unwrap());
    let start = (root - 1) * page_size;
    bytes[start..start + page_size].fill(0x5A);
    fs::write(&path, bytes).unwrap();
    let result = db::verify(&path, None);
    assert!(matches!(result, Err(AppError::IntegrityCheckFailed(_))), "{result:?}");
}

#[test]
fn remove_files_reports_a_file_it_cannot_delete() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.db");
    fs::write(&path, b"x").unwrap();
    let _held = crate::common::hold_exclusively(&path);
    assert!(matches!(db::remove_files(&path), Err(AppError::Io { .. })));
}
