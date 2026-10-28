// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.081050
//+ Revised: 	2026/10/28 15:53:08.459048
// -----------------------------------------------------------------------------

use std::path::{Path, PathBuf};

use crate::i18n::{self, Message};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

const DATABASE_EXTENSIONS: &[&str] = &["db"];

/// The questions the app asks through the operating system. Behind a trait so tests can answer
/// them without a native dialog opening.
pub trait Dialogs {
    /// Folder picker, opened at `start` when it exists. `None` means the user cancelled.
    fn pick_folder(&self, start: &Path) -> Option<PathBuf>;
    /// Picker for a database file. `None` means the user cancelled.
    fn pick_database_file(&self) -> Option<PathBuf>;
    /// Yes/No question before anything that deletes data.
    fn confirm(&self, question: &str) -> bool;
    /// Last-resort error shown when the main window can't be used to report it.
    fn show_fatal(&self, message: &str);
}

/// The Windows dialogs, through `rfd`.
pub struct NativeDialogs;

impl Dialogs for NativeDialogs {
    fn pick_folder(&self, start: &Path) -> Option<PathBuf> {
        let dialog = FileDialog::new();
        let dialog = if start.is_dir() { dialog.set_directory(start) } else { dialog };
        dialog.pick_folder()
    }

    fn pick_database_file(&self) -> Option<PathBuf> {
        FileDialog::new()
            .add_filter(i18n::text(i18n::current(), &Message::DatabaseFileType), DATABASE_EXTENSIONS)
            .pick_file()
    }

    fn confirm(&self, question: &str) -> bool {
        MessageDialog::new()
            .set_level(MessageLevel::Warning)
            .set_title("avalio-lite")
            .set_description(question)
            .set_buttons(MessageButtons::YesNo)
            .show()
            == MessageDialogResult::Yes
    }

    fn show_fatal(&self, message: &str) {
        MessageDialog::new()
            .set_level(MessageLevel::Error)
            .set_title("avalio-lite")
            .set_description(message)
            .set_buttons(MessageButtons::Ok)
            .show();
    }
}
