// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 22:29:57.490512
//+ Revised: 	2026/11/03 10:00:17.108352
// -----------------------------------------------------------------------------

//! Starting up and choosing a database: the settings file read at start, the first-run choice,
//! and the check that an existing file is an avalio-lite database before anything opens it.

use std::fs;
use std::path::{Path, PathBuf};

use super::AppState;
use crate::error::{AppError, Folder, Result, io_at};
use crate::i18n;
use crate::platform::config::{AppConfig, DATABASE_FILE_NAME};
use crate::platform::db;

/// SQLite's plain header is 16 bytes; a shorter file can't be a database of any kind.
const MIN_DATABASE_BYTES: u64 = 16;

/// Why the first-run screen is being shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupReason {
    FirstRun,
    DatabaseMissing(PathBuf),
    ConfigUnreadable(String),
    DatabaseUnusable(String),
    /// The user left the lock screen ("Forgot the PIN?"); the locked database at this path stays.
    LeftLocked(PathBuf),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Startup {
    Ready,
    /// The database is encrypted: the lock screen must get the PIN before it can be opened.
    Locked,
    NeedsSetup(SetupReason),
}

/// The result of choosing a database: open, or encrypted and waiting for its PIN.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opened {
    Ready,
    Locked,
}

impl AppState {
    pub fn start(&mut self) -> Startup {
        let config = match self.store.load() {
            Ok(Some(config)) => config,
            Ok(None) => return Startup::NeedsSetup(SetupReason::FirstRun),
            Err(error) => {
                log::warn!("settings file unreadable: {error}");
                return Startup::NeedsSetup(SetupReason::ConfigUnreadable(i18n::error_message(
                    i18n::current(),
                    &error,
                )));
            }
        };
        self.adopt_shortcuts(&config);
        self.read = Some(config.clone());
        if let Err(error) = self.logger.open(&config.log_dir) {
            log::warn!("could not open the log folder, logging to stderr: {error}");
        }
        if !config.database_path.is_file() {
            log::warn!("database missing at {}", config.database_path.display());
            return Startup::NeedsSetup(SetupReason::DatabaseMissing(config.database_path));
        }
        match existing_kind(&config.database_path) {
            Ok(FileKind::Encrypted) => {
                self.config = Some(config);
                return Startup::Locked;
            }
            Ok(FileKind::Plain) => {}
            Err(error) => {
                log::error!("could not read the database: {error}");
                return Startup::NeedsSetup(SetupReason::DatabaseUnusable(i18n::error_message(
                    i18n::current(),
                    &error,
                )));
            }
        }
        match db::open(&config.database_path, None) {
            Ok(connection) => {
                self.connection = Some(connection);
                self.config = Some(config);
                Startup::Ready
            }
            Err(error) => {
                log::error!("could not open the database: {error}");
                Startup::NeedsSetup(SetupReason::DatabaseUnusable(i18n::error_message(i18n::current(), &error)))
            }
        }
    }

    /// First-run choice: opens the database in `database_dir`, creating it if the folder has none.
    pub fn create_database(&mut self, database_dir: &Path, log_dir: &Path) -> Result<Opened> {
        require_path(database_dir, Folder::Database)?;
        require_path(log_dir, Folder::Logs)?;
        let config = AppConfig {
            database_path: database_dir.join(DATABASE_FILE_NAME),
            log_dir: log_dir.to_path_buf(),
            preferences: self.preferences(),
            shortcuts: self.shortcut_overrides(),
        };
        self.choose(config)
    }

    pub fn open_existing(&mut self, database_path: &Path, log_dir: &Path) -> Result<Opened> {
        require_path(log_dir, Folder::Logs)?;
        if !database_path.is_file() {
            return Err(AppError::NotFound(database_path.to_path_buf()));
        }
        let config = AppConfig {
            database_path: database_path.to_path_buf(),
            log_dir: log_dir.to_path_buf(),
            preferences: self.preferences(),
            shortcuts: self.shortcut_overrides(),
        };
        self.choose(config)
    }

    /// Switches to the database in `config`: opens it, or, when it is encrypted, closes the
    /// current one and waits for its PIN. A file that is already there must be an avalio-lite
    /// database; it is checked before anything changes, so a stray file leaves the app as it was.
    fn choose(&mut self, config: AppConfig) -> Result<Opened> {
        let kind = if config.database_path.exists() { Some(existing_kind(&config.database_path)?) } else { None };
        self.adopt_shortcuts(&config);
        if kind == Some(FileKind::Encrypted) {
            let log_dir = config.log_dir.clone();
            self.save_config(config)?;
            self.reopen_logger(&log_dir);
            self.close_connection();
            return Ok(Opened::Locked);
        }
        let connection = db::open(&config.database_path, None)?;
        self.activate(config, connection)?;
        Ok(Opened::Ready)
    }
}

/// What an existing database file is, once it is known to be an avalio-lite database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileKind {
    Plain,
    Encrypted,
}

/// Checks that the existing file at `path` can be an avalio-lite database before anything opens
/// it, since opening runs the migrations and would turn any empty or foreign file into one.
/// A plain file needs avalio's schema; an encrypted one can't be read without its PIN, so it
/// must at least be whole pages (as in `backup`). Empty and tiny files are never databases.
fn existing_kind(path: &Path) -> Result<FileKind> {
    let not_a_database = || AppError::NotADatabase(path.to_path_buf());
    let length = fs::metadata(path).map_err(io_at(path))?.len();
    if length < MIN_DATABASE_BYTES {
        return Err(not_a_database());
    }
    if db::is_encrypted(path)? {
        return if db::has_whole_pages(path)? { Ok(FileKind::Encrypted) } else { Err(not_a_database()) };
    }
    match db::stored_version(path, None) {
        Ok(0) | Err(AppError::WrongPin) => Err(not_a_database()),
        Ok(_) => Ok(FileKind::Plain),
        Err(error) => Err(error),
    }
}

fn require_path(path: &Path, purpose: Folder) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(AppError::EmptyPath(purpose));
    }
    Ok(())
}
