// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.917995
//+ Revised: 	2026/10/28 16:22:40.335541
// -----------------------------------------------------------------------------

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::compute::{FULL_WEIGHT, parse_number, weight_total};
use super::model::{Column, ColumnKind, ColumnType, clean_name};
use super::update_one;
use crate::error::{AppError, NumberField, Result};
use crate::i18n::{Language, default_column_title};

pub const DEFAULT_MAX_SCORE: f64 = 10.0;
pub const MAX_DECIMALS: u8 = 3;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

pub fn list_columns(connection: &Connection, class_id: i64) -> Result<Vec<Column>> {
    let mut statement = connection.prepare(
        "SELECT id, title, kind, weight, max_score, source_column_id, decimals, bonus
         FROM sheet_columns WHERE class_id = ?1 ORDER BY position",
    )?;
    let columns = statement.query_map([class_id], column_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(columns)
}

/// Adds a column at the end with defaults: a test takes the weight still missing to reach 100 %,
/// a rounded column rounds the last numeric column. The title is in `language` and can be renamed.
pub fn add_column(connection: &Connection, class_id: i64, column_type: ColumnType, language: Language) -> Result<i64> {
    let existing = list_columns(connection, class_id)?;
    let kind = default_kind(column_type, &existing);
    let (weight, max_score, source, decimals) = kind_fields(kind);
    connection.execute(
        "INSERT INTO sheet_columns (class_id, position, title, kind, weight, max_score, source_column_id, decimals)
         VALUES (?1, (SELECT COALESCE(MAX(position), 0) + 1 FROM sheet_columns WHERE class_id = ?1),
                 ?2, ?3, ?4, ?5, ?6, ?7)",
        params![
            class_id,
            default_title(column_type, &existing, language),
            column_type.code(),
            weight,
            max_score,
            source,
            decimals
        ],
    )?;
    let id = connection.last_insert_rowid();
    log::info!("added {} column {id} to class {class_id}", column_type.code());
    Ok(id)
}

pub fn rename_column(connection: &Connection, column_id: i64, title: &str) -> Result<()> {
    let title = clean_name(title)?;
    update_one(connection, "UPDATE sheet_columns SET title = ?2 WHERE id = ?1", params![column_id, title])
}

/// Makes a test count on top of the 100 % (extra credit), or back into it.
pub fn set_bonus(connection: &Connection, column_id: i64, bonus: bool) -> Result<()> {
    update_one(
        connection,
        "UPDATE sheet_columns SET bonus = ?2 WHERE id = ?1 AND kind = 'test'",
        params![column_id, bonus],
    )
}

pub fn set_weight(connection: &Connection, column_id: i64, text: &str) -> Result<()> {
    let weight = parse_number(text)
        .filter(|weight| *weight >= 0.0)
        .ok_or_else(|| AppError::InvalidNumber { field: NumberField::Weight, value: text.to_owned() })?;
    update_one(
        connection,
        "UPDATE sheet_columns SET weight = ?2 WHERE id = ?1 AND kind = 'test'",
        params![column_id, weight],
    )
}

pub fn set_max_score(connection: &Connection, column_id: i64, text: &str) -> Result<()> {
    let max_score = parse_number(text)
        .filter(|max| *max > 0.0)
        .ok_or_else(|| AppError::InvalidNumber { field: NumberField::MaxScore, value: text.to_owned() })?;
    update_one(
        connection,
        "UPDATE sheet_columns SET max_score = ?2 WHERE id = ?1 AND kind IN ('test', 'weighted', 'average')",
        params![column_id, max_score],
    )
}

pub fn set_decimals(connection: &Connection, column_id: i64, text: &str) -> Result<()> {
    let decimals = text
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|decimals| *decimals <= MAX_DECIMALS)
        .ok_or_else(|| AppError::InvalidNumber { field: NumberField::Decimals, value: text.to_owned() })?;
    update_one(
        connection,
        "UPDATE sheet_columns SET decimals = ?2 WHERE id = ?1 AND kind = 'round'",
        params![column_id, decimals],
    )
}

/// Points a rounded column at another column of the same class (or at nothing).
pub fn set_round_source(connection: &Connection, column_id: i64, source: Option<i64>) -> Result<()> {
    let changed = connection.execute(
        "UPDATE sheet_columns SET source_column_id = ?2
         WHERE id = ?1 AND kind = 'round' AND (?2 IS NULL OR EXISTS (
             SELECT 1 FROM sheet_columns s
             WHERE s.id = ?2 AND s.class_id = sheet_columns.class_id
               AND s.kind IN ('test', 'weighted', 'average')))",
        params![column_id, source],
    )?;
    if changed == 0 {
        return Err(AppError::InvalidRoundSource);
    }
    Ok(())
}

pub fn delete_column(connection: &Connection, column_id: i64) -> Result<()> {
    connection.execute("DELETE FROM sheet_columns WHERE id = ?1", [column_id])?;
    log::info!("deleted column {column_id}");
    Ok(())
}

/// Swaps the column with its neighbour; does nothing at either end.
pub fn move_column(connection: &Connection, column_id: i64, direction: Direction) -> Result<()> {
    let (comparison, order) = match direction {
        Direction::Left => ("<", "DESC"),
        Direction::Right => (">", "ASC"),
    };
    let neighbour: Option<(i64, i64, i64)> = connection
        .query_row(
            &format!(
                "SELECT n.id, n.position, c.position
                 FROM sheet_columns c JOIN sheet_columns n ON n.class_id = c.class_id
                 WHERE c.id = ?1 AND n.position {comparison} c.position
                 ORDER BY n.position {order} LIMIT 1"
            ),
            [column_id],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )
        .optional()?;
    let Some((neighbour_id, neighbour_position, own_position)) = neighbour else { return Ok(()) };
    let transaction = connection.unchecked_transaction()?;
    transaction
        .execute("UPDATE sheet_columns SET position = ?2 WHERE id = ?1", params![column_id, neighbour_position])?;
    transaction.execute("UPDATE sheet_columns SET position = ?2 WHERE id = ?1", params![neighbour_id, own_position])?;
    transaction.commit()?;
    Ok(())
}

fn default_kind(column_type: ColumnType, existing: &[Column]) -> ColumnKind {
    match column_type {
        ColumnType::Info => ColumnKind::Info,
        ColumnType::Test => ColumnKind::Test {
            weight: (FULL_WEIGHT - weight_total(existing)).max(0.0),
            max_score: DEFAULT_MAX_SCORE,
            bonus: false,
        },
        ColumnType::Weighted => ColumnKind::Weighted { max_score: DEFAULT_MAX_SCORE },
        ColumnType::Average => ColumnKind::Average { max_score: DEFAULT_MAX_SCORE },
        ColumnType::Round => ColumnKind::Round {
            source: existing.iter().rev().find(|column| column.kind.can_be_rounded()).map(|column| column.id),
            decimals: 0,
        },
    }
}

fn default_title(column_type: ColumnType, existing: &[Column], language: Language) -> String {
    let same_type = existing.iter().filter(|column| column.kind.column_type() == column_type).count();
    default_column_title(language, column_type, same_type + 1)
}

fn kind_fields(kind: ColumnKind) -> (Option<f64>, Option<f64>, Option<i64>, Option<u8>) {
    match kind {
        ColumnKind::Info => (None, None, None, None),
        ColumnKind::Test { weight, max_score, .. } => (Some(weight), Some(max_score), None, None),
        ColumnKind::Weighted { max_score } | ColumnKind::Average { max_score } => (None, Some(max_score), None, None),
        ColumnKind::Round { source, decimals } => (None, None, source, Some(decimals)),
    }
}

fn column_from_row(row: &Row) -> rusqlite::Result<Column> {
    let code: String = row.get(2)?;
    let kind = match code.as_str() {
        "info" => ColumnKind::Info,
        "test" => ColumnKind::Test { weight: row.get(3)?, max_score: row.get(4)?, bonus: row.get(7)? },
        "weighted" => ColumnKind::Weighted { max_score: row.get(4)? },
        "average" => ColumnKind::Average { max_score: row.get(4)? },
        "round" => ColumnKind::Round { source: row.get(5)?, decimals: row.get(6)? },
        other => {
            return Err(rusqlite::Error::FromSqlConversionFailure(
                2,
                Type::Text,
                format!("unknown column kind \"{other}\"").into(),
            ));
        }
    };
    Ok(Column { id: row.get(0)?, title: row.get(1)?, kind })
}
