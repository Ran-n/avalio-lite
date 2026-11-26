// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.087064
//+ Revised: 	2026/11/23 00:26:09.087064
// -----------------------------------------------------------------------------

//! The texts about Excel exports and imports, in Galician.

use crate::i18n::messages::{SpreadsheetMessage, plural};

/// A spreadsheet text in Galician.
pub(super) fn message(message: &SpreadsheetMessage) -> String {
    match message {
        SpreadsheetMessage::SheetExported { class, path } => format!("Gardouse {class} en {path}"),
        SpreadsheetMessage::SpreadsheetFileType => "Libro de Excel".to_owned(),
        SpreadsheetMessage::ExportWeightsWarning { total } => {
            format!("Os pesos das probas suman {total} %, non 100 %: as notas finais son provisionais.")
        }
        SpreadsheetMessage::ConfirmImportOverwrite { count, class } => {
            let (verb, what) = if *count == 1 {
                ("substitúese", "nota ou observación")
            } else {
                ("substitúense", "notas ou observacións")
            };
            format!("Ao importar {verb} {count} {what} que xa hai en {class}. Continuar?")
        }
        SpreadsheetMessage::ImportDone { pupils, new_pupils, marks, class } => {
            import_done(*pupils, *new_pupils, *marks, class)
        }
        SpreadsheetMessage::ImportLeftOut { headers, count } => {
            format!("{} fóra: {headers}.", plural(*count, "Quedou", "Quedaron"))
        }
        SpreadsheetMessage::ImportErrorCells { count } => error_cells(*count),
        SpreadsheetMessage::ImportUnnamedRows { count } => unnamed_rows(*count),
    }
}

/// The summary after an import: the pupils and marks it brought in.
fn import_done(pupils: usize, new_pupils: usize, marks: usize, class: &str) -> String {
    let added =
        if new_pupils > 0 { format!(" ({new_pupils} {})", plural(new_pupils, "novo", "novos")) } else { String::new() };
    let verb = plural(pupils, "Importouse", "Importáronse");
    format!(
        "{verb} {pupils} {}{added} e {marks} {} en {class}.",
        plural(pupils, "alumno", "alumnos"),
        plural(marks, "nota ou observación", "notas ou observacións")
    )
}

/// The cells an import skipped because Excel showed an error in them.
fn error_cells(count: usize) -> String {
    if count == 1 {
        "Saltouse 1 cela cun erro de Excel.".to_owned()
    } else {
        format!("Saltáronse {count} celas con erros de Excel.")
    }
}

/// The rows an import skipped because they have no first name.
fn unnamed_rows(count: usize) -> String {
    if count == 1 { "Saltouse 1 fila sen nome.".to_owned() } else { format!("Saltáronse {count} filas sen nome.") }
}
