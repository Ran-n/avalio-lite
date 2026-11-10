// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.782114
//+ Revised: 	2026/11/09 21:54:00.535454
// -----------------------------------------------------------------------------

use rusqlite::{Connection, ErrorCode, params};

use super::model::{ClassSummary, DuplicateContents, MIN_NAME_WIDTH, clamp_width, clean_name, name_sort_key};
use super::{cells, columns, pupils, sheet, update_one};
use crate::error::{AppError, Result};

pub fn list_classes(connection: &Connection) -> Result<Vec<ClassSummary>> {
    let mut statement = connection.prepare(
        "SELECT c.id, c.name, COUNT(cp.pupil_id)
         FROM classes c LEFT JOIN class_pupils cp ON cp.class_id = c.id
         GROUP BY c.id
         ORDER BY c.id",
    )?;
    let mut classes: Vec<ClassSummary> = statement
        .query_map([], |row| Ok(ClassSummary { id: row.get(0)?, name: row.get(1)?, pupil_count: row.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?;
    // sorted here: SQLite's NOCASE only folds A to Z, which put "Árabe" after "Zoo"
    classes.sort_by_cached_key(|class| name_sort_key(&class.name));
    Ok(classes)
}

pub fn class_name(connection: &Connection, class_id: i64) -> Result<String> {
    Ok(connection.query_row("SELECT name FROM classes WHERE id = ?1", [class_id], |row| row.get(0))?)
}

/// The width the pupil column of the class is shown at; `None` for the default.
pub fn class_name_width(connection: &Connection, class_id: i64) -> Result<Option<u16>> {
    Ok(connection.query_row("SELECT name_width FROM classes WHERE id = ?1", [class_id], |row| row.get(0))?)
}

/// Sets the width of the pupil column of the class, clamped to the limits; `None` puts it back to
/// the default.
pub fn set_name_width(connection: &Connection, class_id: i64, width: Option<u16>) -> Result<()> {
    let width = clamp_width(width, MIN_NAME_WIDTH);
    update_one(connection, "UPDATE classes SET name_width = ?2 WHERE id = ?1", params![class_id, width])?;
    log::info!("set the width of the pupil column of class {class_id} to {width:?}");
    Ok(())
}

/// Gives class `to_class` (in `to`) the pupil column width of class `from_class` (in `from`).
pub fn copy_name_width(from: &Connection, from_class: i64, to: &Connection, to_class: i64) -> Result<()> {
    match class_name_width(from, from_class)? {
        Some(width) => set_name_width(to, to_class, Some(width)),
        None => Ok(()),
    }
}

pub fn create_class(connection: &Connection, name: &str) -> Result<i64> {
    let name = clean_name(name)?;
    connection
        .execute("INSERT INTO classes (name) VALUES (?1)", [&name])
        .map_err(|error| duplicate_or(error, &name))?;
    let id = connection.last_insert_rowid();
    log::info!("created class {id} \"{name}\"");
    Ok(id)
}

/// `name` if no class has it yet, else `name (n)` with the lowest free n from 2 up.
pub fn free_class_name(connection: &Connection, name: &str) -> Result<String> {
    let name = clean_name(name)?;
    let taken = |candidate: &str| -> Result<bool> {
        let count: i64 =
            connection.query_row("SELECT COUNT(*) FROM classes WHERE name = ?1", [candidate], |row| row.get(0))?;
        Ok(count > 0)
    };
    if !taken(&name)? {
        return Ok(name);
    }
    let mut number = 2;
    loop {
        let candidate = format!("{name} ({number})");
        if !taken(&candidate)? {
            return Ok(candidate);
        }
        number += 1;
    }
}

/// Copies a class into a new one named by `free_class_name` ("1A (2)"), with its columns and,
/// depending on `contents`, its pupils (shared, not new ones) and typed marks. All or nothing.
/// Returns the copy's id.
pub fn duplicate_class(connection: &Connection, class_id: i64, contents: DuplicateContents) -> Result<i64> {
    let transaction = connection.unchecked_transaction()?;
    let name = free_class_name(&transaction, &class_name(&transaction, class_id)?)?;
    let new_id = create_class(&transaction, &name)?;
    let original = sheet::load_sheet(&transaction, class_id)?;

    let column_ids = columns::copy_columns(&transaction, new_id, &original.columns)?;
    copy_name_width(&transaction, class_id, &transaction, new_id)?;

    if contents != DuplicateContents::Columns {
        // enrolling appends, so this keeps the row order
        for row in &original.rows {
            pupils::add_existing_pupil(&transaction, new_id, row.pupil.id)?;
        }
    }
    if contents == DuplicateContents::Everything {
        for row in &original.rows {
            for (column, value) in original.columns.iter().zip(&row.values) {
                if !column.kind.is_computed() && !value.is_empty() {
                    cells::set_cell(&transaction, column_ids[&column.id], row.pupil.id, value)?;
                }
            }
        }
    }
    transaction.commit()?;
    log::info!("duplicated class {class_id} as {new_id} ({contents:?})");
    Ok(new_id)
}

pub fn rename_class(connection: &Connection, class_id: i64, name: &str) -> Result<()> {
    let name = clean_name(name)?;
    let changed = connection
        .execute("UPDATE classes SET name = ?2 WHERE id = ?1", params![class_id, name])
        .map_err(|error| duplicate_or(error, &name))?;
    if changed == 0 {
        return Err(AppError::NotEditable);
    }
    Ok(())
}

/// The position `delta` steps from `current` in a list of `len` classes, going round at either end;
/// with nothing selected, the first (forwards) or last (backwards). `None` when there are none.
pub fn adjacent_index(len: usize, current: Option<usize>, delta: i32) -> Option<usize> {
    let len_signed = i64::try_from(len).ok().filter(|len| *len > 0)?;
    let start = match current {
        Some(index) => i64::try_from(index).ok()?,
        None if delta > 0 => -1,
        None => len_signed,
    };
    usize::try_from((start + i64::from(delta)).rem_euclid(len_signed)).ok()
}

pub fn delete_class(connection: &Connection, class_id: i64) -> Result<()> {
    update_one(connection, "DELETE FROM classes WHERE id = ?1", [class_id])?;
    log::info!("deleted class {class_id}");
    Ok(())
}

fn duplicate_or(error: rusqlite::Error, name: &str) -> AppError {
    match error.sqlite_error_code() {
        Some(ErrorCode::ConstraintViolation) => AppError::DuplicateClassName(name.to_owned()),
        _ => AppError::Database(error),
    }
}
