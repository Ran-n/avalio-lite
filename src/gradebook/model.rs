// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.337118
//+ Revised: 	2026/11/02 14:00:09.456203
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

/// Names (and titles) are the same name ignoring case and spaces at either end, in any
/// alphabet: "ÁLVARO " is "álvaro". Accents still count: "Alvaro" is someone else.
pub fn name_match_key(name: &str) -> String {
    let mut key = String::with_capacity(name.len());
    for character in name.trim().chars().flat_map(char::to_lowercase) {
        // an accent typed as a mark of its own (a + U+0301, as some spreadsheets store it) is
        // joined to its letter, so it matches the same name typed as "á"
        match key.pop() {
            Some(base) => match composed(base, character) {
                Some(joined) => key.push(joined),
                None => key.extend([base, character]),
            },
            None => key.push(character),
        }
    }
    key
}

/// The one letter a lower-case Latin letter and a combining accent make, for the accents
/// Galician, Spanish, Portuguese and French names use.
fn composed(base: char, mark: char) -> Option<char> {
    const BASES: &str = "aeiouy";
    let row = match mark {
        '\u{301}' => "áéíóúý",
        '\u{300}' => "àèìòù ",
        '\u{302}' => "âêîôû ",
        '\u{308}' => "äëïöüÿ",
        '\u{303}' => {
            return match base {
                'a' => Some('ã'),
                'o' => Some('õ'),
                'n' => Some('ñ'),
                _ => None,
            };
        }
        '\u{327}' => return (base == 'c').then_some('ç'),
        _ => return None,
    };
    let index = BASES.chars().position(|candidate| candidate == base)?;
    row.chars().nth(index).filter(|joined| *joined != ' ')
}

/// The order names are listed in: ignoring case and accents, so "Álvaro" sorts with the A's
/// rather than after Z, with ñ as its own letter after n, as Galician and Spanish sort it.
pub fn name_sort_key(name: &str) -> String {
    const COMBINING_MARKS: std::ops::RangeInclusive<char> = '\u{300}'..='\u{36F}';
    const COMBINING_TILDE: char = '\u{303}';
    const AFTER_N: char = '\u{7F}';
    let mut key = String::with_capacity(name.len());
    for character in name.trim().chars().flat_map(char::to_lowercase) {
        let base = match character {
            'á' | 'à' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'é' | 'è' | 'ê' | 'ë' => 'e',
            'í' | 'ì' | 'î' | 'ï' => 'i',
            'ó' | 'ò' | 'ô' | 'ö' | 'õ' => 'o',
            'ú' | 'ù' | 'û' | 'ü' => 'u',
            'ý' | 'ÿ' => 'y',
            'ç' => 'c',
            'ñ' => {
                key.extend(['n', AFTER_N]);
                continue;
            }
            // an accent typed as a mark of its own (a + U+0301) is dropped, but n + tilde is ñ
            mark if COMBINING_MARKS.contains(&mark) => {
                if mark == COMBINING_TILDE && key.ends_with('n') {
                    key.push(AFTER_N);
                }
                continue;
            }
            other => other,
        };
        key.push(base);
    }
    key
}
