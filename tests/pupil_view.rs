// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:12:05.943780
//+ Revised: 	2026/11/25 11:43:15.977591
// -----------------------------------------------------------------------------

//! What a pupil is shown: their own marks and grades, in sheet order, and nothing else.

use avalio_lite::gradebook::compute::Tone;
use avalio_lite::gradebook::model::{Column, ColumnKind, Pupil, Sheet, SheetRow};
use avalio_lite::pupil_view::{Grade, Weight, pupil_grades};
use proptest::prelude::*;

const ANA: i64 = 1;
const BEA: i64 = 2;

fn column(id: i64, title: &str, kind: ColumnKind) -> Column {
    Column { id, title: title.to_owned(), kind, width: None }
}

fn test(id: i64, title: &str, weight: f64, max_score: f64) -> Column {
    column(id, title, ColumnKind::Test { weight, max_score, bonus: false })
}

fn row(id: i64, first_name: &str, values: &[&str]) -> SheetRow {
    let pupil = Pupil { id, first_name: first_name.to_owned(), surname: String::new() };
    SheetRow { pupil, values: values.iter().map(|value| (*value).to_owned()).collect() }
}

/// Names, an exam (60 %), a note, a lab out of 5 (40 %), a bonus, the final grade and its rounding.
fn sheet() -> Sheet {
    Sheet {
        columns: vec![
            column(1, "First name", ColumnKind::FirstName),
            column(2, "Surname", ColumnKind::Surname),
            test(3, "Exam", 60.0, 10.0),
            column(4, "Notes", ColumnKind::Info),
            test(5, "Lab", 40.0, 5.0),
            column(6, "Extra", ColumnKind::Test { weight: 10.0, max_score: 1.0, bonus: true }),
            column(7, "Final", ColumnKind::Weighted { max_score: 10.0 }),
            column(8, "Average", ColumnKind::Average { max_score: 20.0 }),
            column(9, "Card", ColumnKind::Round { source: Some(7), decimals: 0 }),
        ],
        rows: vec![
            row(ANA, "Ana", &["Ana", "", " 8 ", "Cheeky in class", "4,5", "1", "", "", ""]),
            row(BEA, "Bea", &["Bea", "", "2", "Resit", "absent", "", "", "", ""]),
        ],
    }
}

fn grades(pupil: i64) -> Vec<Grade> {
    pupil_grades(&sheet(), pupil).unwrap()
}

fn texts(grades: &[Grade]) -> Vec<(&str, &str)> {
    grades.iter().map(|grade| (grade.title.as_str(), grade.text.as_str())).collect()
}

#[test]
fn a_pupil_sees_their_marks_and_grades_in_sheet_order() {
    let expected = [("Exam", "8"), ("Lab", "4,5"), ("Extra", "1"), ("Final", "9.4"), ("Average", "17"), ("Card", "9")];
    assert_eq!(texts(&grades(ANA)), expected);
}

#[test]
fn notes_and_names_are_never_shown() {
    for pupil in [ANA, BEA] {
        for grade in grades(pupil) {
            assert!(!["First name", "Surname", "Notes"].contains(&grade.title.as_str()), "{}", grade.title);
            assert!(!["Cheeky in class", "Resit", "Ana", "Bea"].contains(&grade.text.as_str()), "{}", grade.text);
        }
    }
}

/// "absent" typed in a test's cell is a note to the teacher, not a mark.
#[test]
fn a_score_that_is_not_a_valid_mark_shows_as_no_mark() {
    let bea = grades(BEA);
    assert_eq!(texts(&bea)[1], ("Lab", ""));
    assert_eq!(bea[1].tone, Tone::Neutral);
}

#[test]
fn each_mark_says_what_it_is_out_of_and_a_test_its_weight() {
    let ana = grades(ANA);
    let maxima: Vec<Option<f64>> = ana.iter().map(|grade| grade.max_score).collect();
    assert_eq!(
        maxima,
        [Some(10.0), Some(5.0), Some(1.0), Some(10.0), Some(20.0), Some(10.0)],
        "a rounding has its source's"
    );
    let weights: Vec<Option<Weight>> = ana.iter().map(|grade| grade.weight).collect();
    let weight = |percent, bonus| Some(Weight { percent, bonus });
    assert_eq!(weights, [weight(60.0, false), weight(40.0, false), weight(10.0, true), None, None, None]);
    let computed: Vec<bool> = ana.iter().map(|grade| grade.computed).collect();
    assert_eq!(computed, [false, false, false, true, true, true]);
}

#[test]
fn a_rounding_of_nothing_has_no_mark_and_no_maximum() {
    let mut sheet = sheet();
    sheet.columns[8].kind = ColumnKind::Round { source: None, decimals: 0 };
    let card = pupil_grades(&sheet, ANA).unwrap().pop().unwrap();
    assert_eq!((card.text.as_str(), card.max_score), ("", None));
}

#[test]
fn marks_keep_the_sheets_pass_and_fail() {
    let tones = |pupil| grades(pupil).iter().map(|grade| grade.tone).collect::<Vec<_>>();
    assert!(tones(ANA).iter().all(|tone| *tone == Tone::Pass));
    let bea = tones(BEA);
    assert_eq!(bea[0], Tone::Fail, "2 out of 10");
    assert_eq!(bea[3], Tone::Fail, "the final grade of one failed exam");
}

#[test]
fn final_grades_are_provisional_while_the_weights_do_not_add_up() {
    let provisional = |sheet: &Sheet| {
        let grades = pupil_grades(sheet, ANA).unwrap();
        grades.into_iter().filter(|grade| grade.provisional).map(|grade| grade.title).collect::<Vec<_>>()
    };
    let mut sheet = sheet();
    assert!(provisional(&sheet).is_empty());
    sheet.columns[4].kind = ColumnKind::Test { weight: 30.0, max_score: 5.0, bonus: false };
    assert_eq!(provisional(&sheet), ["Final", "Card"], "the final grade and what rounds it");
}

#[test]
fn a_pupil_of_another_class_has_no_grades_here() {
    assert_eq!(pupil_grades(&sheet(), 99), None);
}

#[test]
fn a_class_with_no_tests_has_an_empty_list() {
    let sheet = Sheet {
        columns: vec![column(1, "First name", ColumnKind::FirstName), column(2, "Notes", ColumnKind::Info)],
        rows: vec![row(ANA, "Ana", &["Ana", "Cheeky"])],
    };
    assert_eq!(pupil_grades(&sheet, ANA), Some(Vec::new()));
}

proptest! {
    /// Whatever the other pupils' rows and the notes hold, they change nothing of what Ana sees.
    #[test]
    fn nothing_of_another_pupil_or_of_the_notes_reaches_a_pupil(
        note in r"\PC{0,12}",
        others in prop::collection::vec(prop::collection::vec(r"\PC{0,6}", 9), 0..4),
    ) {
        let mut changed = sheet();
        changed.rows.truncate(1);
        changed.rows[0].values[3] = note;
        for (index, values) in (10..).zip(others) {
            let pupil = Pupil { id: index, first_name: values[0].clone(), surname: values[1].clone() };
            changed.rows.insert(0, SheetRow { pupil, values });
        }
        prop_assert_eq!(pupil_grades(&changed, ANA), pupil_grades(&sheet(), ANA));
    }

    /// A mark on screen is always one the calculations used: a number from 0 to the maximum.
    #[test]
    fn a_shown_test_mark_is_always_a_valid_one(typed in r"[0-9a-z,. -]{0,6}") {
        let sheet = Sheet {
            columns: vec![test(1, "Exam", 100.0, 10.0)],
            rows: vec![row(ANA, "Ana", &[&typed])],
        };
        let shown = pupil_grades(&sheet, ANA).unwrap().remove(0).text;
        if !shown.is_empty() {
            let value = avalio_lite::gradebook::compute::parse_number(&shown);
            prop_assert!(value.is_some_and(|value| (0.0..=10.0).contains(&value)), "{shown:?}");
        }
    }
}
