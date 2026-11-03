// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.615582
//+ Revised: 	2026/11/03 11:13:31.347188
// -----------------------------------------------------------------------------

use std::io;

use super::{Language, english, galician};
use crate::error::AppError;
use crate::shortcuts::ShortcutProblem;

/// The error as shown to the user, in their language. Paths are passed through as they are; the
/// system's own error text never is (it is English and technical): errors from files, the
/// database and the settings are worded by what went wrong, and their details go to the log.
pub fn error_message(language: Language, error: &AppError) -> String {
    match language {
        Language::English => english::error(error),
        Language::Galician => galician::error(error),
    }
}

/// The ignored entries as written in `config.toml`, the same in every language.
pub(super) fn entries(problems: &[ShortcutProblem]) -> String {
    problems.iter().map(|problem| problem.entry.as_str()).collect::<Vec<_>>().join(", ")
}

/// What went wrong with a file, as far as the user can act on it.
pub(super) enum IoProblem {
    NotFound,
    Denied,
    InUse,
    Full,
    Other,
}

impl IoProblem {
    pub(super) fn of(error: &io::Error) -> Self {
        // Windows: 32 and 33 are sharing and lock violations, 39 and 112 a full disk
        match (error.kind(), error.raw_os_error()) {
            (io::ErrorKind::NotFound, _) => Self::NotFound,
            (_, Some(32 | 33)) => Self::InUse,
            (io::ErrorKind::PermissionDenied, _) => Self::Denied,
            (io::ErrorKind::StorageFull, _) | (_, Some(39 | 112)) => Self::Full,
            _ => Self::Other,
        }
    }
}

/// What went wrong with the database, as far as the user can act on it.
pub(super) enum DatabaseProblem {
    Busy,
    Damaged,
    Full,
    Other,
}

impl DatabaseProblem {
    pub(super) fn of(error: &rusqlite::Error) -> Self {
        use rusqlite::ErrorCode;
        match error.sqlite_error_code() {
            Some(ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked) => Self::Busy,
            Some(ErrorCode::NotADatabase | ErrorCode::DatabaseCorrupt) => Self::Damaged,
            Some(ErrorCode::DiskFull) => Self::Full,
            _ => Self::Other,
        }
    }
}
