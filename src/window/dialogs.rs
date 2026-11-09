// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.081050
//+ Revised: 	2026/11/07 20:25:36.558700
// -----------------------------------------------------------------------------

use std::path::{Path, PathBuf};

use crate::backup;
use crate::i18n::{self, Language, Message};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

/// The kinds of file the app opens and saves, each with its own filter in the file dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Database,
    Spreadsheet,
    Backup,
}

impl FileKind {
    /// What the open dialog shows: a spreadsheet may come from any program calamine reads, and a
    /// backup can also be a plain database file (a copied `avalio-lite.db`): it is the same format.
    pub fn open_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Spreadsheet => &["xlsx", "xls", "ods"],
            Self::Database => self.save_extensions(),
            Self::Backup => &[backup::EXTENSION, "db"],
        }
    }

    /// What the save dialog offers, the first being what the app writes.
    pub fn save_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Database => &["db"],
            Self::Spreadsheet => &["xlsx"],
            Self::Backup => &[backup::EXTENSION],
        }
    }

    /// The filter's name in the file dialogs, such as "Excel workbook".
    pub fn filter_name(self, language: Language) -> String {
        let message = match self {
            Self::Database => Message::DatabaseFileType,
            Self::Spreadsheet => Message::SpreadsheetFileType,
            Self::Backup => Message::BackupFileType,
        };
        i18n::text(language, &message)
    }
}

/// The questions the app asks through the operating system. Behind a trait so tests can answer
/// them without a native dialog opening.
pub trait Dialogs {
    /// Folder picker, opened at `start` when it exists. `None` means the user cancelled.
    fn pick_folder(&self, start: &Path) -> Option<PathBuf>;
    /// Open dialog for a file of `kind`. `None` means the user cancelled.
    fn pick_file(&self, kind: FileKind) -> Option<PathBuf>;
    /// Save dialog for a file of `kind`, opened at `start` when it exists and proposing
    /// `file_name`. Windows itself asks before replacing a file. `None` means the user cancelled.
    fn save_file(&self, kind: FileKind, start: &Path, file_name: &str) -> Option<PathBuf>;
    /// Yes/No question before anything that deletes data.
    fn confirm(&self, question: &str) -> bool;
    /// Last-resort error shown when the main window can't be used to report it.
    fn show_fatal(&self, message: &str);
    /// Opens a web page in the default browser; `Err` says why it couldn't.
    fn open_link(&self, url: &str) -> Result<(), String>;
}

/// The Windows dialogs, through `rfd`.
pub struct NativeDialogs;

impl Dialogs for NativeDialogs {
    fn pick_folder(&self, start: &Path) -> Option<PathBuf> {
        opened_at(FileDialog::new(), start).pick_folder()
    }

    fn pick_file(&self, kind: FileKind) -> Option<PathBuf> {
        filtered(kind, kind.open_extensions()).pick_file()
    }

    fn save_file(&self, kind: FileKind, start: &Path, file_name: &str) -> Option<PathBuf> {
        opened_at(filtered(kind, kind.save_extensions()), start).set_file_name(file_name).save_file()
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

    fn open_link(&self, url: &str) -> Result<(), String> {
        use windows_sys::Win32::UI::Shell::ShellExecuteW;
        use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
        let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
        let (verb, target) = (wide("open"), wide(url));
        // SAFETY: two NUL-terminated UTF-16 strings that outlive the call; no owner window.
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                verb.as_ptr(),
                target.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                SW_SHOWNORMAL,
            )
        };
        // ShellExecuteW reports success with a value above 32, and an error code otherwise
        let code = result as isize;
        if code > 32 { Ok(()) } else { Err(format!("ShellExecuteW failed ({code})")) }
    }
}

/// A file dialog showing only files of `kind` with one of `extensions`.
fn filtered(kind: FileKind, extensions: &[&str]) -> FileDialog {
    FileDialog::new().add_filter(kind.filter_name(i18n::current()), extensions)
}

/// Starts the dialog in `start`, unless that folder is gone (Windows then picks the folder).
fn opened_at(dialog: FileDialog, start: &Path) -> FileDialog {
    if start.is_dir() { dialog.set_directory(start) } else { dialog }
}
