// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:31.077103
//+ Revised: 	2026/11/03 11:13:31.077103
// -----------------------------------------------------------------------------

//! Fixed labels: the words of an exported workbook, column type names and new columns' titles.

use super::Language;
use crate::gradebook::model::{ColumnTitles, ColumnType};

/// The fixed words of an exported workbook.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExportLabels {
    /// the header over the pupils' names
    pub pupil: &'static str,
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
            pupil: "Pupil",
            sheet: "Sheet",
            columns_tab: "Columns",
            column_headers: ["Column", "Kind", "Weight %", "Maximum", "Bonus", "Rounds", "Decimals"],
            yes: "Yes",
        },
        Language::Galician => ExportLabels {
            pupil: "Alumno",
            sheet: "Folla",
            columns_tab: "Columnas",
            column_headers: ["Columna", "Tipo", "Peso %", "Máximo", "Extra", "Redondea", "Decimais"],
            yes: "Si",
        },
    }
}

/// A column kind's name, as the columns tab of an export lists it.
pub fn column_type_name(language: Language, column_type: ColumnType) -> &'static str {
    match (language, column_type) {
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

/// Title given to a new column; `number` counts tests ("Test 3").
pub fn default_column_title(language: Language, column_type: ColumnType, number: usize) -> String {
    match (language, column_type) {
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
