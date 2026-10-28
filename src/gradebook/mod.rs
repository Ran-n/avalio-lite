// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.189679
//+ Revised: 	2026/10/28 16:22:40.272513
// -----------------------------------------------------------------------------

//! Classes, pupils and each class's sheet: storage and grade calculation, with no UI code.

pub mod cells;
pub mod classes;
pub mod columns;
pub mod compute;
pub mod model;
pub mod pupils;
pub mod sheet;

use rusqlite::{Connection, Params};

use crate::error::{AppError, Result};

/// Runs a write that must hit exactly the one row it targets; a missing row is `NotEditable`
/// rather than a silent no-op.
fn update_one(connection: &Connection, sql: &str, parameters: impl Params) -> Result<()> {
    if connection.execute(sql, parameters)? == 0 {
        return Err(AppError::NotEditable);
    }
    Ok(())
}
