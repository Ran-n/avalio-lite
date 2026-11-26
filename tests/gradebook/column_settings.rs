// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.272220
//+ Revised: 	2026/11/24 17:54:10.803191
// -----------------------------------------------------------------------------

//! Column settings: the defaults for each kind, bonus tests and validation.

use crate::support::{open, titles};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::classes;
use avalio_lite::gradebook::columns::{self, Direction};
use avalio_lite::gradebook::model::{ColumnKind, ColumnType};
use avalio_lite::i18n::Language;
use rusqlite::Connection;

/// The class's columns without the first-name and surname columns.
fn mark_columns(connection: &Connection, class_id: i64) -> Vec<avalio_lite::gradebook::model::Column> {
    columns::list_columns(connection, class_id).unwrap().into_iter().filter(|column| !column.kind.is_name()).collect()
}

/// A column's weight, maximum, rounding source and decimals, as stored.
type StoredFields = (Option<f64>, Option<f64>, Option<i64>, Option<u8>);

/// A class with the six columns a new class gets from the add menu, in the order the tests check.
fn class_with_new_columns() -> (tempfile::TempDir, Connection, i64) {
    let (dir, connection) = open();
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
    (dir, connection, class_id)
}

#[test]
fn new_columns_are_titled_in_order() {
    let (_dir, connection, class_id) = class_with_new_columns();
    assert_eq!(titles(&connection, class_id), ["Test 1", "Test 2", "Final grade", "Rounded", "Notes", "Average"]);
}

#[test]
fn a_new_test_column_takes_the_weight_left_to_reach_100_percent() {
    let (_dir, connection, class_id) = class_with_new_columns();
    let listed = mark_columns(&connection, class_id);
    assert_eq!(listed[0].kind, ColumnKind::Test { weight: 100.0, max_score: 10.0, bonus: false });
    assert_eq!(
        listed[1].kind,
        ColumnKind::Test { weight: 0.0, max_score: 10.0, bonus: false },
        "weight left to reach 100 %"
    );
}

#[test]
fn a_new_weighted_column_gets_a_max_score_of_ten() {
    let (_dir, connection, class_id) = class_with_new_columns();
    let listed = mark_columns(&connection, class_id);
    assert_eq!(listed[2].kind, ColumnKind::Weighted { max_score: 10.0 });
}

#[test]
fn a_new_rounded_column_rounds_the_weighted_column_before_it() {
    let (_dir, connection, class_id) = class_with_new_columns();
    let listed = mark_columns(&connection, class_id);
    assert_eq!(listed[3].kind, ColumnKind::Round { source: Some(listed[2].id), decimals: 0 });
}

#[test]
fn a_new_info_and_average_column_get_their_defaults() {
    let (_dir, connection, class_id) = class_with_new_columns();
    let listed = mark_columns(&connection, class_id);
    assert_eq!(listed[4].kind, ColumnKind::Info);
    assert_eq!(listed[5].kind, ColumnKind::Average { max_score: 10.0 });
}

#[test]
fn new_columns_store_only_the_fields_their_kind_reads_back() {
    let (_dir, connection, class_id) = class_with_new_columns();
    let listed = mark_columns(&connection, class_id);
    // kills the kind_fields mutants that store a field the kind doesn't read back
    let mut statement = connection
        .prepare(
            "SELECT sc.weight, sc.max_score, r.source_column_id, sc.decimals FROM sheet_columns sc
             LEFT JOIN round_sources r ON r.column_id = sc.id
             WHERE sc.kind NOT IN ('first_name', 'surname') ORDER BY sc.position",
        )
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
    for refused in ["+2", "-1", "1.0", "", " "] {
        let result = columns::set_decimals(&connection, round, refused);
        assert!(matches!(result, Err(AppError::InvalidNumber { .. })), "{refused:?}: {result:?}");
    }
    columns::set_decimals(&connection, round, " 2 ").unwrap();
    columns::set_decimals(&connection, round, "2").unwrap();
    assert!(matches!(columns::rename_column(&connection, test, ""), Err(AppError::EmptyName)));
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

/// Two final grades both called "Final grade" couldn't be told apart in the rounded column's source
/// list, the import or the export; every kind's default title is free, numbered once it is taken.
#[test]
fn a_new_column_of_any_kind_gets_a_title_of_its_own() {
    let (_dir, connection) = open();
    let class = classes::create_class(&connection, "1A").unwrap();
    for column_type in
        [ColumnType::Weighted, ColumnType::Weighted, ColumnType::Info, ColumnType::Info, ColumnType::Info]
    {
        columns::add_column(&connection, class, column_type, Language::English).unwrap();
    }
    assert_eq!(titles(&connection, class), ["Final grade", "Final grade 2", "Notes", "Notes 2", "Notes 3"]);
    // a title taken by renaming another column, in other letter case, counts as taken
    let average = columns::add_column(&connection, class, ColumnType::Info, Language::English).unwrap();
    columns::rename_column(&connection, average, "AVERAGE").unwrap();
    columns::add_column(&connection, class, ColumnType::Average, Language::English).unwrap();
    assert_eq!(titles(&connection, class).last().unwrap(), "Average 2");
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

/// Each setting belongs to some kinds only; on a column of another kind, or on none, nothing is written.
#[test]
fn a_setting_is_refused_on_a_column_it_does_not_belong_to() {
    let (_dir, connection, class_id) = class_with_new_columns();
    let before = columns::list_columns(&connection, class_id).unwrap();
    let info = before.iter().find(|column| column.kind == ColumnKind::Info).unwrap().id;
    let nowhere = 9999;
    for column in [info, nowhere] {
        for result in [
            columns::set_weight(&connection, column, "10"),
            columns::set_max_score(&connection, column, "10"),
            columns::set_decimals(&connection, column, "1"),
            columns::set_bonus(&connection, column, true),
            columns::set_round_source(&connection, column, None),
        ] {
            assert!(result.is_err(), "column {column}");
        }
    }
    assert_eq!(columns::list_columns(&connection, class_id).unwrap(), before);
}
