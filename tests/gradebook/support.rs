// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.723640
//+ Revised: 	2026/11/23 13:00:35.723640
// -----------------------------------------------------------------------------

//! Helpers shared by the gradebook tests: fixtures, titles and numbered columns.

use avalio_lite::gradebook::columns::{self};
use avalio_lite::gradebook::model::ColumnKind;
use avalio_lite::platform::db;
use rusqlite::Connection;
use tempfile::TempDir;

pub(crate) fn open() -> (TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    (dir, connection)
}

/// The mark columns' titles in order, without the first-name and surname columns.
pub(crate) fn titles(connection: &Connection, class_id: i64) -> Vec<String> {
    columns::list_columns(connection, class_id)
        .unwrap()
        .into_iter()
        .filter(|column| !column.kind.is_name())
        .map(|column| column.title)
        .collect()
}

/// The ids of the class's first-name and surname columns.
pub(crate) fn name_ids(connection: &Connection, class_id: i64) -> (i64, i64) {
    let listed = columns::list_columns(connection, class_id).unwrap();
    let id = |kind: ColumnKind| listed.iter().find(|column| column.kind == kind).unwrap().id;
    (id(ColumnKind::FirstName), id(ColumnKind::Surname))
}
