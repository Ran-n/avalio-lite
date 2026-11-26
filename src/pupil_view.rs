// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:12:05.667334
//+ Revised: 	2026/11/25 11:43:09.641377
// -----------------------------------------------------------------------------

//! What one pupil is shown of a class: their own marks and grades, and nothing else. Info columns
//! are left out (they hold the teacher's notes), and so is a typed score that isn't a valid mark.
//! `ui` wires the pupil view, which covers the whole window while a pupil is at the computer.

pub mod ui;

use crate::gradebook::compute::{Tone, depends_on_weights, evaluate_row, scale, weights_add_up};
use crate::gradebook::model::{Column, ColumnKind, Sheet};

/// A test's share of the final grade; a bonus test's counts on top of the others' 100 %.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Weight {
    pub percent: f64,
    pub bonus: bool,
}

/// One mark or grade as its pupil sees it.
#[derive(Debug, Clone, PartialEq)]
pub struct Grade {
    pub title: String,
    /// The mark as the sheet shows it; empty when there is none yet.
    pub text: String,
    /// Calculated rather than typed, so it is shown with the language's decimal separator.
    pub computed: bool,
    /// What the mark is out of; `None` for a rounded column that rounds nothing yet.
    pub max_score: Option<f64>,
    /// `None` for a calculated grade.
    pub weight: Option<Weight>,
    pub tone: Tone,
    /// A final grade while the tests' weights don't add up to 100 %.
    pub provisional: bool,
}

/// The marks and grades of `pupil_id`, in sheet order; `None` when they aren't in the class.
pub fn pupil_grades(sheet: &Sheet, pupil_id: i64) -> Option<Vec<Grade>> {
    let row = sheet.rows.iter().find(|row| row.pupil.id == pupil_id)?;
    let complete = weights_add_up(&sheet.columns);
    let grades = evaluate_row(&sheet.columns, &row.values)
        .into_iter()
        .zip(&sheet.columns)
        .filter(|(_, column)| is_grade(column.kind))
        .map(|(cell, column)| Grade {
            title: column.title.clone(),
            text: if cell.is_invalid { String::new() } else { cell.text.trim().to_owned() },
            computed: column.kind.is_computed(),
            max_score: scale(&sheet.columns, column),
            weight: weight(column),
            tone: cell.tone,
            provisional: !complete && depends_on_weights(&sheet.columns, column),
        })
        .collect();
    Some(grades)
}

/// The columns a pupil sees. Listed one by one, so a new kind has to be decided on here.
fn is_grade(kind: ColumnKind) -> bool {
    match kind {
        ColumnKind::Test { .. }
        | ColumnKind::Weighted { .. }
        | ColumnKind::Average { .. }
        | ColumnKind::Round { .. } => true,
        ColumnKind::FirstName | ColumnKind::Surname | ColumnKind::Info => false,
    }
}

fn weight(column: &Column) -> Option<Weight> {
    match column.kind {
        ColumnKind::Test { weight, bonus, .. } => Some(Weight { percent: weight, bonus }),
        _ => None,
    }
}
