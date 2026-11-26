// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 17:09:13.529648
//+ Revised: 	2026/11/24 17:54:10.906363
// -----------------------------------------------------------------------------

//! Rounding links: which column a rounded column rounds, how the link is set, changed, cleared and
//! shown on the next load, and what happens to it when either column goes.

use crate::support::open;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::classes;
use avalio_lite::gradebook::columns;
use avalio_lite::gradebook::model::{ColumnKind, ColumnType};
use avalio_lite::i18n::Language;
use rusqlite::Connection;

#[test]
fn round_source_must_be_a_numeric_column_of_the_same_class() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let test = columns::add_column(&connection, a, ColumnType::Test, Language::English).unwrap();
    let info = columns::add_column(&connection, a, ColumnType::Info, Language::English).unwrap();
    let round = columns::add_column(&connection, a, ColumnType::Round, Language::English).unwrap();
    let other_round = columns::add_column(&connection, a, ColumnType::Round, Language::English).unwrap();
    let foreign = columns::add_column(&connection, b, ColumnType::Test, Language::English).unwrap();

    for bad in [info, other_round, foreign, round] {
        assert!(matches!(columns::set_round_source(&connection, round, Some(bad)), Err(AppError::InvalidRoundSource)));
    }
    columns::set_round_source(&connection, round, Some(test)).unwrap();
    columns::set_round_source(&connection, round, None).unwrap();
}

/// The source a rounded column shows when the sheet is loaded again, `None` when it has none.
fn rounding_source(connection: &Connection, class_id: i64, rounded: i64) -> Option<i64> {
    let listed = columns::list_columns(connection, class_id).unwrap();
    match listed.iter().find(|column| column.id == rounded).unwrap().kind {
        ColumnKind::Round { source, .. } => source,
        other => panic!("{other:?}"),
    }
}

/// A rounding's source is stored in its own link, so setting, changing and clearing it each show
/// on the next load, and a refused source leaves the link as it was.
#[test]
fn a_rounding_source_is_set_changed_and_cleared_and_shows_on_load() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let exam = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    let average = columns::add_column(&connection, class_id, ColumnType::Average, Language::English).unwrap();
    let notes = columns::add_column(&connection, class_id, ColumnType::Info, Language::English).unwrap();
    let rounded = columns::add_column(&connection, class_id, ColumnType::Round, Language::English).unwrap();
    assert_eq!(rounding_source(&connection, class_id, rounded), Some(average), "a new rounding takes the last grade");

    columns::set_round_source(&connection, rounded, Some(exam)).unwrap();
    assert_eq!(rounding_source(&connection, class_id, rounded), Some(exam), "changed");
    assert!(matches!(columns::set_round_source(&connection, rounded, Some(notes)), Err(AppError::InvalidRoundSource)));
    assert_eq!(rounding_source(&connection, class_id, rounded), Some(exam), "a refused source keeps the link");
    columns::set_round_source(&connection, rounded, None).unwrap();
    assert_eq!(rounding_source(&connection, class_id, rounded), None, "cleared");
    columns::set_round_source(&connection, rounded, None).unwrap();
    assert_eq!(rounding_source(&connection, class_id, rounded), None, "clearing again changes nothing");
}

/// A rounded column can't be added with a source of another class: the refusal comes before the
/// column is written, so no half-made column is left behind.
#[test]
fn a_new_rounded_column_with_a_foreign_source_adds_nothing() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let foreign = columns::add_column(&connection, b, ColumnType::Test, Language::English).unwrap();
    let before = columns::list_columns(&connection, a).unwrap().len();
    let kind = ColumnKind::Round { source: Some(foreign), decimals: 0 };
    let result = columns::insert_column(&connection, a, "Rounded", kind);
    assert!(matches!(result, Err(AppError::InvalidRoundSource)), "{result:?}");
    assert_eq!(columns::list_columns(&connection, a).unwrap().len(), before, "nothing was added");
}

#[test]
fn deleting_a_rounded_columns_source_leaves_it_unset() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let test = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    columns::add_column(&connection, class_id, ColumnType::Round, Language::English).unwrap();
    columns::delete_column(&connection, test).unwrap();
    assert_eq!(
        columns::list_columns(&connection, class_id).unwrap()[2].kind,
        ColumnKind::Round { source: None, decimals: 0 }
    );
}
