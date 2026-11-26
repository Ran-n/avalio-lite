// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.470617
//+ Revised: 	2026/11/23 11:49:20.163034
// -----------------------------------------------------------------------------

//! Reading the workbook's first tab into text cells, as the sheet would show them: the headers
//! of its first row, the names' columns among them, and one row per pupil below.

use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use calamine::{Data, Range, Reader, open_workbook_auto};

use super::cell::cell_text;
use super::{ImportRow, ImportTable, NameHeaders};
use crate::error::{AppError, Result};
use crate::gradebook::model::{name_match_key, split_listed_name};

/// The first tab of the workbook at `path` (.xlsx, .xls or .ods), its name columns found by their
/// headers: the first-name and surname columns wherever their headers are, else the whole name in
/// column A.
pub fn read_table(path: &Path, names: &NameHeaders) -> Result<ImportTable> {
    // a malformed file could make the reader panic; that is a file it can't read, not a crash
    panic::catch_unwind(AssertUnwindSafe(|| read_first_tab(path, names))).unwrap_or_else(|_| {
        Err(AppError::SpreadsheetRead { path: path.to_path_buf(), reason: "the reader panicked".to_owned() })
    })
}

fn read_first_tab(path: &Path, names: &NameHeaders) -> Result<ImportTable> {
    let read_error = |reason: String| AppError::SpreadsheetRead { path: path.to_path_buf(), reason };
    let mut workbook = open_workbook_auto(path).map_err(|error| read_error(error.to_string()))?;
    let no_pupils = || AppError::NoPupilsInFile(path.to_path_buf());
    let first = workbook.sheet_names().into_iter().next().ok_or_else(no_pupils)?;
    let range = workbook.worksheet_range(&first).map_err(|error| read_error(error.to_string()))?;
    // the range starts at the first cell holding anything, which needn't be A1: the headers are
    // its first row and the names' fallback column its first column
    let (first_row, first_column) = range.start().ok_or_else(no_pupils)?;
    let (last_row, last_column) = range.end().ok_or_else(no_pupils)?;
    let span = Span { first_row, first_column, last_row, last_column };
    let mut grid = Grid { range: &range, error_cells: 0 };
    let (found, header_columns, headers) = read_headers(&mut grid, span, names);
    let (rows, unnamed_rows) = read_pupils(&mut grid, span, found, &header_columns);
    if rows.is_empty() {
        return Err(no_pupils());
    }
    let header_columns = header_columns.into_iter().map(|column| column as usize).collect();
    Ok(ImportTable { header_columns, headers, rows, error_cells: grid.error_cells, unnamed_rows })
}

/// The cells of the first tab, from its first row and column to its last.
#[derive(Debug, Clone, Copy)]
struct Span {
    first_row: u32,
    first_column: u32,
    last_row: u32,
    last_column: u32,
}

/// The tab's cells as text, counting the error cells read as empty.
struct Grid<'a> {
    range: &'a Range<Data>,
    /// cells holding an Excel error (`#DIV/0!`, `#N/A`), read as empty
    error_cells: usize,
}

impl Grid<'_> {
    /// The cell's text, or `None` when it is empty or an error.
    fn text(&mut self, row: u32, column: u32) -> Option<String> {
        let data = self.range.get_value((row, column)).unwrap_or(&Data::Empty);
        if matches!(data, Data::Error(_)) {
            self.error_cells += 1;
        }
        cell_text(data)
    }
}

/// The first row's headers: where the names are, the columns of the other headers (as columns of
/// the sheet), and those headers' text.
fn read_headers(grid: &mut Grid, span: Span, names: &NameHeaders) -> (NameColumns, Vec<u32>, Vec<String>) {
    let columns = span.first_column..=span.last_column;
    let first_row: Vec<String> =
        columns.clone().map(|column| grid.text(span.first_row, column).unwrap_or_default()).collect();
    let found = NameColumns::find(&first_row, names).offset_by(span.first_column);
    let header_columns: Vec<u32> = columns.filter(|column| !found.holds(*column)).collect();
    let headers =
        header_columns.iter().map(|column| first_row[(column - span.first_column) as usize].clone()).collect();
    (found, header_columns, headers)
}

/// Every row under the headers: the pupils, and how many rows were skipped as unnamed.
fn read_pupils(grid: &mut Grid, span: Span, found: NameColumns, header_columns: &[u32]) -> (Vec<ImportRow>, usize) {
    let mut rows = Vec::new();
    let mut unnamed_rows = 0;
    for row in span.first_row + 1..=span.last_row {
        match read_row(grid, row, found, header_columns) {
            RowRead::Pupil(pupil) => rows.push(pupil),
            RowRead::Unnamed => unnamed_rows += 1,
            RowRead::Blank => {}
        }
    }
    (rows, unnamed_rows)
}

/// What a row under the headers holds.
enum RowRead {
    Pupil(ImportRow),
    /// no first name, but something else in the row: counted, so the report can say it was skipped
    Unnamed,
    /// nothing in it at all
    Blank,
}

/// One row: its pupil when it has a first name. A row with no first name isn't a pupil, even with
/// a surname; one with anything else in it is `Unnamed`.
fn read_row(grid: &mut Grid, row: u32, found: NameColumns, header_columns: &[u32]) -> RowRead {
    let surname = found.surname.and_then(|column| grid.text(row, column));
    let values: Vec<Option<String>> = header_columns.iter().map(|column| grid.text(row, *column)).collect();
    let Some(first_name) = grid.text(row, found.first_name) else {
        let has_content = surname.is_some() || values.iter().any(Option::is_some);
        return if has_content { RowRead::Unnamed } else { RowRead::Blank };
    };
    let (first_name, surname) = split_name(first_name, surname, found);
    RowRead::Pupil(ImportRow { first_name, surname, values })
}

/// A pupil's names. With no surname column, a name written surname first ("Pérez López, Ana") is
/// split at its comma, as school lists write it; otherwise the first name stands alone.
fn split_name(first_name: String, surname: Option<String>, found: NameColumns) -> (String, String) {
    match surname {
        Some(surname) => (first_name, surname),
        None if found.surname.is_none() => split_listed_name(&first_name).unwrap_or((first_name, String::new())),
        None => (first_name, String::new()),
    }
}

/// Where a file keeps its pupils' names, from the headers of its first row (compared as names are,
/// in any language): the column headed by a first-name label and the one headed by a surname label,
/// wherever they are. With no first-name header, column A holds the first name (or the whole name,
/// when there is no surname header either), or column B when column A is the surname.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct NameColumns {
    first_name: u32,
    surname: Option<u32>,
}

impl NameColumns {
    fn find(first_row: &[String], names: &NameHeaders) -> Self {
        let headed = |labels: &[&str]| {
            let keys: Vec<String> = labels.iter().map(|label| name_match_key(label)).collect();
            first_row.iter().position(|header| keys.contains(&name_match_key(header))).map(|index| index as u32)
        };
        let surname = headed(names.surname);
        let first_name = headed(names.first_name)
            .filter(|column| Some(*column) != surname)
            .unwrap_or(if surname == Some(0) { 1 } else { 0 });
        Self { first_name, surname }
    }

    /// The columns found among headers read from column `first` on, as columns of the sheet.
    fn offset_by(self, first: u32) -> Self {
        Self { first_name: self.first_name + first, surname: self.surname.map(|column| column + first) }
    }

    fn holds(&self, column: u32) -> bool {
        column == self.first_name || Some(column) == self.surname
    }
}
