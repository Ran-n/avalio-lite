// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 10:48:30.784631
//+ Revised: 	2026/11/03 11:39:47.094562
// -----------------------------------------------------------------------------

//! Reads pupils and marks from the first tab of a workbook into a class: names in column A, and
//! from column B on one column per header, matched to the class's info and test columns by
//! title. Planning only reads, so the user can be asked before anything is replaced; applying
//! writes it all in one transaction, through the same functions typing uses.

mod apply;
mod plan;
mod read;

pub use apply::apply_import;
use plan::PlannedPupil;
pub use plan::plan_import;
pub use read::{cell_text, clock, iso_seconds, read_table, without_float_noise};

/// The first tab of a workbook: the headers from column B on, and one row per named pupil.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportTable {
    pub headers: Vec<String>,
    pub rows: Vec<ImportRow>,
    /// cells holding an Excel error (`#DIV/0!`, `#N/A`), read as empty
    pub error_cells: usize,
}

/// One pupil of the file: the name in column A and the value under each header (`None` when empty).
#[derive(Debug, Clone, PartialEq)]
pub struct ImportRow {
    pub name: String,
    pub values: Vec<Option<String>>,
}

/// A column of the file that isn't imported.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LeftOut {
    /// a header matching no info or test column, or repeating an earlier one
    Header(String),
    /// values under an empty header, by the column's letter
    Untitled(String),
}

/// What an import did, or will do.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportSummary {
    pub pupils: usize,
    pub new_pupils: usize,
    /// cells whose value changes
    pub marks: usize,
    pub left_out: Vec<LeftOut>,
    pub error_cells: usize,
}

/// An import worked out against the class, before anything is written.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportPlan {
    pupils: Vec<PlannedPupil>,
    pub summary: ImportSummary,
    /// marks or notes already in the class that the import replaces with a different value
    pub overwrites: usize,
}
