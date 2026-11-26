// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.002818
//+ Revised: 	2026/11/23 13:00:35.002818
// -----------------------------------------------------------------------------

//! Cells: typed values, calculated columns, and the sheets of each class.

use crate::support::open;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::columns::{self};
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, classes, pupils, sheet};
use avalio_lite::i18n::Language;

/// A row's values in the mark columns only.
fn marks(sheet: &avalio_lite::gradebook::model::Sheet, row: usize) -> Vec<String> {
    sheet
        .columns
        .iter()
        .zip(&sheet.rows[row].values)
        .filter(|(column, _)| !column.kind.is_name())
        .map(|(_, value)| value.clone())
        .collect()
}

#[test]
fn cells_store_typed_values_and_clear_when_emptied() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let ana = pupils::add_new_pupil(&connection, class_id, "Ana", "").unwrap();
    let test = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    let notes = columns::add_column(&connection, class_id, ColumnType::Info, Language::English).unwrap();
    cells::set_cell(&connection, test, ana, "7,5").unwrap();
    cells::set_cell(&connection, notes, ana, "shy").unwrap();
    cells::set_cell(&connection, notes, ana, "very shy").unwrap();
    assert_eq!(marks(&sheet::load_sheet(&connection, class_id).unwrap(), 0), ["7,5", "very shy"]);
    cells::set_cell(&connection, test, ana, "  ").unwrap();
    assert_eq!(marks(&sheet::load_sheet(&connection, class_id).unwrap(), 0), ["", "very shy"]);
}

#[test]
fn calculated_columns_cannot_be_typed_into() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let ana = pupils::add_new_pupil(&connection, class_id, "Ana", "").unwrap();
    let final_grade = columns::add_column(&connection, class_id, ColumnType::Weighted, Language::English).unwrap();
    assert!(matches!(cells::set_cell(&connection, final_grade, ana, "10"), Err(AppError::NotEditable)));
    assert!(matches!(cells::set_cell(&connection, 9999, ana, "10"), Err(AppError::NotEditable)));
}

#[test]
fn sheets_only_show_their_own_class() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    let in_a = columns::add_column(&connection, a, ColumnType::Info, Language::English).unwrap();
    let in_b = columns::add_column(&connection, b, ColumnType::Info, Language::English).unwrap();
    cells::set_cell(&connection, in_a, ana, "A note").unwrap();
    cells::set_cell(&connection, in_b, ana, "B note").unwrap();
    let sheet_b = sheet::load_sheet(&connection, b).unwrap();
    assert_eq!(sheet_b.columns.len(), 3, "its names and its one column");
    assert_eq!(marks(&sheet_b, 0), ["B note"]);
}

#[test]
fn removing_a_pupil_from_one_class_keeps_them_and_their_other_marks() {
    let (_dir, connection) = open();
    let first = classes::create_class(&connection, "1A").unwrap();
    let second = classes::create_class(&connection, "2B").unwrap();
    let ana = pupils::add_new_pupil(&connection, first, "Ana", "").unwrap();
    pupils::add_existing_pupil(&connection, second, ana).unwrap();
    let exam_1a = columns::add_column(&connection, first, ColumnType::Test, Language::English).unwrap();
    let exam_2b = columns::add_column(&connection, second, ColumnType::Test, Language::English).unwrap();
    cells::set_cell(&connection, exam_1a, ana, "7").unwrap();
    cells::set_cell(&connection, exam_2b, ana, "9").unwrap();

    pupils::remove_from_class(&connection, first, ana).unwrap();
    assert!(pupils::list_class_pupils(&connection, first).unwrap().is_empty());
    let other = sheet::load_sheet(&connection, second).unwrap();
    assert_eq!(other.rows[0].pupil.first_name, "Ana");
    assert_eq!(marks(&other, 0), ["9"]);
    let marks_left: i64 = connection.query_row("SELECT COUNT(*) FROM cells", [], |row| row.get(0)).unwrap();
    assert_eq!(marks_left, 1, "only the 1A mark went");
}

#[test]
fn regression_a_cell_for_a_pupil_outside_the_class_is_refused() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let test = columns::add_column(&connection, a, ColumnType::Test, Language::English).unwrap();
    let outsider = pupils::add_new_pupil(&connection, b, "Bea", "").unwrap();
    assert!(matches!(cells::set_cell(&connection, test, outsider, "7"), Err(AppError::NotEditable)));
    // had the cell been stored, it would surface as soon as the pupil joined the class
    pupils::add_existing_pupil(&connection, a, outsider).unwrap();
    assert_eq!(marks(&sheet::load_sheet(&connection, a).unwrap(), 0), [""]);
    cells::set_cell(&connection, test, outsider, "7").unwrap();
    cells::set_cell(&connection, test, outsider, "8").unwrap();
    assert_eq!(marks(&sheet::load_sheet(&connection, a).unwrap(), 0), ["8"]);
}
