// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.903872
//+ Revised: 	2026/11/23 00:26:09.903872
// -----------------------------------------------------------------------------

//! The spreadsheet texts: exports, imports and what an import left out; one of each, checked in
//! both languages.

use avalio_lite::i18n::{DataMessage, Language, Message, SpreadsheetMessage, text};

use super::assert_messages_translated;

/// One of each spreadsheet message. The match below has no wildcard, so a new variant won't compile
/// until it is added here too.
pub fn every_spreadsheet_message() -> Vec<Message<'static>> {
    let all = [
        SpreadsheetMessage::SheetExported { class: "1A", path: "D:\\x" },
        SpreadsheetMessage::SpreadsheetFileType,
        SpreadsheetMessage::ExportWeightsWarning { total: "boom" },
        SpreadsheetMessage::ConfirmImportOverwrite { count: 2, class: "1A" },
        SpreadsheetMessage::ImportDone { pupils: 2, new_pupils: 1, marks: 3, class: "1A" },
        SpreadsheetMessage::ImportLeftOut { headers: "Phone", count: 1 },
        SpreadsheetMessage::ImportErrorCells { count: 2 },
        SpreadsheetMessage::ImportUnnamedRows { count: 1 },
        SpreadsheetMessage::ImportUnnamedRows { count: 3 },
    ];
    for message in &all {
        match message {
            SpreadsheetMessage::SheetExported { .. }
            | SpreadsheetMessage::SpreadsheetFileType
            | SpreadsheetMessage::ExportWeightsWarning { .. }
            | SpreadsheetMessage::ConfirmImportOverwrite { .. }
            | SpreadsheetMessage::ImportDone { .. }
            | SpreadsheetMessage::ImportLeftOut { .. }
            | SpreadsheetMessage::ImportErrorCells { .. }
            | SpreadsheetMessage::ImportUnnamedRows { .. } => {}
        }
    }
    all.into_iter().map(Message::Spreadsheet).collect()
}

#[test]
fn every_spreadsheet_message_is_translated_and_keeps_its_details() {
    assert_messages_translated(&every_spreadsheet_message());
}

/// One of a thing is singular, and so is its verb in Galician; none or several are plural.
#[test]
fn import_messages_count_in_every_language() {
    let english = |message: &Message| text(Language::English, message);
    let galician = |message: &Message| text(Language::Galician, message);
    let done = |pupils, new_pupils, marks| {
        Message::Spreadsheet(SpreadsheetMessage::ImportDone { pupils, new_pupils, marks, class: "1A" })
    };
    assert_eq!(english(&done(1, 0, 1)), "Imported 1 pupil and 1 mark or note into 1A.");
    assert_eq!(english(&done(3, 2, 0)), "Imported 3 pupils (2 new) and 0 marks or notes into 1A.");
    assert_eq!(galician(&done(1, 1, 1)), "Importouse 1 alumno (1 novo) e 1 nota ou observación en 1A.");
    assert_eq!(galician(&done(3, 2, 0)), "Importáronse 3 alumnos (2 novos) e 0 notas ou observacións en 1A.");
    assert_eq!(galician(&done(2, 0, 2)), "Importáronse 2 alumnos e 2 notas ou observacións en 1A.");
    let added =
        |classes, pupils| Message::Data(DataMessage::BackupClassesAdded { classes, pupils, existing: 0, file: "Exam" });
    assert_eq!(galician(&added(1, 3)), "Engadiuse 1 clase e 3 alumnos desde Exam.");
    assert_eq!(galician(&added(2, 1)), "Engadíronse 2 clases e 1 alumno desde Exam.");
    assert_eq!(english(&added(1, 1)), "Added 1 class and 1 pupil from Exam.");
    let overwrite = |count| Message::Spreadsheet(SpreadsheetMessage::ConfirmImportOverwrite { count, class: "1A" });
    assert_eq!(english(&overwrite(1)), "Importing replaces 1 mark or note already in 1A. Continue?");
    assert_eq!(galician(&overwrite(1)), "Ao importar substitúese 1 nota ou observación que xa hai en 1A. Continuar?");
    assert_eq!(
        galician(&overwrite(12)),
        "Ao importar substitúense 12 notas ou observacións que xa hai en 1A. Continuar?"
    );
    let left_out = |headers, count| Message::Spreadsheet(SpreadsheetMessage::ImportLeftOut { headers, count });
    assert_eq!(galician(&left_out("Teléfono", 1)), "Quedou fóra: Teléfono.");
    assert_eq!(
        galician(&left_out("Teléfono, columna D (sen título)", 2)),
        "Quedaron fóra: Teléfono, columna D (sen título)."
    );
    assert_eq!(english(&left_out("Phone, column D (no title)", 2)), "Left out: Phone, column D (no title).");
    let errors = |count| Message::Spreadsheet(SpreadsheetMessage::ImportErrorCells { count });
    assert_eq!(english(&errors(1)), "1 cell with an Excel error was skipped.");
    assert_eq!(english(&errors(2)), "2 cells with Excel errors were skipped.");
    assert_eq!(galician(&errors(1)), "Saltouse 1 cela cun erro de Excel.");
    assert_eq!(galician(&errors(2)), "Saltáronse 2 celas con erros de Excel.");
}
