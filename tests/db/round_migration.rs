// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 17:09:13.447115
//+ Revised: 	2026/11/23 17:29:56.073010
// -----------------------------------------------------------------------------

//! Migration 11, the rounding links: a version 10 file keeps each link and its decimals, a link the
//! new rules refuse is dropped with a logged warning, and every other trigger and index stays.

use crate::support::count;
use avalio_lite::gradebook::columns;
use avalio_lite::gradebook::model::ColumnKind;
use avalio_lite::platform::db::migrations;
use log::{Level, LevelFilter, Log, Metadata, Record};
use rusqlite::Connection;
use std::collections::BTreeSet;
use std::sync::{Mutex, Once};

/// The warnings the migrations log, as `capture_warnings` keeps them.
static WARNINGS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static CAPTURE: Capture = Capture;

struct Capture;

impl Log for Capture {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() == Level::Warn
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            WARNINGS.lock().unwrap().push(record.args().to_string());
        }
    }

    fn flush(&self) {}
}

/// Installs the capture once for this test binary. Tests look for their own message, so the
/// messages of the others, which use other ids, don't matter.
fn capture_warnings() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        log::set_logger(&CAPTURE).unwrap();
        log::set_max_level(LevelFilter::Warn);
    });
}

/// A version 10 file: class "1A" with a test (101), a final grade (102), a rounded column (103) that
/// rounds the final grade to 2 decimals, and an unlinked rounded column (104); one mark in the test.
fn version_ten_file(dir: &tempfile::TempDir) -> Connection {
    let mut connection = Connection::open(dir.path().join("x.db")).unwrap();
    migrations::migrate_to(&mut connection, 10).unwrap();
    connection
        .execute_batch(
            "INSERT INTO classes (id, name) VALUES (1, '1A'), (2, '2B');
             INSERT INTO pupils (id, first_name) VALUES (1, 'Ana');
             INSERT INTO class_pupils VALUES (1, 1, 1);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, weight, max_score)
                 VALUES (101, 1, 3, 'Exam', 'test', 100, 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, max_score)
                 VALUES (102, 1, 4, 'Final grade', 'weighted', 10);
             INSERT INTO sheet_columns (id, class_id, position, title, kind, source_column_id, decimals)
                 VALUES (103, 1, 5, 'Rounded', 'round', 102, 2), (104, 1, 6, 'Plain', 'round', NULL, 0);
             INSERT INTO cells VALUES (101, 1, '7,5');",
        )
        .unwrap();
    connection
}

/// The rounded columns of class 1 with their kinds, which show their source and decimals.
fn rounded_columns(connection: &Connection) -> Vec<(i64, ColumnKind)> {
    columns::list_columns(connection, 1)
        .unwrap()
        .into_iter()
        .filter(|column| matches!(column.kind, ColumnKind::Round { .. }))
        .map(|column| (column.id, column.kind))
        .collect()
}

/// The triggers and indexes of the file, by kind and name.
fn schema_objects(connection: &Connection) -> BTreeSet<String> {
    let mut statement = connection
        .prepare("SELECT type || ' ' || name FROM sqlite_master WHERE type IN ('trigger', 'index') AND name NOT LIKE 'sqlite_%'")
        .unwrap();
    statement.query_map([], |row| row.get(0)).unwrap().map(Result::unwrap).collect()
}

#[test]
fn a_version_10_file_keeps_each_link_and_its_decimals() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = version_ten_file(&dir);
    migrations::migrate(&mut connection).unwrap();
    assert_eq!(migrations::schema_version(&connection).unwrap(), migrations::latest_version());
    assert_eq!(
        rounded_columns(&connection),
        [
            (103, ColumnKind::Round { source: Some(102), decimals: 2 }),
            (104, ColumnKind::Round { source: None, decimals: 0 })
        ],
        "the link and the decimals survive; the unlinked column has no row"
    );
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM round_sources"), 1);
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM cells WHERE column_id = 101"), 1, "marks are kept");
}

/// Version 10 let a source move to another class (`UPDATE ... SET class_id` on the source only), so
/// its rounded column pointed across classes. The migration drops that link, keeps the decimals,
/// and says so in the log, rather than failing and locking the file.
#[test]
fn a_link_across_classes_is_dropped_and_logged_and_the_file_opens() {
    capture_warnings();
    let dir = tempfile::tempdir().unwrap();
    let mut connection = version_ten_file(&dir);
    connection.execute("UPDATE sheet_columns SET class_id = 2 WHERE id = 102", []).unwrap();
    migrations::migrate(&mut connection).unwrap();
    assert_eq!(
        rounded_columns(&connection)[0],
        (103, ColumnKind::Round { source: None, decimals: 2 }),
        "the rounded column keeps its decimals and no source"
    );
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM pragma_foreign_key_check"), 0);
    let logged = WARNINGS.lock().unwrap().iter().any(|message| {
        message
            == "schema migration 11: rounded column 103 lost its link to column 102: it must round a test, \
                    final grade or average of its own class"
    });
    assert!(logged, "the dropped link is logged as a warning");
}

/// Version 10 let a source change to a kind that can't be rounded (an info column, here) while its
/// rounded column still pointed at it.
#[test]
fn a_link_to_a_source_that_is_no_longer_rounded_as_a_test_is_dropped() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = version_ten_file(&dir);
    connection.execute("UPDATE sheet_columns SET kind = 'info', max_score = NULL WHERE id = 102", []).unwrap();
    migrations::migrate(&mut connection).unwrap();
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM round_sources WHERE column_id = 103"), 0);
    assert_eq!(rounded_columns(&connection)[0], (103, ColumnKind::Round { source: None, decimals: 2 }));
}

/// Migration 11 rebuilds `sheet_columns`, so the triggers and indexes that were on it, or named it,
/// are made again. Only the rounding ones are replaced.
#[test]
fn migration_11_keeps_every_other_trigger_and_index() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = version_ten_file(&dir);
    let before = schema_objects(&connection);
    migrations::migrate(&mut connection).unwrap();
    let replaced =
        ["trigger round_source_on_insert", "trigger round_source_on_update", "index idx_sheet_columns_source"];
    let added = [
        "index idx_round_sources_source",
        "trigger round_sources_keep_their_columns",
        "trigger round_sources_on_insert",
        "trigger round_sources_on_update",
    ];
    let expected: BTreeSet<String> = before
        .iter()
        .filter(|object| !replaced.contains(&object.as_str()))
        .cloned()
        .chain(added.iter().map(|object| (*object).to_owned()))
        .collect();
    assert_eq!(schema_objects(&connection), expected);
}

#[test]
fn migration_11_leaves_foreign_keys_on_and_no_dangling_reference() {
    let dir = tempfile::tempdir().unwrap();
    let mut connection = version_ten_file(&dir);
    migrations::migrate(&mut connection).unwrap();
    let enforced: bool = connection.pragma_query_value(None, "foreign_keys", |row| row.get(0)).unwrap();
    assert!(enforced, "foreign keys are on again");
    assert_eq!(count(&connection, "SELECT COUNT(*) FROM pragma_foreign_key_check"), 0);
}
