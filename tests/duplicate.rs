// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 10:10:25.359630
//+ Revised: 	2026/11/03 08:27:21.284291
// -----------------------------------------------------------------------------

//! Duplicating a class: its columns, and on request its pupils and marks (`classes::duplicate_class`).

use std::collections::HashMap;

use avalio_lite::error::AppError;
use avalio_lite::gradebook::compute::evaluate_row;
use avalio_lite::gradebook::model::{Column, ColumnKind, ColumnType, DuplicateContents, Sheet};
use avalio_lite::gradebook::{cells, classes, columns, pupils, sheet};
use avalio_lite::i18n::Language;
use avalio_lite::platform::db;
use proptest::prelude::*;
use rusqlite::Connection;
use tempfile::TempDir;

const ALL_CONTENTS: [DuplicateContents; 3] =
    [DuplicateContents::Columns, DuplicateContents::ColumnsAndPupils, DuplicateContents::Everything];

fn open() -> (TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    (dir, connection)
}

fn add(connection: &Connection, class_id: i64, column_type: ColumnType) -> i64 {
    columns::add_column(connection, class_id, column_type, Language::English).unwrap()
}

/// "1A": info, a test (weight 60, max 20), a bonus test, a final grade, an average and a rounded
/// final grade with 1 decimal; two pupils with marks.
fn sample_class(connection: &Connection) -> i64 {
    let class = classes::create_class(connection, "1A").unwrap();
    add(connection, class, ColumnType::Info);
    let exam = add(connection, class, ColumnType::Test);
    columns::set_weight(connection, exam, "60").unwrap();
    columns::set_max_score(connection, exam, "20").unwrap();
    let homework = add(connection, class, ColumnType::Test);
    let extra = add(connection, class, ColumnType::Test);
    columns::set_bonus(connection, extra, true).unwrap();
    columns::set_weight(connection, extra, "5").unwrap();
    let final_grade = add(connection, class, ColumnType::Weighted);
    add(connection, class, ColumnType::Average);
    let rounded = add(connection, class, ColumnType::Round);
    columns::set_round_source(connection, rounded, Some(final_grade)).unwrap();
    columns::set_decimals(connection, rounded, "1").unwrap();
    let ana = pupils::add_new_pupil(connection, class, "Ana").unwrap();
    let brais = pupils::add_new_pupil(connection, class, "Brais").unwrap();
    for (pupil, mark) in [(ana, "15"), (brais, "8,5")] {
        cells::set_cell(connection, exam, pupil, mark).unwrap();
        cells::set_cell(connection, homework, pupil, "7").unwrap();
    }
    cells::set_cell(connection, extra, ana, "10").unwrap();
    class
}

/// Columns without their ids, with each rounded column's source given as a position instead.
fn shape(columns: &[Column]) -> Vec<(String, ColumnKind)> {
    let position: HashMap<i64, i64> =
        columns.iter().enumerate().map(|(index, column)| (column.id, index as i64)).collect();
    columns
        .iter()
        .map(|column| {
            let kind = match column.kind {
                ColumnKind::Round { source, decimals } => {
                    ColumnKind::Round { source: source.map(|source| position[&source]), decimals }
                }
                kind => kind,
            };
            (column.title.clone(), kind)
        })
        .collect()
}

/// A pupil's row as compared: id, name and values.
type RowShape = (i64, String, Vec<String>);

/// A sheet without the column ids: column shapes, then each pupil with their values.
fn sheet_shape(sheet: &Sheet) -> (Vec<(String, ColumnKind)>, Vec<RowShape>) {
    let rows = sheet.rows.iter().map(|row| (row.pupil.id, row.pupil.name.clone(), row.values.clone())).collect();
    (shape(&sheet.columns), rows)
}

fn count(connection: &Connection, table: &str) -> i64 {
    connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0)).unwrap()
}

#[test]
fn duplicating_copies_every_column_setting_in_order() {
    let (_dir, connection) = open();
    let class = sample_class(&connection);
    let copy = classes::duplicate_class(&connection, class, DuplicateContents::Columns).unwrap();
    let original = columns::list_columns(&connection, class).unwrap();
    let copied = columns::list_columns(&connection, copy).unwrap();
    assert_eq!(shape(&copied), shape(&original));
    assert!(copied.iter().all(|column| original.iter().all(|old| old.id != column.id)), "new columns");
    assert!(matches!(copied[3].kind, ColumnKind::Test { bonus: true, .. }), "the bonus flag is copied");
}

#[test]
fn a_duplicated_rounding_points_at_the_copys_own_column() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    // one rounding of a column to its left, one of a column to its right
    let exam = add(&connection, class, ColumnType::Test);
    let left = add(&connection, class, ColumnType::Round);
    columns::set_round_source(&connection, left, Some(exam)).unwrap();
    let right = add(&connection, class, ColumnType::Round);
    let average = add(&connection, class, ColumnType::Average);
    columns::set_round_source(&connection, right, Some(average)).unwrap();

    let copy = classes::duplicate_class(&connection, class, DuplicateContents::Columns).unwrap();
    let copied = columns::list_columns(&connection, copy).unwrap();
    assert_eq!(copied[1].kind, ColumnKind::Round { source: Some(copied[0].id), decimals: 0 });
    assert_eq!(copied[2].kind, ColumnKind::Round { source: Some(copied[3].id), decimals: 0 });
}

#[test]
fn duplicating_without_pupils_leaves_the_sheet_empty() {
    let (_dir, connection) = open();
    let class = sample_class(&connection);
    let copy = classes::duplicate_class(&connection, class, DuplicateContents::Columns).unwrap();
    assert!(sheet::load_sheet(&connection, copy).unwrap().rows.is_empty());
}

#[test]
fn duplicating_with_pupils_shares_them_in_order_with_empty_marks() {
    let (_dir, connection) = open();
    let class = sample_class(&connection);
    let copy = classes::duplicate_class(&connection, class, DuplicateContents::ColumnsAndPupils).unwrap();
    let original = pupils::list_class_pupils(&connection, class).unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, copy).unwrap(), original, "the same pupils, in order");
    assert_eq!(count(&connection, "pupils"), 2, "no new pupils");
    let copied = sheet::load_sheet(&connection, copy).unwrap();
    assert!(copied.rows.iter().all(|row| row.values.iter().all(String::is_empty)), "no marks");

    pupils::rename_pupil(&connection, original[0].id, "Ana María").unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, copy).unwrap()[0].name, "Ana María");
    pupils::remove_from_class(&connection, copy, original[1].id).unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, class).unwrap().len(), 2, "the original keeps them");
}

#[test]
fn duplicating_with_marks_gives_the_same_calculated_sheet() {
    let (_dir, connection) = open();
    let class = sample_class(&connection);
    let copy = classes::duplicate_class(&connection, class, DuplicateContents::Everything).unwrap();
    let original = sheet::load_sheet(&connection, class).unwrap();
    let copied = sheet::load_sheet(&connection, copy).unwrap();
    assert_eq!(sheet_shape(&copied), sheet_shape(&original));
    for (old, new) in original.rows.iter().zip(&copied.rows) {
        assert_eq!(evaluate_row(&copied.columns, &new.values), evaluate_row(&original.columns, &old.values));
    }
}

#[test]
fn copies_take_the_lowest_free_number() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let first = classes::duplicate_class(&connection, class, DuplicateContents::Columns).unwrap();
    assert_eq!(classes::class_name(&connection, first).unwrap(), "1A (2)");
    let second = classes::duplicate_class(&connection, class, DuplicateContents::Columns).unwrap();
    assert_eq!(classes::class_name(&connection, second).unwrap(), "1A (3)");
    classes::delete_class(&connection, first).unwrap();
    let third = classes::duplicate_class(&connection, class, DuplicateContents::Columns).unwrap();
    assert_eq!(classes::class_name(&connection, third).unwrap(), "1A (2)", "a freed number is reused");
    assert_eq!(classes::free_class_name(&connection, "2B").unwrap(), "2B", "a free name is kept");
}

#[test]
fn an_empty_class_duplicates_to_an_empty_class() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    for contents in ALL_CONTENTS {
        let copy = classes::duplicate_class(&connection, class, contents).unwrap();
        let copied = sheet::load_sheet(&connection, copy).unwrap();
        assert!(copied.columns.is_empty() && copied.rows.is_empty(), "{contents:?}");
    }
}

#[test]
fn a_failed_duplicate_adds_nothing() {
    let (_dir, connection) = open();
    let class = sample_class(&connection);
    let tables = ["classes", "sheet_columns", "class_pupils", "cells"];
    let before: Vec<i64> = tables.iter().map(|table| count(&connection, table)).collect();
    connection
        .execute_batch("CREATE TEMP TRIGGER no_cells BEFORE INSERT ON cells BEGIN SELECT RAISE(ABORT, 'test'); END")
        .unwrap();
    let error = classes::duplicate_class(&connection, class, DuplicateContents::Everything).unwrap_err();
    assert!(matches!(error, AppError::Database(_)), "{error:?}");
    let after: Vec<i64> = tables.iter().map(|table| count(&connection, table)).collect();
    assert_eq!(after, before);
}

#[test]
fn duplicating_a_missing_class_is_an_error() {
    let (_dir, connection) = open();
    assert!(matches!(
        classes::duplicate_class(&connection, 999, DuplicateContents::Columns),
        Err(AppError::Database(rusqlite::Error::QueryReturnedNoRows))
    ));
    assert_eq!(count(&connection, "classes"), 0);
}

#[test]
fn the_original_is_unchanged() {
    let (_dir, connection) = open();
    let class = sample_class(&connection);
    let before = sheet::load_sheet(&connection, class).unwrap();
    for contents in ALL_CONTENTS {
        classes::duplicate_class(&connection, class, contents).unwrap();
        assert_eq!(sheet::load_sheet(&connection, class).unwrap(), before, "{contents:?}");
    }
}

/// A random typed column: info, or a test with a weight and maximum.
fn typed_column() -> impl Strategy<Value = Option<(u8, u8)>> {
    prop::option::of((0u8..=100, 1u8..=20))
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(32))]

    #[test]
    fn duplicating_everything_reproduces_the_sheet(
        typed in prop::collection::vec(typed_column(), 0..5),
        computed in prop::collection::vec(0u8..3, 0..3),
        marks in prop::collection::vec(prop::collection::vec(prop::option::of("[0-9a-z ,.]{1,4}"), 8), 0..5),
    ) {
        let (_dir, connection) = open();
        let class = classes::create_class(&connection, "1A").unwrap();
        for column in &typed {
            match column {
                None => { add(&connection, class, ColumnType::Info); }
                Some((weight, max)) => {
                    let test = add(&connection, class, ColumnType::Test);
                    columns::set_weight(&connection, test, &weight.to_string()).unwrap();
                    columns::set_max_score(&connection, test, &max.to_string()).unwrap();
                }
            }
        }
        for kind in &computed {
            add(&connection, class, [ColumnType::Weighted, ColumnType::Average, ColumnType::Round][usize::from(*kind)]);
        }
        let column_ids: Vec<i64> = columns::list_columns(&connection, class).unwrap().iter().map(|column| column.id).collect();
        for (index, row) in marks.iter().enumerate() {
            let pupil = pupils::add_new_pupil(&connection, class, &format!("Pupil {index}")).unwrap();
            for (column, mark) in column_ids.iter().take(typed.len()).zip(row) {
                if let Some(mark) = mark {
                    cells::set_cell(&connection, *column, pupil, mark).unwrap();
                }
            }
        }

        let copy = classes::duplicate_class(&connection, class, DuplicateContents::Everything).unwrap();
        let original = sheet::load_sheet(&connection, class).unwrap();
        prop_assert_eq!(sheet_shape(&sheet::load_sheet(&connection, copy).unwrap()), sheet_shape(&original));
    }
}
