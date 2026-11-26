// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.255058
//+ Revised: 	2026/11/23 00:26:08.255058
// -----------------------------------------------------------------------------

//! The texts about Excel exports and imports, in English.

use crate::i18n::messages::{SpreadsheetMessage, plural};

/// A spreadsheet text in English.
pub(super) fn message(message: &SpreadsheetMessage) -> String {
    match message {
        SpreadsheetMessage::SheetExported { class, path } => format!("Saved {class} to {path}"),
        SpreadsheetMessage::SpreadsheetFileType => "Excel workbook".to_owned(),
        SpreadsheetMessage::ExportWeightsWarning { total } => {
            format!("Test weights add up to {total}%, not 100%: final grades are provisional.")
        }
        SpreadsheetMessage::ConfirmImportOverwrite { count, class } => {
            let what = plural(*count, "mark or note", "marks or notes");
            format!("Importing replaces {count} {what} already in {class}. Continue?")
        }
        SpreadsheetMessage::ImportDone { pupils, new_pupils, marks, class } => {
            import_done(*pupils, *new_pupils, *marks, class)
        }
        SpreadsheetMessage::ImportLeftOut { headers, .. } => format!("Left out: {headers}."),
        SpreadsheetMessage::ImportErrorCells { count } => error_cells(*count),
        SpreadsheetMessage::ImportUnnamedRows { count } => unnamed_rows(*count),
    }
}

/// The summary after an import: the pupils and marks it brought in.
fn import_done(pupils: usize, new_pupils: usize, marks: usize, class: &str) -> String {
    let added = if new_pupils > 0 { format!(" ({new_pupils} new)") } else { String::new() };
    format!(
        "Imported {pupils} {}{added} and {marks} {} into {class}.",
        plural(pupils, "pupil", "pupils"),
        plural(marks, "mark or note", "marks or notes")
    )
}

/// The cells an import skipped because Excel showed an error in them.
fn error_cells(count: usize) -> String {
    if count == 1 {
        "1 cell with an Excel error was skipped.".to_owned()
    } else {
        format!("{count} cells with Excel errors were skipped.")
    }
}

/// The rows an import skipped because they have no first name.
fn unnamed_rows(count: usize) -> String {
    if count == 1 {
        "1 row with no first name was skipped.".to_owned()
    } else {
        format!("{count} rows with no first name were skipped.")
    }
}
