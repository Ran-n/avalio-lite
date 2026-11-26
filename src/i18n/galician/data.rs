// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.569036
//+ Revised: 	2026/11/23 00:26:08.569036
// -----------------------------------------------------------------------------

//! The texts about the database, the logs, backups, the PIN and the lock, in Galician.

use crate::i18n::messages::{DataMessage, counted, plural, renamings};

/// A data text in Galician.
pub(super) fn message(message: &DataMessage) -> String {
    match message {
        DataMessage::DatabaseMoved { path } => format!("Base de datos movida a {path}"),
        DataMessage::DatabaseMovedOldKept { path, old } => format!(
            "Base de datos movida a {path}, pero non se puido borrar a copia antiga de {old}. Bórraa ti cando ningún outro programa a use."
        ),
        DataMessage::LogsMoved { path } => format!("Rexistros movidos a {path}"),
        DataMessage::DatabaseFileType => "Base de datos de avalio lite".to_owned(),
        DataMessage::BackupFileType => "Copia de seguranza de avalio lite".to_owned(),
        DataMessage::BackupExported { path } => format!("Copia gardada en {path}"),
        DataMessage::ConfirmRestore { file, classes, pupils } => confirm_restore(file, *classes, *pupils),
        DataMessage::BackupRestored { file } => format!("Restaurouse a copia {file}."),
        DataMessage::BackupClassesAdded { classes, pupils, existing, file } => {
            classes_added(*classes, *pupils, *existing, file)
        }
        DataMessage::BackupClassesRenamed { pairs } => {
            format!("Renomeadas porque o nome xa existía: {}.", renamings(pairs))
        }
        DataMessage::BackupHasNoClasses => "A copia non ten clases; non se engadiu nada.".to_owned(),
        DataMessage::LockNeedsPin => "Define un PIN en Axustes para bloquear a aplicación.".to_owned(),
        DataMessage::PinTurnedOn => "A base de datos xa está cifrada co teu PIN.".to_owned(),
        DataMessage::PinChanged => "PIN cambiado; a base de datos cifrouse de novo con el.".to_owned(),
        DataMessage::PinTurnedOff => "PIN eliminado; a base de datos xa non está cifrada.".to_owned(),
    }
}

/// The question before a restore replaces everything in the app.
fn confirm_restore(file: &str, classes: usize, pupils: usize) -> String {
    format!(
        "Substituír todo o que hai na aplicación pola copia {file} ({}, {})?\n\nSubstitúense todas as clases, alumnos e notas que hai agora. Non se pode desfacer. Exporta antes unha copia se as podes necesitar.",
        counted(classes, "clase", "clases"),
        counted(pupils, "alumno", "alumnos"),
    )
}

/// What a restore added, and how many classes were already here.
fn classes_added(classes: usize, pupils: usize, existing: usize, file: &str) -> String {
    let already = match existing {
        0 => String::new(),
        1 => " (1 xa estaba)".to_owned(),
        _ => format!(" ({existing} xa estaban)"),
    };
    let verb = plural(classes, "Engadiuse", "Engadíronse");
    format!(
        "{verb} {} e {}{already} desde {file}.",
        counted(classes, "clase", "clases"),
        counted(pupils, "alumno", "alumnos")
    )
}
