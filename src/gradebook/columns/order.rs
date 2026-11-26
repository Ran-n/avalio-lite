// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.590060
//+ Revised: 	2026/11/23 11:49:16.590060
// -----------------------------------------------------------------------------

//! The order of a class's columns: moving one column a place, or to a place, and numbering the places
//! again. Each move is one transaction, so the order is never half-moved.

use rusqlite::{Connection, OptionalExtension, params};

use crate::error::{AppError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

/// Gives the columns positions 1..n in the order of `ids`.
pub(super) fn renumber(connection: &Connection, ids: &[i64]) -> Result<()> {
    for (position, id) in (1_i64..).zip(ids) {
        connection.execute("UPDATE sheet_columns SET position = ?2 WHERE id = ?1", params![id, position])?;
    }
    Ok(())
}

/// Moves the column one place left or right; does nothing at either end.
pub fn move_column(connection: &Connection, column_id: i64, direction: Direction) -> Result<()> {
    let (_, index) = class_order(connection, column_id)?;
    let target = match direction {
        Direction::Left => index.saturating_sub(1),
        Direction::Right => index + 1,
    };
    move_column_to(connection, column_id, target)
}

/// Moves the column to place `index` of its class's columns (counting from 0, with the column
/// taken out first), past the end meaning last, and numbers the positions 1..n again; all in one
/// transaction. A column that doesn't exist is `NotEditable`. Moving it to where it is writes
/// nothing.
pub fn move_column_to(connection: &Connection, column_id: i64, index: usize) -> Result<()> {
    let (mut order, current) = class_order(connection, column_id)?;
    let target = index.min(order.len() - 1);
    if target == current {
        return Ok(());
    }
    order.remove(current);
    order.insert(target, column_id);
    let transaction = connection.unchecked_transaction()?;
    renumber(&transaction, &order)?;
    transaction.commit()?;
    log::info!("moved column {column_id} to {target}");
    Ok(())
}

/// The ids of the column's class's columns in order, and where the column is among them.
fn class_order(connection: &Connection, column_id: i64) -> Result<(Vec<i64>, usize)> {
    let class_id: i64 = connection
        .query_row("SELECT class_id FROM sheet_columns WHERE id = ?1", [column_id], |row| row.get(0))
        .optional()?
        .ok_or(AppError::NotEditable)?;
    let mut statement = connection.prepare("SELECT id FROM sheet_columns WHERE class_id = ?1 ORDER BY position")?;
    let order: Vec<i64> = statement.query_map([class_id], |row| row.get(0))?.collect::<rusqlite::Result<_>>()?;
    let index = order.iter().position(|id| *id == column_id).ok_or(AppError::NotEditable)?;
    Ok((order, index))
}
