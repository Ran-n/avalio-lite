// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 10:48:30.784631
//+ Revised: 	2026/11/12 14:39:54.178024
// -----------------------------------------------------------------------------

//! Reads pupils and marks from the first tab of a workbook into a class: the names from the columns
//! headed by a first-name and a surname label of any language, wherever they are (else the whole
//! name from column A), and every other column by its header, matched to the class's info and test
//! columns by title. Planning only reads, so the user can be asked before anything is replaced; applying
//! writes it all in one transaction, through the same functions typing uses.

mod apply;
mod plan;
mod read;

pub use apply::apply_import;
use plan::PlannedPupil;
pub use plan::plan_import;
pub use read::{cell_text, clock, iso_seconds, read_table, without_float_noise};

/// The labels a file's name columns may be headed by: those of every language, so a file made in
/// either is read the same whatever the interface language is.
#[derive(Debug, Clone, Copy)]
pub struct NameHeaders<'a> {
    pub first_name: &'a [&'a str],
    pub surname: &'a [&'a str],
}

/// The first tab of a workbook: the headers of every column but the names, and one row per named
/// pupil.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportTable {
    /// where each header is in the file (0 for column A), so a column is named by its real letter
    pub header_columns: Vec<usize>,
    pub headers: Vec<String>,
    pub rows: Vec<ImportRow>,
    /// cells holding an Excel error (`#DIV/0!`, `#N/A`), read as empty
    pub error_cells: usize,
    /// rows with something in them but no first name, which aren't pupils and are skipped
    pub unnamed_rows: usize,
}

/// One pupil of the file: the names and the value under each header (`None` when empty). From a
/// one-name-column file, a name written surname first ("Pérez López, Ana") is split at its comma;
/// any other is the first name, with no surname.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportRow {
    pub first_name: String,
    pub surname: String,
    pub values: Vec<Option<String>>,
}

impl ImportRow {
    /// The name as one string, how the row is matched to pupils: "Ana Pérez", or "Ana".
    pub fn full_name(&self) -> String {
        crate::gradebook::model::full_name(&self.first_name, &self.surname)
    }
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
    /// rows skipped for having no first name
    pub unnamed_rows: usize,
}

/// An import worked out against the class, before anything is written.
#[derive(Debug, Clone, PartialEq)]
pub struct ImportPlan {
    pupils: Vec<PlannedPupil>,
    pub summary: ImportSummary,
    /// marks or notes already in the class that the import replaces with a different value
    pub overwrites: usize,
}
