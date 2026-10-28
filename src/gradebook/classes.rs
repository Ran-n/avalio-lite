// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.782114
//+ Revised: 	2026/10/28 16:22:40.398544
// -----------------------------------------------------------------------------

use rusqlite::{Connection, ErrorCode, params};

use super::model::{ClassSummary, clean_name};
use crate::error::{AppError, Result};

pub fn list_classes(connection: &Connection) -> Result<Vec<ClassSummary>> {
    let mut statement = connection.prepare(
        "SELECT c.id, c.name, COUNT(cp.pupil_id)
         FROM classes c LEFT JOIN class_pupils cp ON cp.class_id = c.id
         GROUP BY c.id
         ORDER BY c.name COLLATE NOCASE",
    )?;
    let classes = statement
        .query_map([], |row| Ok(ClassSummary { id: row.get(0)?, name: row.get(1)?, pupil_count: row.get(2)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(classes)
}

pub fn class_name(connection: &Connection, class_id: i64) -> Result<String> {
    Ok(connection.query_row("SELECT name FROM classes WHERE id = ?1", [class_id], |row| row.get(0))?)
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

pub fn delete_class(connection: &Connection, class_id: i64) -> Result<()> {
    connection.execute("DELETE FROM classes WHERE id = ?1", [class_id])?;
    log::info!("deleted class {class_id}");
    Ok(())
}

fn duplicate_or(error: rusqlite::Error, name: &str) -> AppError {
    match error.sqlite_error_code() {
        Some(ErrorCode::ConstraintViolation) => AppError::DuplicateClassName(name.to_owned()),
        _ => AppError::Database(error),
    }
}
