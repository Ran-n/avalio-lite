// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.368158
//+ Revised: 	2026/11/23 17:29:49.278483
// -----------------------------------------------------------------------------

//! The schema rules: what the file itself refuses, checked through raw SQL.

use crate::support::count;
use avalio_lite::platform::db::{self};

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

/// The file refuses cells outside their column's class, and a cell moved to another class's pupil.
/// Rounding links, which also join classes, are in `round_links`.
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
             INSERT INTO cells VALUES (101, 1, '7');",
        )
        .unwrap();
    let refused = |sql: &str| connection.execute_batch(sql).is_err();
    assert!(refused("INSERT INTO cells VALUES (101, 2, '9')"), "Bea is not in 1A");
    assert!(refused("UPDATE cells SET pupil_id = 2"), "nor can a cell be moved to her");
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
             INSERT INTO sheet_columns (id, class_id, position, title, kind, decimals)
                 VALUES (104, 1, 6, 'Rounded', 'round', 0);
             INSERT INTO round_sources VALUES (104, 102);
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
