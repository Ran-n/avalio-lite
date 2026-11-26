// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:37.012854
//+ Revised: 	2026/11/23 17:29:55.772636
// -----------------------------------------------------------------------------

//! Helpers shared by the backup tests: fixtures, the backup PIN and class lookups.

use crate::common::Fixture;
use avalio_lite::gradebook::model::Sheet;
use avalio_lite::gradebook::{classes, sheet};
use avalio_lite::platform::db::migrations;
use avalio_lite::state::AppState;
use rusqlite::Connection;
use std::path::PathBuf;

pub(crate) const PIN: &str = "246810";

/// A database with the sample class and a second, empty one.
pub(crate) fn filled(fixture: &Fixture) -> AppState {
    let state = fixture.set_up();
    let connection = state.connection().unwrap();
    crate::common::sample_class(connection);
    classes::create_class(connection, "1º ESO B").unwrap();
    state
}

/// Every class with its whole sheet (columns, pupils in order, typed values), by name.
pub(crate) fn everything(connection: &Connection) -> Vec<(String, Sheet)> {
    let mut all = Vec::new();
    for class in classes::list_classes(connection).unwrap() {
        all.push((class.name, sheet::load_sheet(connection, class.id).unwrap()));
    }
    all
}

// ---------------------------------------------------------------------------------------------
// Restoring

/// The `.restore` copy the app opens a backup through, next to the database.
pub(crate) fn restore_copy(fixture: &Fixture) -> PathBuf {
    PathBuf::from(format!("{}.restore", fixture.database().display()))
}

/// A backup of a filled database of its own, encrypted with `pin` when given.
pub(crate) fn backup_of_another(pin: Option<&str>) -> (Fixture, PathBuf) {
    let other = Fixture::new();
    let mut state = filled(&other);
    if let Some(pin) = pin {
        state.set_pin(pin, pin).unwrap();
    }
    let file = other.path("theirs.avalio");
    state.export_backup(&file).unwrap();
    state.shutdown();
    (other, file)
}

/// A version 10 backup file: class "1A" with a test (101), a final grade (102) and a rounded column
/// (103) that rounds the final grade to 2 decimals, and an empty class "2B". Version 10 let the final
/// grade move to "2B" (`UPDATE ... SET class_id` on the source), which left the rounding pointing
/// across classes, the one link the migration to 11 drops.
pub(crate) fn version_ten_backup_across_classes(fixture: &Fixture) -> PathBuf {
    let file = fixture.path("old.avalio");
    let mut connection = Connection::open(&file).unwrap();
    migrations::migrate_to(&mut connection, 10).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A'), (2, '2B');
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (101, 1, 3, 'Exam', 'test', 100, 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, max_score)
                 VALUES (102, 1, 4, 'Final grade', 'weighted', 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, source_column_id, decimals)
                 VALUES (103, 1, 5, 'Rounded', 'round', 102, 2);
             UPDATE sheet_columns SET class_id = 2 WHERE id = 102;",
        )
        .unwrap();
    file
}

pub(crate) fn class_names(state: &AppState) -> Vec<String> {
    classes::list_classes(state.connection().unwrap()).unwrap().into_iter().map(|class| class.name).collect()
}
