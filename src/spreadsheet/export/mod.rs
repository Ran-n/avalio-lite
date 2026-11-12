// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 10:48:30.700090
//+ Revised: 	2026/11/11 17:11:00.036326
// -----------------------------------------------------------------------------

//! Saves a class sheet as an .xlsx workbook: the sheet as it shows on screen, then a tab listing
//! each column's settings. The numbers come from `compute`, the same code the screen uses.

mod names;

use std::fmt::Display;
use std::fs;
use std::path::Path;

use rusqlite::Connection;
use rust_xlsxwriter::{ColNum, Format, FormatBorder, RowNum, Workbook, Worksheet, XlsxError};

use crate::error::{AppError, Result};
use crate::gradebook::compute::{
    Evaluated, evaluate_row, format_number, grades_are_provisional, parse_number, weight_total,
};
use crate::gradebook::model::{Column, ColumnKind, Sheet};
use crate::gradebook::{classes, sheet};
use crate::i18n::{self, ExportLabels, Language, Message};
use crate::platform::storage::partial_path;

use names::TAKEN_SUFFIX;
pub use names::{file_name, tab_name};

/// Saves class `class_id` to `path`, replacing the file there.
pub fn export_sheet(connection: &Connection, class_id: i64, language: Language, path: &Path) -> Result<()> {
    let class_name = classes::class_name(connection, class_id)?;
    let sheet = sheet::load_sheet(connection, class_id)?;
    let mut workbook = sheet_workbook(&class_name, &sheet, language).map_err(|error| write_error(path, error))?;
    save(&mut workbook, path)?;
    log::info!("exported class {class_id} to {}", path.display());
    Ok(())
}

/// Both tabs of the export, in `language`.
pub fn sheet_workbook(class_name: &str, sheet: &Sheet, language: Language) -> std::result::Result<Workbook, XlsxError> {
    let labels = i18n::export_labels(language);
    let mut marks_tab = tab_name(class_name, labels.sheet);
    if marks_tab.to_lowercase() == labels.columns_tab.to_lowercase() {
        marks_tab.push_str(TAKEN_SUFFIX);
    }
    let mut workbook = Workbook::new();
    write_marks(workbook.add_worksheet().set_name(marks_tab)?, sheet, language)?;
    write_settings(workbook.add_worksheet().set_name(labels.columns_tab)?, &sheet.columns, &labels, language)?;
    Ok(workbook)
}

/// What one cell of the sheet becomes in the workbook.
enum Value<'a> {
    Empty,
    Number(f64),
    Text(&'a str),
}

/// Valid scores and calculated results are numbers, so Excel can work with them and shows them
/// with the computer's decimal separator; a score that isn't valid stays the text that was typed.
fn exported_value<'a>(kind: ColumnKind, raw: &'a str, shown: &Evaluated) -> Value<'a> {
    match kind {
        ColumnKind::FirstName | ColumnKind::Surname | ColumnKind::Info if raw.is_empty() => Value::Empty,
        ColumnKind::FirstName | ColumnKind::Surname | ColumnKind::Info => Value::Text(raw),
        ColumnKind::Test { .. } if shown.is_invalid => Value::Text(raw),
        ColumnKind::Test { .. } => parse_number(raw).map_or(Value::Empty, Value::Number),
        ColumnKind::Weighted { .. } | ColumnKind::Average { .. } | ColumnKind::Round { .. } => {
            parse_number(&shown.text).map_or(Value::Empty, Value::Number)
        }
    }
}

fn write_marks(tab: &mut Worksheet, sheet: &Sheet, language: Language) -> std::result::Result<(), XlsxError> {
    write_header(tab, sheet.columns.iter().map(|column| export_title(column, language)))?;
    for (index, row) in sheet.rows.iter().enumerate() {
        let excel_row = row_number(index + 1)?;
        let shown = evaluate_row(&sheet.columns, &row.values);
        for (index, ((column, raw), shown)) in sheet.columns.iter().zip(&row.values).zip(&shown).enumerate() {
            let excel_column = column_number(index)?;
            match exported_value(column.kind, raw, shown) {
                Value::Empty => {}
                Value::Number(number) => {
                    tab.write_number(excel_row, excel_column, number)?;
                }
                Value::Text(text) => {
                    tab.write_string(excel_row, excel_column, text)?;
                }
            }
        }
    }
    tab.set_freeze_panes(1, frozen_columns(&sheet.columns))?;
    // before the note, which would otherwise widen the names column to its whole length
    tab.autofit();
    if grades_are_provisional(&sheet.columns) {
        let total = i18n::localize_number(language, &format_number(weight_total(&sheet.columns)));
        let note = i18n::text(language, &Message::ExportWeightsWarning { total: &total });
        tab.write_string_with_format(row_number(sheet.rows.len() + 2)?, 0, note, &Format::new().set_italic())?;
    }
    Ok(())
}

/// A column's header in the workbook: a name column's in `language`, the others' their title.
fn export_title(column: &Column, language: Language) -> &str {
    i18n::name_column_title(language, column.kind.column_type()).unwrap_or(&column.title)
}

/// The columns kept in view while scrolling sideways: the leading ones up to the rightmost name
/// column when both names are among the first two, none otherwise, since freezing marks in front
/// of the names would keep the names that scroll away out of view.
fn frozen_columns(columns: &[Column]) -> ColNum {
    let names_lead = columns.len() >= 2 && columns[..2].iter().all(|column| column.kind.is_name());
    if names_lead { 2 } else { 0 }
}

/// One row per column: its kind, and the settings that kind has.
fn write_settings(
    tab: &mut Worksheet,
    columns: &[Column],
    labels: &ExportLabels,
    language: Language,
) -> std::result::Result<(), XlsxError> {
    const KIND: ColNum = 1;
    const WEIGHT: ColNum = 2;
    const MAXIMUM: ColNum = 3;
    const BONUS: ColNum = 4;
    const ROUNDS: ColNum = 5;
    const DECIMALS: ColNum = 6;
    write_header(tab, labels.column_headers.into_iter())?;
    for (index, column) in columns.iter().enumerate() {
        let excel_row = row_number(index + 1)?;
        tab.write_string(excel_row, 0, export_title(column, language))?;
        tab.write_string(excel_row, KIND, i18n::column_type_name(language, column.kind.column_type()))?;
        match column.kind {
            ColumnKind::FirstName | ColumnKind::Surname | ColumnKind::Info => {}
            ColumnKind::Test { weight, max_score, bonus } => {
                tab.write_number(excel_row, WEIGHT, weight)?;
                tab.write_number(excel_row, MAXIMUM, max_score)?;
                if bonus {
                    tab.write_string(excel_row, BONUS, labels.yes)?;
                }
            }
            ColumnKind::Weighted { max_score } | ColumnKind::Average { max_score } => {
                tab.write_number(excel_row, MAXIMUM, max_score)?;
            }
            ColumnKind::Round { source, decimals } => {
                if let Some(source) = columns.iter().find(|candidate| Some(candidate.id) == source) {
                    tab.write_string(excel_row, ROUNDS, &source.title)?;
                }
                tab.write_number(excel_row, DECIMALS, decimals)?;
            }
        }
    }
    tab.set_freeze_panes(1, 0)?;
    tab.autofit();
    Ok(())
}

/// Row 1, in bold over a bottom border.
fn write_header<'a>(tab: &mut Worksheet, titles: impl Iterator<Item = &'a str>) -> std::result::Result<(), XlsxError> {
    let format = Format::new().set_bold().set_border_bottom(FormatBorder::Thin);
    for (index, title) in titles.enumerate() {
        tab.write_string_with_format(0, column_number(index)?, title, &format)?;
    }
    Ok(())
}

fn row_number(index: usize) -> std::result::Result<RowNum, XlsxError> {
    RowNum::try_from(index).map_err(|_| XlsxError::RowColumnLimitError)
}

fn column_number(index: usize) -> std::result::Result<ColNum, XlsxError> {
    ColNum::try_from(index).map_err(|_| XlsxError::RowColumnLimitError)
}

/// The workbook is written next to `path` first and then moved over it, so a failure never
/// leaves half a workbook behind, nor spoils the file being replaced.
fn save(workbook: &mut Workbook, path: &Path) -> Result<()> {
    let bytes = workbook.save_to_buffer().map_err(|error| write_error(path, error))?;
    let partial = partial_path(path);
    if let Err(error) = fs::write(&partial, bytes).and_then(|()| fs::rename(&partial, path)) {
        if partial.exists()
            && let Err(cleanup) = fs::remove_file(&partial)
        {
            log::warn!("could not remove the incomplete export {}: {cleanup}", partial.display());
        }
        return Err(write_error(path, error));
    }
    Ok(())
}

fn write_error(path: &Path, error: impl Display) -> AppError {
    AppError::SpreadsheetWrite { path: path.to_path_buf(), reason: error.to_string() }
}

#[cfg(test)]
mod tests {
    use super::frozen_columns;
    use crate::gradebook::model::{Column, ColumnKind};

    fn columns(kinds: &[ColumnKind]) -> Vec<Column> {
        (1..).zip(kinds).map(|(id, kind)| Column { id, title: String::new(), kind: *kind, width: None }).collect()
    }

    #[test]
    fn the_names_stay_in_view_only_when_they_lead() {
        use ColumnKind::{FirstName, Info, Surname};
        assert_eq!(frozen_columns(&columns(&[FirstName, Surname, Info])), 2);
        assert_eq!(frozen_columns(&columns(&[Surname, FirstName, Info])), 2, "in either order");
        assert_eq!(frozen_columns(&columns(&[FirstName, Info, Surname])), 0, "a mark between them: only the header");
        assert_eq!(frozen_columns(&columns(&[Info, FirstName, Surname])), 0);
    }
}
