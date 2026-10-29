// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:32.116536
//+ Revised: 	2026/10/29 00:10:41.430443
// -----------------------------------------------------------------------------

use std::fs;

use avalio_lite::db::{self, migrations};
use avalio_lite::error::AppError;
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
    connection.execute("INSERT INTO pupils (name) VALUES ('Ana Pérez')", []).unwrap();
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
             INSERT INTO pupils (id, name) VALUES (1, 'Ana');
             INSERT INTO class_pupils VALUES (1, 1, 1), (2, 1, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (1, 1, 1, 'a', 'info'), (2, 2, 1, 'b', 'info');
             INSERT INTO cells VALUES (1, 1, 'in 1A'), (2, 1, 'in 2B');
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
    connection.execute("INSERT INTO pupils (name) VALUES ('Ana Pérez')", []).unwrap();
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
        connection.execute("INSERT INTO pupils (name) VALUES (?1)", [format!("Pupil number {index}")]).unwrap();
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
        let kept: (String, String, String) = connection
            .query_row(
                "SELECT c.name, p.name, v.value FROM classes c, pupils p, cells v WHERE c.id = 1 AND p.id = 1",
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
             INSERT INTO pupils (id, name) VALUES (1, 'Ana'), (2, 'Bea');
             INSERT INTO class_pupils VALUES (1, 1, 1), (2, 2, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (1, 1, 1, 'Exam', 'test', 100, 10), (2, 2, 1, 'Exam', 'test', 100, 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (3, 1, 2, 'Notes', 'info');
             INSERT INTO cells VALUES (1, 1, '7');",
        )
        .unwrap();
    let refused = |sql: &str| connection.execute_batch(sql).is_err();
    assert!(refused("INSERT INTO cells VALUES (1, 2, '9')"), "Bea is not in 1A");
    assert!(refused("UPDATE cells SET pupil_id = 2"), "nor can a cell be moved to her");
    let round = "INSERT INTO sheet_columns (id, class_id, position, title, kind, source_column_id, decimals)
                 VALUES (4, 1, 3, 'Rounded', 'round', ?1, 0)";
    assert!(connection.execute(round, [2]).is_err(), "a column of another class");
    assert!(connection.execute(round, [3]).is_err(), "an info column");
    connection.execute(round, [1]).unwrap();
    assert!(refused("UPDATE sheet_columns SET source_column_id = 2 WHERE id = 4"));
    connection.execute("DELETE FROM sheet_columns WHERE id = 1", []).unwrap();
    let source: Option<i64> =
        connection.query_row("SELECT source_column_id FROM sheet_columns WHERE id = 4", [], |row| row.get(0)).unwrap();
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
             INSERT INTO pupils (id, name) VALUES (1, 'Ana');
             INSERT INTO class_pupils VALUES (1, 1, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (1, 1, 1, 'Exam', 'test', 100, 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, max_score)
                 VALUES (2, 1, 2, 'Final grade', 'weighted', 10), (3, 1, 3, 'Average', 'average', 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, source_column_id, decimals)
                 VALUES (4, 1, 4, 'Rounded', 'round', 2, 0);
             INSERT INTO cells VALUES (1, 1, '7');",
        )
        .unwrap();
    let refused = |sql: &str| connection.execute_batch(sql).is_err();
    for column in [2, 3, 4] {
        assert!(refused(&format!("INSERT INTO cells VALUES ({column}, 1, '9')")), "column {column} is calculated");
    }
    assert!(refused("UPDATE cells SET column_id = 2"), "nor can a cell be moved into one");
    connection
        .execute_batch(
            "INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (5, 1, 5, 'Notes', 'info');
                              INSERT INTO cells VALUES (5, 1, 'ok')",
        )
        .unwrap();
}

#[test]
fn migrating_a_database_already_at_the_latest_version_changes_nothing() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.db");
    let connection = db::open(&path, None).unwrap();
    connection.execute("INSERT INTO pupils (name) VALUES ('Ana')", []).unwrap();
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
