// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.380621
//+ Revised: 	2026/11/03 11:39:46.380621
// -----------------------------------------------------------------------------

//! The weight rules: what the tests add up to, and when grades are only provisional.

use crate::gradebook::model::{Column, ColumnKind};

pub const FULL_WEIGHT: f64 = 100.0;

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

/// Whether the sheet warns that its final grades are provisional: the weights only matter once
/// there is a final grade to compute, so the warning waits for one.
pub fn grades_are_provisional(columns: &[Column]) -> bool {
    columns.iter().any(|column| matches!(column.kind, ColumnKind::Weighted { .. })) && !weights_add_up(columns)
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
