// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.354347
//+ Revised: 	2026/11/23 13:00:35.354347
// -----------------------------------------------------------------------------

//! Column widths: stored, clamped to their limits and reset.

use crate::support::{name_ids, open, titles};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::classes;
use avalio_lite::gradebook::columns::{self};
use avalio_lite::gradebook::model::{ColumnTitles, ColumnType, MAX_COLUMN_WIDTH, MIN_COLUMN_WIDTH};
use avalio_lite::i18n::Language;
use rusqlite::Connection;

/// Words every title as its type and number, so a test sees exactly what the gradebook asked for.
struct Numbered;

impl ColumnTitles for Numbered {
    fn default_title(&self, column_type: ColumnType, number: usize) -> String {
        format!("{column_type:?}#{number}")
    }
}

#[test]
fn add_column_names_columns_through_the_given_titles() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let first = columns::add_column(&connection, class_id, ColumnType::Test, Numbered).unwrap();
    columns::add_column(&connection, class_id, ColumnType::Test, Numbered).unwrap();
    assert_eq!(titles(&connection, class_id), ["Test#1", "Test#2"]);
    // one test is left, so the next would be number 2, which is taken: the gradebook moves on to 3
    columns::delete_column(&connection, first).unwrap();
    columns::add_column(&connection, class_id, ColumnType::Test, Numbered).unwrap();
    columns::add_column(&connection, class_id, ColumnType::Info, Numbered).unwrap();
    assert_eq!(titles(&connection, class_id), ["Test#2", "Test#3", "Info#1"]);
}

#[test]
fn a_column_width_is_stored_clamped_and_reset() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let exam = columns::add_column(&connection, class, ColumnType::Test, Language::English).unwrap();
    let width = |connection: &Connection| columns::list_columns(connection, class).unwrap()[2].width;
    assert_eq!(width(&connection), None, "a new column has its kind's default width");
    columns::set_column_width(&connection, exam, Some(200)).unwrap();
    assert_eq!(width(&connection), Some(200));
    columns::set_column_width(&connection, exam, Some(10)).unwrap();
    assert_eq!(width(&connection), Some(MIN_COLUMN_WIDTH));
    columns::set_column_width(&connection, exam, Some(900)).unwrap();
    assert_eq!(width(&connection), Some(MAX_COLUMN_WIDTH));
    columns::set_column_width(&connection, exam, None).unwrap();
    assert_eq!(width(&connection), None, "back to the default");
    assert!(matches!(columns::set_column_width(&connection, 9999, Some(200)), Err(AppError::NotEditable)));
}

#[test]
fn a_name_column_width_is_stored_clamped_and_reset() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let other = classes::create_class(&connection, "2B").unwrap();
    let (first_name, surname) = name_ids(&connection, class);
    let width =
        |id: i64| columns::list_columns(&connection, class).unwrap().into_iter().find(|c| c.id == id).unwrap().width;
    assert_eq!((width(first_name), width(surname)), (None, None), "a new class's names have their defaults");
    columns::set_column_width(&connection, first_name, Some(300)).unwrap();
    assert_eq!((width(first_name), width(surname)), (Some(300), None), "each name column has its own");
    let other_widths: Vec<Option<u16>> =
        columns::list_columns(&connection, other).unwrap().iter().map(|c| c.width).collect();
    assert_eq!(other_widths, [None, None], "and each class");
    columns::set_column_width(&connection, surname, Some(10)).unwrap();
    assert_eq!(width(surname), Some(MIN_COLUMN_WIDTH), "the same limits as any column");
    columns::set_column_width(&connection, surname, Some(900)).unwrap();
    assert_eq!(width(surname), Some(MAX_COLUMN_WIDTH));
    columns::set_column_width(&connection, first_name, None).unwrap();
    assert_eq!(width(first_name), None);
}

#[test]
fn widths_are_clamped_to_their_own_minimum() {
    use avalio_lite::gradebook::model::clamp_width;
    assert_eq!(clamp_width(None, MIN_COLUMN_WIDTH), None);
    assert_eq!(clamp_width(Some(0), MIN_COLUMN_WIDTH), Some(MIN_COLUMN_WIDTH));
    assert_eq!(clamp_width(Some(100), MIN_COLUMN_WIDTH), Some(100));
    assert_eq!(clamp_width(Some(u16::MAX), MIN_COLUMN_WIDTH), Some(MAX_COLUMN_WIDTH));
}
