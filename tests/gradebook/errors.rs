// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.434431
//+ Revised: 	2026/11/24 02:56:19.602589
// -----------------------------------------------------------------------------

//! Storage errors: missing tables, damaged cells, unknown kinds and the deletion of what is gone.

use crate::support::open;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::columns::{self, Direction};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, classes, pupils, sheet};
use avalio_lite::i18n::Language;
use rusqlite::Connection;

/// A file without the app's tables: every query fails, and the failure comes back as a database
/// error instead of an empty list or a silent no-op.
#[test]
fn queries_on_a_file_without_the_tables_are_database_errors() {
    let connection = Connection::open_in_memory().unwrap();
    assert!(matches!(classes::list_classes(&connection), Err(AppError::Database(_))));
    assert!(matches!(classes::create_class(&connection, "1A"), Err(AppError::Database(_))));
    assert!(matches!(columns::list_columns(&connection, 1), Err(AppError::Database(_))));
    assert!(matches!(columns::set_round_source(&connection, 1, None), Err(AppError::Database(_))));
    assert!(matches!(columns::move_column(&connection, 1, Direction::Left), Err(AppError::Database(_))));
    assert!(matches!(cells::set_cell(&connection, 1, 1, "7"), Err(AppError::Database(_))));
    assert!(matches!(pupils::pupils_in_no_class(&connection), Err(AppError::Database(_))));
    assert!(matches!(pupils::add_new_pupil(&connection, 1, "Ana", ""), Err(AppError::Database(_))));
}

#[test]
fn a_rounded_column_whose_link_cannot_be_stored_is_a_database_error() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let test = columns::add_column(&connection, class, ColumnType::Test, Language::English).unwrap();
    let rounded = columns::add_column(&connection, class, ColumnType::Round, Language::English).unwrap();
    connection.execute_batch("DROP TABLE round_sources").unwrap();
    for source in [Some(test), None] {
        let result = columns::set_round_source(&connection, rounded, source);
        assert!(matches!(result, Err(AppError::Database(_))), "{source:?}: {result:?}");
    }
}

#[test]
fn a_column_for_a_class_that_is_gone_is_refused_and_nothing_is_added() {
    let (_dir, connection) = open();
    let result = columns::add_column(&connection, 999, ColumnType::Test, Language::English);
    assert!(matches!(result, Err(AppError::Database(_))), "{result:?}");
    let columns: i64 = connection.query_row("SELECT COUNT(*) FROM sheet_columns", [], |row| row.get(0)).unwrap();
    assert_eq!(columns, 0);
}

#[test]
fn a_sheet_whose_cells_table_is_damaged_is_a_database_error() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    pupils::add_new_pupil(&connection, class, "Ana", "").unwrap();
    connection.execute_batch("DROP TABLE cells").unwrap();
    assert!(matches!(sheet::load_sheet(&connection, class), Err(AppError::Database(_))));
}

#[test]
fn an_unknown_column_kind_in_the_file_is_reported() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    // a file from a newer or damaged version; the CHECK constraint is bypassed to write it
    connection.pragma_update(None, "ignore_check_constraints", true).unwrap();
    connection
        .execute(
            "INSERT INTO sheet_columns (class_id, position, title, kind) VALUES (?1, 1, 'Mystery', 'mystery')",
            [class],
        )
        .unwrap();
    let error = columns::list_columns(&connection, class).unwrap_err();
    assert!(error.to_string().contains("unknown column kind \"mystery\""), "{error}");
}

#[test]
fn deleting_what_is_gone_is_reported() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let column = columns::add_column(&connection, class, ColumnType::Info, Language::English).unwrap();
    let pupil = pupils::add_new_pupil(&connection, class, "Ana", "").unwrap();
    pupils::remove_from_class(&connection, class, pupil).unwrap();
    assert!(matches!(pupils::remove_from_class(&connection, class, pupil), Err(AppError::NotEditable)));
    columns::delete_column(&connection, column).unwrap();
    assert!(matches!(columns::delete_column(&connection, column), Err(AppError::NotEditable)));
    classes::delete_class(&connection, class).unwrap();
    assert!(matches!(classes::delete_class(&connection, class), Err(AppError::NotEditable)));
}
