// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.190729
//+ Revised: 	2026/10/28 22:23:16.879338
// -----------------------------------------------------------------------------

use std::io;
use std::path::{Path, PathBuf};

pub type Result<T> = std::result::Result<T, AppError>;

/// Which folder an `EmptyPath` error is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Folder {
    Database,
    Logs,
}

/// Which setting an `InvalidNumber` error is about.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NumberField {
    Weight,
    MaxScore,
    Decimals,
}

/// Display text is English and meant for the log; the UI shows `i18n::error_message` instead.
#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("could not access {path}: {source}")]
    Io { path: PathBuf, source: io::Error },

    #[error("database error: {0}")]
    Database(#[from] rusqlite::Error),

    #[error("could not read the settings file {path}: {source}")]
    ConfigParse { path: PathBuf, source: toml::de::Error },

    #[error("could not write the settings file: {0}")]
    ConfigSerialize(#[from] toml::ser::Error),

    #[error("could not find the user settings folder")]
    NoConfigDir,

    #[error("no folder was chosen for the {0:?}")]
    EmptyPath(Folder),

    #[error("{0} does not exist")]
    NotFound(PathBuf),

    #[error("{0} already exists")]
    AlreadyExists(PathBuf),

    #[error(
        "this database was made by a newer version of avalio-lite \
         (schema {found}, this version supports up to {supported})"
    )]
    NewerSchema { found: usize, supported: usize },

    #[error("the database at {0} failed its integrity check")]
    IntegrityCheckFailed(PathBuf),

    #[error("no database is open")]
    NoDatabaseOpen,

    #[error("a name can't be empty")]
    EmptyName,

    #[error("there is already a class called \"{0}\"")]
    DuplicateClassName(String),

    #[error("\"{value}\" is not a valid {field:?}")]
    InvalidNumber { field: NumberField, value: String },

    #[error("that value can't be edited")]
    NotEditable,

    #[error("a rounded column can only round a test, final grade or average column of the same class")]
    InvalidRoundSource,

    #[error("a PIN must be 4 to 128 characters, without control characters")]
    InvalidPin,

    #[error("the two PINs don't match")]
    PinMismatch,

    #[error("wrong PIN")]
    WrongPin,

    #[error("too many wrong PINs; wait {seconds} s")]
    PinWait { seconds: u64 },
}

/// Builds a `map_err` adapter that tags an I/O error with the path it happened on.
pub fn io_at(path: &Path) -> impl FnOnce(io::Error) -> AppError + '_ {
    move |source| AppError::Io { path: path.to_path_buf(), source }
}
