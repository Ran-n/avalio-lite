// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.173993
//+ Revised: 	2026/11/23 18:33:52.199130
// -----------------------------------------------------------------------------

//! The migrations that split and place names: 7 to 10 keep every pupil and cell.

use crate::support::{count, pupil_names};
use avalio_lite::platform::db::{self, migrations};
use rusqlite::Connection;

/// A version 6 database with two classes, their names and marks, and a rounded column: the state
/// migration 7 starts from. The migration is run by each test, so its effect is visible in the test.
fn version_six_database() -> (tempfile::TempDir, Connection) {
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
    (dir, connection)
}

/// The columns of `class` in order, each as its kind, position and width.
fn columns_of(connection: &Connection, class: i64) -> Vec<(String, i64, Option<i64>)> {
    let mut statement = connection
        .prepare("SELECT kind, position, width FROM sheet_columns WHERE class_id = ?1 ORDER BY position")
        .unwrap();
    statement
        .query_map([class], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
        .unwrap()
        .map(Result::unwrap)
        .collect()
}

/// Migration 7 makes the names columns of the sheet: it rebuilds sheet_columns, which with foreign
/// keys on would have deleted every cell.
#[test]
fn migration_7_puts_the_name_columns_first_and_keeps_every_cell() {
    let (_dir, mut connection) = version_six_database();
    migrations::migrate_to(&mut connection, 7).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM cells"), 3, "every cell is kept");
    assert_eq!(
        columns_of(&connection, 1),
        [
            ("first_name".to_owned(), 1, Some(260)),
            ("surname".to_owned(), 2, Some(180)),
            ("test".to_owned(), 3, Some(90)),
            ("weighted".to_owned(), 4, None),
            ("round".to_owned(), 5, None),
        ],
        "the names first with the class's widths, the marks shifted"
    );
    assert_eq!(columns_of(&connection, 2)[..2], [("first_name".to_owned(), 1, None), ("surname".to_owned(), 2, None)]);
}

#[test]
fn migration_7_keeps_the_rounded_column_rounding_the_final_grade() {
    let (_dir, mut connection) = version_six_database();
    migrations::migrate_to(&mut connection, 7).unwrap();
    let source: Option<i64> =
        connection.query_row("SELECT source_column_id FROM sheet_columns WHERE id = 12", [], |row| row.get(0)).unwrap();
    assert_eq!(source, Some(11), "the rounded column still rounds the final grade");
}

#[test]
fn migration_7_leaves_foreign_keys_on_and_no_dangling_reference() {
    let (_dir, mut connection) = version_six_database();
    migrations::migrate_to(&mut connection, 7).unwrap();
    let enforced: bool = connection.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap();
    assert!(enforced, "foreign keys are on again");
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM pragma_foreign_key_check"), 0);
}

#[test]
fn migration_7_points_the_references_at_the_rebuilt_table() {
    let (_dir, mut connection) = version_six_database();
    migrations::migrate_to(&mut connection, 7).unwrap();
    let schema: String = connection
        .query_row("SELECT sql FROM sqlite_master WHERE name = 'sheet_columns'", [], |row| row.get(0))
        .unwrap();
    assert!(schema.contains("REFERENCES sheet_columns (id)") && !schema.contains("sheet_columns_new"), "{schema}");
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
    // the sheet reads the rounding links of migration 11, so the marks are checked at the latest version
    migrations::migrate(&mut connection).unwrap();
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

/// Migration 10: a class always has a name and a pupil a first name, also for a write that
/// doesn't go through the app's own check; a surname may be empty. Rows from before are kept.
#[test]
fn migration_10_keeps_classes_and_pupils_named() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    migrations::migrate_to(&mut connection, 9).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, ' ');
             INSERT INTO pupils (id, first_name) VALUES (1, '');",
        )
        .unwrap();
    migrations::migrate(&mut connection).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM classes"), 1, "a nameless class from before is kept");
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM pupils"), 1, "and so is a nameless pupil");

    let class = |name: &str| connection.execute("INSERT INTO classes (name) VALUES (?1)", [name]);
    assert!(class("1A").is_ok());
    assert!(class("").is_err(), "no name");
    assert!(class("   ").is_err(), "only spaces");
    assert!(connection.execute("UPDATE classes SET name = '' WHERE name = '1A'", []).is_err());
    assert!(connection.execute("UPDATE classes SET name = '1B' WHERE name = '1A'", []).is_ok());

    let pupil = |first_name: &str, surname: &str| {
        connection.execute("INSERT INTO pupils (first_name, surname) VALUES (?1, ?2)", [first_name, surname])
    };
    assert!(pupil("Ana", "").is_ok(), "a surname may be empty");
    assert!(pupil("", "Pérez").is_err(), "no first name");
    assert!(pupil("  ", "Pérez").is_err(), "only spaces");
    assert!(connection.execute("UPDATE pupils SET first_name = ' ' WHERE first_name = 'Ana'", []).is_err());
    assert!(connection.execute("UPDATE pupils SET surname = '' WHERE first_name = 'Ana'", []).is_ok());
    // the nameless rows from before can still be renamed, and their other fields changed
    assert!(connection.execute("UPDATE pupils SET surname = 'x' WHERE id = 1", []).is_ok());
    assert!(connection.execute("UPDATE pupils SET first_name = 'Bea' WHERE id = 1", []).is_ok());
    assert!(connection.execute("UPDATE classes SET name = '2A' WHERE id = 1", []).is_ok());
}
