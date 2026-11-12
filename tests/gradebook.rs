// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:55.006271
//+ Revised: 	2026/11/11 23:00:49.335269
// -----------------------------------------------------------------------------

use avalio_lite::error::AppError;
use avalio_lite::gradebook::columns::{self, Direction};
use avalio_lite::gradebook::model::{ColumnKind, ColumnTitles, ColumnType, MAX_COLUMN_WIDTH, MIN_COLUMN_WIDTH};
use avalio_lite::gradebook::{cells, classes, pupils, sheet};
use avalio_lite::i18n::Language;
use avalio_lite::platform::db;
use rusqlite::Connection;
use tempfile::TempDir;

fn open() -> (TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    (dir, connection)
}

/// The mark columns' titles in order, without the first-name and surname columns.
fn titles(connection: &Connection, class_id: i64) -> Vec<String> {
    columns::list_columns(connection, class_id)
        .unwrap()
        .into_iter()
        .filter(|column| !column.kind.is_name())
        .map(|column| column.title)
        .collect()
}

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

/// The class's columns without the first-name and surname columns.
fn mark_columns(connection: &Connection, class_id: i64) -> Vec<avalio_lite::gradebook::model::Column> {
    columns::list_columns(connection, class_id).unwrap().into_iter().filter(|column| !column.kind.is_name()).collect()
}

/// The ids of the class's first-name and surname columns.
fn name_ids(connection: &Connection, class_id: i64) -> (i64, i64) {
    let listed = columns::list_columns(connection, class_id).unwrap();
    let id = |kind: ColumnKind| listed.iter().find(|column| column.kind == kind).unwrap().id;
    (id(ColumnKind::FirstName), id(ColumnKind::Surname))
}

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
fn classes_are_listed_by_name_with_pupil_counts() {
    let (_dir, connection) = open();
    let b = classes::create_class(&connection, " 2B ").unwrap();
    let a = classes::create_class(&connection, "1A").unwrap();
    pupils::add_new_pupil(&connection, b, "Ana", "").unwrap();
    let listed = classes::list_classes(&connection).unwrap();
    assert_eq!(
        listed.iter().map(|class| (class.id, class.name.as_str(), class.pupil_count)).collect::<Vec<_>>(),
        [(a, "1A", 0), (b, "2B", 1)]
    );
}

#[test]
fn class_names_must_be_unique_and_non_empty() {
    let (_dir, connection) = open();
    let id = classes::create_class(&connection, "1A").unwrap();
    assert!(matches!(classes::create_class(&connection, "1A"), Err(AppError::DuplicateClassName(_))));
    assert!(matches!(classes::create_class(&connection, "   "), Err(AppError::EmptyName)));
    let other = classes::create_class(&connection, "2B").unwrap();
    assert!(matches!(classes::rename_class(&connection, other, "1A"), Err(AppError::DuplicateClassName(_))));
    classes::rename_class(&connection, id, "1A bis").unwrap();
    assert_eq!(classes::class_name(&connection, id).unwrap(), "1A bis");
}

#[test]
fn adjacent_class_wraps_around() {
    use classes::adjacent_index;
    assert_eq!(adjacent_index(0, None, 1), None, "no classes, nothing to switch to");
    assert_eq!(adjacent_index(0, None, -1), None);
    assert_eq!(adjacent_index(1, Some(0), 1), Some(0), "one class stays selected");
    assert_eq!(adjacent_index(1, Some(0), -1), Some(0));
    assert_eq!(adjacent_index(3, Some(0), 1), Some(1));
    assert_eq!(adjacent_index(3, Some(2), 1), Some(0), "past the last goes round to the first");
    assert_eq!(adjacent_index(3, Some(0), -1), Some(2), "before the first goes round to the last");
    assert_eq!(adjacent_index(3, Some(1), -1), Some(0));
    assert_eq!(adjacent_index(3, None, 1), Some(0), "with none selected, next is the first");
    assert_eq!(adjacent_index(3, None, -1), Some(2), "and previous the last");
}

#[test]
fn deleting_a_class_keeps_its_pupils_in_other_classes() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    classes::delete_class(&connection, a).unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, b).unwrap()[0].first_name, "Ana");
}

#[test]
fn pupils_are_shared_and_renamed_everywhere() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "").unwrap();
    assert_eq!(pupils::pupils_not_in_class(&connection, b).unwrap()[0].id, ana);
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    assert!(pupils::pupils_not_in_class(&connection, b).unwrap().is_empty());
    assert!(pupils::add_existing_pupil(&connection, b, ana).is_err(), "can't be added twice");
    pupils::set_first_name(&connection, ana, "Ana Pérez").unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, a).unwrap()[0].first_name, "Ana Pérez");
    assert!(matches!(pupils::set_first_name(&connection, ana, " "), Err(AppError::EmptyName)));
}

#[test]
fn pupils_keep_the_order_they_were_added_in() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    for name in ["Xoán", "Ana", "Brais"] {
        pupils::add_new_pupil(&connection, class_id, name, "").unwrap();
    }
    let names: Vec<String> =
        pupils::list_class_pupils(&connection, class_id).unwrap().into_iter().map(|pupil| pupil.first_name).collect();
    assert_eq!(names, ["Xoán", "Ana", "Brais"]);
}

/// A column's weight, maximum, rounding source and decimals, as stored.
type StoredFields = (Option<f64>, Option<f64>, Option<i64>, Option<u8>);

#[test]
fn new_columns_get_sensible_defaults() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    for column_type in [
        ColumnType::Test,
        ColumnType::Test,
        ColumnType::Weighted,
        ColumnType::Round,
        ColumnType::Info,
        ColumnType::Average,
    ] {
        columns::add_column(&connection, class_id, column_type, Language::English).unwrap();
    }
    let listed = mark_columns(&connection, class_id);
    assert_eq!(titles(&connection, class_id), ["Test 1", "Test 2", "Final grade", "Rounded", "Notes", "Average"]);
    assert_eq!(listed[0].kind, ColumnKind::Test { weight: 100.0, max_score: 10.0, bonus: false });
    assert_eq!(
        listed[1].kind,
        ColumnKind::Test { weight: 0.0, max_score: 10.0, bonus: false },
        "weight left to reach 100 %"
    );
    assert_eq!(listed[2].kind, ColumnKind::Weighted { max_score: 10.0 });
    assert_eq!(listed[3].kind, ColumnKind::Round { source: Some(listed[2].id), decimals: 0 });
    assert_eq!(listed[4].kind, ColumnKind::Info);
    assert_eq!(listed[5].kind, ColumnKind::Average { max_score: 10.0 });
    // kills the kind_fields mutants that store a field the kind doesn't read back
    let mut statement = connection
        .prepare("SELECT weight, max_score, source_column_id, decimals FROM sheet_columns \n             WHERE kind NOT IN ('first_name', 'surname') ORDER BY position")
        .unwrap();
    let stored: Vec<StoredFields> = statement
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)))
        .unwrap()
        .collect::<rusqlite::Result<_>>()
        .unwrap();
    assert_eq!(
        stored,
        [
            (Some(100.0), Some(10.0), None, None),
            (Some(0.0), Some(10.0), None, None),
            (None, Some(10.0), None, None),
            (None, None, Some(listed[2].id), Some(0)),
            (None, None, None, None),
            (None, Some(10.0), None, None),
        ]
    );
}

#[test]
fn new_column_titles_follow_the_interface_language() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    for column_type in [ColumnType::Test, ColumnType::Test, ColumnType::Weighted, ColumnType::Info] {
        columns::add_column(&connection, class_id, column_type, Language::Galician).unwrap();
    }
    assert_eq!(titles(&connection, class_id), ["Proba 1", "Proba 2", "Nota final", "Observacións"]);
}

#[test]
fn tests_can_become_bonus_tests_and_new_tests_ignore_bonus_weight() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let exam = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    columns::set_weight(&connection, exam, "70").unwrap();
    let extra = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    columns::set_bonus(&connection, extra, true).unwrap();
    columns::set_weight(&connection, extra, "10").unwrap();
    columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();

    let listed = mark_columns(&connection, class_id);
    assert_eq!(listed[1].kind, ColumnKind::Test { weight: 10.0, max_score: 10.0, bonus: true });
    assert_eq!(
        listed[2].kind,
        ColumnKind::Test { weight: 30.0, max_score: 10.0, bonus: false },
        "70 + 30, bonus not counted"
    );

    columns::set_bonus(&connection, extra, false).unwrap();
    assert_eq!(
        columns::list_columns(&connection, class_id).unwrap()[3].kind,
        ColumnKind::Test { weight: 10.0, max_score: 10.0, bonus: false }
    );
}

#[test]
fn only_tests_can_be_bonus() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let notes = columns::add_column(&connection, class_id, ColumnType::Info, Language::English).unwrap();
    assert!(matches!(columns::set_bonus(&connection, notes, true), Err(AppError::NotEditable)));
}

#[test]
fn column_settings_are_validated() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let test = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    let info = columns::add_column(&connection, class_id, ColumnType::Info, Language::English).unwrap();
    let round = columns::add_column(&connection, class_id, ColumnType::Round, Language::English).unwrap();

    columns::set_weight(&connection, test, "40,5").unwrap();
    columns::set_max_score(&connection, test, "5").unwrap();
    assert_eq!(
        columns::list_columns(&connection, class_id).unwrap()[2].kind,
        ColumnKind::Test { weight: 40.5, max_score: 5.0, bonus: false }
    );
    assert!(matches!(columns::set_weight(&connection, test, "-1"), Err(AppError::InvalidNumber { .. })));
    assert!(matches!(columns::set_max_score(&connection, test, "0"), Err(AppError::InvalidNumber { .. })));
    assert!(matches!(columns::set_weight(&connection, info, "10"), Err(AppError::NotEditable)));
    assert!(matches!(columns::set_decimals(&connection, round, "4"), Err(AppError::InvalidNumber { .. })));
    columns::set_decimals(&connection, round, "2").unwrap();
    assert!(matches!(columns::rename_column(&connection, test, ""), Err(AppError::EmptyName)));
}

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
fn renaming_a_class_to_another_classes_name_is_refused() {
    let (_dir, connection) = open();
    classes::create_class(&connection, "1A").unwrap();
    let second = classes::create_class(&connection, "2B").unwrap();
    let result = classes::rename_class(&connection, second, "1A");
    assert!(matches!(result, Err(AppError::DuplicateClassName(ref name)) if name == "1A"), "{result:?}");
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

#[test]
fn regression_renaming_something_that_no_longer_exists_is_reported() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let pupil = pupils::add_new_pupil(&connection, class, "Ana", "").unwrap();
    classes::delete_class(&connection, class).unwrap();
    connection.execute("DELETE FROM pupils WHERE id = ?1", [pupil]).unwrap();
    assert!(matches!(classes::rename_class(&connection, class, "2B"), Err(AppError::NotEditable)));
    assert!(matches!(pupils::set_first_name(&connection, pupil, "Bea"), Err(AppError::NotEditable)));
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(32))]

    /// Any sequence of moves only reorders the columns: none is lost, duplicated or renamed,
    /// and the positions stay distinct.
    #[test]
    fn moving_columns_only_reorders_them(moves in proptest::collection::vec((0usize..4, proptest::bool::ANY), 0..12)) {
        let (_dir, connection) = open();
        let class = classes::create_class(&connection, "1A").unwrap();
        let ids: Vec<i64> = (0..4)
            .map(|_| columns::add_column(&connection, class, ColumnType::Info, Language::English).unwrap())
            .collect();
        let mut expected = titles(&connection, class);
        for (index, right) in moves {
            let id = ids[index];
            let direction = if right { Direction::Right } else { Direction::Left };
            columns::move_column(&connection, id, direction).unwrap();
            let current = titles(&connection, class);
            let mut sorted_before = expected.clone();
            let mut sorted_after = current.clone();
            sorted_before.sort();
            sorted_after.sort();
            proptest::prop_assert_eq!(sorted_before, sorted_after);
            expected = current;
        }
        let positions: Vec<i64> = connection
            .prepare("SELECT position FROM sheet_columns WHERE class_id = ?1")
            .unwrap()
            .query_map([class], |row| row.get(0))
            .unwrap()
            .map(Result::unwrap)
            .collect();
        let distinct: std::collections::BTreeSet<i64> = positions.iter().copied().collect();
        proptest::prop_assert_eq!(distinct.len(), positions.len());
    }

    #[test]
    fn names_are_trimmed_and_blank_ones_refused(name in r"\PC{0,30}") {
        let (_dir, connection) = open();
        match classes::create_class(&connection, &name) {
            Ok(id) => {
                proptest::prop_assert!(!name.trim().is_empty());
                proptest::prop_assert_eq!(classes::class_name(&connection, id).unwrap(), name.trim());
            }
            Err(error) => {
                proptest::prop_assert!(name.trim().is_empty());
                proptest::prop_assert!(matches!(error, AppError::EmptyName));
            }
        }
    }
}

#[test]
fn weights_and_maximums_have_an_upper_bound() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let exam = columns::add_column(&connection, class, ColumnType::Test, Language::English).unwrap();
    columns::set_weight(&connection, exam, &columns::MAX_WEIGHT.to_string()).unwrap();
    columns::set_max_score(&connection, exam, &columns::MAX_SCORE_LIMIT.to_string()).unwrap();
    for weight in ["1000.5", "1e308", "-1"] {
        let result = columns::set_weight(&connection, exam, weight);
        assert!(matches!(&result, Err(AppError::InvalidNumber { value, .. }) if value == weight), "{weight}");
    }
    for max in ["10000.01", "0", "1e308"] {
        let result = columns::set_max_score(&connection, exam, max);
        assert!(matches!(&result, Err(AppError::InvalidNumber { value, .. }) if value == max), "{max}");
    }
    let kind = columns::list_columns(&connection, class).unwrap()[2].kind;
    assert_eq!(
        kind,
        ColumnKind::Test { weight: columns::MAX_WEIGHT, max_score: columns::MAX_SCORE_LIMIT, bonus: false }
    );
}

#[test]
fn regression_a_new_test_never_repeats_a_title() {
    // the number was "tests so far + 1": after deleting Test 1, a second "Test 2" appeared
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let first = columns::add_column(&connection, class, ColumnType::Test, Language::English).unwrap();
    columns::add_column(&connection, class, ColumnType::Test, Language::English).unwrap();
    columns::delete_column(&connection, first).unwrap();
    columns::add_column(&connection, class, ColumnType::Test, Language::English).unwrap();
    assert_eq!(titles(&connection, class), ["Test 2", "Test 3"]);
    // a renamed column taking the next default is skipped too
    let renamed = columns::add_column(&connection, class, ColumnType::Info, Language::English).unwrap();
    columns::rename_column(&connection, renamed, "test 4").unwrap();
    columns::add_column(&connection, class, ColumnType::Test, Language::English).unwrap();
    assert_eq!(titles(&connection, class).last().unwrap(), "Test 5");
}

#[test]
fn regression_accented_names_sort_with_their_letter() {
    // SQLite's NOCASE folds only A to Z: "Álvaro" and "Ánxela" were listed after "Zoe"
    let (_dir, connection) = open();
    for name in ["Zoo", "árabe", "Ñu", "Nube", "Oca", "Abeto"] {
        classes::create_class(&connection, name).unwrap();
    }
    let names: Vec<String> = classes::list_classes(&connection).unwrap().into_iter().map(|class| class.name).collect();
    assert_eq!(names, ["Abeto", "árabe", "Nube", "Ñu", "Oca", "Zoo"]);

    let other = classes::create_class(&connection, "Other").unwrap();
    let here = classes::create_class(&connection, "Here").unwrap();
    for name in ["Zoe", "Ánxela", "brais", "Álvaro"] {
        pupils::add_new_pupil(&connection, other, name, "").unwrap();
    }
    let candidates: Vec<String> =
        pupils::pupils_not_in_class(&connection, here).unwrap().into_iter().map(|pupil| pupil.first_name).collect();
    assert_eq!(candidates, ["Álvaro", "Ánxela", "brais", "Zoe"]);
}

#[test]
fn regression_names_match_ignoring_case_in_any_alphabet() {
    // "álvaro" didn't find "Álvaro": NOCASE leaves Á and á apart
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let alvaro = pupils::add_new_pupil(&connection, class, "Álvaro Núñez", "").unwrap();
    pupils::add_new_pupil(&connection, class, "Alvaro Nunez", "").unwrap();
    let found: Vec<i64> =
        pupils::pupils_named(&connection, "  ÁLVARO NÚÑEZ ").unwrap().into_iter().map(|pupil| pupil.id).collect();
    assert_eq!(found, [alvaro], "case is ignored, accents are not");
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

mod name_keys {
    use avalio_lite::gradebook::model::{name_match_key, name_sort_key};
    use proptest::prelude::*;

    #[test]
    fn accents_fold_and_n_tilde_stays_after_n() {
        assert_eq!(name_sort_key("Ánxela"), name_sort_key("anxela"));
        assert_eq!(name_sort_key("A\u{301}lvaro"), name_sort_key("Álvaro"), "an accent typed as a mark of its own");
        assert_eq!(name_sort_key("n\u{303}u"), name_sort_key("ñu"));
        assert_eq!(name_sort_key("Ýago"), name_sort_key("yago"));
        assert!(name_sort_key("nz") < name_sort_key("ña") && name_sort_key("ña") < name_sort_key("o"));
    }

    proptest! {
        #[test]
        fn case_never_changes_the_order_or_the_match(name in "[a-zA-Zá-úÁ-Úñ ]{0,20}") {
            prop_assert_eq!(name_sort_key(&name.to_uppercase()), name_sort_key(&name.to_lowercase()));
            prop_assert_eq!(name_match_key(&name.to_uppercase()), name_match_key(&name.to_lowercase()));
            prop_assert_eq!(name_match_key(&format!("  {name} ")), name_match_key(&name));
        }
    }
}

#[test]
fn a_pupil_in_no_class_can_be_deleted_and_only_then() {
    let (_dir, connection) = open();
    let a = classes::create_class(&connection, "1A").unwrap();
    let b = classes::create_class(&connection, "1B").unwrap();
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    let brais = pupils::add_new_pupil(&connection, b, "Brais", "").unwrap();
    assert!(pupils::pupils_in_no_class(&connection).unwrap().is_empty());

    pupils::remove_from_class(&connection, a, ana).unwrap();
    assert!(matches!(pupils::delete_pupil_in_no_class(&connection, ana), Err(AppError::NotEditable)), "still in 1B");
    // deleting a class leaves its pupils in no class, but kept
    classes::delete_class(&connection, b).unwrap();
    let left: Vec<String> =
        pupils::pupils_in_no_class(&connection).unwrap().into_iter().map(|pupil| pupil.first_name).collect();
    assert_eq!(left, ["Ana", "Brais"]);

    pupils::delete_pupil_in_no_class(&connection, ana).unwrap();
    assert!(pupils::pupils_named(&connection, "Ana").unwrap().is_empty());
    assert!(matches!(pupils::delete_pupil_in_no_class(&connection, ana), Err(AppError::NotEditable)), "already gone");
    assert_eq!(pupils::pupils_in_no_class(&connection).unwrap()[0].id, brais);
}

mod decomposed_accents {
    use avalio_lite::gradebook::model::name_match_key;

    #[test]
    fn regression_an_accent_typed_as_a_mark_matches_the_same_name() {
        // "A" + U+0301 (as some spreadsheets store it) didn't match a stored "Álvaro", and an
        // import or backup made a second pupil
        assert_eq!(
            name_match_key("A\u{301}lvaro Nun\u{303}ez"),
            name_match_key("Álvaro Núñez".replace('ú', "u").as_str())
        );
        assert_eq!(name_match_key("A\u{301}lvaro"), name_match_key("álvaro"));
        assert_eq!(name_match_key("Franc\u{327}ois"), name_match_key("françois"));
        assert_ne!(name_match_key("Alvaro"), name_match_key("Álvaro"), "accents still count");
        assert_eq!(name_match_key("x\u{301}"), "x\u{301}", "a mark no letter takes stays as it is");
        // every accent the table knows, against the same letter typed in one piece
        for (marked, whole) in [
            ("e\u{300}", "è"),
            ("o\u{302}", "ô"),
            ("u\u{308}", "ü"),
            ("y\u{308}", "ÿ"),
            ("a\u{303}", "ã"),
            ("o\u{303}", "õ"),
            ("n\u{303}", "ñ"),
            ("c\u{327}", "ç"),
        ] {
            assert_eq!(name_match_key(marked), whole, "{whole}");
        }
        for unmatched in ["y\u{300}", "e\u{303}", "a\u{327}", "a\u{30A}"] {
            assert_eq!(name_match_key(unmatched), unmatched, "{unmatched:?} has no letter of its own here");
        }
    }
}

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
fn pupils_have_a_first_name_and_an_optional_surname() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    pupils::add_new_pupil(&connection, class, "  Ana ", " Pérez López  ").unwrap();
    pupils::add_new_pupil(&connection, class, "Brais", "").unwrap();
    assert!(matches!(pupils::add_new_pupil(&connection, class, "", "Otero"), Err(AppError::EmptyName)));
    assert!(matches!(pupils::add_new_pupil(&connection, class, "   ", "Otero"), Err(AppError::EmptyName)));
    let stored: Vec<(String, String)> = pupils::list_class_pupils(&connection, class)
        .unwrap()
        .into_iter()
        .map(|pupil| (pupil.first_name, pupil.surname))
        .collect();
    assert_eq!(
        stored,
        [("Ana".to_owned(), "Pérez López".to_owned()), ("Brais".to_owned(), String::new())],
        "both trimmed, and nothing added when the first name is missing"
    );
}

#[test]
fn first_name_and_surname_are_renamed_separately_everywhere() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana", "Pérez").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    pupils::set_first_name(&connection, ana, " Ana María ").unwrap();
    pupils::set_surname(&connection, ana, " Pérez López ").unwrap();
    for class in [a, b] {
        let pupil = &pupils::list_class_pupils(&connection, class).unwrap()[0];
        assert_eq!((pupil.first_name.as_str(), pupil.surname.as_str()), ("Ana María", "Pérez López"));
    }
    assert!(matches!(pupils::set_first_name(&connection, ana, "  "), Err(AppError::EmptyName)));
    pupils::set_surname(&connection, ana, "  ").unwrap();
    let pupil = pupils::pupil_name(&connection, ana).unwrap();
    assert_eq!((pupil.first_name.as_str(), pupil.surname.as_str()), ("Ana María", ""), "a blank surname clears it");
    assert!(matches!(pupils::set_surname(&connection, 9999, "x"), Err(AppError::NotEditable)));
}

#[test]
fn pupil_menus_sort_by_surname_then_first_name() {
    let (_dir, connection) = open();
    let other = classes::create_class(&connection, "Other").unwrap();
    let here = classes::create_class(&connection, "Here").unwrap();
    for (first_name, surname) in
        [("Ana", "Rodríguez"), ("Bea", "Pérez"), ("Álvaro", ""), ("Ana", "Pérez"), ("Xoán", "Alonso")]
    {
        pupils::add_new_pupil(&connection, other, first_name, surname).unwrap();
    }
    let listed: Vec<String> =
        pupils::pupils_not_in_class(&connection, here).unwrap().iter().map(|pupil| pupil.listed_name()).collect();
    assert_eq!(listed, ["Alonso, Xoán", "Álvaro", "Pérez, Ana", "Pérez, Bea", "Rodríguez, Ana"]);
    classes::delete_class(&connection, other).unwrap();
    let in_no_class: Vec<String> =
        pupils::pupils_in_no_class(&connection).unwrap().iter().map(|pupil| pupil.listed_name()).collect();
    assert_eq!(in_no_class, listed, "pupils in no class are listed the same way");
}

#[test]
fn full_names_match_whichever_field_holds_the_words() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let split = pupils::add_new_pupil(&connection, class, "Álvaro", "Núñez").unwrap();
    let whole = pupils::add_new_pupil(&connection, class, "Álvaro Núñez", "").unwrap();
    let found: Vec<i64> =
        pupils::pupils_named(&connection, " álvaro NÚÑEZ ").unwrap().into_iter().map(|pupil| pupil.id).collect();
    assert_eq!(found, [split, whole], "the full name is compared, ignoring case");
}

#[test]
fn widths_are_clamped_to_their_own_minimum() {
    use avalio_lite::gradebook::model::clamp_width;
    assert_eq!(clamp_width(None, MIN_COLUMN_WIDTH), None);
    assert_eq!(clamp_width(Some(0), MIN_COLUMN_WIDTH), Some(MIN_COLUMN_WIDTH));
    assert_eq!(clamp_width(Some(100), MIN_COLUMN_WIDTH), Some(100));
    assert_eq!(clamp_width(Some(u16::MAX), MIN_COLUMN_WIDTH), Some(MAX_COLUMN_WIDTH));
}
