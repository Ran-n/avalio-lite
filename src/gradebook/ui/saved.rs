// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/14 22:47:09.678780
//+ Revised: 	2026/11/15 03:14:39.275214
// -----------------------------------------------------------------------------

//! The acknowledgement on the sheet's summary line that changes are saved: the app has no Save
//! button, so this is where a teacher sees that what they typed was kept (or wasn't).

use slint::ComponentHandle;

use crate::error::AppError;
use crate::ui::{AppWindow, Gradebook};

/// A change was written to the database: the summary line says "Saved" and lights up.
pub fn mark_saved(window: &AppWindow) {
    let gradebook = window.global::<Gradebook>();
    gradebook.set_saved(true);
    gradebook.set_save_failed(false);
    gradebook.set_save_count(gradebook.get_save_count().saturating_add(1));
}

/// A change failed, so the screen may show what the database doesn't hold: the summary line says
/// "Not saved" until a change is written, or the field puts its saved value back.
pub fn mark_not_saved(window: &AppWindow) {
    window.global::<Gradebook>().set_save_failed(true);
}

/// The refused text is gone from the screen, which shows what is saved again.
pub fn clear_not_saved(window: &AppWindow) {
    window.global::<Gradebook>().set_save_failed(false);
}

/// Back to "Saves as you type", for a database just opened: nothing was changed in it yet.
pub fn forget_saved(window: &AppWindow) {
    let gradebook = window.global::<Gradebook>();
    gradebook.set_saved(false);
    gradebook.set_save_failed(false);
    gradebook.set_save_count(0);
}

/// Whether a failed action may have left something typed on screen that the database doesn't
/// hold. Not so for a file that couldn't be read or written (an import, an export) or with no
/// database open, where nothing typed is at stake. Every error is named, so a new one has to be
/// put on one side or the other.
pub fn leaves_typed_data_unsaved(error: &AppError) -> bool {
    match error {
        AppError::NoDatabaseOpen
        | AppError::SpreadsheetRead { .. }
        | AppError::SpreadsheetWrite { .. }
        | AppError::NoPupilsInFile(_)
        | AppError::DuplicatePupilInFile(_) => false,
        AppError::Io { .. }
        | AppError::Database(_)
        | AppError::ConfigParse { .. }
        | AppError::ConfigSerialize(_)
        | AppError::NoConfigDir
        | AppError::EmptyPath(_)
        | AppError::RelativePath(_)
        | AppError::NotFound(_)
        | AppError::AlreadyExists(_)
        | AppError::NewerSchema { .. }
        | AppError::IntegrityCheckFailed(_)
        | AppError::NotADatabase(_)
        | AppError::OpenDatabaseAsBackup(_)
        | AppError::KeptAside { .. }
        | AppError::EmptyName
        | AppError::DuplicateClassName(_)
        | AppError::InvalidNumber { .. }
        | AppError::NotEditable
        | AppError::InvalidRoundSource
        | AppError::InvalidPin
        | AppError::PinMismatch
        | AppError::WrongPin
        | AppError::PinWait { .. }
        | AppError::InvalidShortcuts(_)
        | AppError::Shortcut(_) => true,
    }
}

#[cfg(test)]
mod tests {
    use super::leaves_typed_data_unsaved;
    use crate::error::{AppError, NumberField};

    #[test]
    fn every_error_is_classified_for_the_indicator() {
        let file = || std::path::PathBuf::from("marks.xlsx");
        for error in [
            AppError::NoDatabaseOpen,
            AppError::SpreadsheetRead { path: file(), reason: String::new() },
            AppError::SpreadsheetWrite { path: file(), reason: String::new() },
            AppError::NoPupilsInFile(file()),
            AppError::DuplicatePupilInFile("Ana".into()),
        ] {
            assert!(!leaves_typed_data_unsaved(&error), "{error:?}");
        }
        for error in [
            AppError::Database(rusqlite::Error::InvalidQuery),
            AppError::Io { path: file(), source: std::io::Error::other("denied") },
            AppError::InvalidNumber { field: NumberField::Weight, value: "x".into() },
            AppError::EmptyName,
            AppError::NotEditable,
        ] {
            assert!(leaves_typed_data_unsaved(&error), "{error:?}");
        }
    }
}
