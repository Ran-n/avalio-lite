// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.731717
//+ Revised: 	2026/11/24 14:49:43.492088
// -----------------------------------------------------------------------------

//! Import of rows: pupils matched by name, unchanged values and the error cells.

use crate::support::Cell::{Empty, Number, Text};
use crate::support::{Cell, Scratch, pupil_ids, read, small_class, strings};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::compute::evaluate_row;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{cells, classes, pupils, sheet};
use avalio_lite::spreadsheet::import::{ImportRow, ImportTable, apply_import, plan_import};

#[test]
fn pupils_match_in_class_then_elsewhere_then_new() {
    let scratch = Scratch::new();
    let connection = &scratch.connection;
    let (class, ..) = small_class(&scratch);
    let other = classes::create_class(connection, "2B").unwrap();
    let carla = pupils::add_new_pupil(connection, other, "Carla", "").unwrap();
    pupils::add_new_pupil(connection, other, "Dani", "").unwrap();
    let third = classes::create_class(connection, "3C").unwrap();
    pupils::add_new_pupil(connection, third, "dani", "").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Text(" ana "), Number(5.0)],
            vec![Text("CARLA"), Number(6.0)],
            vec![Text("Dani"), Number(7.0)],
            vec![Text("Eva"), Number(8.0)],
        ],
    );
    let pupils_before = scratch.pupil_count();
    let summary = scratch.import(class, &path);
    assert_eq!((summary.pupils, summary.new_pupils, summary.marks), (4, 2, 4));
    let names: Vec<String> = scratch.rows(class).into_iter().map(|(name, _)| name).collect();
    assert_eq!(names, ["Ana", "Bea", "Carla", "Dani", "Eva"], "new pupils at the end, in file order");
    assert_eq!(pupil_ids(&scratch, class)[2], carla, "the one Carla elsewhere joins the class");
    assert_eq!(scratch.pupil_count(), pupils_before + 2, "two Danis elsewhere: which one is unknown, so a new one");
    assert_eq!(scratch.rows(class)[0].1[1], "5", "Ana matched ignoring case and spaces");
}

#[test]
fn a_name_twice_in_the_file_is_refused_and_changes_nothing() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Text("Ana Pérez"), Number(5.0)],
            vec![Text("ana pérez"), Number(6.0)],
        ],
    );
    let before = scratch.rows(class);
    let error = plan_import(&scratch.connection, class, &read(&path).unwrap()).unwrap_err();
    assert!(matches!(&error, AppError::DuplicatePupilInFile(name) if name == "ana pérez"), "{error:?}");
    assert_eq!(scratch.rows(class), before);
}

#[test]
fn empty_file_cells_keep_existing_values() {
    let scratch = Scratch::new();
    let (class, notes, exam) = small_class(&scratch);
    let ana = pupil_ids(&scratch, class)[0];
    cells::set_cell(&scratch.connection, notes, ana, "Keep me").unwrap();
    cells::set_cell(&scratch.connection, exam, ana, "3").unwrap();
    let path = scratch
        .workbook("x.xlsx", &[vec![Text("Pupil"), Text("Notes"), Text("Exam")], vec![Text("Ana"), Empty, Number(9.0)]]);
    let plan = plan_import(&scratch.connection, class, &read(&path).unwrap()).unwrap();
    assert_eq!(plan.overwrites, 1, "only the exam is replaced");
    apply_import(&scratch.connection, class, &plan).unwrap();
    assert_eq!(scratch.rows(class)[0].1, strings(&["Keep me", "9", ""]));
}

#[test]
fn unchanged_values_are_neither_counted_nor_asked_about() {
    let scratch = Scratch::new();
    let (class, _, exam) = small_class(&scratch);
    let ana = pupil_ids(&scratch, class)[0];
    cells::set_cell(&scratch.connection, exam, ana, "7,5").unwrap();
    let path = scratch.workbook("x.xlsx", &[vec![Text("Pupil"), Text("Exam")], vec![Text("Ana"), Number(7.5)]]);
    let plan = plan_import(&scratch.connection, class, &read(&path).unwrap()).unwrap();
    assert_eq!((plan.overwrites, plan.summary.marks), (0, 0), "7,5 and 7.5 are the same score");
    apply_import(&scratch.connection, class, &plan).unwrap();
    assert_eq!(scratch.rows(class)[0].1[1], "7,5", "kept as the user typed it");
}

#[test]
fn the_same_text_again_is_not_a_change() {
    let scratch = Scratch::new();
    let (class, notes, exam) = small_class(&scratch);
    let ana = pupil_ids(&scratch, class)[0];
    cells::set_cell(&scratch.connection, notes, ana, "Bilingüe").unwrap();
    cells::set_cell(&scratch.connection, exam, ana, "abc").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Pupil"), Text("Notes"), Text("Exam")], vec![Text("Ana"), Text(" Bilingüe "), Text("abc")]],
    );
    let plan = plan_import(&scratch.connection, class, &read(&path).unwrap()).unwrap();
    assert_eq!(
        (plan.overwrites, plan.summary.marks),
        (0, 0),
        "re-importing what is already there neither asks nor counts, for notes and for unreadable scores"
    );
}

#[test]
fn excel_numbers_dates_and_booleans_become_text() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    for title in ["Born", "Repeats", "Count", "Time"] {
        scratch.column(class, ColumnType::Info, title);
    }
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![
                Text("Pupil"),
                Text("Notes"),
                Text("Exam"),
                Text("Born"),
                Text("Repeats"),
                Text("Count"),
                Text("Time"),
            ],
            vec![
                Text("Ana"),
                Text("  spaced  "),
                Number(7.125),
                Cell::Date(2011, 3, 9),
                Cell::Bool(true),
                Number(3.0),
                Cell::Duration(1, 30, 5),
            ],
            vec![Text("Bea"), Empty, Text("7,5")],
        ],
    );
    scratch.import(class, &path);
    assert_eq!(scratch.rows(class)[0].1, strings(&["spaced", "7.125", "", "2011-03-09", "TRUE", "3", "1:30:05"]));
    assert_eq!(scratch.rows(class)[1].1[1], "7,5", "a decimal comma typed as text is kept as typed");
}

#[test]
fn error_cells_are_counted_and_skipped() {
    let scratch = Scratch::new();
    let (class, _, exam) = small_class(&scratch);
    let ana = pupil_ids(&scratch, class)[0];
    cells::set_cell(&scratch.connection, exam, ana, "6").unwrap();
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Text("Ana"), Cell::Error("#DIV/0!")],
            vec![Text("Bea"), Cell::Error("#N/A")],
        ],
    );
    let summary = scratch.import(class, &path);
    assert_eq!((summary.error_cells, summary.marks), (2, 0));
    assert_eq!(scratch.rows(class)[0].1[1], "6", "an error cell counts as empty: the mark stays");
}

#[test]
fn invalid_scores_are_stored_like_typing() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook("x.xlsx", &[vec![Text("Pupil"), Text("Exam")], vec![Text("Ana"), Number(12.0)]]);
    scratch.import(class, &path);
    let loaded = sheet::load_sheet(&scratch.connection, class).unwrap();
    assert_eq!(loaded.rows[0].values[3], "12");
    assert!(evaluate_row(&loaded.columns, &loaded.rows[0].values)[3].is_invalid, "shaded red, as if typed");
}

#[test]
fn rows_without_a_name_are_skipped() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[
            vec![Text("Pupil"), Text("Exam")],
            vec![Empty, Number(3.0)],
            vec![Text("   "), Number(4.0)],
            vec![Text("Bea"), Number(5.0)],
        ],
    );
    let summary = scratch.import(class, &path);
    assert_eq!((summary.pupils, summary.marks), (1, 1));
    assert_eq!(summary.unnamed_rows, 2, "the skipped rows held marks, so the report says so");
}

/// A row that is empty all through is no row at all, so it isn't counted as skipped.
#[test]
fn empty_rows_are_not_counted_as_skipped() {
    let scratch = Scratch::new();
    let (class, ..) = small_class(&scratch);
    let path = scratch.workbook(
        "x.xlsx",
        &[vec![Text("Pupil"), Text("Exam")], vec![Text("Ana"), Number(3.0)], vec![], vec![Text("Bea"), Number(5.0)]],
    );
    assert_eq!(scratch.import(class, &path).unnamed_rows, 0);
}

proptest::proptest! {
    #![proptest_config(proptest::prelude::ProptestConfig::with_cases(32))]
    /// Random tables against a class with marks: the plan's counts are what applying changes.
    #[test]
    fn plan_counts_match_what_apply_writes(
        rows in proptest::collection::vec(
            ("Ana|bea|[A-Ea-e]{1,2}", proptest::option::of("[0-9]{1,2}|[a-z]{1,3}"), proptest::option::of("[a-z]{1,3}")),
            1..8,
        )
    ) {
        let scratch = Scratch::new();
        let (class, notes, exam) = small_class(&scratch);
        let ana = pupil_ids(&scratch, class)[0];
        cells::set_cell(&scratch.connection, exam, ana, "5").unwrap();
        cells::set_cell(&scratch.connection, notes, ana, "a").unwrap();
        let mut seen = std::collections::HashSet::new();
        let table = ImportTable {
            header_columns: vec![1, 2],
            headers: strings(&["Exam", "Notes"]),
            rows: rows
                .into_iter()
                .filter(|(name, ..)| seen.insert(name.to_lowercase()))
                .map(|(first_name, exam, notes)| ImportRow { first_name, surname: String::new(), values: vec![exam, notes] })
                .collect(),
            error_cells: 0,
            unnamed_rows: 0,
        };
        let (before, pupils_before) = (scratch.rows(class), scratch.pupil_count());
        let plan = plan_import(&scratch.connection, class, &table).unwrap();
        let summary = apply_import(&scratch.connection, class, &plan).unwrap();
        let changed: usize = scratch
            .rows(class)
            .iter()
            .map(|(name, values)| {
                let old = before.iter().find(|(old_name, _)| old_name == name).map(|(_, old)| old.clone());
                let old = old.unwrap_or_else(|| vec![String::new(); values.len()]);
                values.iter().zip(&old).filter(|(new, old)| new != old).count()
            })
            .sum();
        proptest::prop_assert_eq!(summary.marks, changed);
        proptest::prop_assert_eq!(scratch.pupil_count() - pupils_before, i64::try_from(summary.new_pupils).unwrap());
        proptest::prop_assert_eq!(summary.pupils, table.rows.len());
    }
}

mod cell_values {
    use avalio_lite::spreadsheet::import::{cell_text, clock, iso_seconds, without_float_noise};
    use calamine::{Data, ExcelDateTime, ExcelDateTimeType};
    use proptest::prelude::*;

    fn moment(value: f64, kind: ExcelDateTimeType) -> Data {
        Data::DateTime(ExcelDateTime::new(value, kind, false))
    }

    #[test]
    fn regression_a_time_of_day_is_a_time_not_a_date() {
        // an 08:30 cell (no date) imported as "1899-12-31"
        assert_eq!(cell_text(&moment(8.5 / 24.0, ExcelDateTimeType::DateTime)).as_deref(), Some("8:30:00"));
        assert_eq!(cell_text(&moment(45_000.25, ExcelDateTimeType::DateTime)).as_deref(), Some("2023-03-15"));
        assert_eq!(cell_text(&moment(1.5, ExcelDateTimeType::TimeDelta)).as_deref(), Some("36:00:00"));
        // Excel's first day is a date; anything short of it is a time
        assert_eq!(cell_text(&moment(1.0, ExcelDateTimeType::DateTime)).as_deref(), Some("1900-01-01"));
    }

    #[test]
    fn regression_negative_durations_carry_one_sign() {
        // -5405 s showed as "-1:-30:-05"
        assert_eq!(clock(-5405.0), "-1:30:05");
        assert_eq!(clock(5405.0), "1:30:05");
        assert_eq!(clock(0.0), "0:00:00");
    }

    #[test]
    fn regression_formula_results_lose_their_float_noise() {
        // 7.3 worked out by a formula is 7.2999999999999989, which imported as "7.299999999999999"
        assert_eq!(cell_text(&Data::Float(7.3_f64.next_down())).as_deref(), Some("7.3"));
        assert_eq!(cell_text(&Data::Float(0.1 + 0.2)).as_deref(), Some("0.3"));
        assert_eq!(cell_text(&Data::Float(f64::NAN)), None);
        assert_eq!(cell_text(&Data::Float(f64::INFINITY)), None);
    }

    #[test]
    fn iso_durations_and_their_fallback() {
        assert_eq!(cell_text(&Data::DurationIso("PT01H30M05S".to_owned())).as_deref(), Some("1:30:05"));
        assert_eq!(cell_text(&Data::DurationIso("P1D".to_owned())).as_deref(), Some("P1D"), "kept as written");
        assert_eq!(cell_text(&Data::DateTimeIso("2011-03-09T08:30:00".to_owned())).as_deref(), Some("2011-03-09"));
        assert_eq!(cell_text(&Data::String("   ".to_owned())), None, "spaces only is empty");
    }

    proptest! {
        #[test]
        fn clock_reads_back(seconds in -1_000_000i64..1_000_000) {
            let text = clock(seconds as f64);
            let (sign, rest) = text.strip_prefix('-').map_or((1, text.as_str()), |rest| (-1, rest));
            let parts: Vec<i64> = rest.split(':').map(|part| part.parse().unwrap()).collect();
            prop_assert_eq!(sign * (parts[0] * 3600 + parts[1] * 60 + parts[2]), seconds);
            prop_assert!(parts[1] < 60 && parts[2] < 60);
        }

        #[test]
        fn iso_durations_read_as_seconds(hours in 0u32..1000, minutes in 0u32..60, seconds in 0u32..60) {
            let text = format!("PT{hours:02}H{minutes:02}M{seconds:02}S");
            prop_assert_eq!(iso_seconds(&text), Some(f64::from(hours * 3600 + minutes * 60 + seconds)));
        }

        #[test]
        fn cell_text_is_never_empty_or_padded(text in ".*") {
            if let Some(shown) = cell_text(&Data::String(text)) {
                prop_assert!(!shown.is_empty());
                prop_assert_eq!(shown.trim(), shown.as_str());
            }
        }

        #[test]
        fn short_decimals_import_as_typed(whole in 0u32..100_000, hundredths in 0u32..100) {
            // read from its decimal text, as Excel stores what was typed: adding the hundredths
            // to the whole part would bring in noise of its own (1 + 82/100 is 1.8199999999999998)
            let typed: f64 = format!("{whole}.{hundredths:02}").parse().unwrap();
            prop_assert_eq!(cell_text(&Data::Float(typed)), Some(typed.to_string()));
            prop_assert_eq!(without_float_noise(typed), typed);
        }
    }
}
