// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:18.120661
//+ Revised: 	2026/11/25 11:43:11.674561
// -----------------------------------------------------------------------------

//! Migration 12: the pictures' table, what the file itself refuses, and the data from before it.

use crate::support::count;
use avalio_lite::platform::db::{self, migrations};
use rusqlite::Connection;

fn version_eleven() -> (tempfile::TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = Connection::open(dir.path().join("old.db")).unwrap();
    migrations::migrate_to(&mut connection, 11).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A'); INSERT INTO pupils (id, first_name) VALUES (7, 'Ana');",
        )
        .unwrap();
    (dir, connection)
}

#[test]
fn the_pictures_table_is_added_empty_and_keeps_the_pupils() {
    let (_dir, mut connection) = version_eleven();
    migrations::migrate_to(&mut connection, 12).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), 12);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM pupils"), 1);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM pupil_pictures"), 0);
}

#[test]
fn the_file_refuses_what_is_not_a_small_picture_of_a_pupil() {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("x.db"), None).unwrap();
    connection.execute_batch("INSERT INTO pupils (id, first_name) VALUES (7, 'Ana')").unwrap();
    for refused in [
        "INSERT INTO pupil_pictures VALUES (7, 'text')",
        "INSERT INTO pupil_pictures VALUES (7, x'')",
        "INSERT INTO pupil_pictures VALUES (7, zeroblob(262145))",
        "INSERT INTO pupil_pictures VALUES (7, NULL)",
        "INSERT INTO pupil_pictures VALUES (99, x'00')",
    ] {
        assert!(connection.execute_batch(refused).is_err(), "{refused}");
    }
    connection.execute_batch("INSERT INTO pupil_pictures VALUES (7, zeroblob(262144))").unwrap();
    assert!(connection.execute_batch("INSERT INTO pupil_pictures VALUES (7, x'00')").is_err(), "one each");
    connection.execute_batch("DELETE FROM pupils WHERE id = 7").unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM pupil_pictures"), 0, "gone with the pupil");
}
