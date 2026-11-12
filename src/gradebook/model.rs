// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.337118
//+ Revised: 	2026/11/11 17:10:58.416291
// -----------------------------------------------------------------------------

use crate::error::{AppError, Result};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ClassSummary {
    pub id: i64,
    pub name: String,
    pub pupil_count: i64,
}

/// A pupil, shared by every class they are in. The first name is never empty; the surname may be
/// (some pupils have one name, and pupils from before surnames existed have none).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pupil {
    pub id: i64,
    pub first_name: String,
    pub surname: String,
}

impl Pupil {
    /// The name as one string, for confirmations and screen readers: "Ana Pérez López", or just
    /// "Ana" without a surname.
    pub fn full_name(&self) -> String {
        if self.surname.is_empty() { self.first_name.clone() } else { format!("{} {}", self.first_name, self.surname) }
    }

    /// The name as lists show and sort it, surname first: "Pérez López, Ana", or just "Ana"
    /// without a surname, which then sorts by the first name among the surnames.
    pub fn listed_name(&self) -> String {
        if self.surname.is_empty() { self.first_name.clone() } else { format!("{}, {}", self.surname, self.first_name) }
    }
}

/// A column's kind without its settings. Every kind but the name columns can be added (see
/// `columns::add_column`, which gives the settings sensible defaults); a class gets its one first-name
/// and one surname column when it is created.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    FirstName,
    Surname,
    Info,
    Test,
    Weighted,
    Average,
    Round,
}

/// Names new columns: the gradebook numbers them, the caller says how a title reads.
pub trait ColumnTitles {
    fn default_title(&self, column_type: ColumnType, number: usize) -> String;
}

/// A column's kind together with the settings that kind uses.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ColumnKind {
    /// The pupil's first name or surname: stored with the pupil (shared by their classes), so the
    /// column holds no cells and no settings, only its place and width in this class.
    FirstName,
    Surname,
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
            Self::FirstName => ColumnType::FirstName,
            Self::Surname => ColumnType::Surname,
            Self::Info => ColumnType::Info,
            Self::Test { .. } => ColumnType::Test,
            Self::Weighted { .. } => ColumnType::Weighted,
            Self::Average { .. } => ColumnType::Average,
            Self::Round { .. } => ColumnType::Round,
        }
    }

    /// Final grades, averages and roundings are calculated; names and typed cells are not.
    pub fn is_computed(&self) -> bool {
        matches!(self, Self::Weighted { .. } | Self::Average { .. } | Self::Round { .. })
    }

    /// Info and test columns hold the cells typed into them; names live with the pupil, and the
    /// rest are calculated.
    pub fn holds_cells(&self) -> bool {
        matches!(self, Self::Info | Self::Test { .. })
    }

    /// The first-name and surname columns.
    pub fn is_name(&self) -> bool {
        matches!(self, Self::FirstName | Self::Surname)
    }

    /// A rounded column may round any numeric column except another rounded one, which rules out cycles.
    pub fn can_be_rounded(&self) -> bool {
        matches!(self, Self::Test { .. } | Self::Weighted { .. } | Self::Average { .. })
    }
}

impl ColumnType {
    pub fn code(self) -> &'static str {
        match self {
            Self::FirstName => "first_name",
            Self::Surname => "surname",
            Self::Info => "info",
            Self::Test => "test",
            Self::Weighted => "weighted",
            Self::Average => "average",
            Self::Round => "round",
        }
    }
}

/// What a duplicated class brings along (see `classes::duplicate_class`). An enum rather than two
/// flags, so marks without pupils can't be asked for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DuplicateContents {
    /// The columns and their settings, no pupils.
    Columns,
    /// The columns and the same pupils, with empty marks.
    ColumnsAndPupils,
    /// An exact copy: columns, pupils and every typed mark.
    Everything,
}

/// The narrowest any column can be set to, in logical pixels: it still fits a "10.00" pill.
pub const MIN_COLUMN_WIDTH: u16 = 72;
/// The widest any column can be set to.
pub const MAX_COLUMN_WIDTH: u16 = 640;

#[derive(Debug, Clone, PartialEq)]
pub struct Column {
    pub id: i64,
    pub title: String,
    pub kind: ColumnKind,
    /// The width the user dragged it to, in logical pixels; `None` follows the default for its kind.
    pub width: Option<u16>,
}

/// `width` within `min..=MAX_COLUMN_WIDTH`; `None` (the default width) stays `None`.
pub fn clamp_width(width: Option<u16>, min: u16) -> Option<u16> {
    width.map(|width| width.clamp(min, MAX_COLUMN_WIDTH))
}

/// One pupil's row: the value per column, in column order: the pupil's first name or surname in the
/// name columns, the typed text in the others ("" when empty or computed).
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

/// A surname as stored: trimmed, and unlike a first name it may be empty.
pub fn clean_surname(text: &str) -> String {
    text.trim().to_owned()
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

#[cfg(test)]
mod tests {
    use super::{Pupil, clean_surname};

    fn pupil(first_name: &str, surname: &str) -> Pupil {
        Pupil { id: 1, first_name: first_name.to_owned(), surname: surname.to_owned() }
    }

    #[test]
    fn a_full_name_joins_with_one_space_and_drops_an_empty_surname() {
        assert_eq!(pupil("Ana", "Pérez López").full_name(), "Ana Pérez López");
        assert_eq!(pupil("Ana", "").full_name(), "Ana");
    }

    #[test]
    fn a_listed_name_puts_the_surname_first() {
        assert_eq!(pupil("Ana", "Pérez López").listed_name(), "Pérez López, Ana");
        assert_eq!(pupil("Ana", "").listed_name(), "Ana");
    }

    #[test]
    fn a_surname_is_trimmed_and_may_be_empty() {
        assert_eq!(clean_surname("  Pérez "), "Pérez");
        assert_eq!(clean_surname("   "), "");
    }
}
