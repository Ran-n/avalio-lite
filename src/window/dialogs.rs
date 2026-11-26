// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.081050
//+ Revised: 	2026/11/25 11:43:10.223951
// -----------------------------------------------------------------------------

//! The questions and file pickers Windows shows, and opening a link, behind the `Dialogs` trait, so tests
//! answer in their place. What each question says is in `i18n`.

use std::path::{Path, PathBuf};

use crate::backup;
use crate::i18n::{self, DataMessage, Language, Message, SheetMessage, SpreadsheetMessage};
use rfd::{FileDialog, MessageButtons, MessageDialog, MessageDialogResult, MessageLevel};

/// The kinds of file the app opens and saves, each with its own filter in the file dialogs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileKind {
    Database,
    Spreadsheet,
    Backup,
    Picture,
}

impl FileKind {
    /// What the open dialog shows: a spreadsheet may come from any program calamine reads, and a
    /// backup can also be a plain database file (a copied `avalio-lite.db`): it is the same format.
    pub fn open_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Spreadsheet => &["xlsx", "xls", "ods"],
            Self::Database => self.save_extensions(),
            Self::Backup => &[backup::EXTENSION, "db"],
            Self::Picture => &["jpg", "jpeg", "png", "webp"],
        }
    }

    /// What the save dialog offers, the first being what the app writes.
    pub fn save_extensions(self) -> &'static [&'static str] {
        match self {
            Self::Database => &["db"],
            Self::Spreadsheet => &["xlsx"],
            Self::Backup => &[backup::EXTENSION],
            // pictures are only opened
            Self::Picture => &[],
        }
    }

    /// The filter's name in the file dialogs, such as "Excel workbook".
    pub fn filter_name(self, language: Language) -> String {
        let message = match self {
            Self::Database => Message::Data(DataMessage::DatabaseFileType),
            Self::Spreadsheet => Message::Spreadsheet(SpreadsheetMessage::SpreadsheetFileType),
            Self::Backup => Message::Data(DataMessage::BackupFileType),
            Self::Picture => Message::Sheet(SheetMessage::PictureFileType),
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

    /// The question is in the app's language; the Yes and No buttons are Windows' own, in Windows'
    /// language, since the system dialog can't be given other words for them (a Galician app on an
    /// English Windows asks in Galician and answers "Yes"/"No"). A dialog of the app's own would
    /// fix that, at the cost of the system's look and its screen-reader support.
    fn confirm(&self, question: &str) -> bool {
        message_box(MessageLevel::Warning, question, MessageButtons::YesNo).show() == MessageDialogResult::Yes
    }

    fn show_fatal(&self, message: &str) {
        message_box(MessageLevel::Error, message, MessageButtons::Ok).show();
    }

    fn open_link(&self, url: &str) -> Result<(), String> {
        shell_outcome(shell_open(url))
    }
}

/// One of the app's message boxes, not yet shown.
fn message_box(level: MessageLevel, text: &str, buttons: MessageButtons) -> MessageDialog {
    MessageDialog::new()
        .set_level(level)
        .set_title(crate::about::DISPLAY_NAME)
        .set_description(text)
        .set_buttons(buttons)
}

/// Asks Windows to open `target` with its default program; returns ShellExecuteW's own answer.
fn shell_open(target: &str) -> isize {
    use windows_sys::Win32::UI::Shell::ShellExecuteW;
    use windows_sys::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;
    let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (verb, target) = (wide("open"), wide(target));
    let (none, null) = (std::ptr::null_mut(), std::ptr::null());
    // SAFETY: two NUL-terminated UTF-16 strings that outlive the call; no owner window.
    unsafe { ShellExecuteW(none, verb.as_ptr(), target.as_ptr(), null, null, SW_SHOWNORMAL) as isize }
}

/// ShellExecuteW reports success with a value above 32, and an error code otherwise.
fn shell_outcome(code: isize) -> Result<(), String> {
    if code > 32 { Ok(()) } else { Err(format!("ShellExecuteW failed ({code})")) }
}

/// A file dialog showing only files of `kind` with one of `extensions`.
fn filtered(kind: FileKind, extensions: &[&str]) -> FileDialog {
    FileDialog::new().add_filter(kind.filter_name(i18n::current()), extensions)
}

/// Starts the dialog in `start`, unless that folder is gone (Windows then picks the folder).
fn opened_at(dialog: FileDialog, start: &Path) -> FileDialog {
    if start.is_dir() { dialog.set_directory(start) } else { dialog }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_link_windows_cannot_open_is_an_error_with_its_code() {
        // a file that isn't there: Windows answers with an error code and shows nothing
        let missing = std::env::temp_dir().join("avalio-lite-no-such-folder").join("nothing.avalio-lite-test");
        assert!(!missing.exists());
        let refused = NativeDialogs.open_link(&missing.display().to_string()).unwrap_err();
        assert!(refused.starts_with("ShellExecuteW failed ("), "{refused}");
    }

    #[test]
    fn only_an_answer_above_32_means_the_link_opened() {
        assert_eq!(shell_outcome(33), Ok(()));
        assert_eq!(shell_outcome(32), Err("ShellExecuteW failed (32)".to_owned()));
        assert_eq!(shell_outcome(2), Err("ShellExecuteW failed (2)".to_owned()));
    }

    #[test]
    fn the_message_boxes_carry_the_apps_name_the_text_and_their_buttons() {
        let question = format!("{:?}", message_box(MessageLevel::Warning, "Delete 1A?", MessageButtons::YesNo));
        for part in ["avalio lite", "Delete 1A?", "Warning", "YesNo"] {
            assert!(question.contains(part), "{part} in {question}");
        }
        let fatal = format!("{:?}", message_box(MessageLevel::Error, "It broke", MessageButtons::Ok));
        for part in ["avalio lite", "It broke", "Error", "Ok"] {
            assert!(fatal.contains(part), "{part} in {fatal}");
        }
    }

    #[test]
    fn a_file_dialog_shows_the_kinds_files_and_starts_in_a_folder_that_exists() {
        let dialog = format!("{:?}", filtered(FileKind::Spreadsheet, FileKind::Spreadsheet.open_extensions()));
        for part in ["xlsx", "xls", "ods", &FileKind::Spreadsheet.filter_name(i18n::current())] {
            assert!(dialog.contains(part), "{part} in {dialog}");
        }
        let folder = std::env::temp_dir();
        let name = folder.file_name().unwrap().to_string_lossy().into_owned();
        assert!(format!("{:?}", opened_at(FileDialog::new(), &folder)).contains(&name));
        let gone = folder.join("avalio-lite-no-such-folder");
        assert!(!format!("{:?}", opened_at(FileDialog::new(), &gone)).contains("avalio-lite-no-such-folder"));
    }
}
