// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.654243
//+ Revised: 	2026/11/12 14:39:51.842607
// -----------------------------------------------------------------------------

use std::collections::HashMap;

use rusqlite::{Connection, OptionalExtension, params};

use super::model::Column;
use super::update_one;

use crate::error::{AppError, Result};

/// Copies one pupil's typed marks from a sheet row (`values`, under `columns`) into another class,
/// as pupil `pupil_id` there: `copied` maps each original column's id to its copy's (what
/// `columns::copy_columns` returns). Computed and name columns hold no cells, so they are skipped.
pub fn copy_row_marks(
    connection: &Connection,
    columns: &[Column],
    values: &[String],
    copied: &HashMap<i64, i64>,
    pupil_id: i64,
) -> Result<()> {
    for (column, value) in columns.iter().zip(values) {
        if column.kind.holds_cells() && !value.is_empty() {
            let copy = copied.get(&column.id).ok_or(AppError::NotEditable)?;
            set_cell(connection, *copy, pupil_id, value)?;
        }
    }
    Ok(())
}

/// Stores what was typed in an info or test cell; an empty value clears the cell.
/// Calculated columns can't be typed into, and only pupils of the column's class have cells.
pub fn set_cell(connection: &Connection, column_id: i64, pupil_id: i64, value: &str) -> Result<()> {
    let kind: Option<String> = connection
        .query_row("SELECT kind FROM sheet_columns WHERE id = ?1", [column_id], |row| row.get(0))
        .optional()?;
    if !matches!(kind.as_deref(), Some("info" | "test")) {
        return Err(AppError::NotEditable);
    }
    if value.trim().is_empty() {
        connection.execute("DELETE FROM cells WHERE column_id = ?1 AND pupil_id = ?2", params![column_id, pupil_id])?;
    } else {
        // a cell for a pupil outside the class would never show, and would come back if they joined it later
        update_one(
            connection,
            "INSERT INTO cells (column_id, pupil_id, value)
             SELECT ?1, ?2, ?3 WHERE EXISTS (
                 SELECT 1 FROM class_pupils cp JOIN sheet_columns sc ON sc.class_id = cp.class_id
                 WHERE sc.id = ?1 AND cp.pupil_id = ?2)
             ON CONFLICT (column_id, pupil_id) DO UPDATE SET value = excluded.value",
            params![column_id, pupil_id, value],
        )?;
    }
    Ok(())
}
