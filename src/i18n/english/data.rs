// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:07.720622
//+ Revised: 	2026/11/23 00:26:07.720622
// -----------------------------------------------------------------------------

//! The texts about the database, the logs, backups, the PIN and the lock, in English.

use crate::i18n::messages::{DataMessage, counted, renamings};

/// A data text in English.
pub(super) fn message(message: &DataMessage) -> String {
    match message {
        DataMessage::DatabaseMoved { path } => format!("Database moved to {path}"),
        DataMessage::DatabaseMovedOldKept { path, old } => format!(
            "Database moved to {path}, but the old copy at {old} couldn't be deleted. Delete it yourself once no other program uses it."
        ),
        DataMessage::LogsMoved { path } => format!("Logs moved to {path}"),
        DataMessage::DatabaseFileType => "avalio lite database".to_owned(),
        DataMessage::BackupFileType => "avalio lite backup".to_owned(),
        DataMessage::BackupExported { path } => format!("Backup saved to {path}"),
        DataMessage::ConfirmRestore { file, classes, pupils } => confirm_restore(file, *classes, *pupils),
        DataMessage::BackupRestored { file } => format!("Restored the backup {file}."),
        DataMessage::BackupClassesAdded { classes, pupils, existing, file } => {
            classes_added(*classes, *pupils, *existing, file)
        }
        DataMessage::BackupClassesRenamed { pairs } => {
            format!("Renamed because the name was taken: {}.", renamings(pairs))
        }
        DataMessage::BackupHasNoClasses => "The backup has no classes; nothing was added.".to_owned(),
        DataMessage::LockNeedsPin => "Set a PIN in Settings to lock the app.".to_owned(),
        DataMessage::PinTurnedOn => "The database is now encrypted with your PIN.".to_owned(),
        DataMessage::PinChanged => "PIN changed; the database is re-encrypted with it.".to_owned(),
        DataMessage::PinTurnedOff => "PIN removed; the database is no longer encrypted.".to_owned(),
    }
}

/// The question before a restore replaces everything in the app.
fn confirm_restore(file: &str, classes: usize, pupils: usize) -> String {
    format!(
        "Replace everything in the app with the backup {file} ({}, {})?\n\nEvery class, pupil and mark in the app now is replaced. This can't be undone. Export a backup first if you might need them.",
        counted(classes, "class", "classes"),
        counted(pupils, "pupil", "pupils"),
    )
}

/// What a restore added, and how many classes were already here.
fn classes_added(classes: usize, pupils: usize, existing: usize, file: &str) -> String {
    let already = match existing {
        0 => String::new(),
        1 => " (1 was already here)".to_owned(),
        _ => format!(" ({existing} were already here)"),
    };
    format!(
        "Added {} and {}{already} from {file}.",
        counted(classes, "class", "classes"),
        counted(pupils, "pupil", "pupils")
    )
}
