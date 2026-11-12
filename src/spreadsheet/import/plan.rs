// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.545991
//+ Revised: 	2026/11/12 14:39:54.288154
// -----------------------------------------------------------------------------

//! Matching the workbook's rows and columns to the class, before anything changes.

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;
use rust_xlsxwriter::utility::column_number_to_name;

use super::{ImportPlan, ImportSummary, ImportTable, LeftOut};
use crate::error::{AppError, Result};
use crate::gradebook::compute::parse_number;
use crate::gradebook::model::{ColumnKind, SheetRow, name_match_key};
use crate::gradebook::{pupils, sheet};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct PlannedPupil {
    pub(super) target: Target,
    /// column id and the value it gets
    pub(super) writes: Vec<(i64, String)>,
}

/// Who a row of the file is.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum Target {
    InClass(i64),
    /// the one pupil of that name in another class (or in none), who joins this one
    Enroll(i64),
    New {
        first_name: String,
        surname: String,
    },
}

/// Works out who each row is and which cells change. Reads only.
pub fn plan_import(connection: &Connection, class_id: i64, table: &ImportTable) -> Result<ImportPlan> {
    refuse_repeated_names(table)?;
    let sheet = sheet::load_sheet(connection, class_id)?;
    let mut left_out = Vec::new();
    let targets = match_headers(&sheet.columns, table, &mut left_out);

    let mut in_class: HashMap<String, &SheetRow> = HashMap::new();
    for row in &sheet.rows {
        in_class.entry(name_match_key(&row.pupil.full_name())).or_insert(row);
    }
    let mut elsewhere: HashMap<String, Vec<i64>> = HashMap::new();
    for pupil in pupils::pupils_not_in_class(connection, class_id)? {
        elsewhere.entry(name_match_key(&pupil.full_name())).or_default().push(pupil.id);
    }

    let mut overwrites = 0;
    let mut planned = Vec::new();
    for row in &table.rows {
        let key = name_match_key(&row.full_name());
        let existing = in_class.get(&key);
        let target = match (existing, elsewhere.get(&key).map(Vec::as_slice)) {
            (Some(row), _) => Target::InClass(row.pupil.id),
            (None, Some([pupil_id])) => Target::Enroll(*pupil_id),
            (None, _) => Target::New { first_name: row.first_name.clone(), surname: row.surname.clone() },
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
        new_pupils: planned.iter().filter(|pupil| matches!(pupil.target, Target::New { .. })).count(),
        marks: planned.iter().map(|pupil| pupil.writes.len()).sum(),
        left_out,
        error_cells: table.error_cells,
        unnamed_rows: table.unnamed_rows,
    };
    Ok(ImportPlan { pupils: planned, summary, overwrites })
}

/// Two rows for one pupil would have to be merged, and a mark could be lost.
fn refuse_repeated_names(table: &ImportTable) -> Result<()> {
    let mut seen = HashSet::new();
    match table.rows.iter().find(|row| !seen.insert(name_match_key(&row.full_name()))) {
        Some(row) => Err(AppError::DuplicatePupilInFile(row.full_name())),
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
            let key = name_match_key(header);
            if key.is_empty() {
                let has_values = table.rows.iter().any(|row| row.values[header_index].is_some());
                // named by where it is in the file, the name columns counted
                let column = table.header_columns.get(header_index).copied().unwrap_or(header_index);
                let letter = u16::try_from(column).map(column_number_to_name).unwrap_or_default();
                return if has_values { leave_out(LeftOut::Untitled(letter)) } else { None };
            }
            let typed = columns
                .iter()
                .enumerate()
                .find(|(_, column)| column.kind.holds_cells() && name_match_key(&column.title) == key);
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
