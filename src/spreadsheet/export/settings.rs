// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:15.861001
//+ Revised: 	2026/11/23 11:49:15.861001
// -----------------------------------------------------------------------------

//! The export's second tab: one row per column with its title, kind and the settings that kind has
//! (weight, maximum, bonus, rounding source and decimals), so the workbook says how the marks were
//! calculated.

use rust_xlsxwriter::{ColNum, Worksheet, XlsxError};

use super::{Value, export_title, row_number, write_header, write_value};
use crate::gradebook::model::{Column, ColumnKind};
use crate::i18n::{self, ExportLabels, Language};

/// The title column, the kind's name, then the settings columns of the tab.
const KIND: ColNum = 1;
const WEIGHT: ColNum = 2;
const MAXIMUM: ColNum = 3;
const BONUS: ColNum = 4;
const ROUNDS: ColNum = 5;
const DECIMALS: ColNum = 6;

/// One row per column: its kind, and the settings that kind has.
pub(super) fn write_settings(
    tab: &mut Worksheet,
    columns: &[Column],
    labels: &ExportLabels,
    language: Language,
) -> std::result::Result<(), XlsxError> {
    write_header(tab, labels.column_headers.into_iter())?;
    for (index, column) in columns.iter().enumerate() {
        let excel_row = row_number(index + 1)?;
        tab.write_string(excel_row, 0, export_title(column, language))?;
        tab.write_string(excel_row, KIND, i18n::column_type_name(language, column.kind.column_type()))?;
        for (settings_column, value) in settings_values(column, columns, labels.yes) {
            write_value(tab, excel_row, settings_column, value)?;
        }
    }
    tab.set_freeze_panes(1, 0)?;
    tab.autofit();
    Ok(())
}

/// The settings `column`'s kind has, each with its settings column, as the values to write. A
/// rounded column names its source by title, so a source that is gone is left blank.
fn settings_values<'a>(column: &'a Column, columns: &'a [Column], yes: &'a str) -> Vec<(ColNum, Value<'a>)> {
    match column.kind {
        ColumnKind::FirstName | ColumnKind::Surname | ColumnKind::Info => Vec::new(),
        ColumnKind::Test { weight, max_score, bonus } => {
            let mut values = vec![(WEIGHT, Value::Number(weight)), (MAXIMUM, Value::Number(max_score))];
            if bonus {
                values.push((BONUS, Value::Text(yes)));
            }
            values
        }
        ColumnKind::Weighted { max_score } | ColumnKind::Average { max_score } => {
            vec![(MAXIMUM, Value::Number(max_score))]
        }
        ColumnKind::Round { source, decimals } => {
            let source = columns.iter().find(|candidate| Some(candidate.id) == source);
            let mut values: Vec<_> = source.map(|source| (ROUNDS, Value::Text(&source.title))).into_iter().collect();
            values.push((DECIMALS, Value::Number(f64::from(decimals))));
            values
        }
    }
}
