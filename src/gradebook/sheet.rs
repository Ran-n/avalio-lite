// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.606846
//+ Revised: 	2026/10/28 10:52:45.339516
// -----------------------------------------------------------------------------

use std::collections::HashMap;

use rusqlite::Connection;

use super::columns::list_columns;
use super::model::{Sheet, SheetRow};
use super::pupils::list_class_pupils;
use crate::error::Result;

pub fn load_sheet(connection: &Connection, class_id: i64) -> Result<Sheet> {
    let columns = list_columns(connection, class_id)?;
    let pupils = list_class_pupils(connection, class_id)?;
    let mut values = class_cells(connection, class_id)?;
    let rows = pupils
        .into_iter()
        .map(|pupil| SheetRow {
            values: columns.iter().map(|column| values.remove(&(column.id, pupil.id)).unwrap_or_default()).collect(),
            pupil,
        })
        .collect();
    Ok(Sheet { columns, rows })
}

fn class_cells(connection: &Connection, class_id: i64) -> Result<HashMap<(i64, i64), String>> {
    let mut statement = connection.prepare(
        "SELECT c.column_id, c.pupil_id, c.value
         FROM cells c JOIN sheet_columns sc ON sc.id = c.column_id
         WHERE sc.class_id = ?1",
    )?;
    let cells = statement
        .query_map([class_id], |row| Ok(((row.get(0)?, row.get(1)?), row.get(2)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(cells)
}
