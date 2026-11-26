// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.818040
//+ Revised: 	2026/11/23 13:00:35.818040
// -----------------------------------------------------------------------------

//! Stored column widths: each one kept within its limits.

use avalio_lite::platform::db::{self, migrations};
use rusqlite::Connection;

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
