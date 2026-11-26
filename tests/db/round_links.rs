// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 17:09:13.361779
//+ Revised: 	2026/11/24 17:54:07.191279
// -----------------------------------------------------------------------------

//! The rounding links as the file keeps them: refused when they cross classes, name a column that
//! can't be rounded, or leave a column that isn't rounded, and removed with either column they join.
//! Written through raw SQL, so only the file's own rules can refuse them.

use crate::support::count;
use avalio_lite::platform::db;
use rusqlite::Connection;

/// Class "1A" with a test (101), a final grade (102), an info column (103) and a rounded column
/// (104) that rounds the test; class "2B" with a test (201). The names come with each class.
fn two_classes() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A'), (2, '2B');
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (101, 1, 3, 'Exam', 'test', 100, 10), (201, 2, 3, 'Exam', 'test', 100, 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, max_score)
                 VALUES (102, 1, 4, 'Final grade', 'weighted', 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind) VALUES (103, 1, 5, 'Notes', 'info');
             INSERT INTO sheet_columns (id, class_id, position, title, kind, decimals)
                 VALUES (104, 1, 6, 'Rounded', 'round', 0);
             INSERT INTO round_sources VALUES (104, 101);",
        )
        .unwrap();
    (dir, connection)
}

/// The message of the refusal, which must be the file's own, not a failure to run the statement.
fn refusal(connection: &Connection, sql: &str) -> String {
    let error = connection.execute(sql, []).expect_err(sql);
    error.to_string()
}

const REFUSED_LINK: &str = "a rounded column must round a test, final grade or average of its own class";

#[test]
fn a_link_refuses_a_source_of_another_class_on_insert_and_update() {
    let (_dir, connection) = two_classes();
    connection
        .execute("INSERT INTO sheet_columns (id, class_id, position, title, kind, decimals) VALUES (105, 1, 7, 'Other', 'round', 0)", [])
        .unwrap();
    assert!(refusal(&connection, "INSERT INTO round_sources VALUES (105, 201)").contains(REFUSED_LINK));
    assert!(
        refusal(&connection, "UPDATE round_sources SET source_column_id = 201 WHERE column_id = 104")
            .contains(REFUSED_LINK)
    );
    // a link may move to another rounded column of the same class
    connection.execute("UPDATE round_sources SET column_id = 105 WHERE column_id = 104", []).unwrap();
}

#[test]
fn a_link_refuses_a_source_that_is_not_a_test_final_grade_or_average() {
    let (_dir, connection) = two_classes();
    connection
        .execute("INSERT INTO sheet_columns (id, class_id, position, title, kind, decimals) VALUES (105, 1, 7, 'Other', 'round', 0)", [])
        .unwrap();
    let first_name: i64 = connection
        .query_row("SELECT id FROM sheet_columns WHERE class_id = 1 AND kind = 'first_name'", [], |row| row.get(0))
        .unwrap();
    for source in [103, 105, first_name] {
        assert!(
            refusal(&connection, &format!("INSERT INTO round_sources VALUES (105, {source})")).contains(REFUSED_LINK)
        );
    }
}

#[test]
fn a_link_refuses_a_column_that_is_not_rounded() {
    let (_dir, connection) = two_classes();
    assert!(refusal(&connection, "INSERT INTO round_sources VALUES (101, 102)").contains(REFUSED_LINK));
    assert!(
        refusal(&connection, "UPDATE round_sources SET column_id = 102 WHERE column_id = 104").contains(REFUSED_LINK)
    );
}

#[test]
fn a_rounded_column_cannot_leave_its_class_or_stop_being_rounded() {
    let (_dir, connection) = two_classes();
    assert!(
        refusal(&connection, "UPDATE sheet_columns SET class_id = 2 WHERE id = 104").contains("stay in their class")
    );
    assert!(
        refusal(&connection, "UPDATE sheet_columns SET kind = 'info', decimals = NULL WHERE id = 104")
            .contains("stay in their class")
    );
}

#[test]
fn a_source_cannot_leave_its_class_or_change_to_a_kind_it_cannot_be_rounded_as() {
    let (_dir, connection) = two_classes();
    assert!(
        refusal(&connection, "UPDATE sheet_columns SET class_id = 2 WHERE id = 101").contains("stay in their class")
    );
    let to_info = "UPDATE sheet_columns SET kind = 'info', weight = NULL, max_score = NULL WHERE id = 101";
    assert!(refusal(&connection, to_info).contains("stay in their class"));
}

#[test]
fn a_source_may_change_between_test_final_grade_and_average_and_no_op_updates_pass() {
    let (_dir, connection) = two_classes();
    connection
        .execute_batch(
            "UPDATE sheet_columns SET kind = 'weighted', weight = NULL, max_score = 10 WHERE id = 101;
             UPDATE sheet_columns SET class_id = class_id WHERE id = 104;
             UPDATE sheet_columns SET class_id = class_id WHERE id = 101;",
        )
        .unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM round_sources WHERE column_id = 104"), 1, "still linked");
}

#[test]
fn deleting_the_source_removes_the_link_and_keeps_the_rounded_column() {
    let (_dir, connection) = two_classes();
    connection.execute("DELETE FROM sheet_columns WHERE id = 101", []).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM round_sources"), 0, "the link goes with its source");
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM sheet_columns WHERE id = 104 AND decimals = 0"), 1);
}

#[test]
fn deleting_the_rounded_column_removes_its_link_and_keeps_the_source() {
    let (_dir, connection) = two_classes();
    connection.execute("DELETE FROM sheet_columns WHERE id = 104", []).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM round_sources"), 0, "the link goes with the rounded column");
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM sheet_columns WHERE id = 101"), 1);
}

/// Deleting a class deletes its columns, source and rounded column among them, in whatever order
/// the cascade takes them; the other class and its columns stay.
#[test]
fn deleting_a_class_removes_its_links_in_any_cascade_order() {
    let (_dir, connection) = two_classes();
    connection.execute("DELETE FROM classes WHERE id = 1", []).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM round_sources"), 0);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM sheet_columns WHERE class_id = 1"), 0);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM sheet_columns WHERE id = 201"), 1, "2B keeps its test");
}
