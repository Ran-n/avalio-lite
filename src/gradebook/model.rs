// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.337118
//+ Revised: 	2026/10/28 10:52:45.088113
// -----------------------------------------------------------------------------

use crate::error::{AppError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassSummary {
    pub id: i64,
    pub name: String,
    pub pupil_count: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pupil {
    pub id: i64,
    pub name: String,
}

/// The kind of column to add; settings get sensible defaults (see `columns::add_column`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Info,
    Test,
    Weighted,
    Average,
    Round,
}

/// A column's kind together with the settings that kind uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnKind {
    Info,
    /// `bonus` tests are extra credit: their weight counts on top of the normal tests' 100 %.
    Test {
        weight: f64,
        max_score: f64,
        bonus: bool,
    },
    Weighted {
        max_score: f64,
    },
    Average {
        max_score: f64,
    },
    Round {
        source: Option<i64>,
        decimals: u8,
    },
}

impl ColumnKind {
    pub fn column_type(&self) -> ColumnType {
        match self {
            Self::Info => ColumnType::Info,
            Self::Test { .. } => ColumnType::Test,
            Self::Weighted { .. } => ColumnType::Weighted,
            Self::Average { .. } => ColumnType::Average,
            Self::Round { .. } => ColumnType::Round,
        }
    }

    /// Info and test cells are typed in; the others are calculated.
    pub fn is_computed(&self) -> bool {
        matches!(self, Self::Weighted { .. } | Self::Average { .. } | Self::Round { .. })
    }

    /// A rounded column may round any numeric column except another rounded one, which rules out cycles.
    pub fn can_be_rounded(&self) -> bool {
        matches!(self, Self::Test { .. } | Self::Weighted { .. } | Self::Average { .. })
    }
}

impl ColumnType {
    pub fn code(self) -> &'static str {
        match self {
            Self::Info => "info",
            Self::Test => "test",
            Self::Weighted => "weighted",
            Self::Average => "average",
            Self::Round => "round",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub id: i64,
    pub title: String,
    pub kind: ColumnKind,
}

/// One pupil's row: the stored value per column, in column order ("" when empty or computed).
#[derive(Debug, Clone, PartialEq)]
pub struct SheetRow {
    pub pupil: Pupil,
    pub values: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Sheet {
    pub columns: Vec<Column>,
    pub rows: Vec<SheetRow>,
}

pub fn clean_name(text: &str) -> Result<String> {
    let name = text.trim();
    if name.is_empty() {
        return Err(AppError::EmptyName);
    }
    Ok(name.to_owned())
}
