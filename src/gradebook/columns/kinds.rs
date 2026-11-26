// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.449464
//+ Revised: 	2026/11/23 17:29:49.130915
// -----------------------------------------------------------------------------

//! Each column kind's storage and defaults: a column's fields in the database and back, and the
//! settings and title a new column of a type starts with.

use rusqlite::Row;
use rusqlite::types::Type;

use super::DEFAULT_MAX_SCORE;
use crate::gradebook::compute::{FULL_WEIGHT, weight_total};
use crate::gradebook::model::{Column, ColumnKind, ColumnTitles, ColumnType, name_match_key};

pub(super) fn default_kind(column_type: ColumnType, existing: &[Column]) -> ColumnKind {
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

/// A title no other column of the class has, so the rounded column's source list, the import and
/// the export's settings tab can tell columns apart: tests are numbered on from how many there are,
/// skipping a title in use ("Test 2" after "Test 1" was deleted); another kind's title gets a
/// number only once its plain one is taken ("Final grade", then "Final grade 2").
pub(super) fn default_title(column_type: ColumnType, existing: &[Column], titles: &impl ColumnTitles) -> String {
    let taken = |title: &str| existing.iter().any(|column| name_match_key(&column.title) == name_match_key(title));
    if column_type == ColumnType::Test {
        let tests = existing.iter().filter(|column| column.kind.column_type() == column_type).count();
        return (tests + 1..)
            .map(|number| titles.default_title(column_type, number))
            .find(|title| !taken(title))
            .unwrap_or_default();
    }
    let plain = titles.default_title(column_type, 1);
    (1..)
        .map(|number| if number == 1 { plain.clone() } else { format!("{plain} {number}") })
        .find(|title| !taken(title))
        .unwrap_or_default()
}

/// The fields `sheet_columns` stores for a kind: weight, maximum score and decimals. A rounded
/// column's source is not among them: it is stored in `round_sources` (`round_source`).
pub(super) fn kind_fields(kind: ColumnKind) -> (Option<f64>, Option<f64>, Option<u8>) {
    match kind {
        ColumnKind::FirstName | ColumnKind::Surname | ColumnKind::Info => (None, None, None),
        ColumnKind::Test { weight, max_score, .. } => (Some(weight), Some(max_score), None),
        ColumnKind::Weighted { max_score } | ColumnKind::Average { max_score } => (None, Some(max_score), None),
        ColumnKind::Round { decimals, .. } => (None, None, Some(decimals)),
    }
}

/// The source a kind names for a rounded column, which is linked in `round_sources`.
pub(super) fn round_source(kind: ColumnKind) -> Option<i64> {
    match kind {
        ColumnKind::Round { source, .. } => source,
        _ => None,
    }
}

pub(super) fn column_from_row(row: &Row) -> rusqlite::Result<Column> {
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
