// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 22:29:57.490512
//+ Revised: 	2026/11/23 11:49:20.381304
// -----------------------------------------------------------------------------

//! Starting up and choosing a database: the settings file read at start, the first-run choice,
//! and the check that an existing file is an avalio-lite database before anything opens it.

use std::path::{Path, PathBuf};

use super::AppState;
use super::file_kind::{FileKind, existing_kind};
use crate::error::{AppError, Folder, Result};
use crate::i18n;
use crate::platform::config::{AppConfig, DATABASE_FILE_NAME};
use crate::platform::db;

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
    /// Reads the settings and opens the database they name: `Ready`, `Locked` when it is encrypted,
    /// or `NeedsSetup` with the reason the first-run screen gives.
    pub fn start(&mut self) -> Startup {
        let config = match self.read_settings() {
            Ok(config) => config,
            Err(startup) => return startup,
        };
        match check_database(&config) {
            Ok(FileKind::Encrypted) => {
                self.config = Some(config);
                Startup::Locked
            }
            Ok(FileKind::Plain) => self.open_plain(config),
            Err(startup) => startup,
        }
    }

    /// The settings file read at start: its shortcuts are adopted and its log folder opened. `Err`
    /// is the answer `start` gives at once, when there are no usable settings.
    fn read_settings(&mut self) -> std::result::Result<AppConfig, Startup> {
        let config = match self.store.load() {
            Ok(Some(config)) => config,
            Ok(None) => return Err(Startup::NeedsSetup(SetupReason::FirstRun)),
            Err(error) => {
                log::warn!("settings file unreadable: {error}");
                let reason = i18n::error_message(i18n::current(), &error);
                return Err(Startup::NeedsSetup(SetupReason::ConfigUnreadable(reason)));
            }
        };
        self.adopt_shortcuts(&config);
        self.read = Some(config.clone());
        if let Err(error) = self.logger.open(&config.log_dir) {
            log::warn!("could not open the log folder, logging to stderr: {error}");
        }
        Ok(config)
    }

    /// Opens the plain database `config` names, upgrading it, and clears what an interrupted change
    /// left beside it. A failure is logged and the first-run screen is shown instead.
    fn open_plain(&mut self, config: AppConfig) -> Startup {
        match db::open_upgrading(&config.database_path, None) {
            Ok(connection) => {
                super::protection::clear_leftovers(&config.database_path);
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
            last_class: None,
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
            last_class: None,
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
        let connection = db::open_upgrading(&config.database_path, None)?;
        let path = config.database_path.clone();
        self.activate(config, connection)?;
        // a database made here just now (none was there) has no leftovers of its own: a
        // `.previous` beside it is what a crashed PIN change left of the one that went missing,
        // maybe the only good copy, so it stays
        if kind.is_some() {
            super::protection::clear_leftovers(&path);
        }
        Ok(Opened::Ready)
    }
}

/// Checks the database `config` names before anything opens it: the file must be there, and be an
/// avalio-lite database. `Err` is the answer `start` gives when it isn't.
fn check_database(config: &AppConfig) -> std::result::Result<FileKind, Startup> {
    if !config.database_path.is_file() {
        log::warn!("database missing at {}", config.database_path.display());
        return Err(Startup::NeedsSetup(SetupReason::DatabaseMissing(config.database_path.clone())));
    }
    existing_kind(&config.database_path).map_err(|error| {
        log::error!("could not read the database: {error}");
        Startup::NeedsSetup(SetupReason::DatabaseUnusable(i18n::error_message(i18n::current(), &error)))
    })
}

fn require_path(path: &Path, purpose: Folder) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(AppError::EmptyPath(purpose));
    }
    if !path.is_absolute() {
        return Err(AppError::RelativePath(purpose));
    }
    Ok(())
}
