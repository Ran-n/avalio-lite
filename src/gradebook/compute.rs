// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.047841
//+ Revised: 	2026/10/28 10:52:44.853266
// -----------------------------------------------------------------------------

use super::model::{Column, ColumnKind};

pub const FULL_WEIGHT: f64 = 100.0;
/// A mark passes at half of its scale (5 out of 10).
pub const PASS_FRACTION: f64 = 0.5;
const DISPLAY_DECIMALS: usize = 2;
const FLOAT_SNAP: f64 = 1e6;

/// Whether a numeric cell is a pass or a fail; `Neutral` for text and empty cells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Pass,
    Fail,
}

/// What a cell shows. `is_invalid` marks a typed test score that isn't a number in range.
#[derive(Debug, Clone, PartialEq)]
pub struct Evaluated {
    pub text: String,
    pub is_invalid: bool,
    pub tone: Tone,
}

/// Accepts both "7.5" and "7,5".
pub fn parse_number(text: &str) -> Option<f64> {
    let number: f64 = text.trim().replace(',', ".").parse().ok()?;
    number.is_finite().then_some(number)
}

/// Up to two decimals, without trailing zeros: 8, 7.5, 6.25.
pub fn format_number(value: f64) -> String {
    let text = format!("{value:.DISPLAY_DECIMALS$}");
    let trimmed = text.trim_end_matches('0').trim_end_matches('.');
    if trimmed == "-0" { "0".to_owned() } else { trimmed.to_owned() }
}

/// Rounds half away from zero. Float noise is snapped off first, so 7.45 becomes 7.5
/// rather than 7.4 (7.45 × 10 is 74.4999… in binary).
pub fn round_to(value: f64, decimals: u8) -> f64 {
    let factor = 10f64.powi(i32::from(decimals));
    ((value * factor * FLOAT_SNAP).round() / FLOAT_SNAP).round() / factor
}

/// Sum of the normal (non-bonus) tests' weights; this is what has to reach 100 %.
pub fn weight_total(columns: &[Column]) -> f64 {
    test_weights(columns, false)
}

/// Sum of the bonus tests' weights, which count on top of the 100 %.
pub fn bonus_weight_total(columns: &[Column]) -> f64 {
    test_weights(columns, true)
}

fn test_weights(columns: &[Column], of_bonus_tests: bool) -> f64 {
    columns
        .iter()
        .filter_map(|column| match column.kind {
            ColumnKind::Test { weight, bonus, .. } if bonus == of_bonus_tests => Some(weight),
            _ => None,
        })
        .sum()
}

const WEIGHT_TOLERANCE: f64 = 1e-6;

/// Bonus tests are left out: they are meant to take the total past 100 %.
pub fn weights_add_up(columns: &[Column]) -> bool {
    (weight_total(columns) - FULL_WEIGHT).abs() < WEIGHT_TOLERANCE
}

/// Final grades and roundings of them are only trustworthy once the test weights add up to 100 %.
pub fn depends_on_weights(columns: &[Column], column: &Column) -> bool {
    match column.kind {
        ColumnKind::Weighted { .. } => true,
        ColumnKind::Round { source, .. } => columns
            .iter()
            .any(|candidate| Some(candidate.id) == source && matches!(candidate.kind, ColumnKind::Weighted { .. })),
        _ => false,
    }
}

/// The class at a glance, from its first final grade column.
#[derive(Debug, Clone, PartialEq)]
pub struct GradeSummary {
    pub average: f64,
    pub passing: usize,
    pub failing: usize,
}

/// `None` when the class has no final grade column or nobody has a final grade yet.
pub fn grade_summary(columns: &[Column], rows: &[Vec<String>]) -> Option<GradeSummary> {
    let (index, max_score) = columns.iter().enumerate().find_map(|(index, column)| match column.kind {
        ColumnKind::Weighted { max_score } => Some((index, max_score)),
        _ => None,
    })?;
    let grades: Vec<f64> = rows.iter().filter_map(|values| row_numbers(columns, values)[index]).collect();
    if grades.is_empty() {
        return None;
    }
    let passing = grades.iter().filter(|grade| **grade >= max_score * PASS_FRACTION).count();
    Some(GradeSummary {
        average: grades.iter().sum::<f64>() / grades.len() as f64,
        passing,
        failing: grades.len() - passing,
    })
}

/// Calculates every cell of one row from the typed values (one per column, in order).
pub fn evaluate_row(columns: &[Column], values: &[String]) -> Vec<Evaluated> {
    let numbers = row_numbers(columns, values);
    columns
        .iter()
        .zip(values)
        .zip(&numbers)
        .map(|((column, raw), number)| {
            let tone = tone(*number, scale(columns, column));
            display(column.kind, raw, *number, tone)
        })
        .collect()
}

/// The maximum a column's numbers are measured against; a rounded column uses its source's.
fn scale(columns: &[Column], column: &Column) -> Option<f64> {
    match column.kind {
        ColumnKind::Info => None,
        ColumnKind::Test { max_score, .. } | ColumnKind::Weighted { max_score } | ColumnKind::Average { max_score } => {
            Some(max_score)
        }
        ColumnKind::Round { source, .. } => {
            let source = columns.iter().find(|candidate| Some(candidate.id) == source)?;
            source.kind.can_be_rounded().then(|| scale(columns, source)).flatten()
        }
    }
}

fn tone(number: Option<f64>, scale: Option<f64>) -> Tone {
    match (number, scale) {
        (Some(value), Some(max)) if value >= max * PASS_FRACTION => Tone::Pass,
        (Some(_), Some(_)) => Tone::Fail,
        _ => Tone::Neutral,
    }
}

fn row_numbers(columns: &[Column], values: &[String]) -> Vec<Option<f64>> {
    let scores: Vec<Option<f64>> =
        columns.iter().zip(values).map(|(column, raw)| test_score(column.kind, raw)).collect();
    let marks: Vec<TestMark> = columns
        .iter()
        .zip(&scores)
        .filter_map(|(column, score)| match column.kind {
            ColumnKind::Test { weight, max_score, bonus } => {
                Some(TestMark { weight, fraction: score.map(|score| score / max_score), bonus })
            }
            _ => None,
        })
        .collect();
    let first_pass: Vec<Option<f64>> = columns
        .iter()
        .zip(&scores)
        .map(|(column, score)| match column.kind {
            ColumnKind::Test { .. } => *score,
            ColumnKind::Weighted { max_score } => weighted(&marks, max_score),
            ColumnKind::Average { max_score } => average(&marks, max_score),
            ColumnKind::Info | ColumnKind::Round { .. } => None,
        })
        .collect();
    columns
        .iter()
        .zip(&first_pass)
        .map(|(column, number)| match column.kind {
            ColumnKind::Round { source, decimals } => {
                source_number(columns, &first_pass, source).map(|value| round_to(value, decimals))
            }
            _ => *number,
        })
        .collect()
}

fn test_score(kind: ColumnKind, raw: &str) -> Option<f64> {
    let ColumnKind::Test { max_score, .. } = kind else { return None };
    parse_number(raw).filter(|score| (0.0..=max_score).contains(score))
}

/// One test's contribution to a row: its weight, the score as a share of its maximum (if marked)
/// and whether it is a bonus test.
struct TestMark {
    weight: f64,
    fraction: Option<f64>,
    bonus: bool,
}

/// Ungraded tests count as 0, so the grade builds up as tests are marked. Bonus tests add on top,
/// but the grade never goes past its maximum. Blank until at least one test has a score.
fn weighted(marks: &[TestMark], max_score: f64) -> Option<f64> {
    if marks.iter().all(|mark| mark.fraction.is_none()) {
        return None;
    }
    let earned: f64 = marks.iter().map(|mark| mark.weight * mark.fraction.unwrap_or(0.0)).sum();
    Some((earned / FULL_WEIGHT * max_score).min(max_score))
}

/// Mean of the graded normal tests; ungraded and bonus tests are left out,
/// so extra credit can't pull the mean up or down.
fn average(marks: &[TestMark], max_score: f64) -> Option<f64> {
    let graded: Vec<f64> = marks.iter().filter(|mark| !mark.bonus).filter_map(|mark| mark.fraction).collect();
    if graded.is_empty() {
        return None;
    }
    Some(graded.iter().sum::<f64>() / graded.len() as f64 * max_score)
}

fn source_number(columns: &[Column], numbers: &[Option<f64>], source: Option<i64>) -> Option<f64> {
    let index = columns.iter().position(|column| Some(column.id) == source)?;
    numbers[index]
}

fn display(kind: ColumnKind, raw: &str, number: Option<f64>, tone: Tone) -> Evaluated {
    let (text, is_invalid) = match kind {
        ColumnKind::Info => (raw.to_owned(), false),
        ColumnKind::Test { .. } => (raw.to_owned(), !raw.trim().is_empty() && number.is_none()),
        ColumnKind::Round { decimals, .. } => {
            (number.map(|value| format!("{value:.*}", usize::from(decimals))).unwrap_or_default(), false)
        }
        ColumnKind::Weighted { .. } | ColumnKind::Average { .. } => {
            (number.map(format_number).unwrap_or_default(), false)
        }
    };
    Evaluated { text, is_invalid, tone }
}
