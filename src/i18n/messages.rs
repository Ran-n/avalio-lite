// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.721797
//+ Revised: 	2026/11/12 14:39:53.068972
// -----------------------------------------------------------------------------

use super::{Language, english, galician};
use chrono::NaiveDate;

use crate::licence::{Notice, Reminder, SUPPORT_EMAIL};

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
    /// the database was moved to `path`, but its original at `old` couldn't be deleted
    DatabaseMovedOldKept {
        path: &'a str,
        old: &'a str,
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
    /// rows of the file with marks or a surname but no first name, skipped
    ImportUnnamedRows {
        count: usize,
    },
    /// how a left-out column with no header is named, by its letter
    UntitledColumn {
        letter: &'a str,
    },
    LicenceAskForKey,
    LicenceInvalidKey,
    LicenceInUseElsewhere {
        email: &'a str,
    },
    LicenceRevoked {
        email: &'a str,
    },
    /// `date` is already formatted (day/month/year); `None` when the server didn't say
    LicenceExpired {
        date: Option<&'a str>,
        email: &'a str,
    },
    LicenceWithdrawn {
        email: &'a str,
    },
    LicenceNeedsInternet,
    LicenceWait {
        minutes: u64,
    },
    LicenceUnreachable,
    /// the licence couldn't be written to this computer (the details go to the log)
    LicenceNotSaved,
    /// work meant to run in the background (a check or a download) couldn't be started
    BackgroundFailed,
    /// on the sheet: 30 days offline run out on `date` (formatted)
    LicenceConnectBy {
        date: &'a str,
    },
    /// on the sheet: a licence for a set period ends on `date` (formatted)
    LicenceEndsOn {
        date: &'a str,
        email: &'a str,
    },
    /// the Settings card: how long the licence lasts
    LicencePermanent,
    LicenceUntil {
        date: &'a str,
    },
    LicenceOfflineUntil {
        date: &'a str,
    },
    /// Check now found the licence valid
    LicenceConfirmed,
    /// the update bar: a newer version is out
    UpdateAvailable {
        version: &'a str,
    },
    /// Check now found no newer version than `version`, this one
    UpToDate {
        version: &'a str,
    },
    UpdateCheckFailed {
        problem: UpdateProblem,
    },
    /// a web page (a release's) couldn't be opened in the browser; `url` is its address
    LinkFailed {
        url: &'a str,
    },
    /// the download isn't signed with the release key
    UpdateNotVerified,
    UpdateDownloadFailed {
        problem: UpdateProblem,
    },
    /// the new portable exe couldn't take the old one's place; the old one is where it was
    UpdateNotPlaced,
    /// the new portable exe couldn't take the old one's place, and the old one couldn't be put
    /// back either: it is at `path` (`<exe>.old`)
    UpdateLeftAside {
        path: &'a str,
    },
    /// a portable copy in a folder that can't be written to
    UpdateFolderReadOnly,
    /// the release lacks the file this copy needs
    UpdateFileMissing {
        file: &'a str,
    },
    /// shown once, after the app was updated
    UpdatedTo {
        version: &'a str,
    },
    /// the installer or new exe couldn't be started as the app closed
    UpdateNotStarted,
}

/// Why looking for or downloading an update failed, as far as the user can act on it; the
/// technical details go to the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UpdateProblem {
    /// no connection, or GitHub couldn't be reached
    Offline,
    /// GitHub asks to wait before asking again
    RateLimited,
    /// GitHub's answer wasn't a release the app understands
    BadAnswer,
    /// the download couldn't be written to this computer
    Disk,
}

/// A day as the licence texts show it, in both languages: 30/06/2027.
pub fn licence_day(day: NaiveDate) -> String {
    day.format("%d/%m/%Y").to_string()
}

/// The sheet's warning for `reminder`.
pub fn licence_reminder(language: Language, reminder: Reminder) -> String {
    match reminder {
        Reminder::ConnectBy(day) => text(language, &Message::LicenceConnectBy { date: &licence_day(day) }),
        Reminder::EndsOn(day) => {
            text(language, &Message::LicenceEndsOn { date: &licence_day(day), email: SUPPORT_EMAIL })
        }
    }
}

/// What the licence screen says for `notice`, with the support address and dates filled in.
pub fn licence_notice(language: Language, notice: &Notice) -> String {
    let email = SUPPORT_EMAIL;
    let date = match notice {
        Notice::Expired(Some(day)) => licence_day(*day),
        _ => String::new(),
    };
    let message = match notice {
        Notice::AskForKey => Message::LicenceAskForKey,
        Notice::InvalidKey => Message::LicenceInvalidKey,
        Notice::InUseElsewhere => Message::LicenceInUseElsewhere { email },
        Notice::Revoked => Message::LicenceRevoked { email },
        Notice::Expired(day) => Message::LicenceExpired { date: day.map(|_| date.as_str()), email },
        Notice::Withdrawn => Message::LicenceWithdrawn { email },
        Notice::NeedsInternet => Message::LicenceNeedsInternet,
        Notice::Wait { minutes } => Message::LicenceWait { minutes: *minutes },
        Notice::Unreachable => Message::LicenceUnreachable,
        Notice::NotSaved => Message::LicenceNotSaved,
    };
    text(language, &message)
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
