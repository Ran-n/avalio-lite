// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:55.006271
//+ Revised: 	2026/10/28 16:22:40.962246
// -----------------------------------------------------------------------------

use avalio_lite::db;
use avalio_lite::error::AppError;
use avalio_lite::gradebook::columns::{self, Direction};
use avalio_lite::gradebook::model::{ColumnKind, ColumnType};
use avalio_lite::gradebook::{cells, classes, pupils, sheet};
use avalio_lite::i18n::Language;
use rusqlite::Connection;
use tempfile::TempDir;

fn open() -> (TempDir, Connection) {
    let dir = tempfile::tempdir().unwrap();
    let connection = db::open(&dir.path().join("test.db"), None).unwrap();
    (dir, connection)
}

fn titles(connection: &Connection, class_id: i64) -> Vec<String> {
    columns::list_columns(connection, class_id).unwrap().into_iter().map(|column| column.title).collect()
}

#[test]
fn classes_are_listed_by_name_with_pupil_counts() {
    let (_dir, connection) = open();
    let b = classes::create_class(&connection, " 2B ").unwrap();
    let a = classes::create_class(&connection, "1A").unwrap();
    pupils::add_new_pupil(&connection, b, "Ana").unwrap();
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
fn deleting_a_class_keeps_its_pupils_in_other_classes() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    classes::delete_class(&connection, a).unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, b).unwrap()[0].name, "Ana");
}

#[test]
fn pupils_are_shared_and_renamed_everywhere() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana").unwrap();
    assert_eq!(pupils::pupils_not_in_class(&connection, b).unwrap()[0].id, ana);
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    assert!(pupils::pupils_not_in_class(&connection, b).unwrap().is_empty());
    assert!(pupils::add_existing_pupil(&connection, b, ana).is_err(), "can't be added twice");
    pupils::rename_pupil(&connection, ana, "Ana Pérez").unwrap();
    assert_eq!(pupils::list_class_pupils(&connection, a).unwrap()[0].name, "Ana Pérez");
    assert!(matches!(pupils::rename_pupil(&connection, ana, " "), Err(AppError::EmptyName)));
}

#[test]
fn pupils_keep_the_order_they_were_added_in() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    for name in ["Xoán", "Ana", "Brais"] {
        pupils::add_new_pupil(&connection, class_id, name).unwrap();
    }
    let names: Vec<String> =
        pupils::list_class_pupils(&connection, class_id).unwrap().into_iter().map(|pupil| pupil.name).collect();
    assert_eq!(names, ["Xoán", "Ana", "Brais"]);
}

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
    let listed = columns::list_columns(&connection, class_id).unwrap();
    assert_eq!(titles(&connection, class_id), ["Test 1", "Test 2", "Final grade", "Rounded", "Notes", "Average"]);
    assert_eq!(listed[0].kind, ColumnKind::Test { weight: 100.0, max_score: 10.0, bonus: false });
    assert_eq!(
        listed[1].kind,
        ColumnKind::Test { weight: 0.0, max_score: 10.0, bonus: false },
        "weight left to reach 100 %"
    );
    assert_eq!(listed[3].kind, ColumnKind::Round { source: Some(listed[2].id), decimals: 0 });
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

    let listed = columns::list_columns(&connection, class_id).unwrap();
    assert_eq!(listed[1].kind, ColumnKind::Test { weight: 10.0, max_score: 10.0, bonus: true });
    assert_eq!(
        listed[2].kind,
        ColumnKind::Test { weight: 30.0, max_score: 10.0, bonus: false },
        "70 + 30, bonus not counted"
    );

    columns::set_bonus(&connection, extra, false).unwrap();
    assert_eq!(
        columns::list_columns(&connection, class_id).unwrap()[1].kind,
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
        columns::list_columns(&connection, class_id).unwrap()[0].kind,
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
        columns::list_columns(&connection, class_id).unwrap()[0].kind,
        ColumnKind::Round { source: None, decimals: 0 }
    );
}

#[test]
fn columns_move_left_and_right_and_stop_at_the_ends() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let first = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    columns::add_column(&connection, class_id, ColumnType::Info, Language::English).unwrap();
    columns::move_column(&connection, first, Direction::Left).unwrap();
    assert_eq!(titles(&connection, class_id), ["Test 1", "Notes"]);
    columns::move_column(&connection, first, Direction::Right).unwrap();
    assert_eq!(titles(&connection, class_id), ["Notes", "Test 1"]);
    columns::move_column(&connection, first, Direction::Right).unwrap();
    assert_eq!(titles(&connection, class_id), ["Notes", "Test 1"]);
}

#[test]
fn cells_store_typed_values_and_clear_when_emptied() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let ana = pupils::add_new_pupil(&connection, class_id, "Ana").unwrap();
    let test = columns::add_column(&connection, class_id, ColumnType::Test, Language::English).unwrap();
    let notes = columns::add_column(&connection, class_id, ColumnType::Info, Language::English).unwrap();
    cells::set_cell(&connection, test, ana, "7,5").unwrap();
    cells::set_cell(&connection, notes, ana, "shy").unwrap();
    cells::set_cell(&connection, notes, ana, "very shy").unwrap();
    assert_eq!(sheet::load_sheet(&connection, class_id).unwrap().rows[0].values, ["7,5", "very shy"]);
    cells::set_cell(&connection, test, ana, "  ").unwrap();
    assert_eq!(sheet::load_sheet(&connection, class_id).unwrap().rows[0].values, ["", "very shy"]);
}

#[test]
fn calculated_columns_cannot_be_typed_into() {
    let (_dir, connection) = open();
    let class_id = classes::create_class(&connection, "1A").unwrap();
    let ana = pupils::add_new_pupil(&connection, class_id, "Ana").unwrap();
    let final_grade = columns::add_column(&connection, class_id, ColumnType::Weighted, Language::English).unwrap();
    assert!(matches!(cells::set_cell(&connection, final_grade, ana, "10"), Err(AppError::NotEditable)));
    assert!(matches!(cells::set_cell(&connection, 9999, ana, "10"), Err(AppError::NotEditable)));
}

#[test]
fn sheets_only_show_their_own_class() {
    let (_dir, connection) = open();
    let (a, b) = (classes::create_class(&connection, "1A").unwrap(), classes::create_class(&connection, "2B").unwrap());
    let ana = pupils::add_new_pupil(&connection, a, "Ana").unwrap();
    pupils::add_existing_pupil(&connection, b, ana).unwrap();
    let in_a = columns::add_column(&connection, a, ColumnType::Info, Language::English).unwrap();
    let in_b = columns::add_column(&connection, b, ColumnType::Info, Language::English).unwrap();
    cells::set_cell(&connection, in_a, ana, "A note").unwrap();
    cells::set_cell(&connection, in_b, ana, "B note").unwrap();
    let sheet_b = sheet::load_sheet(&connection, b).unwrap();
    assert_eq!(sheet_b.columns.len(), 1);
    assert_eq!(sheet_b.rows[0].values, ["B note"]);
}

#[test]
fn removing_a_pupil_from_one_class_keeps_them_and_their_other_marks() {
    let (_dir, connection) = open();
    let first = classes::create_class(&connection, "1A").unwrap();
    let second = classes::create_class(&connection, "2B").unwrap();
    let ana = pupils::add_new_pupil(&connection, first, "Ana").unwrap();
    pupils::add_existing_pupil(&connection, second, ana).unwrap();
    let exam_1a = columns::add_column(&connection, first, ColumnType::Test, Language::English).unwrap();
    let exam_2b = columns::add_column(&connection, second, ColumnType::Test, Language::English).unwrap();
    cells::set_cell(&connection, exam_1a, ana, "7").unwrap();
    cells::set_cell(&connection, exam_2b, ana, "9").unwrap();

    pupils::remove_from_class(&connection, first, ana).unwrap();
    assert!(pupils::list_class_pupils(&connection, first).unwrap().is_empty());
    let other = sheet::load_sheet(&connection, second).unwrap();
    assert_eq!(other.rows[0].pupil.name, "Ana");
    assert_eq!(other.rows[0].values, ["9"]);
    let marks_left: i64 = connection.query_row("SELECT COUNT(*) FROM cells", [], |row| row.get(0)).unwrap();
    assert_eq!(marks_left, 1, "only the 1A mark went");
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
    let outsider = pupils::add_new_pupil(&connection, b, "Bea").unwrap();
    assert!(matches!(cells::set_cell(&connection, test, outsider, "7"), Err(AppError::NotEditable)));
    // had the cell been stored, it would surface as soon as the pupil joined the class
    pupils::add_existing_pupil(&connection, a, outsider).unwrap();
    assert_eq!(sheet::load_sheet(&connection, a).unwrap().rows[0].values, [""]);
    cells::set_cell(&connection, test, outsider, "7").unwrap();
    cells::set_cell(&connection, test, outsider, "8").unwrap();
    assert_eq!(sheet::load_sheet(&connection, a).unwrap().rows[0].values, ["8"]);
}

#[test]
fn regression_renaming_something_that_no_longer_exists_is_reported() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    let pupil = pupils::add_new_pupil(&connection, class, "Ana").unwrap();
    classes::delete_class(&connection, class).unwrap();
    connection.execute("DELETE FROM pupils WHERE id = ?1", [pupil]).unwrap();
    assert!(matches!(classes::rename_class(&connection, class, "2B"), Err(AppError::NotEditable)));
    assert!(matches!(pupils::rename_pupil(&connection, pupil, "Bea"), Err(AppError::NotEditable)));
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
