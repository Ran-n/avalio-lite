// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:31.077103
//+ Revised: 	2026/11/11 17:10:59.726530
// -----------------------------------------------------------------------------

//! Fixed labels: the words of an exported workbook, column type names and new columns' titles.

use super::{LANGUAGES, Language};
use crate::gradebook::model::{ColumnTitles, ColumnType};

/// The fixed words of an exported workbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportLabels {
    /// the headers over the pupils' first names and surnames
    pub first_name: &'static str,
    pub surname: &'static str,
    /// the sheet tab's name when the class name leaves nothing usable
    pub sheet: &'static str,
    pub columns_tab: &'static str,
    /// the columns tab's header: column, kind, weight %, maximum, bonus, rounds, decimals
    pub column_headers: [&'static str; 7],
    /// what the bonus column says for a bonus test
    pub yes: &'static str,
}

pub fn export_labels(language: Language) -> ExportLabels {
    match language {
        Language::English => ExportLabels {
            first_name: "First name",
            surname: "Surname",
            sheet: "Sheet",
            columns_tab: "Columns",
            column_headers: ["Column", "Kind", "Weight %", "Maximum", "Bonus", "Rounds", "Decimals"],
            yes: "Yes",
        },
        Language::Galician => ExportLabels {
            first_name: "Nome",
            surname: "Apelidos",
            sheet: "Folla",
            columns_tab: "Columnas",
            column_headers: ["Columna", "Tipo", "Peso %", "Máximo", "Extra", "Redondea", "Decimais"],
            yes: "Si",
        },
    }
}

/// The surname header of every language, which an import recognises in column B whatever the
/// interface language is.
pub fn surname_headers() -> impl Iterator<Item = &'static str> {
    LANGUAGES.iter().map(|language| export_labels(*language).surname)
}

/// The first-name and surname headers of every language, which an import looks for in the first
/// row whatever the interface language is.
pub fn first_name_headers() -> impl Iterator<Item = &'static str> {
    LANGUAGES.iter().map(|language| export_labels(*language).first_name)
}

/// A name column's title: the same words the export writes over it. `None` for the other kinds,
/// whose titles are their own.
pub fn name_column_title(language: Language, column_type: ColumnType) -> Option<&'static str> {
    let labels = export_labels(language);
    match column_type {
        ColumnType::FirstName => Some(labels.first_name),
        ColumnType::Surname => Some(labels.surname),
        _ => None,
    }
}

/// A column kind's name, as the columns tab of an export lists it.
pub fn column_type_name(language: Language, column_type: ColumnType) -> &'static str {
    match (language, column_type) {
        (_, ColumnType::FirstName) => export_labels(language).first_name,
        (_, ColumnType::Surname) => export_labels(language).surname,
        (Language::English, ColumnType::Info) => "Info",
        (Language::English, ColumnType::Test) => "Test",
        (Language::English, ColumnType::Weighted) => "Final grade",
        (Language::English, ColumnType::Average) => "Average",
        (Language::English, ColumnType::Round) => "Rounded",
        (Language::Galician, ColumnType::Info) => "Información",
        (Language::Galician, ColumnType::Test) => "Proba",
        (Language::Galician, ColumnType::Weighted) => "Nota final",
        (Language::Galician, ColumnType::Average) => "Media",
        (Language::Galician, ColumnType::Round) => "Redondeo",
    }
}

/// The interface language words the titles the gradebook gives new columns.
impl ColumnTitles for Language {
    fn default_title(&self, column_type: ColumnType, number: usize) -> String {
        default_column_title(*self, column_type, number)
    }
}

/// Title given to a new column; `number` counts tests ("Test 3"). A name column's is its header.
pub fn default_column_title(language: Language, column_type: ColumnType, number: usize) -> String {
    match (language, column_type) {
        (_, ColumnType::FirstName | ColumnType::Surname) => column_type_name(language, column_type).to_owned(),
        (Language::English, ColumnType::Info) => "Notes".to_owned(),
        (Language::English, ColumnType::Test) => format!("Test {number}"),
        (Language::English, ColumnType::Weighted) => "Final grade".to_owned(),
        (Language::English, ColumnType::Average) => "Average".to_owned(),
        (Language::English, ColumnType::Round) => "Rounded".to_owned(),
        (Language::Galician, ColumnType::Info) => "Observacións".to_owned(),
        (Language::Galician, ColumnType::Test) => format!("Proba {number}"),
        (Language::Galician, ColumnType::Weighted) => "Nota final".to_owned(),
        (Language::Galician, ColumnType::Average) => "Media".to_owned(),
        (Language::Galician, ColumnType::Round) => "Redondeo".to_owned(),
    }
}
