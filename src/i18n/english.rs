// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:30.944950
//+ Revised: 	2026/11/08 04:36:59.523637
// -----------------------------------------------------------------------------

//! The Rust-side texts and error messages in English.

use super::Language;
use super::errors::{DatabaseProblem, IoProblem, entries};
use super::messages::{Message, counted, plural, renamings};
use super::shortcuts::{action_label, display};
use crate::error::{AppError, Folder, NumberField};
use crate::shortcuts::ShortcutError;

/// A message in English.
pub(super) fn message(message: &Message) -> String {
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
        Message::ConfirmDeletePupil { pupil } => {
            format!("Delete {pupil}? They're in no class; this can't be undone.")
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
        Message::WindowFailed => {
            "avalio-lite couldn't start. Close it and open it again; if that keeps happening, the log has the details."
                .to_owned()
        }
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
        Message::LicenceAskForKey => {
            "avalio-lite needs a licence key to run. You received it by email; it looks like this:
BRN-XXXX-XXXX-XXXX-XXXX"
                .to_owned()
        }
        Message::LicenceInvalidKey => "That key isn't valid for avalio-lite. Check it and try again.".to_owned(),
        Message::LicenceInUseElsewhere { email } => {
            format!("This key is already in use on another computer. To move it here, write to {email}.")
        }
        Message::LicenceRevoked { email } => {
            format!("This licence has been cancelled. Write to {email} if you think this is a mistake.")
        }
        Message::LicenceExpired { date: Some(date), email } => {
            format!("Your licence ended on {date}. Write to {email} to renew it.")
        }
        Message::LicenceExpired { date: None, email } => {
            format!("Your licence has ended. Write to {email} to renew it.")
        }
        Message::LicenceWithdrawn { email } => format!("avalio-lite can't be licensed at the moment. Write to {email}."),
        Message::LicenceNeedsInternet => "Connect to the internet so avalio-lite can confirm your licence. It works offline for up to 30 days at a time.".to_owned(),
        Message::LicenceWait { minutes } => format!("Too many attempts. Try again in {minutes} min."),
        Message::LicenceUnreachable => {
            "The licence server can't be reached. Check the internet connection and try again.".to_owned()
        }
        Message::LicenceNotSaved { error } => format!("The licence couldn't be saved on this computer: {error}"),
        Message::BackgroundFailed { error } => format!("The check couldn't be started: {error}"),
        Message::LicenceConnectBy { date } => format!("Connect to the internet by {date} to keep using avalio-lite."),
        Message::LicenceEndsOn { date, email } => {
            format!("Your licence ends on {date}. Write to {email} to renew it.")
        }
        Message::LicencePermanent => "Permanent".to_owned(),
        Message::LicenceUntil { date } => format!("Until {date}"),
        Message::LicenceOfflineUntil { date } => format!("Works offline until {date}"),
        Message::LicenceConfirmed => "Licence confirmed.".to_owned(),
        Message::UpdateAvailable { version } => format!("avalio-lite {version} is available."),
        Message::UpToDate { version } => format!("avalio-lite is up to date ({version})."),
        Message::UpdateCheckFailed { reason } => format!("Couldn't check for updates: {reason}"),
        Message::LinkFailed { error } => format!("Couldn't open the page: {error}"),
        Message::UpdateNotVerified => "The update couldn't be verified, so it wasn't installed.".to_owned(),
        Message::UpdateDownloadFailed { reason } => format!("Couldn't download the update: {reason}"),
        Message::UpdateNotPlaced { reason } => format!("The update couldn't be put in place: {reason}"),
        Message::UpdateFolderReadOnly => {
            "This folder can't be written to. Download the new version from the release page.".to_owned()
        }
        Message::UpdateFileMissing { file } => {
            format!("The release has no {file}. Download the new version from the release page.")
        }
        Message::UpdatedTo { version } => format!("Updated to avalio-lite {version}."),
        Message::UpdateNotStarted { reason } => {
            format!("The update couldn't be started: {reason}. Download it from the release page.")
        }
    }
}

/// An error in English, as `error_message` describes.
pub(super) fn error(error: &AppError) -> String {
    match error {
        AppError::Io { path, source } => {
            let path = path.display();
            match IoProblem::of(source) {
                IoProblem::NotFound => format!("Couldn't find {path}."),
                IoProblem::Denied => {
                    format!("avalio-lite isn't allowed to use {path}. Choose another place, or check its permissions.")
                }
                IoProblem::InUse => format!("{path} is being used by another program. Close it and try again."),
                IoProblem::Full => {
                    format!("There isn't enough free space for {path}. Free some space or choose another place.")
                }
                IoProblem::Other => {
                    format!(
                        "Couldn't access {path}. Check that it is available and try again; the log has the details."
                    )
                }
            }
        }
        AppError::Database(source) => match DatabaseProblem::of(source) {
            DatabaseProblem::Busy => {
                "The database is being used by another program. Close it and try again.".to_owned()
            }
            DatabaseProblem::Damaged => "The database file is damaged, or isn't an avalio-lite database.".to_owned(),
            DatabaseProblem::Full => {
                "There isn't enough free space for the database. Free some space and try again.".to_owned()
            }
            DatabaseProblem::Other => {
                "Something went wrong with the database. Try again; the log has the details.".to_owned()
            }
        },
        AppError::ConfigParse { path, .. } => {
            format!("The settings file {} is damaged and can't be read.", path.display())
        }
        AppError::ConfigSerialize(_) => "Couldn't save your settings. Try again; the log has the details.".to_owned(),
        AppError::NoConfigDir => "Couldn't find your settings folder.".to_owned(),
        AppError::EmptyPath(Folder::Database) => "Choose a folder for the database.".to_owned(),
        AppError::EmptyPath(Folder::Logs) => "Choose a folder for the logs.".to_owned(),
        AppError::NotFound(path) => format!("{} doesn't exist.", path.display()),
        AppError::AlreadyExists(path) => format!("{} already exists.", path.display()),
        AppError::NewerSchema { found, supported } => format!(
            "This database was made by a newer version of avalio-lite (schema {found}; this version reads up to {supported}). Update the app to open it."
        ),
        AppError::IntegrityCheckFailed(path) => format!("The database at {} is damaged.", path.display()),
        AppError::NotADatabase(path) => format!("{} isn't an avalio-lite backup or database.", path.display()),
        AppError::KeptAside { kept, path } => {
            let (kept, path) = (kept.display(), path.display());
            format!(
                "That didn't work, and your data couldn't be put back in place. It is safe in {kept}. With the app closed, move away anything at {path}, then give the safe copy that name to use it again."
            )
        }
        AppError::NoDatabaseOpen => "No database is open.".to_owned(),
        AppError::EmptyName => "A name can't be empty.".to_owned(),
        AppError::DuplicateClassName(name) => format!("There's already a class called \"{name}\"."),
        AppError::InvalidNumber { field: NumberField::Weight, value } => {
            format!("\"{value}\" isn't a valid weight. Use a number from 0 to 1000.")
        }
        AppError::InvalidNumber { field: NumberField::MaxScore, value } => {
            format!("\"{value}\" isn't a valid maximum. Use a number above 0 and up to 10000.")
        }
        AppError::InvalidNumber { field: NumberField::Decimals, value } => {
            format!("\"{value}\" isn't a valid number of decimals. Use 0, 1, 2 or 3.")
        }
        AppError::NotEditable => "That value can't be edited.".to_owned(),
        AppError::InvalidRoundSource => {
            "A rounded column can only round a test, final grade or average of the same class.".to_owned()
        }
        AppError::InvalidPin => "A PIN is 4 to 128 characters: digits, letters, spaces or symbols.".to_owned(),
        AppError::PinMismatch => "The two PINs don't match. Type the same PIN twice.".to_owned(),
        AppError::WrongPin => "Wrong PIN.".to_owned(),
        AppError::PinWait { seconds } => format!("Too many wrong PINs. Try again in {seconds} s."),
        AppError::InvalidShortcuts(problems) => format!(
            "Some keyboard shortcuts in config.toml weren't valid and use their defaults: {}.",
            entries(problems)
        ),
        AppError::Shortcut(ShortcutError::UnknownKey) => "That key can't be used for a shortcut.".to_owned(),
        AppError::Shortcut(ShortcutError::NeedsCtrlOrAlt) => {
            "Add Ctrl or Alt: a plain letter would type into the cell.".to_owned()
        }
        AppError::Shortcut(ShortcutError::CtrlAlt) => {
            "Ctrl+Alt is the AltGr key on many keyboards, so it can't be used.".to_owned()
        }
        AppError::Shortcut(ShortcutError::EditingKey(letter)) => format!("Ctrl+{letter} is used for editing text."),
        AppError::Shortcut(ShortcutError::Clash { keys, taken_by }) => format!(
            "{} is already used by {}.",
            display(Language::English, keys),
            action_label(Language::English, *taken_by)
        ),
        AppError::SpreadsheetWrite { path, .. } => {
            format!("Couldn't save {}. Check that it isn't open in Excel and that you can save there.", path.display())
        }
        AppError::SpreadsheetRead { path, .. } => format!(
            "{} isn't a workbook that can be read. Check that it is an Excel .xlsx file and isn't damaged.",
            path.display()
        ),
        AppError::NoPupilsInFile(path) => {
            format!("{} has no pupils: names go in column A, from row 2.", path.display())
        }
        AppError::DuplicatePupilInFile(name) => {
            format!("The file has \"{name}\" twice; each pupil can only be in it once.")
        }
    }
}
