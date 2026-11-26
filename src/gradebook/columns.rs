// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.917995
//+ Revised: 	2026/11/23 17:29:48.921405
// -----------------------------------------------------------------------------

//! A class's columns: listing, adding, copying, renaming, width and deleting them. The settings each
//! kind takes are in `settings`; the kinds' defaults and their storage are in `kinds`; their order in
//! `order`.

mod kinds;
mod order;
mod settings;

pub use order::{Direction, move_column, move_column_to};
pub use settings::{set_bonus, set_decimals, set_max_score, set_round_source, set_weight};

use std::collections::HashMap;

use rusqlite::{Connection, params};

use super::model::{Column, ColumnKind, ColumnTitles, ColumnType, MIN_COLUMN_WIDTH, clamp_width, clean_name};
use super::update_one;
use crate::error::{AppError, Result};
use kinds::{column_from_row, default_kind, default_title, kind_fields, round_source};
use order::renumber;
use settings::require_round_source;

pub const DEFAULT_MAX_SCORE: f64 = 10.0;
pub const MAX_DECIMALS: u8 = 3;
/// The largest weight a test takes, in %: ten times the whole grade is already far past any real
/// use, and a bound keeps the sums finite.
pub const MAX_WEIGHT: f64 = 1000.0;
/// The largest maximum score a test, final grade or average takes.
pub const MAX_SCORE_LIMIT: f64 = 10_000.0;

pub fn list_columns(connection: &Connection, class_id: i64) -> Result<Vec<Column>> {
    let mut statement = connection.prepare(
        "SELECT sheet_columns.id, title, kind, weight, max_score, round_sources.source_column_id,
                decimals, bonus, width
         FROM sheet_columns
         LEFT JOIN round_sources ON round_sources.column_id = sheet_columns.id
         WHERE sheet_columns.class_id = ?1 ORDER BY position",
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
    // a name column is refused by `insert_column`: the class came with its two
    let existing = list_columns(connection, class_id)?;
    insert_column(
        connection,
        class_id,
        &default_title(column_type, &existing, &titles),
        default_kind(column_type, &existing),
    )
}

/// Adds a column at the end with exactly these title and settings. Not a name column: those come
/// with the class (`NotEditable`). A rounded column's source is checked before anything is written,
/// so a source that doesn't fit adds nothing (`InvalidRoundSource`).
pub fn insert_column(connection: &Connection, class_id: i64, title: &str, kind: ColumnKind) -> Result<i64> {
    if kind.is_name() {
        return Err(AppError::NotEditable);
    }
    let title = clean_name(title)?;
    let source = round_source(kind);
    if let Some(source) = source {
        require_round_source(connection, class_id, source)?;
    }
    let (weight, max_score, decimals) = kind_fields(kind);
    let bonus = matches!(kind, ColumnKind::Test { bonus: true, .. });
    let code = kind.column_type().code();
    connection.execute(
        "INSERT INTO sheet_columns (class_id, position, title, kind, weight, max_score, decimals, bonus)
         VALUES (?1, (SELECT COALESCE(MAX(position), 0) + 1 FROM sheet_columns WHERE class_id = ?1),
                 ?2, ?3, ?4, ?5, ?6, ?7)",
        params![class_id, title, code, weight, max_score, decimals, bonus],
    )?;
    let id = connection.last_insert_rowid();
    if let Some(source) = source {
        set_round_source(connection, id, Some(source))?;
    }
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
    )?;
    log::info!("renamed column {column_id}");
    Ok(())
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

/// Sets the width the column is shown at, clamped to the limits; `None` puts it back to the
/// default for its kind.
pub fn set_column_width(connection: &Connection, column_id: i64, width: Option<u16>) -> Result<()> {
    let width = clamp_width(width, MIN_COLUMN_WIDTH);
    update_one(connection, "UPDATE sheet_columns SET width = ?2 WHERE id = ?1", params![column_id, width])?;
    log::info!("set the width of column {column_id} to {width:?}");
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
