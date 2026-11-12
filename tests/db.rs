// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:32.116536
//+ Revised: 	2026/11/12 14:39:56.432186
// -----------------------------------------------------------------------------

use std::fs;

use avalio_lite::error::AppError;
use avalio_lite::platform::db::{self, migrations};
use rusqlite::Connection;

mod common;

const TABLES: &[&str] = &["cells", "class_pupils", "classes", "pupils", "sheet_columns"];

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
fn a_version_1_database_upgrades_and_keeps_its_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    let old = Connection::open(&path).unwrap();
    old.execute_batch(include_str!("../migrations/001_initial.sql")).unwrap();
    old.pragma_update(None, "user_version", 1).unwrap();
    old.execute_batch(
        "INSERT INTO classes (id, name) VALUES (1, '1A');
         INSERT INTO pupils (id, name) VALUES (1, 'Ana');
         INSERT INTO class_pupils VALUES (1, 1, 1);
         INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
             VALUES (1, 1, 1, 'Exam', 'test', 100, 10);
         INSERT INTO cells VALUES (1, 1, '7,5');",
    )
    .unwrap();
    drop(old);

    let upgraded = db::open(&path, None).unwrap();
    assert_eq!(migrations::schema_version(&upgraded).unwrap(), migrations::latest_version());
    let (bonus, value): (bool, String) = upgraded
        .query_row("SELECT sc.bonus, c.value FROM sheet_columns sc JOIN cells c ON c.column_id = sc.id", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert!(!bonus, "existing tests become normal tests");
    assert_eq!(value, "7,5");
}

#[test]
fn only_test_columns_can_be_bonus() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    connection.execute("INSERT INTO classes (id, name) VALUES (1, '1A')", []).unwrap();
    let insert = |kind: &str, bonus: i64| {
        connection.execute(
            "INSERT INTO sheet_columns (class_id, position, title, kind, weight, max_score, bonus)
             VALUES (1, 1, 'x', ?1, CASE ?1 WHEN 'test' THEN 10 END, CASE ?1 WHEN 'test' THEN 10 END, ?2)",
            rusqlite::params![kind, bonus],
        )
    };
    assert!(insert("test", 1).is_ok());
    assert!(insert("info", 1).is_err());
    assert!(insert("test", 2).is_err());
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
fn column_settings_must_match_their_kind() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    connection.execute("INSERT INTO classes (id, name) VALUES (1, '1A')", []).unwrap();
    let insert = |kind: &str, weight: Option<f64>, max: Option<f64>, decimals: Option<i64>| {
        connection.execute(
            "INSERT INTO sheet_columns (class_id, position, title, kind, weight, max_score, decimals)
             VALUES (1, 1, 'x', ?1, ?2, ?3, ?4)",
            rusqlite::params![kind, weight, max, decimals],
        )
    };
    assert!(insert("info", None, None, None).is_ok());
    assert!(insert("info", Some(10.0), None, None).is_err(), "info has no weight");
    assert!(insert("test", Some(40.0), Some(10.0), None).is_ok());
    assert!(insert("test", None, Some(10.0), None).is_err(), "a test needs a weight");
    assert!(insert("test", Some(-1.0), Some(10.0), None).is_err(), "weights can't be negative");
    assert!(insert("test", Some(10.0), Some(0.0), None).is_err(), "the scale must be above 0");
    assert!(insert("weighted", None, Some(10.0), None).is_ok());
    assert!(insert("average", None, None, None).is_err(), "an average needs a scale");
    assert!(insert("round", None, None, Some(1)).is_ok());
    assert!(insert("round", None, None, Some(4)).is_err(), "at most 3 decimals");
    assert!(insert("formula", None, None, None).is_err(), "unknown kinds are refused");
}

#[test]
fn leaving_a_class_removes_only_that_class_marks() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A'), (2, '2B');
             INSERT INTO pupils (id, first_name) VALUES (1, 'Ana');
             INSERT INTO class_pupils VALUES (1, 1, 1), (2, 1, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (101, 1, 3, 'a', 'info'), (102, 2, 3, 'b', 'info');
             INSERT INTO cells VALUES (101, 1, 'in 1A'), (102, 1, 'in 2B');
             DELETE FROM class_pupils WHERE class_id = 1;",
        )
        .unwrap();
    let remaining: Vec<String> = connection
        .prepare("SELECT value FROM cells")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(remaining, ["in 2B"]);
}

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
    let _held = common::hold_exclusively(&path);
    assert!(matches!(db::is_encrypted(&path), Err(AppError::Io { .. })));
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
    let _held = common::hold_exclusively(&path);
    assert!(matches!(db::remove_files(&path), Err(AppError::Io { .. })));
}

/// Every schema version in turn, with data put in as soon as the tables exist: each migration
/// must keep what the earlier versions stored.
#[test]
fn stepping_through_every_migration_keeps_the_data() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    for version in 1..=migrations::latest_version() {
        migrations::migrate_to(&mut connection, version).unwrap();
        assert_eq!(migrations::schema_version(&connection).unwrap(), version);
        if version == 1 {
            connection
                .execute_batch(
                    "INSERT INTO classes (id, name) VALUES (1, '1A');
                     INSERT INTO pupils (id, name) VALUES (1, 'Ana');
                     INSERT INTO class_pupils VALUES (1, 1, 1);
                     INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                         VALUES (1, 1, 1, 'Exam', 'test', 100, 10);
                     INSERT INTO cells VALUES (1, 1, '7,5');",
                )
                .unwrap();
        }
        // the pupil's name is its first name from migration 6 on
        let pupil_name = if version >= 6 { "p.first_name" } else { "p.name" };
        let kept: (String, String, String) = connection
            .query_row(
                &format!(
                    "SELECT c.name, {pupil_name}, v.value FROM classes c, pupils p, cells v WHERE c.id = 1 AND p.id = 1"
                ),
                [],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .unwrap();
        assert_eq!(kept, ("1A".into(), "Ana".into(), "7,5".into()), "after migration {version}");
    }
}

/// Migration 3 drops cells that were never shown: those of pupils outside their column's class.
#[test]
fn migration_3_removes_cells_of_pupils_outside_the_class() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    migrations::migrate_to(&mut connection, 2).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A'), (2, '2B');
             INSERT INTO pupils (id, name) VALUES (1, 'Ana'), (2, 'Bea');
             INSERT INTO class_pupils VALUES (1, 1, 1), (2, 2, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (1, 1, 1, 'Exam', 'test', 100, 10);
             INSERT INTO cells VALUES (1, 1, '7'), (1, 2, '9');",
        )
        .unwrap();
    migrations::migrate_to(&mut connection, 3).unwrap();
    let kept: Vec<i64> = connection
        .prepare("SELECT pupil_id FROM cells")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(kept, [1], "Bea is not in 1A");
}

/// The file refuses what the app never writes: cells outside the class, and rounded columns
/// pointing at another class or at a column that can't be rounded.
#[test]
fn the_schema_enforces_class_membership() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A'), (2, '2B');
             INSERT INTO pupils (id, first_name) VALUES (1, 'Ana'), (2, 'Bea');
             INSERT INTO class_pupils VALUES (1, 1, 1), (2, 2, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (101, 1, 3, 'Exam', 'test', 100, 10), (102, 2, 3, 'Exam', 'test', 100, 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (103, 1, 4, 'Notes', 'info');
             INSERT INTO cells VALUES (101, 1, '7');",
        )
        .unwrap();
    let refused = |sql: &str| connection.execute_batch(sql).is_err();
    assert!(refused("INSERT INTO cells VALUES (101, 2, '9')"), "Bea is not in 1A");
    assert!(refused("UPDATE cells SET pupil_id = 2"), "nor can a cell be moved to her");
    let round = "INSERT INTO sheet_columns (id, class_id, position, title, kind, source_column_id, decimals)
                 VALUES (104, 1, 5, 'Rounded', 'round', ?1, 0)";
    assert!(connection.execute(round, [102]).is_err(), "a column of another class");
    assert!(connection.execute(round, [103]).is_err(), "an info column");
    assert!(connection.execute(round, [1]).is_err(), "a name column");
    connection.execute(round, [101]).unwrap();
    assert!(refused("UPDATE sheet_columns SET source_column_id = 102 WHERE id = 104"));
    connection.execute("DELETE FROM sheet_columns WHERE id = 101", []).unwrap();
    let source: Option<i64> = connection
        .query_row("SELECT source_column_id FROM sheet_columns WHERE id = 104", [], |row| row.get(0))
        .unwrap();
    assert_eq!(source, None, "deleting the source still just unsets it");
}

/// Migration 4 drops cells that were never shown: values stored in calculated columns.
#[test]
fn migration_4_removes_cells_of_calculated_columns() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    migrations::migrate_to(&mut connection, 3).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A');
             INSERT INTO pupils (id, name) VALUES (1, 'Ana');
             INSERT INTO class_pupils VALUES (1, 1, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (1, 1, 1, 'Exam', 'test', 100, 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, max_score)
                 VALUES (2, 1, 2, 'Final grade', 'weighted', 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (3, 1, 3, 'Notes', 'info');
             INSERT INTO cells VALUES (1, 1, '7'), (2, 1, '9'), (3, 1, 'ok');",
        )
        .unwrap();
    migrations::migrate_to(&mut connection, 4).unwrap();
    let kept: Vec<i64> = connection
        .prepare("SELECT column_id FROM cells ORDER BY column_id")
        .unwrap()
        .query_map([], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(kept, [1, 3], "the final grade is calculated, never typed");
}

/// The file refuses typed values in final grade, average and rounded columns.
#[test]
fn the_schema_keeps_calculated_columns_free_of_cells() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A');
             INSERT INTO pupils (id, first_name) VALUES (1, 'Ana');
             INSERT INTO class_pupils VALUES (1, 1, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (101, 1, 3, 'Exam', 'test', 100, 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, max_score)
                 VALUES (102, 1, 4, 'Final grade', 'weighted', 10), (103, 1, 5, 'Average', 'average', 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, source_column_id, decimals)
                 VALUES (104, 1, 6, 'Rounded', 'round', 102, 0);
             INSERT INTO cells VALUES (101, 1, '7');",
        )
        .unwrap();
    let refused = |sql: &str| connection.execute_batch(sql).is_err();
    for column in [102, 103, 104] {
        assert!(refused(&format!("INSERT INTO cells VALUES ({column}, 1, '9')")), "column {column} is calculated");
    }
    assert!(refused("UPDATE cells SET column_id = 102"), "nor can a cell be moved into one");
    connection
        .execute_batch(
            "INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (105, 1, 7, 'Notes', 'info');
             INSERT INTO cells VALUES (105, 1, 'ok')",
        )
        .unwrap();
}

#[test]
fn migrating_a_database_already_at_the_latest_version_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.db");
    let connection = db::open(&path, None).unwrap();
    connection.execute("INSERT INTO pupils (first_name) VALUES ('Ana')", []).unwrap();
    let mut connection = connection;
    migrations::migrate(&mut connection).unwrap();
    migrations::migrate_to(&mut connection, 1).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version(), "never goes back down");
    let pupils: i64 = connection.query_row("SELECT COUNT(*) FROM pupils", [], |row| row.get(0)).unwrap();
    assert_eq!(pupils, 1);
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

#[test]
fn column_widths_are_stored_within_their_limits() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    connection.execute("INSERT INTO classes (id, name) VALUES (1, '1A')", []).unwrap();
    connection
        .execute("INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (101, 1, 3, 'x', 'info')", [])
        .unwrap();
    let column_width =
        |width: Option<i64>| connection.execute("UPDATE sheet_columns SET width = ?1 WHERE id = 101", [width]);
    assert!(column_width(Some(72)).is_ok() && column_width(Some(640)).is_ok());
    assert!(column_width(Some(50)).is_err(), "narrower than the limit is refused");
    assert!(column_width(Some(700)).is_err(), "wider than the limit is refused");
    assert!(column_width(None).is_ok(), "NULL is the default width");
    let name_width = |width: Option<i64>| {
        connection.execute("UPDATE sheet_columns SET width = ?1 WHERE class_id = 1 AND kind = 'first_name'", [width])
    };
    assert!(name_width(Some(72)).is_ok() && name_width(Some(640)).is_ok(), "the names have the same limits");
    assert!(name_width(Some(71)).is_err() && name_width(Some(641)).is_err());
}

/// Migration 6 splits names into first name and surname; names already typed stay whole as the
/// first name, with no surname, since guessing where they split would get some wrong.
#[test]
fn migration_6_keeps_whole_names_as_first_names() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    migrations::migrate_to(&mut connection, 5).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name, name_width) VALUES (1, '1A', 260);
             INSERT INTO pupils (id, name) VALUES (1, 'Xoán Carlos Pérez');
             INSERT INTO class_pupils VALUES (1, 1, 1);",
        )
        .unwrap();
    migrations::migrate_to(&mut connection, 6).unwrap();
    let pupil: (String, String) = connection
        .query_row("SELECT first_name, surname FROM pupils WHERE id = 1", [], |row| Ok((row.get(0)?, row.get(1)?)))
        .unwrap();
    assert_eq!(pupil, ("Xoán Carlos Pérez".to_owned(), String::new()));
    let widths: (Option<i64>, Option<i64>) = connection
        .query_row("SELECT name_width, surname_width FROM classes WHERE id = 1", [], |row| {
            Ok((row.get(0)?, row.get(1)?))
        })
        .unwrap();
    assert_eq!(widths, (Some(260), None), "the first-name width is kept; the surname follows the default");
}

#[test]
fn the_surname_width_is_stored_within_its_limits() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    migrations::migrate_to(&mut connection, 6).unwrap();
    connection.execute("INSERT INTO classes (id, name) VALUES (1, '1A')", []).unwrap();
    let surname_width =
        |width: Option<i64>| connection.execute("UPDATE classes SET surname_width = ?1 WHERE id = 1", [width]);
    assert!(surname_width(Some(72)).is_ok() && surname_width(Some(640)).is_ok());
    assert!(surname_width(Some(71)).is_err(), "narrower than any column is refused");
    assert!(surname_width(Some(641)).is_err(), "wider than any column is refused");
    assert!(surname_width(None).is_ok(), "NULL is the default width");
}

fn count(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

/// Migration 7 makes the names columns of the sheet: it rebuilds sheet_columns, which with foreign
/// keys on would have deleted every cell.
#[test]
fn migration_7_puts_the_name_columns_first_and_keeps_every_cell() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    connection.pragma_update(None, "foreign_keys", true).unwrap();
    migrations::migrate_to(&mut connection, 6).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name, name_width, surname_width) VALUES (1, '1A', 260, 180), (2, '2B', NULL, NULL);
             INSERT INTO pupils (id, first_name, surname) VALUES (1, 'Ana', 'Pérez'), (2, 'Bea', '');
             INSERT INTO class_pupils VALUES (1, 1, 1), (1, 2, 2), (2, 1, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score, width)
                 VALUES (10, 1, 1, 'Exam', 'test', 100, 10, 90);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, max_score)
                 VALUES (11, 1, 2, 'Final grade', 'weighted', 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, source_column_id, decimals)
                 VALUES (12, 1, 3, 'Rounded', 'round', 11, 0);
             INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (20, 2, 1, 'Notes', 'info');
             INSERT INTO cells VALUES (10, 1, '7,5'), (10, 2, '4'), (20, 1, 'shy');",
        )
        .unwrap();
    migrations::migrate_to(&mut connection, 7).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM cells"), 3, "every cell is kept");
    let kinds = |class: i64| -> Vec<(String, i64, Option<i64>)> {
        let mut statement = connection
            .prepare("SELECT kind, position, width FROM sheet_columns WHERE class_id = ?1 ORDER BY position")
            .unwrap();
        statement
            .query_map([class], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            .unwrap()
            .map(Result::unwrap)
            .collect()
    };
    assert_eq!(
        kinds(1),
        [
            ("first_name".to_owned(), 1, Some(260)),
            ("surname".to_owned(), 2, Some(180)),
            ("test".to_owned(), 3, Some(90)),
            ("weighted".to_owned(), 4, None),
            ("round".to_owned(), 5, None),
        ],
        "the names first with the class's widths, the marks shifted"
    );
    assert_eq!(kinds(2)[..2], [("first_name".to_owned(), 1, None), ("surname".to_owned(), 2, None)]);
    let source: Option<i64> =
        connection.query_row("SELECT source_column_id FROM sheet_columns WHERE id = 12", [], |row| row.get(0)).unwrap();
    assert_eq!(source, Some(11), "the rounded column still rounds the final grade");
    let enforced: bool = connection.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap();
    assert!(enforced, "foreign keys are on again");
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM pragma_foreign_key_check"), 0);
    let schema: String = connection
        .query_row("SELECT sql FROM sqlite_master WHERE name = 'sheet_columns'", [], |row| row.get(0))
        .unwrap();
    assert!(schema.contains("REFERENCES sheet_columns (id)") && !schema.contains("sheet_columns_new"), "{schema}");
}

/// The teacher's own database keeps a copy of itself as it was before an update changed its
/// schema, so a migration that rewrote data (008 splits names) can be undone by hand; opening it
/// again, already up to date, makes no new copy.
#[test]
fn opening_an_older_database_keeps_a_copy_from_before_the_update() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("avalio-lite.db");
    let mut old = Connection::open(&path).unwrap();
    migrations::migrate_to(&mut old, 7).unwrap();
    old.execute("INSERT INTO pupils (first_name, surname) VALUES ('Pérez, Ana', '')", []).unwrap();
    drop(old);
    let copy = dir.path().join(format!("avalio-lite.db{}", db::BEFORE_UPDATE_SUFFIX));

    let connection = db::open_upgrading(&path, None).unwrap();
    assert_eq!(pupil_names(&connection), [("Ana".to_owned(), "Pérez".to_owned())]);
    drop(connection);
    let kept = Connection::open(&copy).unwrap();
    assert_eq!(migrations::schema_version(&kept).unwrap(), 7, "the copy is as the file was");
    assert_eq!(pupil_names(&kept), [("Pérez, Ana".to_owned(), String::new())]);
    drop(kept);

    fs::remove_file(&copy).unwrap();
    db::open_upgrading(&path, None).unwrap();
    assert!(!copy.exists(), "an up-to-date database needs no copy");
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

/// The `(first_name, surname)` of every pupil, by id.
fn pupil_names(connection: &Connection) -> Vec<(String, String)> {
    let mut statement = connection.prepare("SELECT first_name, surname FROM pupils ORDER BY id").unwrap();
    statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().map(Result::unwrap).collect()
}

/// Migration 8 splits names typed surname first at their one comma, and leaves every name it
/// can't split without guessing as it was.
#[test]
fn migration_8_splits_names_written_surname_first() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    migrations::migrate_to(&mut connection, 7).unwrap();
    let stored: &[(&str, &str)] = &[
        ("Pérez López, Ana", ""),
        ("Pérez López,Ana", ""),
        ("Pérez, Ana María", ""),
        (" de la Fuente García ,\u{a0}Xoán Carlos\t", ""),
        ("Xoán Carlos Pérez", ""),
        ("Pérez, Ana, B", ""),
        (", Ana", ""),
        ("Pérez,", ""),
        (" , ", ""),
        ("Pérez, Ana", "Gómez"),
    ];
    for (first_name, surname) in stored {
        connection.execute("INSERT INTO pupils (first_name, surname) VALUES (?1, ?2)", [first_name, surname]).unwrap();
    }
    migrations::migrate_to(&mut connection, 8).unwrap();
    let expected: Vec<(String, String)> = [
        ("Ana", "Pérez López"),
        ("Ana", "Pérez López"),
        ("Ana María", "Pérez"),
        ("Xoán Carlos", "de la Fuente García"),
        ("Xoán Carlos Pérez", ""),
        ("Pérez, Ana, B", ""),
        (", Ana", ""),
        ("Pérez,", ""),
        (" , ", ""),
        ("Pérez, Ana", "Gómez"),
    ]
    .iter()
    .map(|(first_name, surname)| ((*first_name).to_owned(), (*surname).to_owned()))
    .collect();
    assert_eq!(pupil_names(&connection), expected);
}

/// Migration 8 changes the names only: the pupil, their classes and their marks stay.
#[test]
fn migration_8_leaves_marks_and_classes_alone() {
    use avalio_lite::gradebook::sheet;
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    connection.pragma_update(None, "foreign_keys", true).unwrap();
    migrations::migrate_to(&mut connection, 7).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A'), (2, '2B');
             INSERT INTO pupils (id, first_name, surname) VALUES (7, 'Pérez, Ana', '');
             INSERT INTO class_pupils VALUES (1, 7, 1), (2, 7, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (10, 1, 3, 'Exam', 'test', 100, 10), (20, 2, 3, 'Quiz', 'test', 100, 10);
             INSERT INTO cells VALUES (10, 7, '7,5'), (20, 7, '9');",
        )
        .unwrap();
    migrations::migrate_to(&mut connection, 8).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM class_pupils WHERE pupil_id = 7"), 2);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM cells WHERE pupil_id = 7"), 2);
    for (class, mark) in [(1, "7,5"), (2, "9")] {
        let loaded = sheet::load_sheet(&connection, class).unwrap();
        assert_eq!(loaded.rows.len(), 1);
        assert_eq!(loaded.rows[0].pupil.id, 7, "the same pupil in class {class}");
        assert_eq!(loaded.rows[0].values, ["Ana", "Pérez", mark], "class {class}");
    }
}

/// Migration 9: the limits typing keeps are kept by the database too, and a class never loses
/// its name columns, except with the class itself.
#[test]
fn migration_9_keeps_column_limits_and_the_name_columns() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    connection.execute("INSERT INTO classes (id, name) VALUES (1, '1A')", []).unwrap();
    let insert = |title: &str, weight: f64, max_score: f64| {
        connection.execute(
            "INSERT INTO sheet_columns (class_id, position, title, kind, weight, max_score) VALUES (1, 3, ?1, 'test', ?2, ?3)",
            rusqlite::params![title, weight, max_score],
        )
    };
    assert!(insert("Exam", 1000.0, 10_000.0).is_ok(), "the limits themselves are allowed");
    assert!(insert("Big", 1000.5, 10.0).is_err(), "a weight over 1000");
    assert!(insert("Huge", 10.0, 10_000.5).is_err(), "a maximum score over 10 000");
    assert!(insert("  ", 10.0, 10.0).is_err(), "no title");
    assert!(connection.execute("UPDATE sheet_columns SET weight = 2000 WHERE title = 'Exam'", []).is_err());
    assert!(connection.execute("UPDATE sheet_columns SET title = '' WHERE title = 'Exam'", []).is_err());

    let delete_name = connection.execute("DELETE FROM sheet_columns WHERE class_id = 1 AND kind = 'surname'", []);
    assert!(delete_name.is_err(), "a class keeps its surname column");
    let into_info =
        connection.execute("UPDATE sheet_columns SET kind = 'info', title = 'x' WHERE kind = 'first_name'", []);
    assert!(into_info.is_err(), "nor turns it into another kind");
    let into_name = connection.execute("UPDATE sheet_columns SET kind = 'surname' WHERE title = 'Exam'", []);
    assert!(into_name.is_err(), "and no column turns into a name column");

    connection.execute("DELETE FROM classes WHERE id = 1", []).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM sheet_columns"), 0, "deleting the class takes them all");
}

#[test]
fn the_migration_runner_turns_foreign_keys_back_on() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = db::open(&dir.path().join("x.db"), None).unwrap();
    let enforced = |connection: &Connection| -> bool {
        connection.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap()
    };
    assert!(enforced(&connection), "on once the database is open and migrated");
    let failed = migrations::apply_unrecorded(&mut connection, 99, "SELECT * FROM no_such_table;");
    assert!(matches!(failed, Err(AppError::Database(_))), "{failed:?}");
    assert!(enforced(&connection), "on again after a failing migration too");
}

#[test]
fn a_migration_that_breaks_a_foreign_key_is_rolled_back() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = db::open(&dir.path().join("x.db"), None).unwrap();
    connection.execute("INSERT INTO classes (id, name) VALUES (1, '1A')", []).unwrap();
    // with foreign keys off nothing stops a row pointing at a class that isn't there
    let broken = migrations::apply_unrecorded(
        &mut connection,
        99,
        "INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (42, 9, 'x', 'info');",
    );
    let message = format!("{broken:?}");
    assert!(matches!(broken, Err(AppError::Database(_))), "{message}");
    assert!(message.contains("migration 99") && message.contains("sheet_columns"), "names the migration: {message}");
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM sheet_columns WHERE class_id = 42"), 0, "rolled back");
}

#[test]
fn every_class_has_one_column_of_each_name_kind() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    connection.execute("INSERT INTO classes (id, name) VALUES (1, '1A')", []).unwrap();
    assert_eq!(
        count(
            &connection,
            "SELECT COUNT(*) FROM sheet_columns WHERE class_id = 1 AND kind IN ('first_name', 'surname')"
        ),
        2,
        "a new class comes with both"
    );
    let second = connection
        .execute("INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (1, 3, '', 'first_name')", []);
    assert!(second.is_err(), "a second first-name column is refused");
}

#[test]
fn name_columns_hold_no_settings() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    connection.execute("INSERT INTO classes (id, name) VALUES (1, '1A')", []).unwrap();
    let set = |sql: &str| connection.execute(&format!("UPDATE sheet_columns SET {sql} WHERE kind = 'surname'"), []);
    assert!(set("weight = 10").is_err(), "no weight");
    assert!(set("max_score = 10").is_err(), "no maximum");
    assert!(set("title = 'Apelidos'").is_err(), "no title of its own");
    assert!(set("bonus = 1").is_err());
    connection.execute("INSERT INTO pupils (id, first_name) VALUES (1, 'Ana')", []).unwrap();
    connection.execute("INSERT INTO class_pupils VALUES (1, 1, 1)", []).unwrap();
    let cell =
        connection.execute("INSERT INTO cells SELECT id, 1, 'Ana' FROM sheet_columns WHERE kind = 'first_name'", []);
    assert!(cell.is_err(), "and no cells: the names live with the pupil");
}
