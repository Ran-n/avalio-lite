// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:07.840131
//+ Revised: 	2026/11/25 11:43:06.183693
// -----------------------------------------------------------------------------

//! The error texts in English, grouped by what went wrong: the database, files and folders, the
//! fields and PIN, and keyboard shortcuts.

use crate::error::{AppError, Folder, NumberField};
use crate::i18n::Language;
use crate::i18n::errors::{DatabaseProblem, IoProblem, entries};
use crate::i18n::shortcuts::{action_label, display};
use crate::shortcuts::ShortcutError;

/// An error in English, as `error_message` describes. Each arm is sent to its group's function;
/// the groups match only their own errors, which this routing guarantees.
pub(in crate::i18n) fn error(error: &AppError) -> String {
    match error {
        AppError::Database(_)
        | AppError::NewerSchema { .. }
        | AppError::IntegrityCheckFailed(_)
        | AppError::NotADatabase(_)
        | AppError::OpenDatabaseAsBackup(_)
        | AppError::KeptAside { .. }
        | AppError::NoDatabaseOpen => database(error),
        AppError::Io { .. }
        | AppError::ConfigParse { .. }
        | AppError::ConfigSerialize(_)
        | AppError::NoConfigDir
        | AppError::EmptyPath(_)
        | AppError::RelativePath(_)
        | AppError::NotFound(_)
        | AppError::AlreadyExists(_)
        | AppError::SpreadsheetWrite { .. }
        | AppError::SpreadsheetRead { .. }
        | AppError::PictureRead { .. }
        | AppError::NoPupilsInFile(_)
        | AppError::DuplicatePupilInFile(_) => files(error),
        AppError::EmptyName
        | AppError::DuplicateClassName(_)
        | AppError::InvalidNumber { .. }
        | AppError::NotEditable
        | AppError::InvalidRoundSource
        | AppError::InvalidPin
        | AppError::PinMismatch
        | AppError::WrongPin
        | AppError::PinWait { .. } => fields(error),
        AppError::InvalidShortcuts(_) | AppError::Shortcut(_) => shortcuts(error),
    }
}

/// The database errors.
fn database(error: &AppError) -> String {
    match error {
        AppError::Database(source) => database_problem(source),
        AppError::NewerSchema { found, supported } => format!(
            "This database was made by a newer version of avalio lite (schema {found}; this version reads up to {supported}). Update the app to open it."
        ),
        AppError::IntegrityCheckFailed(path) => format!("The database at {} is damaged.", path.display()),
        AppError::NotADatabase(path) => format!("{} isn't an avalio lite backup or database.", path.display()),
        AppError::OpenDatabaseAsBackup(path) => {
            format!("{} is the database the app is using now, not a backup. Choose a backup file.", path.display())
        }
        AppError::KeptAside { kept, path } => {
            let (kept, path) = (kept.display(), path.display());
            format!(
                "That didn't work, and your data couldn't be put back in place. It is safe in {kept}. With the app closed, move away anything at {path}, then give the safe copy that name to use it again."
            )
        }
        AppError::NoDatabaseOpen => "No database is open.".to_owned(),
        _ => unreachable!("error() sends only database errors to database()"),
    }
}

/// What went wrong with the database, in words the user can act on.
fn database_problem(source: &rusqlite::Error) -> String {
    match DatabaseProblem::of(source) {
        DatabaseProblem::Busy => "The database is being used by another program. Close it and try again.".to_owned(),
        DatabaseProblem::Damaged => "The database file is damaged, or isn't an avalio lite database.".to_owned(),
        DatabaseProblem::Full => {
            "There isn't enough free space for the database. Free some space and try again.".to_owned()
        }
        DatabaseProblem::Other => {
            "Something went wrong with the database. Try again; the log has the details.".to_owned()
        }
    }
}

/// The errors about files, folders, the settings file and spreadsheets.
fn files(error: &AppError) -> String {
    match error {
        AppError::Io { path, source } => io_problem(path, source),
        AppError::ConfigParse { path, .. } => {
            format!("The settings file {} is damaged and can't be read.", path.display())
        }
        AppError::ConfigSerialize(_) => "Couldn't save your settings. Try again; the log has the details.".to_owned(),
        AppError::NoConfigDir => "Couldn't find your settings folder.".to_owned(),
        AppError::EmptyPath(Folder::Database) => "Choose a folder for the database.".to_owned(),
        AppError::EmptyPath(Folder::Logs) => "Choose a folder for the logs.".to_owned(),
        AppError::RelativePath(Folder::Database) => {
            r"Give the database folder in full, with its drive, such as D:\Marks.".to_owned()
        }
        AppError::RelativePath(Folder::Logs) => {
            r"Give the log folder in full, with its drive, such as D:\Marks.".to_owned()
        }
        AppError::NotFound(path) => format!("{} doesn't exist.", path.display()),
        AppError::AlreadyExists(path) => format!("{} already exists.", path.display()),
        AppError::SpreadsheetWrite { path, .. } => {
            format!("Couldn't save {}. Check that it isn't open in Excel and that you can save there.", path.display())
        }
        AppError::SpreadsheetRead { path, .. } => format!(
            "{} isn't a workbook that can be read. Check that it is an Excel .xlsx file and isn't damaged.",
            path.display()
        ),
        AppError::PictureRead { path, .. } => format!(
            "{} isn't a picture that can be read. Choose a JPEG, PNG or WebP file that isn't damaged.",
            path.display()
        ),
        AppError::NoPupilsInFile(path) => format!(
            "{} has no pupils: its first row needs a First name header (or the names in its first column), with one pupil on each row below.",
            path.display()
        ),
        AppError::DuplicatePupilInFile(name) => {
            format!("The file has \"{name}\" twice; each pupil can only be in it once.")
        }
        _ => unreachable!("error() sends only file errors to files()"),
    }
}

/// What went wrong with a file, in words the user can act on.
fn io_problem(path: &std::path::Path, source: &std::io::Error) -> String {
    let path = path.display();
    match IoProblem::of(source) {
        IoProblem::NotFound => format!("Couldn't find {path}."),
        IoProblem::Denied => {
            format!("avalio lite isn't allowed to use {path}. Choose another place, or check its permissions.")
        }
        IoProblem::InUse => format!("{path} is being used by another program. Close it and try again."),
        IoProblem::Full => {
            format!("There isn't enough free space for {path}. Free some space or choose another place.")
        }
        IoProblem::Other => {
            format!("Couldn't access {path}. Check that it is available and try again; the log has the details.")
        }
    }
}

/// The errors about a value typed into a field: a name, a number, a rounding source or a PIN.
fn fields(error: &AppError) -> String {
    match error {
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
        _ => unreachable!("error() sends only field errors to fields()"),
    }
}

/// The errors about keyboard shortcuts: a bad entry in the settings file, or a key that can't be used.
fn shortcuts(error: &AppError) -> String {
    match error {
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
        _ => unreachable!("error() sends only shortcut errors to shortcuts()"),
    }
}
