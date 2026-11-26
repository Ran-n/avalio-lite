// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.470621
//+ Revised: 	2026/11/23 13:00:36.470621
// -----------------------------------------------------------------------------

//! Helpers shared by the database tests: the table list, pupil names and row counts.

use rusqlite::Connection;

pub(crate) fn count(connection: &Connection, sql: &str) -> i64 {
    connection.query_row(sql, [], |row| row.get(0)).unwrap()
}

/// The `(first_name, surname)` of every pupil, by id.
pub(crate) fn pupil_names(connection: &Connection) -> Vec<(String, String)> {
    let mut statement = connection.prepare("SELECT first_name, surname FROM pupils ORDER BY id").unwrap();
    statement.query_map([], |row| Ok((row.get(0)?, row.get(1)?))).unwrap().map(Result::unwrap).collect()
}
