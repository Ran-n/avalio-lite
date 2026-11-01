// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 10:48:30.784631
//+ Revised: 	2026/10/29 15:12:04.022576
// -----------------------------------------------------------------------------

//! Reads pupils and marks from the first tab of a workbook into a class: names in column A, and
//! from column B on one column per header, matched to the class's info and test columns by
//! title. Planning only reads, so the user can be asked before anything is replaced; applying
//! writes it all in one transaction, through the same functions typing uses.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use calamine::{Data, Reader, open_workbook_auto};
use rusqlite::Connection;
use rust_xlsxwriter::utility::column_number_to_name;

use crate::error::{AppError, Result};
use crate::gradebook::compute::parse_number;
use crate::gradebook::model::{ColumnKind, SheetRow};
use crate::gradebook::{cells, pupils, sheet};

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

#[derive(Debug, Clone, PartialEq)]
struct PlannedPupil {
    target: Target,
    /// column id and the value it gets
    writes: Vec<(i64, String)>,
}

/// Who a row of the file is.
#[derive(Debug, Clone, PartialEq)]
enum Target {
    InClass(i64),
    /// the one pupil of that name in another class (or in none), who joins this one
    Enroll(i64),
    New(String),
}

/// The first tab of the workbook at `path` (.xlsx, .xls or .ods).
pub fn read_table(path: &Path) -> Result<ImportTable> {
    let read_error = |reason: String| AppError::SpreadsheetRead { path: path.to_path_buf(), reason };
    let mut workbook = open_workbook_auto(path).map_err(|error| read_error(error.to_string()))?;
    let no_pupils = || AppError::NoPupilsInFile(path.to_path_buf());
    let first = workbook.sheet_names().into_iter().next().ok_or_else(no_pupils)?;
    let range = workbook.worksheet_range(&first).map_err(|error| read_error(error.to_string()))?;
    let (last_row, last_column) = range.end().ok_or_else(no_pupils)?;
    let mut error_cells = 0;
    let mut text = |row: u32, column: u32| {
        let data = range.get_value((row, column)).unwrap_or(&Data::Empty);
        if matches!(data, Data::Error(_)) {
            error_cells += 1;
        }
        cell_text(data)
    };
    let headers = (1..=last_column).map(|column| text(0, column).unwrap_or_default()).collect();
    let mut rows = Vec::new();
    for row in 1..=last_row {
        if let Some(name) = text(row, 0) {
            rows.push(ImportRow { name, values: (1..=last_column).map(|column| text(row, column)).collect() });
        }
    }
    if rows.is_empty() {
        return Err(no_pupils());
    }
    Ok(ImportTable { headers, rows, error_cells })
}

/// A cell as the text typing it would store: numbers without a locale (`7.5`, Rust's shortest
/// form), text trimmed, `TRUE`/`FALSE`, dates as `YYYY-MM-DD`, durations as `H:MM:SS`, the same
/// from .xlsx, .xls and .ods. `None` for empty and error cells.
fn cell_text(data: &Data) -> Option<String> {
    let text = match data {
        Data::Empty | Data::Error(_) => return None,
        Data::Int(number) => number.to_string(),
        Data::Float(number) => number.to_string(),
        Data::String(text) => text.trim().to_owned(),
        Data::Bool(value) => (if *value { "TRUE" } else { "FALSE" }).to_owned(),
        Data::DateTime(moment) if moment.is_duration() => clock((moment.as_f64() * SECONDS_PER_DAY).round()),
        Data::DateTime(moment) => {
            let (year, month, day, ..) = moment.to_ymd_hms_milli();
            format!("{year:04}-{month:02}-{day:02}")
        }
        // .ods: "2011-03-09" or "2011-03-09T08:30:00"
        Data::DateTimeIso(text) => text.split('T').next().unwrap_or_default().to_owned(),
        // .ods: "PT01H30M05S"; anything else is kept as written
        Data::DurationIso(text) => iso_seconds(text).map_or_else(|| text.clone(), clock),
    };
    (!text.is_empty()).then_some(text)
}

const SECONDS_PER_DAY: f64 = 86_400.0;

/// `seconds` (whole) as hours, minutes and seconds: 5405 is "1:30:05".
fn clock(seconds: f64) -> String {
    let seconds = seconds as i64;
    format!("{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

/// The length of a time duration as .ods files write it, `PT01H30M05S`, in whole seconds.
fn iso_seconds(text: &str) -> Option<f64> {
    let (hours, rest) = text.strip_prefix("PT")?.split_once('H')?;
    let (minutes, rest) = rest.split_once('M')?;
    let seconds = rest.strip_suffix('S')?;
    let number = |text: &str| text.parse::<f64>().ok();
    Some((number(hours)? * 3600.0 + number(minutes)? * 60.0 + number(seconds)?).round())
}

/// Names and titles match ignoring case and spaces at either end.
fn match_key(text: &str) -> String {
    text.trim().to_lowercase()
}

/// Works out who each row is and which cells change. Reads only.
pub fn plan_import(connection: &Connection, class_id: i64, table: &ImportTable) -> Result<ImportPlan> {
    refuse_repeated_names(table)?;
    let sheet = sheet::load_sheet(connection, class_id)?;
    let mut left_out = Vec::new();
    let targets = match_headers(&sheet.columns, table, &mut left_out);

    let mut in_class: HashMap<String, &SheetRow> = HashMap::new();
    for row in &sheet.rows {
        in_class.entry(match_key(&row.pupil.name)).or_insert(row);
    }
    let mut elsewhere: HashMap<String, Vec<i64>> = HashMap::new();
    for pupil in pupils::pupils_not_in_class(connection, class_id)? {
        elsewhere.entry(match_key(&pupil.name)).or_default().push(pupil.id);
    }

    let mut overwrites = 0;
    let mut planned = Vec::new();
    for row in &table.rows {
        let key = match_key(&row.name);
        let existing = in_class.get(&key);
        let target = match (existing, elsewhere.get(&key).map(Vec::as_slice)) {
            (Some(row), _) => Target::InClass(row.pupil.id),
            (None, Some([pupil_id])) => Target::Enroll(*pupil_id),
            (None, _) => Target::New(row.name.clone()),
        };
        let mut writes = Vec::new();
        for (value, target) in row.values.iter().zip(&targets) {
            let (Some(value), Some((index, column_id))) = (value, target) else { continue };
            let current = existing.map_or("", |row| row.values[*index].as_str());
            if same_value(sheet.columns[*index].kind, current, value) {
                continue;
            }
            if !current.trim().is_empty() {
                overwrites += 1;
            }
            writes.push((*column_id, value.clone()));
        }
        planned.push(PlannedPupil { target, writes });
    }

    let summary = ImportSummary {
        pupils: planned.len(),
        new_pupils: planned.iter().filter(|pupil| matches!(pupil.target, Target::New(_))).count(),
        marks: planned.iter().map(|pupil| pupil.writes.len()).sum(),
        left_out,
        error_cells: table.error_cells,
    };
    Ok(ImportPlan { pupils: planned, summary, overwrites })
}

/// Two rows for one pupil would have to be merged, and a mark could be lost.
fn refuse_repeated_names(table: &ImportTable) -> Result<()> {
    let mut seen = HashSet::new();
    match table.rows.iter().find(|row| !seen.insert(match_key(&row.name))) {
        Some(row) => Err(AppError::DuplicatePupilInFile(row.name.clone())),
        None => Ok(()),
    }
}

/// For each header of the file, the sheet column it fills (its index and id), if any. Each
/// header goes to the first info or test column with its title; calculated columns, unknown
/// titles and a repeated header are left out, and so is an untitled column with values.
fn match_headers(
    columns: &[crate::gradebook::model::Column],
    table: &ImportTable,
    left_out: &mut Vec<LeftOut>,
) -> Vec<Option<(usize, i64)>> {
    let mut seen = HashSet::new();
    let mut leave_out = |reason: LeftOut| {
        if !left_out.contains(&reason) {
            left_out.push(reason);
        }
        None
    };
    table
        .headers
        .iter()
        .enumerate()
        .map(|(header_index, header)| {
            let key = match_key(header);
            if key.is_empty() {
                let has_values = table.rows.iter().any(|row| row.values[header_index].is_some());
                // column A holds the names, so the first header is column B
                let letter = u16::try_from(header_index + 1).map(column_number_to_name).unwrap_or_default();
                return if has_values { leave_out(LeftOut::Untitled(letter)) } else { None };
            }
            let typed = columns
                .iter()
                .enumerate()
                .find(|(_, column)| !column.kind.is_computed() && match_key(&column.title) == key);
            match typed {
                Some((index, column)) if seen.insert(key.clone()) => Some((index, column.id)),
                _ => leave_out(LeftOut::Header(header.trim().to_owned())),
            }
        })
        .collect()
}

/// Whether writing `new` over `current` would change nothing: the same text, or for a test the
/// same score however it is written ("7,5" and 7.5).
fn same_value(kind: ColumnKind, current: &str, new: &str) -> bool {
    if current.trim() == new.trim() {
        return true;
    }
    matches!(kind, ColumnKind::Test { .. })
        && parse_number(current).is_some_and(|current| parse_number(new) == Some(current))
}

/// Writes the plan in one transaction: any failure leaves the database as it was.
pub fn apply_import(connection: &Connection, class_id: i64, plan: &ImportPlan) -> Result<ImportSummary> {
    let transaction = connection.unchecked_transaction()?;
    for pupil in &plan.pupils {
        let pupil_id = match &pupil.target {
            Target::InClass(pupil_id) => *pupil_id,
            Target::Enroll(pupil_id) => {
                pupils::add_existing_pupil(&transaction, class_id, *pupil_id)?;
                *pupil_id
            }
            Target::New(name) => pupils::insert_new_pupil(&transaction, class_id, name)?,
        };
        for (column_id, value) in &pupil.writes {
            cells::set_cell(&transaction, *column_id, pupil_id, value)?;
        }
    }
    transaction.commit()?;
    let summary = &plan.summary;
    log::info!(
        "imported {} pupils ({} new) and {} values into class {class_id}",
        summary.pupils,
        summary.new_pupils,
        summary.marks
    );
    Ok(summary.clone())
}
