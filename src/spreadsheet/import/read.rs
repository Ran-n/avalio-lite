// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.470617
//+ Revised: 	2026/11/11 17:11:00.785570
// -----------------------------------------------------------------------------

//! Reading the workbook's first tab into text cells, as the sheet would show them.

use std::panic::{self, AssertUnwindSafe};
use std::path::Path;

use calamine::{Data, Reader, open_workbook_auto};

use super::{ImportRow, ImportTable, NameHeaders};
use crate::error::{AppError, Result};
use crate::gradebook::model::name_match_key;

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
    let (last_row, last_column) = range.end().ok_or_else(no_pupils)?;
    let mut error_cells = 0;
    let mut text = |row: u32, column: u32| {
        let data = range.get_value((row, column)).unwrap_or(&Data::Empty);
        if matches!(data, Data::Error(_)) {
            error_cells += 1;
        }
        cell_text(data)
    };
    let first_row: Vec<String> = (0..=last_column).map(|column| text(0, column).unwrap_or_default()).collect();
    let found = NameColumns::find(&first_row, names);
    let header_columns: Vec<u32> = (0..=last_column).filter(|column| !found.holds(*column)).collect();
    let headers = header_columns.iter().map(|column| first_row[*column as usize].clone()).collect();
    let mut rows = Vec::new();
    for row in 1..=last_row {
        // a row with no first name isn't a pupil, even with a surname
        let Some(first_name) = text(row, found.first_name) else { continue };
        let surname = found.surname.and_then(|column| text(row, column)).unwrap_or_default();
        let values = header_columns.iter().map(|column| text(row, *column)).collect();
        rows.push(ImportRow { first_name, surname, values });
    }
    if rows.is_empty() {
        return Err(no_pupils());
    }
    let header_columns = header_columns.into_iter().map(|column| column as usize).collect();
    Ok(ImportTable { header_columns, headers, rows, error_cells })
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

    fn holds(&self, column: u32) -> bool {
        column == self.first_name || Some(column) == self.surname
    }
}

/// A cell as the text typing it would store: numbers without a locale (`7.5`, Rust's shortest
/// form of the number Excel shows), text trimmed, `TRUE`/`FALSE`, dates as `YYYY-MM-DD`,
/// durations and times of day as `H:MM:SS`, the same from .xlsx, .xls and .ods. `None` for
/// empty and error cells.
pub fn cell_text(data: &Data) -> Option<String> {
    let text = match data {
        Data::Empty | Data::Error(_) => return None,
        Data::Int(number) => number.to_string(),
        Data::Float(number) if number.is_finite() => without_float_noise(*number).to_string(),
        Data::Float(_) => return None,
        Data::String(text) => text.trim().to_owned(),
        Data::Bool(value) => (if *value { "TRUE" } else { "FALSE" }).to_owned(),
        // a duration, or a time of day with no date (Excel counts it from day 0)
        Data::DateTime(moment) if moment.is_duration() || moment.as_f64().abs() < 1.0 => {
            clock((moment.as_f64() * SECONDS_PER_DAY).round())
        }
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

/// What Excel itself shows of a number: the digits past these are binary noise.
const SIGNIFICANT_DIGITS: usize = 15;

/// `number` to Excel's 15 significant digits, so a formula's 7.2999999999999989 is 7.3.
pub fn without_float_noise(number: f64) -> f64 {
    format!("{number:.*e}", SIGNIFICANT_DIGITS - 1).parse().unwrap_or(number)
}

/// `seconds` (whole) as hours, minutes and seconds: 5405 is "1:30:05", -5405 is "-1:30:05".
pub fn clock(seconds: f64) -> String {
    let sign = if seconds < 0.0 { "-" } else { "" };
    let seconds = seconds.abs() as i64;
    format!("{sign}{}:{:02}:{:02}", seconds / 3600, seconds / 60 % 60, seconds % 60)
}

/// The length of a time duration as .ods files write it, `PT01H30M05S`, in whole seconds.
pub fn iso_seconds(text: &str) -> Option<f64> {
    let (hours, rest) = text.strip_prefix("PT")?.split_once('H')?;
    let (minutes, rest) = rest.split_once('M')?;
    let seconds = rest.strip_suffix('S')?;
    let number = |text: &str| text.parse::<f64>().ok();
    Some((number(hours)? * 3600.0 + number(minutes)? * 60.0 + number(seconds)?).round())
}
