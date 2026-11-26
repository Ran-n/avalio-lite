// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.521636
//+ Revised: 	2026/11/23 17:29:49.004371
// -----------------------------------------------------------------------------

//! The settings a column's kind takes, each set on its own and checked: a test's weight and maximum,
//! a weighted or average column's maximum, a rounded column's decimals and source, and whether a
//! test counts as a bonus.

use rusqlite::{Connection, OptionalExtension, params};

use super::{MAX_DECIMALS, MAX_SCORE_LIMIT, MAX_WEIGHT};
use crate::error::{AppError, NumberField, Result};
use crate::gradebook::compute::parse_number;
use crate::gradebook::update_one;

/// Makes a test count on top of the 100 % (extra credit), or back into it.
#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the yes/no value being saved or shown, not a switch between two behaviours"
)]
pub fn set_bonus(connection: &Connection, column_id: i64, bonus: bool) -> Result<()> {
    update_one(
        connection,
        "UPDATE sheet_columns SET bonus = ?2 WHERE id = ?1 AND kind = 'test'",
        params![column_id, bonus],
    )?;
    log::info!("set column {column_id} to count as a bonus: {bonus}");
    Ok(())
}

pub fn set_weight(connection: &Connection, column_id: i64, text: &str) -> Result<()> {
    let weight = parse_number(text)
        .filter(|weight| (0.0..=MAX_WEIGHT).contains(weight))
        .ok_or_else(|| AppError::InvalidNumber { field: NumberField::Weight, value: text.to_owned() })?;
    update_one(
        connection,
        "UPDATE sheet_columns SET weight = ?2 WHERE id = ?1 AND kind = 'test'",
        params![column_id, weight],
    )?;
    log::info!("set the weight of column {column_id} to {weight}");
    Ok(())
}

pub fn set_max_score(connection: &Connection, column_id: i64, text: &str) -> Result<()> {
    let max_score = parse_number(text)
        .filter(|max| *max > 0.0 && *max <= MAX_SCORE_LIMIT)
        .ok_or_else(|| AppError::InvalidNumber { field: NumberField::MaxScore, value: text.to_owned() })?;
    update_one(
        connection,
        "UPDATE sheet_columns SET max_score = ?2 WHERE id = ?1 AND kind IN ('test', 'weighted', 'average')",
        params![column_id, max_score],
    )?;
    log::info!("set the maximum score of column {column_id} to {max_score}");
    Ok(())
}

pub fn set_decimals(connection: &Connection, column_id: i64, text: &str) -> Result<()> {
    // digits only: Rust's parse would take "+2", which no other number field accepts
    let digits = text.trim();
    let decimals = (!digits.is_empty() && digits.bytes().all(|byte| byte.is_ascii_digit()))
        .then(|| digits.parse::<u8>().ok())
        .flatten()
        .filter(|decimals| *decimals <= MAX_DECIMALS)
        .ok_or_else(|| AppError::InvalidNumber { field: NumberField::Decimals, value: text.to_owned() })?;
    update_one(
        connection,
        "UPDATE sheet_columns SET decimals = ?2 WHERE id = ?1 AND kind = 'round'",
        params![column_id, decimals],
    )?;
    log::info!("set column {column_id} to round to {decimals} decimals");
    Ok(())
}

/// Points a rounded column at a test, final grade or average of its class, or at nothing. The link
/// is one row of `round_sources`. A column that isn't rounded, or a source that doesn't fit, is
/// `InvalidRoundSource`, checked here before the file's own triggers would refuse it.
pub fn set_round_source(connection: &Connection, column_id: i64, source: Option<i64>) -> Result<()> {
    let class_id = rounded_column_class(connection, column_id)?.ok_or(AppError::InvalidRoundSource)?;
    match source {
        Some(source) => {
            require_round_source(connection, class_id, source)?;
            connection.execute(
                "INSERT OR REPLACE INTO round_sources (column_id, source_column_id) VALUES (?1, ?2)",
                params![column_id, source],
            )?;
        }
        None => {
            connection.execute("DELETE FROM round_sources WHERE column_id = ?1", [column_id])?;
        }
    }
    log::info!("set rounded column {column_id} to round {source:?}");
    Ok(())
}

/// The class of a rounded column; `None` when there is no such column or it isn't rounded.
fn rounded_column_class(connection: &Connection, column_id: i64) -> Result<Option<i64>> {
    let class_id = connection
        .query_row("SELECT class_id FROM sheet_columns WHERE id = ?1 AND kind = 'round'", [column_id], |row| row.get(0))
        .optional()?;
    Ok(class_id)
}

/// Refuses a source that a rounded column of `class_id` can't round (`InvalidRoundSource`).
pub(super) fn require_round_source(connection: &Connection, class_id: i64, source: i64) -> Result<()> {
    let found: bool = connection.query_row(
        "SELECT EXISTS (SELECT 1 FROM sheet_columns
             WHERE id = ?1 AND class_id = ?2 AND kind IN ('test', 'weighted', 'average'))",
        params![source, class_id],
        |row| row.get(0),
    )?;
    if found { Ok(()) } else { Err(AppError::InvalidRoundSource) }
}
