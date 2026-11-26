// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.268631
//+ Revised: 	2026/11/25 11:43:11.392863
// -----------------------------------------------------------------------------

//! Opening and creating a database: the schema, WAL and foreign keys, a locked or unreadable file,
//! a damaged plain file, and the encryption key.

use avalio_lite::error::AppError;
use avalio_lite::platform::db::{self, migrations};
use rusqlite::Connection;
use std::fs;

const TABLES: &[&str] =
    &["cells", "class_pupils", "classes", "pupil_pictures", "pupils", "round_sources", "sheet_columns"];

fn table_names(connection: &Connection) -> Vec<String> {
    let mut statement =
        connection.prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name").unwrap();
    statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
}

#[test]
fn open_creates_folder_file_and_full_schema() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("sub").join("test.db");
    let connection = db::open(&path, None).unwrap();
    assert!(path.is_file());
    assert_eq!(table_names(&connection), TABLES);
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
}

#[test]
fn open_enables_foreign_keys_and_wal() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    let foreign_keys: i64 = connection.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap();
    let journal: String = connection.pragma_query_value(None, "journal_mode", |row| row.get(0)).unwrap();
    assert_eq!(foreign_keys, 1);
    assert_eq!(journal, "wal");
}

#[test]
fn reopening_keeps_data_and_schema_version() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("test.db");
    let connection = db::open(&path, None).unwrap();
    connection.execute("INSERT INTO pupils (first_name) VALUES ('Ana Pérez')", []).unwrap();
    db::close(connection).unwrap();

    let connection = db::open(&path, None).unwrap();
    let count: i64 = connection.query_row("SELECT COUNT(*) FROM pupils", [], |row| row.get(0)).unwrap();
    assert_eq!(count, 1);
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
}

#[test]
fn newer_schema_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("future.db");
    let connection = Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", 999).unwrap();
    drop(connection);
    assert!(matches!(db::open(&path, None), Err(AppError::NewerSchema { found: 999, .. })));
}

#[test]
fn open_existing_never_creates_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("missing.db");
    assert!(matches!(db::open_existing(&path, None), Err(AppError::NotFound(_))));
    assert!(!path.exists());
}

#[test]
fn a_database_that_cannot_be_read_is_a_database_error_not_a_wrong_pin() {
    let dir = tempfile::tempdir().unwrap();
    // a folder in the file's place: opening may work, reading it can't, and that isn't a wrong PIN
    let result = db::verify(dir.path(), Some("2468"));
    assert!(matches!(result, Err(AppError::Database(_))), "{result:?}");
}

/// Another program writing to the file (an exclusive lock) makes the first read fail as "busy":
/// a database error, never mistaken for a wrong PIN.
#[test]
fn a_database_locked_by_another_writer_is_a_database_error() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.db");
    db::close(db::open(&path, None).unwrap()).unwrap();
    let other = Connection::open(&path).unwrap();
    other.execute_batch("PRAGMA journal_mode = DELETE; PRAGMA locking_mode = EXCLUSIVE; BEGIN EXCLUSIVE;").unwrap();
    let result = db::open(&path, None);
    assert!(matches!(result, Err(AppError::Database(_))), "{result:?}");
    other.execute_batch("ROLLBACK").unwrap();
}

#[test]
fn open_reports_a_folder_it_cannot_create() {
    let dir = tempfile::tempdir().unwrap();
    let in_the_way = dir.path().join("data");
    fs::write(&in_the_way, b"a file where the folder should be").unwrap();
    assert!(matches!(db::open(&in_the_way.join("x.db"), None), Err(AppError::Io { .. })));
}

#[test]
fn a_file_held_by_another_program_cant_be_checked_for_encryption() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.db");
    db::close(db::open(&path, None).unwrap()).unwrap();
    let _held = crate::common::hold_exclusively(&path);
    assert!(matches!(db::is_encrypted(&path), Err(AppError::Io { .. })));
}

/// SQLCipher wipes the memory it frees (its copies of the PIN and the derived key), whether the
/// file is encrypted or not.
#[test]
fn sqlcipher_wipes_freed_memory() {
    let dir = tempfile::tempdir().unwrap();
    for (name, key) in [("plain.db", None), ("encrypted.db", Some("583920"))] {
        let connection = db::open(&dir.path().join(name), key).unwrap();
        let setting: String = connection.query_row("PRAGMA cipher_memory_security", [], |row| row.get(0)).unwrap();
        assert_eq!(setting, "1", "{name}");
    }
}

proptest::proptest! {
    /// The PIN statement quotes any key correctly and is never reallocated while it is built, so
    /// no unwiped copy of the PIN is left behind in freed memory.
    #[test]
    fn the_key_statement_is_quoted_and_built_in_place(key in r"\PC{0,12}") {
        let statement = db::key_statement(&key);
        proptest::prop_assert_eq!(statement.as_str(), format!("PRAGMA key = '{}'", key.replace('\'', "''")));
        proptest::prop_assert_eq!(statement.capacity(), "PRAGMA key = '".len() + 2 * key.len() + 1);
    }
}

#[test]
fn regression_a_damaged_plain_file_is_not_a_wrong_pin() {
    // a file too short to be encrypted, opened without a key, said "Wrong PIN" although it has none
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("short.db");
    fs::write(&path, b"SQLite fo").unwrap();
    assert!(!db::is_encrypted(&path).unwrap(), "shorter than the header: never taken for encrypted");
    assert!(matches!(db::open(&path, None), Err(AppError::NotADatabase(found)) if found == path));
    // a file that looks encrypted, opened without a key, still needs its PIN
    let random = dir.path().join("random.db");
    fs::write(&random, [0x5Au8; 4096]).unwrap();
    assert!(matches!(db::open(&random, None), Err(AppError::WrongPin)));
}

#[test]
fn has_whole_pages_checks_the_page_size() {
    let dir = tempfile::tempdir().unwrap();
    for (length, whole) in [(512, true), (1024, true), (513, false), (100, false)] {
        let path = dir.path().join(format!("{length}.db"));
        fs::write(&path, vec![0u8; length]).unwrap();
        assert_eq!(db::has_whole_pages(&path).unwrap(), whole, "{length} bytes");
    }
    let missing = dir.path().join("missing.db");
    assert!(matches!(db::has_whole_pages(&missing), Err(AppError::Io { path, .. }) if path == missing));
}

/// A new database, and the plain `open` used for backups and scratch copies, make no copy.
#[test]
fn a_new_database_or_a_scratch_copy_keeps_no_copy() {
    let dir = tempfile::tempdir().unwrap();
    db::open_upgrading(&dir.path().join("new.db"), None).unwrap();
    let mut old = Connection::open(dir.path().join("scratch.db")).unwrap();
    migrations::migrate_to(&mut old, 5).unwrap();
    drop(old);
    db::open(&dir.path().join("scratch.db"), None).unwrap();
    let names: Vec<String> = fs::read_dir(dir.path())
        .unwrap()
        .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert!(names.iter().all(|name| !name.contains(db::BEFORE_UPDATE_SUFFIX)), "{names:?}");
}
