// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.411982
//+ Revised: 	2026/11/25 11:43:15.065149
// -----------------------------------------------------------------------------

//! The error texts, grouped as the translations group them (database, files and folders, fields and
//! PIN, keyboard shortcuts), each one checked in both languages.

use avalio_lite::error::{AppError, Folder, NumberField};
use avalio_lite::i18n::{LANGUAGES, Language, error_message};
use avalio_lite::shortcuts::{Action, ProblemKind, ShortcutError, ShortcutProblem};

use super::assert_errors_translated;

/// What the system said, which must never reach the user.
const RAW: &str = "RAW-SYSTEM-TEXT";

/// The database errors.
pub fn every_database_error() -> Vec<AppError> {
    vec![
        AppError::Database(rusqlite::Error::InvalidQuery),
        AppError::NewerSchema { found: 3, supported: 1 },
        AppError::IntegrityCheckFailed("D:\\x".into()),
        AppError::NotADatabase("D:\\x".into()),
        AppError::OpenDatabaseAsBackup("D:\\x".into()),
        AppError::KeptAside { kept: "D:\\x".into(), path: "D:\\x".into() },
        AppError::NoDatabaseOpen,
    ]
}

/// The errors about files, folders, the settings file and the spreadsheets.
pub fn every_file_error() -> Vec<AppError> {
    let parse_error = toml::from_str::<toml::Table>("not = [valid").unwrap_err();
    vec![
        AppError::Io { path: "D:\\x".into(), source: std::io::Error::other("denied") },
        AppError::ConfigParse { path: "D:\\x".into(), source: parse_error },
        AppError::ConfigSerialize(serde::ser::Error::custom("denied")),
        AppError::NoConfigDir,
        AppError::EmptyPath(Folder::Database),
        AppError::EmptyPath(Folder::Logs),
        AppError::RelativePath(Folder::Database),
        AppError::RelativePath(Folder::Logs),
        AppError::NotFound("D:\\x".into()),
        AppError::AlreadyExists("D:\\x".into()),
        AppError::SpreadsheetWrite { path: "D:\\x".into(), reason: "denied".into() },
        AppError::SpreadsheetRead { path: "D:\\x".into(), reason: "not a workbook".into() },
        AppError::PictureRead { path: "D:\\x".into(), reason: "not a picture".into() },
        AppError::NoPupilsInFile("D:\\x".into()),
        AppError::DuplicatePupilInFile("Ana".into()),
    ]
}

/// The errors about a value typed into a field, and the PIN.
pub fn every_field_error() -> Vec<AppError> {
    vec![
        AppError::EmptyName,
        AppError::DuplicateClassName("1A".into()),
        AppError::InvalidNumber { field: NumberField::Weight, value: "x".into() },
        AppError::InvalidNumber { field: NumberField::MaxScore, value: "x".into() },
        AppError::InvalidNumber { field: NumberField::Decimals, value: "x".into() },
        AppError::NotEditable,
        AppError::InvalidRoundSource,
        AppError::InvalidPin,
        AppError::PinMismatch,
        AppError::WrongPin,
        AppError::PinWait { seconds: 3 },
    ]
}

/// The errors about keyboard shortcuts: a bad entry in the settings file, or a key that can't be used.
pub fn every_shortcut_error() -> Vec<AppError> {
    vec![
        AppError::InvalidShortcuts(vec![ShortcutProblem {
            entry: "lock = \"x\"".into(),
            kind: ProblemKind::BadKeys(ShortcutError::UnknownKey),
        }]),
        AppError::Shortcut(ShortcutError::UnknownKey),
        AppError::Shortcut(ShortcutError::NeedsCtrlOrAlt),
        AppError::Shortcut(ShortcutError::CtrlAlt),
        AppError::Shortcut(ShortcutError::EditingKey('C')),
        AppError::Shortcut(ShortcutError::Clash { keys: "Ctrl+H".parse().unwrap(), taken_by: Action::Lock }),
    ]
}

/// The group an error is translated under. The match has no wildcard, so a new `AppError` variant
/// won't compile until it is named here and in one of the group lists above.
fn error_group(error: &AppError) -> &'static str {
    match error {
        AppError::Database(_)
        | AppError::NewerSchema { .. }
        | AppError::IntegrityCheckFailed(_)
        | AppError::NotADatabase(_)
        | AppError::OpenDatabaseAsBackup(_)
        | AppError::KeptAside { .. }
        | AppError::NoDatabaseOpen => "database",
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
        | AppError::DuplicatePupilInFile(_) => "files",
        AppError::EmptyName
        | AppError::DuplicateClassName(_)
        | AppError::InvalidNumber { .. }
        | AppError::NotEditable
        | AppError::InvalidRoundSource
        | AppError::InvalidPin
        | AppError::PinMismatch
        | AppError::WrongPin
        | AppError::PinWait { .. } => "fields",
        AppError::InvalidShortcuts(_) | AppError::Shortcut(_) => "shortcuts",
    }
}

#[test]
fn every_error_is_listed_under_its_group() {
    let groups = [
        ("database", every_database_error()),
        ("files", every_file_error()),
        ("fields", every_field_error()),
        ("shortcuts", every_shortcut_error()),
    ];
    for (group, errors) in &groups {
        for error in errors {
            assert_eq!(error_group(error), *group, "{error:?}");
        }
    }
}

#[test]
fn every_database_error_is_translated_and_keeps_its_details() {
    assert_errors_translated(&every_database_error());
}

#[test]
fn every_file_error_is_translated_and_keeps_its_details() {
    assert_errors_translated(&every_file_error());
}

#[test]
fn every_field_error_is_translated_and_keeps_its_details() {
    assert_errors_translated(&every_field_error());
}

#[test]
fn every_shortcut_error_is_translated_and_keeps_its_details() {
    assert_errors_translated(&every_shortcut_error());
}

/// Errors from the system, the database and the settings file, as the code gets them.
fn technical_errors() -> Vec<AppError> {
    use std::io::{Error, ErrorKind};
    let path = || std::path::PathBuf::from("D:\\x");
    let io = |kind| AppError::Io { path: path(), source: Error::new(kind, RAW) };
    let sqlite = |code| {
        AppError::Database(rusqlite::Error::SqliteFailure(rusqlite::ffi::Error::new(code), Some(RAW.to_owned())))
    };
    let toml_error = toml::from_str::<toml::Value>("not = [valid").unwrap_err();
    vec![
        io(ErrorKind::NotFound),
        io(ErrorKind::PermissionDenied),
        io(ErrorKind::StorageFull),
        io(ErrorKind::Other),
        AppError::Io { path: path(), source: Error::from_raw_os_error(32) },
        sqlite(5),
        sqlite(6),
        sqlite(26),
        sqlite(11),
        sqlite(13),
        sqlite(1),
        AppError::ConfigParse { path: path(), source: toml_error },
        AppError::SpreadsheetWrite { path: path(), reason: RAW.into() },
        AppError::SpreadsheetRead { path: path(), reason: RAW.into() },
        AppError::SpreadsheetRead { path: path(), reason: "the reader panicked".into() },
        AppError::PictureRead { path: path(), reason: RAW.into() },
    ]
}

/// The system's own error text (English, technical) never reaches the user, in either language;
/// what went wrong is worded by its kind, and the path stays.
#[test]
fn regression_technical_error_text_is_replaced_by_friendly_sentences() {
    let errors = technical_errors();
    for error in &errors {
        for language in LANGUAGES {
            let shown = error_message(language, error);
            for technical in [RAW, "os error", "SQLite", "TOML", "expected", "panicked", "code:"] {
                assert!(!shown.contains(technical), "{technical:?} reaches the user in {language:?}: {shown}");
            }
        }
    }
    let english = |error: &AppError| error_message(Language::English, error);
    assert!(english(&errors[0]).starts_with("Couldn't find D:\\x"));
    assert!(english(&errors[1]).contains("isn't allowed"));
    assert!(english(&errors[4]).contains("another program"));
    assert!(english(&errors[5]).contains("another program"));
    assert!(english(&errors[7]).contains("damaged"));
    assert!(english(&errors[11]).contains("damaged") && english(&errors[11]).contains("D:\\x"));
    assert!(english(&errors[14]).contains("isn't a workbook"));
}
