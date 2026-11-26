// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.188580
//+ Revised: 	2026/11/23 17:29:55.628583
// -----------------------------------------------------------------------------

//! Column order: moving columns and name columns, and the order kept per class.

use crate::support::{name_ids, open, titles};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::classes;
use avalio_lite::gradebook::columns::{self, Direction};
use avalio_lite::gradebook::model::{ColumnKind, ColumnType};
use avalio_lite::i18n::Language;
use rusqlite::Connection;

/// Every column in order, the names as "<first name>" and "<surname>".
fn order(connection: &Connection, class_id: i64) -> Vec<String> {
    columns::list_columns(connection, class_id)
        .unwrap()
        .into_iter()
        .map(|column| match column.kind {
            ColumnKind::FirstName => "<first name>".to_owned(),
            ColumnKind::Surname => "<surname>".to_owned(),
            _ => column.title,
        })
        .collect()
}

#[test]
fn columns_move_left_and_right_and_stop_at_the_ends() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let first = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    columns::add_column(&connection, class_id, ColumnType::Info, Language::English).unwrap();
    columns::move_column(&connection, first, Direction::Right).unwrap();
    assert_eq!(titles(&connection, class_id), ["Notes", "Test 1"]);
    columns::move_column(&connection, first, Direction::Right).unwrap();
    assert_eq!(titles(&connection, class_id), ["Notes", "Test 1"], "nothing past the last");
    let (first_name, _) = name_ids(&connection, class_id);
    columns::move_column(&connection, first_name, Direction::Left).unwrap();
    assert_eq!(order(&connection, class_id), ["<first name>", "<surname>", "Notes", "Test 1"], "nor before the first");
}

#[test]
fn name_columns_move_left_and_right_like_others() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    let (first_name, surname) = name_ids(&connection, class_id);
    columns::move_column(&connection, surname, Direction::Left).unwrap();
    assert_eq!(order(&connection, class_id), ["<surname>", "<first name>", "Test 1"]);
    columns::move_column(&connection, first_name, Direction::Right).unwrap();
    assert_eq!(order(&connection, class_id), ["<surname>", "Test 1", "<first name>"]);
}

#[test]
fn name_columns_cannot_be_deleted_renamed_or_added() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let (first_name, surname) = name_ids(&connection, class_id);
    assert!(matches!(columns::delete_column(&connection, first_name), Err(AppError::NotEditable)));
    assert!(matches!(columns::rename_column(&connection, surname, "Apelidos"), Err(AppError::NotEditable)));
    for column_type in [ColumnType::FirstName, ColumnType::Surname] {
        let added = columns::add_column(&connection, class_id, column_type, Language::English);
        assert!(matches!(added, Err(AppError::NotEditable)), "{column_type:?}");
    }
    for kind in [ColumnKind::FirstName, ColumnKind::Surname] {
        let inserted = columns::insert_column(&connection, class_id, "Name", kind);
        assert!(matches!(inserted, Err(AppError::NotEditable)));
    }
    assert_eq!(order(&connection, class_id), ["<first name>", "<surname>"], "still exactly one of each");
}

#[test]
fn the_column_order_is_per_class() {
    let (_dir, connection) = open();
    let a = classes::create_class(&connection, "1A").unwrap();
    let b = classes::create_class(&connection, "1B").unwrap();
    for class in [a, b] {
        columns::add_column(&connection, class, ColumnType::Test, Language::English).unwrap();
    }
    let (_, surname) = name_ids(&connection, a);
    columns::move_column(&connection, surname, Direction::Right).unwrap();
    assert_eq!(order(&connection, a), ["<first name>", "Test 1", "<surname>"]);
    assert_eq!(order(&connection, b), ["<first name>", "<surname>", "Test 1"], "1B is unchanged");
}

/// Moving a rounded column changes its place only: its link to the column it rounds stays.
#[test]
fn moving_a_rounded_column_keeps_its_source() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let exam = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    let rounded = columns::add_column(&connection, class_id, ColumnType::Round, Language::English).unwrap();
    columns::set_round_source(&connection, rounded, Some(exam)).unwrap();
    columns::move_column(&connection, rounded, Direction::Left).unwrap();
    columns::move_column(&connection, rounded, Direction::Left).unwrap();
    assert_eq!(order(&connection, class_id), ["<first name>", "Rounded", "<surname>", "Test 1"]);
    let listed = columns::list_columns(&connection, class_id).unwrap();
    assert_eq!(listed[1].kind, ColumnKind::Round { source: Some(exam), decimals: 0 });
}

#[test]
fn round_sources_and_weights_ignore_name_columns() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let (first_name, _) = name_ids(&connection, class_id);
    let rounded = columns::add_column(&connection, class_id, ColumnType::Round, Language::English).unwrap();
    let round_source = |connection: &Connection| match columns::list_columns(connection, class_id)
        .unwrap()
        .into_iter()
        .find(|column| column.id == rounded)
        .unwrap()
        .kind
    {
        ColumnKind::Round { source, .. } => source,
        other => panic!("{other:?}"),
    };
    assert_eq!(round_source(&connection), None, "with no numeric column, a new rounding has no source");
    assert!(matches!(
        columns::set_round_source(&connection, rounded, Some(first_name)),
        Err(AppError::InvalidRoundSource)
    ));
    let test = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    let weight =
        match columns::list_columns(&connection, class_id).unwrap().into_iter().find(|c| c.id == test).unwrap().kind {
            ColumnKind::Test { weight, .. } => weight,
            other => panic!("{other:?}"),
        };
    assert_eq!(weight, 100.0, "the names take no share of the weights");
}

#[test]
fn a_column_moves_to_any_place_in_one_step() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let ids: Vec<i64> = ["Notes", "Exam", "Quiz"]
        .iter()
        .map(|title| {
            let id = columns::add_column(&connection, class_id, ColumnType::Info, Language::English).unwrap();
            columns::rename_column(&connection, id, title).unwrap();
            id
        })
        .collect();
    let (first_name, _) = name_ids(&connection, class_id);
    columns::move_column_to(&connection, first_name, 4).unwrap();
    assert_eq!(order(&connection, class_id), ["<surname>", "Notes", "Exam", "Quiz", "<first name>"], "first to last");
    columns::move_column_to(&connection, first_name, 0).unwrap();
    assert_eq!(order(&connection, class_id), ["<first name>", "<surname>", "Notes", "Exam", "Quiz"], "last to first");
    columns::move_column_to(&connection, ids[2], 2).unwrap();
    assert_eq!(order(&connection, class_id), ["<first name>", "<surname>", "Quiz", "Notes", "Exam"], "to the middle");
    let positions: Vec<i64> = connection
        .prepare("SELECT position FROM sheet_columns WHERE class_id = ?1 ORDER BY position")
        .unwrap()
        .query_map([class_id], |row| row.get(0))
        .unwrap()
        .map(Result::unwrap)
        .collect();
    assert_eq!(positions, [1, 2, 3, 4, 5], "numbered 1..n with no gaps");
}

#[test]
fn moving_a_column_to_its_own_place_changes_nothing() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let exam = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    // a trigger counts writes: moving in place must make none
    connection
        .execute_batch(
            "CREATE TEMP TABLE writes (n INTEGER);
             CREATE TEMP TRIGGER count_writes AFTER UPDATE OF position ON sheet_columns
             BEGIN INSERT INTO writes VALUES (1); END;",
        )
        .unwrap();
    columns::move_column_to(&connection, exam, 2).unwrap();
    let writes: i64 = connection.query_row("SELECT COUNT(*) FROM writes", [], |row| row.get(0)).unwrap();
    assert_eq!(writes, 0);
    assert_eq!(order(&connection, class_id), ["<first name>", "<surname>", "Test 1"]);
}

#[test]
fn an_index_past_the_end_puts_it_last() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    let (first_name, _) = name_ids(&connection, class_id);
    columns::move_column_to(&connection, first_name, 99).unwrap();
    assert_eq!(order(&connection, class_id), ["<surname>", "Test 1", "<first name>"]);
}

#[test]
fn moving_a_column_that_is_gone_is_refused() {
    let (_dir, connection) = open();
    assert!(matches!(columns::move_column_to(&connection, 9999, 0), Err(AppError::NotEditable)));
    assert!(matches!(columns::move_column(&connection, 9999, Direction::Left), Err(AppError::NotEditable)));
}

#[test]
fn moving_in_one_class_leaves_the_other_alone() {
    let (_dir, connection) = open();
    let a = classes::create_class(&connection, "1A").unwrap();
    let b = classes::create_class(&connection, "1B").unwrap();
    let exam = columns::add_column(&connection, a, ColumnType::Test, Language::English).unwrap();
    columns::add_column(&connection, b, ColumnType::Test, Language::English).unwrap();
    columns::move_column_to(&connection, exam, 0).unwrap();
    assert_eq!(order(&connection, a), ["Test 1", "<first name>", "<surname>"]);
    assert_eq!(order(&connection, b), ["<first name>", "<surname>", "Test 1"]);
}
