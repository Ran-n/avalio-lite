// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.084891
//+ Revised: 	2026/11/23 13:00:36.084891
// -----------------------------------------------------------------------------

//! The migration steps: each version upgrades the data it must, and a copy is kept before an update.

use crate::support::{count, pupil_names};
use avalio_lite::error::AppError;
use avalio_lite::platform::db::{self, migrations};
use rusqlite::Connection;
use std::fs;

#[test]
fn a_version_1_database_upgrades_and_keeps_its_data() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("old.db");
    let old = Connection::open(&path).unwrap();
    old.execute_batch(include_str!("../../migrations/001_initial.sql")).unwrap();
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
