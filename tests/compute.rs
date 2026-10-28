// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.747058
//+ Revised: 	2026/10/28 15:53:09.386311
// -----------------------------------------------------------------------------

use avalio_lite::gradebook::compute::{
    Tone, bonus_weight_total, depends_on_weights, evaluate_row, format_number, grade_summary, parse_number, round_to,
    weight_total, weights_add_up,
};
use avalio_lite::gradebook::model::{Column, ColumnKind};
use proptest::prelude::*;

fn column(id: i64, kind: ColumnKind) -> Column {
    Column { id, title: format!("c{id}"), kind }
}

fn test(id: i64, weight: f64, max_score: f64) -> Column {
    column(id, ColumnKind::Test { weight, max_score, bonus: false })
}

fn texts(columns: &[Column], values: &[&str]) -> Vec<String> {
    let values: Vec<String> = values.iter().map(|value| (*value).to_owned()).collect();
    evaluate_row(columns, &values).into_iter().map(|cell| cell.text).collect()
}

#[test]
fn parse_accepts_dot_and_comma_and_rejects_text() {
    assert_eq!(parse_number("7.5"), Some(7.5));
    assert_eq!(parse_number(" 7,5 "), Some(7.5));
    assert_eq!(parse_number("abc"), None);
    assert_eq!(parse_number(""), None);
    assert_eq!(parse_number("inf"), None);
}

#[test]
fn format_trims_trailing_zeros() {
    assert_eq!(format_number(8.0), "8");
    assert_eq!(format_number(7.5), "7.5");
    assert_eq!(format_number(6.256), "6.26");
    assert_eq!(format_number(-0.001), "0");
}

#[test]
fn rounding_is_half_up_despite_float_noise() {
    assert_eq!(round_to(7.45, 1), 7.5);
    assert_eq!(round_to(2.675, 2), 2.68);
    assert_eq!(round_to(4.5, 0), 5.0);
    assert_eq!(round_to(4.49, 0), 4.0);
}

#[test]
fn final_grade_weights_each_test_by_its_scale() {
    // 7/10 at 60 % and 4/5 at 40 % = 0.6 × 0.7 + 0.4 × 0.8 = 0.74 → 7.4 out of 10
    let columns = [
        column(1, ColumnKind::Info),
        test(2, 60.0, 10.0),
        test(3, 40.0, 5.0),
        column(4, ColumnKind::Weighted { max_score: 10.0 }),
    ];
    assert_eq!(texts(&columns, &["Ana", "7", "4", ""]), ["Ana", "7", "4", "7.4"]);
}

#[test]
fn final_grade_counts_ungraded_tests_as_zero_and_is_blank_with_none() {
    let columns = [test(1, 50.0, 10.0), test(2, 50.0, 10.0), column(3, ColumnKind::Weighted { max_score: 10.0 })];
    assert_eq!(texts(&columns, &["8", "", ""])[2], "4");
    assert_eq!(texts(&columns, &["", "", ""])[2], "");
}

#[test]
fn average_uses_graded_tests_only() {
    let columns = [
        test(1, 50.0, 10.0),
        test(2, 50.0, 5.0),
        test(3, 0.0, 10.0),
        column(4, ColumnKind::Average { max_score: 10.0 }),
    ];
    // 6/10 and 5/5 → (0.6 + 1.0) / 2 = 0.8 → 8
    assert_eq!(texts(&columns, &["6", "5", "", ""])[3], "8");
}

#[test]
fn rounded_column_rounds_its_source_with_fixed_decimals() {
    let columns = [
        test(1, 60.0, 10.0),
        test(2, 40.0, 5.0),
        column(3, ColumnKind::Weighted { max_score: 10.0 }),
        column(4, ColumnKind::Round { source: Some(3), decimals: 0 }),
        column(5, ColumnKind::Round { source: Some(3), decimals: 1 }),
        column(6, ColumnKind::Round { source: None, decimals: 0 }),
    ];
    assert_eq!(texts(&columns, &["7", "4", "", "", "", ""])[3..], ["7", "7.4", ""]);
}

#[test]
fn out_of_range_or_text_scores_are_marked_and_ignored() {
    let columns = [test(1, 50.0, 10.0), test(2, 50.0, 10.0), column(3, ColumnKind::Weighted { max_score: 10.0 })];
    let values: Vec<String> = ["11", "abc", ""].map(str::to_owned).to_vec();
    let cells = evaluate_row(&columns, &values);
    assert!(cells[0].is_invalid && cells[1].is_invalid);
    assert_eq!(cells[2].text, "");
}

fn bonus_test(id: i64, weight: f64, max_score: f64) -> Column {
    column(id, ColumnKind::Test { weight, max_score, bonus: true })
}

#[test]
fn bonus_tests_add_on_top_of_the_normal_tests() {
    // 6/10 at 60 % + 4/10 at 40 % = 5.2; a full 1/1 bonus worth 10 % adds 1 → 6.2
    let columns = [
        test(1, 60.0, 10.0),
        test(2, 40.0, 10.0),
        bonus_test(3, 10.0, 1.0),
        column(4, ColumnKind::Weighted { max_score: 10.0 }),
    ];
    assert_eq!(texts(&columns, &["6", "4", "1", ""])[3], "6.2");
    assert_eq!(texts(&columns, &["6", "4", "", ""])[3], "5.2", "an unmarked bonus adds nothing");
}

#[test]
fn final_grade_is_capped_at_its_maximum() {
    let columns = [test(1, 100.0, 10.0), bonus_test(2, 10.0, 1.0), column(3, ColumnKind::Weighted { max_score: 10.0 })];
    assert_eq!(texts(&columns, &["9.5", "1", ""])[2], "10");
    assert_eq!(texts(&columns, &["8.7", "1", ""])[2], "9.7");
}

#[test]
fn average_leaves_bonus_tests_out() {
    let columns = [
        test(1, 50.0, 10.0),
        test(2, 50.0, 10.0),
        bonus_test(3, 10.0, 1.0),
        column(4, ColumnKind::Average { max_score: 10.0 }),
    ];
    assert_eq!(texts(&columns, &["6", "8", "1", ""])[3], "7");
}

#[test]
fn only_normal_tests_have_to_add_up_to_100() {
    let complete = [test(1, 60.0, 10.0), test(2, 40.0, 10.0), bonus_test(3, 10.0, 1.0)];
    assert!(weights_add_up(&complete));
    assert_eq!(weight_total(&complete), 100.0);
    assert_eq!(bonus_weight_total(&complete), 10.0);
    let typo = [test(1, 60.0, 10.0), test(2, 60.0, 10.0)];
    assert!(!weights_add_up(&typo), "a typo over 100 % is still caught");
    assert!(!weights_add_up(&[test(1, 90.0, 10.0)]));
}

#[test]
fn final_grades_and_their_roundings_depend_on_the_weights() {
    let columns = [
        test(1, 100.0, 10.0),
        column(2, ColumnKind::Weighted { max_score: 10.0 }),
        column(3, ColumnKind::Round { source: Some(2), decimals: 0 }),
        column(4, ColumnKind::Round { source: Some(1), decimals: 0 }),
        column(5, ColumnKind::Average { max_score: 10.0 }),
    ];
    let depends: Vec<bool> = columns.iter().map(|column| depends_on_weights(&columns, column)).collect();
    assert_eq!(depends, [false, true, true, false, false]);
}

#[test]
fn marks_pass_at_half_their_scale() {
    let columns = [
        test(1, 50.0, 10.0),
        test(2, 50.0, 5.0),
        column(3, ColumnKind::Weighted { max_score: 10.0 }),
        column(4, ColumnKind::Round { source: Some(3), decimals: 0 }),
        column(5, ColumnKind::Info),
    ];
    let values: Vec<String> = ["5", "2", "", "", "note"].map(str::to_owned).to_vec();
    let tones: Vec<Tone> = evaluate_row(&columns, &values).into_iter().map(|cell| cell.tone).collect();
    // 5/10 passes exactly; 2/5 fails; final = 2.5 + 2 = 4.5 fails; rounded to 5 is judged on the final grade's scale and passes
    assert_eq!(tones, [Tone::Pass, Tone::Fail, Tone::Fail, Tone::Pass, Tone::Neutral]);
}

#[test]
fn empty_and_invalid_cells_have_no_tone() {
    let columns = [test(1, 100.0, 10.0), column(2, ColumnKind::Round { source: None, decimals: 0 })];
    let values: Vec<String> = ["abc", ""].map(str::to_owned).to_vec();
    assert!(evaluate_row(&columns, &values).iter().all(|cell| cell.tone == Tone::Neutral));
}

#[test]
fn info_cells_are_never_invalid() {
    let columns = [column(1, ColumnKind::Info)];
    assert!(!evaluate_row(&columns, &["anything at all".to_owned()])[0].is_invalid);
}

#[test]
fn weight_total_sums_tests_only() {
    let columns = [test(1, 30.0, 10.0), test(2, 45.5, 5.0), column(3, ColumnKind::Info)];
    assert_eq!(weight_total(&columns), 75.5);
}

/// One test out of 10 worth 100 % and a final grade out of 10, with each pupil's one score.
fn exam_sheet(scores: &[&str]) -> (Vec<Column>, Vec<Vec<String>>) {
    let columns = vec![test(1, 100.0, 10.0), column(2, ColumnKind::Weighted { max_score: 10.0 })];
    let rows = scores.iter().map(|score| vec![(*score).to_owned(), String::new()]).collect();
    (columns, rows)
}

#[test]
fn the_summary_needs_a_final_grade_column() {
    let columns = [test(1, 100.0, 10.0)];
    assert_eq!(grade_summary(&columns, &[vec!["8".to_owned()]]), None);
}

#[test]
fn the_summary_waits_until_someone_has_a_grade() {
    let (columns, rows) = exam_sheet(&["", "", "not a number"]);
    assert_eq!(grade_summary(&columns, &rows), None);
    assert_eq!(grade_summary(&columns, &[]), None);
}

#[test]
fn the_summary_averages_graded_pupils_and_counts_passes_at_half() {
    let (columns, rows) = exam_sheet(&["8", "5", "4.99", ""]);
    let summary = grade_summary(&columns, &rows).unwrap();
    assert_eq!((summary.passing, summary.failing), (2, 1), "5 out of 10 passes, 4.99 doesn't");
    assert!((summary.average - (8.0 + 5.0 + 4.99) / 3.0).abs() < 1e-9, "the ungraded pupil is left out");
}

proptest! {
    #[test]
    fn the_summary_counts_every_graded_pupil_once(scores in prop::collection::vec(prop::option::of(0.0f64..=10.0), 0..12)) {
        let texts: Vec<String> = scores.iter().map(|score| score.map(|value| value.to_string()).unwrap_or_default()).collect();
        let refs: Vec<&str> = texts.iter().map(String::as_str).collect();
        let (columns, rows) = exam_sheet(&refs);
        let graded = scores.iter().filter(|score| score.is_some()).count();
        match grade_summary(&columns, &rows) {
            None => prop_assert_eq!(graded, 0),
            Some(summary) => {
                prop_assert_eq!(summary.passing + summary.failing, graded);
                prop_assert!((0.0..=10.0).contains(&summary.average));
            }
        }
    }

    #[test]
    fn full_marks_with_full_weights_give_the_maximum(maxes in prop::collection::vec(1u32..=20, 1..6), out_of in 1u32..=20) {
        let weight = 100.0 / maxes.len() as f64;
        let mut columns: Vec<Column> = maxes.iter().enumerate().map(|(i, max)| test(i as i64, weight, f64::from(*max))).collect();
        columns.push(column(99, ColumnKind::Weighted { max_score: f64::from(out_of) }));
        let mut values: Vec<String> = maxes.iter().map(|max| max.to_string()).collect();
        values.push(String::new());
        let grade = parse_number(&evaluate_row(&columns, &values)[maxes.len()].text).unwrap();
        prop_assert!((grade - f64::from(out_of)).abs() < 0.01);
    }

    #[test]
    fn final_grade_stays_within_its_scale(scores in prop::collection::vec(0.0f64..=10.0, 1..6)) {
        let weight = 100.0 / scores.len() as f64;
        let mut columns: Vec<Column> = (0..scores.len()).map(|i| test(i as i64, weight, 10.0)).collect();
        columns.push(column(99, ColumnKind::Weighted { max_score: 10.0 }));
        let mut values: Vec<String> = scores.iter().map(|score| score.to_string()).collect();
        values.push(String::new());
        let grade = parse_number(&evaluate_row(&columns, &values)[scores.len()].text).unwrap();
        prop_assert!((0.0..=10.0).contains(&grade));
    }

    #[test]
    fn rounding_moves_a_value_by_at_most_half_a_step(value in -1000.0f64..1000.0, decimals in 0u8..=3) {
        let step = 10f64.powi(-i32::from(decimals));
        prop_assert!((round_to(value, decimals) - value).abs() <= step / 2.0 + 1e-9);
    }

    #[test]
    fn formatted_numbers_parse_back(value in -1000.0f64..1000.0) {
        let parsed = parse_number(&format_number(value)).unwrap();
        prop_assert!((parsed - value).abs() <= 0.005 + 1e-9);
    }
}

proptest! {
    #[test]
    fn a_decimal_comma_reads_like_a_decimal_point(whole in 0u32..10_000, fraction in 0u32..1000) {
        let with_point = format!("{whole}.{fraction}");
        prop_assert_eq!(parse_number(&with_point.replace('.', ",")), parse_number(&with_point));
    }
}
