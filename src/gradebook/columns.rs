// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.917995
//+ Revised: 	2026/11/11 17:10:58.120730
// -----------------------------------------------------------------------------

use std::collections::HashMap;

use rusqlite::types::Type;
use rusqlite::{Connection, OptionalExtension, Row, params};

use super::compute::{FULL_WEIGHT, parse_number, weight_total};
use super::model::{Column, ColumnKind, ColumnTitles, ColumnType, MIN_COLUMN_WIDTH, clamp_width, clean_name};
use super::update_one;
use crate::error::{AppError, NumberField, Result};

pub const DEFAULT_MAX_SCORE: f64 = 10.0;
pub const MAX_DECIMALS: u8 = 3;
/// The largest weight a test takes, in %: ten times the whole grade is already far past any real
/// use, and a bound keeps the sums finite.
pub const MAX_WEIGHT: f64 = 1000.0;
/// The largest maximum score a test, final grade or average takes.
pub const MAX_SCORE_LIMIT: f64 = 10_000.0;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    Left,
    Right,
}

pub fn list_columns(connection: &Connection, class_id: i64) -> Result<Vec<Column>> {
    let mut statement = connection.prepare(
        "SELECT id, title, kind, weight, max_score, source_column_id, decimals, bonus, width
         FROM sheet_columns WHERE class_id = ?1 ORDER BY position",
    )?;
    let columns = statement.query_map([class_id], column_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(columns)
}

/// Adds a column at the end with defaults: a test takes the weight still missing to reach 100 %,
/// a rounded column rounds the last numeric column. `titles` words the title, which can be renamed.
/// Name columns can't be added (`NotEditable`): every class has its two from the start.
pub fn add_column(
    connection: &Connection,
    class_id: i64,
    column_type: ColumnType,
    titles: impl ColumnTitles,
) -> Result<i64> {
    if matches!(column_type, ColumnType::FirstName | ColumnType::Surname) {
        return Err(AppError::NotEditable);
    }
    let existing = list_columns(connection, class_id)?;
    insert_column(
        connection,
        class_id,
        &default_title(column_type, &existing, &titles),
        default_kind(column_type, &existing),
    )
}

/// Adds a column at the end with exactly these title and settings. Not a name column: those come
/// with the class (`NotEditable`).
pub fn insert_column(connection: &Connection, class_id: i64, title: &str, kind: ColumnKind) -> Result<i64> {
    if kind.is_name() {
        return Err(AppError::NotEditable);
    }
    let title = clean_name(title)?;
    let (weight, max_score, source, decimals) = kind_fields(kind);
    let bonus = matches!(kind, ColumnKind::Test { bonus: true, .. });
    let code = kind.column_type().code();
    connection.execute(
        "INSERT INTO sheet_columns
             (class_id, position, title, kind, weight, max_score, source_column_id, decimals, bonus)
         VALUES (?1, (SELECT COALESCE(MAX(position), 0) + 1 FROM sheet_columns WHERE class_id = ?1),
                 ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![class_id, title, code, weight, max_score, source, decimals, bonus],
    )?;
    let id = connection.last_insert_rowid();
    log::info!("added {code} column {id} to class {class_id}");
    Ok(id)
}

/// Renames a column; a name column's title follows the interface language, so it is refused
/// (`NotEditable`).
pub fn rename_column(connection: &Connection, column_id: i64, title: &str) -> Result<()> {
    let title = clean_name(title)?;
    update_one(
        connection,
        "UPDATE sheet_columns SET title = ?2 WHERE id = ?1 AND kind NOT IN ('first_name', 'surname')",
        params![column_id, title],
    )
}

/// Gives class `class_id`, newly made (so it has only its two name columns), copies of `columns`
/// (from another class or another database) in their order, with every setting and width: its own
/// name columns take the place and width of the originals', the rest are added. A rounded column
/// may round a column to its right, so sources are set once every copy exists; a source that isn't
/// among `columns` is left unset. Callers hold a transaction. Returns original column id -> copy id.
pub fn copy_columns(connection: &Connection, class_id: i64, columns: &[Column]) -> Result<HashMap<i64, i64>> {
    let own_names = list_columns(connection, class_id)?;
    let own_name = |kind: ColumnKind| {
        own_names.iter().find(|column| column.kind == kind).map(|column| column.id).ok_or(AppError::NotEditable)
    };
    let mut ids = HashMap::new();
    for column in columns {
        let copy = match column.kind {
            ColumnKind::FirstName | ColumnKind::Surname => own_name(column.kind)?,
            ColumnKind::Round { decimals, .. } => {
                insert_column(connection, class_id, &column.title, ColumnKind::Round { source: None, decimals })?
            }
            kind => insert_column(connection, class_id, &column.title, kind)?,
        };
        if column.width.is_some() {
            set_column_width(connection, copy, column.width)?;
        }
        ids.insert(column.id, copy);
    }
    let order: Vec<i64> = columns.iter().map(|column| ids[&column.id]).collect();
    // a source missing its name columns (none should be) keeps the class's own at the end
    let rest = own_names.iter().map(|column| column.id).filter(|id| !order.contains(id));
    renumber(connection, &order.iter().copied().chain(rest).collect::<Vec<_>>())?;
    for column in columns {
        if let ColumnKind::Round { source: Some(source), .. } = column.kind {
            set_round_source(connection, ids[&column.id], ids.get(&source).copied())?;
        }
    }
    Ok(ids)
}

/// Gives the columns positions 1..n in the order of `ids`.
fn renumber(connection: &Connection, ids: &[i64]) -> Result<()> {
    for (position, id) in (1_i64..).zip(ids) {
        connection.execute("UPDATE sheet_columns SET position = ?2 WHERE id = ?1", params![id, position])?;
    }
    Ok(())
}

/// Sets the width the column is shown at, clamped to the limits; `None` puts it back to the
/// default for its kind.
pub fn set_column_width(connection: &Connection, column_id: i64, width: Option<u16>) -> Result<()> {
    let width = clamp_width(width, MIN_COLUMN_WIDTH);
    update_one(connection, "UPDATE sheet_columns SET width = ?2 WHERE id = ?1", params![column_id, width])?;
    log::info!("set the width of column {column_id} to {width:?}");
    Ok(())
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
        .filter(|weight| (0.0..=MAX_WEIGHT).contains(weight))
        .ok_or_else(|| AppError::InvalidNumber { field: NumberField::Weight, value: text.to_owned() })?;
    update_one(
        connection,
        "UPDATE sheet_columns SET weight = ?2 WHERE id = ?1 AND kind = 'test'",
        params![column_id, weight],
    )
}

pub fn set_max_score(connection: &Connection, column_id: i64, text: &str) -> Result<()> {
    let max_score = parse_number(text)
        .filter(|max| *max > 0.0 && *max <= MAX_SCORE_LIMIT)
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

/// Deletes a column and its cells. Name columns can't be deleted (`NotEditable`).
pub fn delete_column(connection: &Connection, column_id: i64) -> Result<()> {
    update_one(
        connection,
        "DELETE FROM sheet_columns WHERE id = ?1 AND kind NOT IN ('first_name', 'surname')",
        [column_id],
    )?;
    log::info!("deleted column {column_id}");
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

fn default_kind(column_type: ColumnType, existing: &[Column]) -> ColumnKind {
    match column_type {
        ColumnType::FirstName => ColumnKind::FirstName,
        ColumnType::Surname => ColumnKind::Surname,
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

/// Tests are numbered on from how many there are, skipping a title already in use ("Test 2"
/// after "Test 1" was deleted).
fn default_title(column_type: ColumnType, existing: &[Column], titles: &impl ColumnTitles) -> String {
    let same_type = existing.iter().filter(|column| column.kind.column_type() == column_type).count();
    let taken = |title: &str| existing.iter().any(|column| column.title.to_lowercase() == title.to_lowercase());
    (same_type + 1..)
        .map(|number| titles.default_title(column_type, number))
        .find(|title| column_type != ColumnType::Test || !taken(title))
        .unwrap_or_default()
}

fn kind_fields(kind: ColumnKind) -> (Option<f64>, Option<f64>, Option<i64>, Option<u8>) {
    match kind {
        ColumnKind::FirstName | ColumnKind::Surname | ColumnKind::Info => (None, None, None, None),
        ColumnKind::Test { weight, max_score, .. } => (Some(weight), Some(max_score), None, None),
        ColumnKind::Weighted { max_score } | ColumnKind::Average { max_score } => (None, Some(max_score), None, None),
        ColumnKind::Round { source, decimals } => (None, None, source, Some(decimals)),
    }
}

fn column_from_row(row: &Row) -> rusqlite::Result<Column> {
    let code: String = row.get(2)?;
    let kind = match code.as_str() {
        "first_name" => ColumnKind::FirstName,
        "surname" => ColumnKind::Surname,
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
    Ok(Column { id: row.get(0)?, title: row.get(1)?, kind, width: row.get(8)? })
}
