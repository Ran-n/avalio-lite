// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.721797
//+ Revised: 	2026/11/03 11:13:31.282513
// -----------------------------------------------------------------------------

use super::{Language, english, galician};

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
    /// a pupil who is in no class; deleting them is for good
    ConfirmDeletePupil {
        pupil: &'a str,
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
    /// the window couldn't be created or run; the reason is in the log, not in the dialog
    WindowFailed,
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
        Language::English => english::message(message),
        Language::Galician => galician::message(message),
    }
}

/// `singular` for exactly one, `plural` otherwise (none included), as English and Galician count.
pub(super) fn plural(count: usize, singular: &'static str, plural: &'static str) -> &'static str {
    if count == 1 { singular } else { plural }
}

/// "1 class", "4 classes": both languages here add the plural form for anything but one.
pub(super) fn counted(count: usize, one: &str, many: &str) -> String {
    format!("{count} {}", if count == 1 { one } else { many })
}

/// "1A → 1A (2), 1B → 1B (2)"
pub(super) fn renamings(pairs: &[(String, String)]) -> String {
    pairs.iter().map(|(from, to)| format!("{from} → {to}")).collect::<Vec<_>>().join(", ")
}
