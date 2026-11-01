// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.721797
//+ Revised: 	2026/10/29 15:46:51.591640
// -----------------------------------------------------------------------------

use super::Language;
use crate::gradebook::model::ColumnType;

/// Text shown by Rust code: confirmation questions, status lines and fatal errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message<'a> {
    ConfirmDeleteClass {
        class: &'a str,
    },
    ConfirmRemovePupil {
        pupil: &'a str,
        class: &'a str,
    },
    ConfirmDeleteColumn {
        column: &'a str,
    },
    DatabaseMoved {
        path: &'a str,
    },
    LogsMoved {
        path: &'a str,
    },
    DatabaseFileType,
    BackupFileType,
    BackupExported {
        path: &'a str,
    },
    ConfirmRestore {
        file: &'a str,
        classes: usize,
        pupils: usize,
    },
    BackupRestored {
        file: &'a str,
    },
    BackupClassesAdded {
        classes: usize,
        pupils: usize,
        existing: usize,
        file: &'a str,
    },
    /// (name in the backup, name given here) of each class whose name was taken
    BackupClassesRenamed {
        pairs: &'a [(String, String)],
    },
    BackupHasNoClasses,
    WindowFailed {
        reason: &'a str,
    },
    LockNeedsPin,
    PinTurnedOn,
    PinChanged,
    PinTurnedOff,
    ShortcutChanged {
        action: &'a str,
        keys: &'a str,
    },
    ShortcutsReset,
    SheetExported {
        class: &'a str,
        path: &'a str,
    },
    SpreadsheetFileType,
    /// the note under an exported sheet whose final grades are provisional; `total` is localized
    ExportWeightsWarning {
        total: &'a str,
    },
    ConfirmImportOverwrite {
        count: usize,
        class: &'a str,
    },
    ImportDone {
        pupils: usize,
        new_pupils: usize,
        marks: usize,
        class: &'a str,
    },
    /// the `count` headers an import ignored, already joined
    ImportLeftOut {
        headers: &'a str,
        count: usize,
    },
    ImportErrorCells {
        count: usize,
    },
    /// how a left-out column with no header is named, by its letter
    UntitledColumn {
        letter: &'a str,
    },
}

pub fn text(language: Language, message: &Message) -> String {
    match language {
        Language::English => english(message),
        Language::Galician => galician(message),
    }
}

fn english(message: &Message) -> String {
    match message {
        Message::ConfirmDeleteClass { class } => {
            format!(
                "Delete the class \"{class}\" with all its columns and marks?\n\nIts pupils stay in their other classes."
            )
        }
        Message::ConfirmRemovePupil { pupil, class } => {
            format!(
                "Remove {pupil} from {class}?\n\nTheir marks in this class are deleted. They stay in their other classes."
            )
        }
        Message::ConfirmDeleteColumn { column } => {
            format!("Delete the column \"{column}\" and everything typed in it?")
        }
        Message::DatabaseMoved { path } => format!("Database moved to {path}"),
        Message::LogsMoved { path } => format!("Logs moved to {path}"),
        Message::DatabaseFileType => "avalio-lite database".to_owned(),
        Message::BackupFileType => "avalio-lite backup".to_owned(),
        Message::BackupExported { path } => format!("Backup saved to {path}"),
        Message::ConfirmRestore { file, classes, pupils } => format!(
            "Replace everything in the app with the backup {file} ({}, {})?\n\nEvery class, pupil and mark in the app now is replaced. This can't be undone. Export a backup first if you might need them.",
            counted(*classes, "class", "classes"),
            counted(*pupils, "pupil", "pupils"),
        ),
        Message::BackupRestored { file } => format!("Restored the backup {file}."),
        Message::BackupClassesAdded { classes, pupils, existing, file } => {
            let already = match existing {
                0 => String::new(),
                1 => " (1 was already here)".to_owned(),
                _ => format!(" ({existing} were already here)"),
            };
            format!(
                "Added {} and {}{already} from {file}.",
                counted(*classes, "class", "classes"),
                counted(*pupils, "pupil", "pupils")
            )
        }
        Message::BackupClassesRenamed { pairs } => {
            format!("Renamed because the name was taken: {}.", renamings(pairs))
        }
        Message::BackupHasNoClasses => "The backup has no classes; nothing was added.".to_owned(),
        Message::WindowFailed { reason } => format!("avalio-lite couldn't start: {reason}"),
        Message::LockNeedsPin => "Set a PIN in Settings to lock the app.".to_owned(),
        Message::PinTurnedOn => "The database is now encrypted with your PIN.".to_owned(),
        Message::PinChanged => "PIN changed; the database is re-encrypted with it.".to_owned(),
        Message::PinTurnedOff => "PIN removed; the database is no longer encrypted.".to_owned(),
        Message::ShortcutChanged { action, keys } => format!("{action} is now {keys}."),
        Message::ShortcutsReset => "All keyboard shortcuts are back to their defaults.".to_owned(),
        Message::SheetExported { class, path } => format!("Saved {class} to {path}"),
        Message::SpreadsheetFileType => "Excel workbook".to_owned(),
        Message::ExportWeightsWarning { total } => {
            format!("Test weights add up to {total}%, not 100%: final grades are provisional.")
        }
        Message::ConfirmImportOverwrite { count, class } => {
            let what = plural(*count, "mark or note", "marks or notes");
            format!("Importing replaces {count} {what} already in {class}. Continue?")
        }
        Message::ImportDone { pupils, new_pupils, marks, class } => {
            let added = if *new_pupils > 0 { format!(" ({new_pupils} new)") } else { String::new() };
            format!(
                "Imported {pupils} {}{added} and {marks} {} into {class}.",
                plural(*pupils, "pupil", "pupils"),
                plural(*marks, "mark or note", "marks or notes")
            )
        }
        Message::ImportLeftOut { headers, .. } => format!("Left out: {headers}."),
        Message::ImportErrorCells { count } => {
            if *count == 1 {
                "1 cell with an Excel error was skipped.".to_owned()
            } else {
                format!("{count} cells with Excel errors were skipped.")
            }
        }
        Message::UntitledColumn { letter } => format!("column {letter} (no title)"),
    }
}

fn galician(message: &Message) -> String {
    match message {
        Message::ConfirmDeleteClass { class } => {
            format!(
                "Eliminar a clase «{class}» con todas as súas columnas e notas?\n\nOs seus alumnos seguen nas outras clases."
            )
        }
        Message::ConfirmRemovePupil { pupil, class } => {
            format!("Quitar a {pupil} de {class}?\n\nBórranse as súas notas nesta clase. Segue nas outras clases.")
        }
        Message::ConfirmDeleteColumn { column } => {
            format!("Eliminar a columna «{column}» e todo o que se escribiu nela?")
        }
        Message::DatabaseMoved { path } => format!("Base de datos movida a {path}"),
        Message::LogsMoved { path } => format!("Rexistros movidos a {path}"),
        Message::DatabaseFileType => "Base de datos de avalio-lite".to_owned(),
        Message::BackupFileType => "Copia de seguranza de avalio-lite".to_owned(),
        Message::BackupExported { path } => format!("Copia gardada en {path}"),
        Message::ConfirmRestore { file, classes, pupils } => format!(
            "Substituír todo o que hai na aplicación pola copia {file} ({}, {})?\n\nSubstitúense todas as clases, alumnos e notas que hai agora. Non se pode desfacer. Exporta antes unha copia se as podes necesitar.",
            counted(*classes, "clase", "clases"),
            counted(*pupils, "alumno", "alumnos"),
        ),
        Message::BackupRestored { file } => format!("Restaurouse a copia {file}."),
        Message::BackupClassesAdded { classes, pupils, existing, file } => {
            let already = match existing {
                0 => String::new(),
                1 => " (1 xa estaba)".to_owned(),
                _ => format!(" ({existing} xa estaban)"),
            };
            format!(
                "Engadíronse {} e {}{already} desde {file}.",
                counted(*classes, "clase", "clases"),
                counted(*pupils, "alumno", "alumnos")
            )
        }
        Message::BackupClassesRenamed { pairs } => {
            format!("Renomeadas porque o nome xa existía: {}.", renamings(pairs))
        }
        Message::BackupHasNoClasses => "A copia non ten clases; non se engadiu nada.".to_owned(),
        Message::WindowFailed { reason } => format!("Non se puido iniciar avalio-lite: {reason}"),
        Message::LockNeedsPin => "Define un PIN en Axustes para bloquear a aplicación.".to_owned(),
        Message::PinTurnedOn => "A base de datos xa está cifrada co teu PIN.".to_owned(),
        Message::PinChanged => "PIN cambiado; a base de datos cifrouse de novo con el.".to_owned(),
        Message::PinTurnedOff => "PIN eliminado; a base de datos xa non está cifrada.".to_owned(),
        Message::ShortcutChanged { action, keys } => format!("{action} agora é {keys}."),
        Message::ShortcutsReset => "Todos os atallos de teclado volven ser os de orixe.".to_owned(),
        Message::SheetExported { class, path } => format!("Gardouse {class} en {path}"),
        Message::SpreadsheetFileType => "Libro de Excel".to_owned(),
        Message::ExportWeightsWarning { total } => {
            format!("Os pesos das probas suman {total}%, non 100%: as notas finais son provisionais.")
        }
        Message::ConfirmImportOverwrite { count, class } => {
            let (verb, what) = if *count == 1 {
                ("substitúese", "nota ou observación")
            } else {
                ("substitúense", "notas ou observacións")
            };
            format!("Ao importar {verb} {count} {what} que xa hai en {class}. Continuar?")
        }
        Message::ImportDone { pupils, new_pupils, marks, class } => {
            let added = if *new_pupils > 0 {
                format!(" ({new_pupils} {})", plural(*new_pupils, "novo", "novos"))
            } else {
                String::new()
            };
            format!(
                "Importáronse {pupils} {}{added} e {marks} {} en {class}.",
                plural(*pupils, "alumno", "alumnos"),
                plural(*marks, "nota ou observación", "notas ou observacións")
            )
        }
        Message::ImportLeftOut { headers, count } => {
            format!("{} fóra: {headers}.", plural(*count, "Quedou", "Quedaron"))
        }
        Message::ImportErrorCells { count } => {
            if *count == 1 {
                "Saltouse 1 cela cun erro de Excel.".to_owned()
            } else {
                format!("Saltáronse {count} celas con erros de Excel.")
            }
        }
        Message::UntitledColumn { letter } => format!("columna {letter} (sen título)"),
    }
}

/// `singular` for exactly one, `plural` otherwise (none included), as English and Galician count.
fn plural(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}

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

/// "1 class", "4 classes": both languages here add the plural form for anything but one.
fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// "1A → 1A (2), 1B → 1B (2)"
fn renamings(pairs: &[(String, String)]) -> String {
    pairs.iter().map(|(from, to)| format!("{from} → {to}")).collect::<Vec<_>>().join(", ")
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
