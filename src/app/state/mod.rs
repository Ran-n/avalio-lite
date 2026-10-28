// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.365284
//+ Revised: 	2026/10/28 22:23:17.072660
// -----------------------------------------------------------------------------

mod protection;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::config::{AppConfig, ConfigStore, DATABASE_FILE_NAME, LOG_FILE_NAME, Preferences, ThemeChoice};
use crate::db;
use crate::error::{AppError, Folder, Result};
use crate::i18n::{self, Language};
use crate::lock::Pin;
use crate::logging::FileLogger;
use crate::storage;

/// Why the first-run screen is being shown.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SetupReason {
    FirstRun,
    DatabaseMissing(PathBuf),
    ConfigUnreadable(String),
    DatabaseUnusable(String),
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

/// Owns the settings, the open database and the log file. All data-location
/// changes go through here so the settings file never points at something
/// that doesn't exist.
pub struct AppState {
    store: ConfigStore,
    logger: &'static FileLogger,
    config: Option<AppConfig>,
    connection: Option<Connection>,
    /// The PIN the open database is encrypted with; `None` when it isn't encrypted or is locked.
    /// Wiped from memory as soon as it is dropped.
    key: Option<Pin>,
}

impl AppState {
    pub fn new(store: ConfigStore, logger: &'static FileLogger) -> Self {
        Self { store, logger, config: None, connection: None, key: None }
    }

    pub fn config(&self) -> Option<&AppConfig> {
        self.config.as_ref()
    }

    pub fn connection(&self) -> Option<&Connection> {
        self.connection.as_ref()
    }

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
        if let Err(error) = self.logger.open(&config.log_dir) {
            log::warn!("could not open the log folder, logging to stderr: {error}");
        }
        if !config.database_path.is_file() {
            log::warn!("database missing at {}", config.database_path.display());
            return Startup::NeedsSetup(SetupReason::DatabaseMissing(config.database_path));
        }
        match db::is_encrypted(&config.database_path) {
            Ok(true) => {
                self.config = Some(config);
                return Startup::Locked;
            }
            Ok(false) => {}
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
        };
        self.choose(config)
    }

    /// Switches to the database in `config`: opens it, or, when it is encrypted, closes the
    /// current one and waits for its PIN.
    fn choose(&mut self, config: AppConfig) -> Result<Opened> {
        if db::is_encrypted(&config.database_path)? {
            self.reopen_logger(&config.log_dir);
            self.save_config(config)?;
            self.close_connection();
            return Ok(Opened::Locked);
        }
        let connection = db::open(&config.database_path, None)?;
        self.activate(config, connection)?;
        Ok(Opened::Ready)
    }

    /// The current view preferences; defaults until a database has been chosen.
    pub fn preferences(&self) -> Preferences {
        self.config.as_ref().map(|config| config.preferences).unwrap_or_default()
    }

    pub fn set_show_column_settings(&mut self, show: bool) -> Result<()> {
        let mut config = self.config.clone().ok_or(AppError::NoDatabaseOpen)?;
        config.preferences.show_column_settings = show;
        self.save_config(config)
    }

    pub fn set_theme(&mut self, theme: ThemeChoice) -> Result<()> {
        let mut config = self.config.clone().ok_or(AppError::NoDatabaseOpen)?;
        config.preferences.theme = Some(theme);
        self.save_config(config)
    }

    pub fn set_language(&mut self, language: Language) -> Result<()> {
        let mut config = self.config.clone().ok_or(AppError::NoDatabaseOpen)?;
        config.preferences.language = Some(language);
        self.save_config(config)
    }

    /// Copies the database to `destination_dir`, verifies the copy, switches to it,
    /// and only then deletes the original.
    pub fn move_database(&mut self, destination_dir: &Path) -> Result<PathBuf> {
        let current = self.config.clone().ok_or(AppError::NoDatabaseOpen)?;
        let destination = destination_dir.join(DATABASE_FILE_NAME);
        if storage::same_location(&current.database_path, &destination) {
            return Ok(destination);
        }
        if destination.exists() {
            return Err(AppError::AlreadyExists(destination));
        }
        let connection = self.connection.as_ref().ok_or(AppError::NoDatabaseOpen)?;
        let pin = self.key.clone();
        let key = pin.as_ref().map(Pin::as_str);
        let switched = db::export(connection, &destination, key)
            .and_then(|()| db::verify(&destination, key))
            .and_then(|()| db::open(&destination, key))
            .and_then(|new_connection| {
                self.save_config(AppConfig { database_path: destination.clone(), ..current.clone() })?;
                Ok(new_connection)
            });
        let new_connection = match switched {
            Ok(new_connection) => new_connection,
            Err(error) => {
                discard_copy(&destination);
                return Err(error);
            }
        };
        if let Err(error) = self.replace_connection(new_connection) {
            log::warn!("previous database did not close cleanly: {error}");
        }
        db::remove_files(&current.database_path)?;
        log::info!("moved database from {} to {}", current.database_path.display(), destination.display());
        Ok(destination)
    }

    pub fn move_logs(&mut self, destination_dir: &Path) -> Result<()> {
        let current = self.config.clone().ok_or(AppError::NoDatabaseOpen)?;
        if storage::same_location(&current.log_dir, destination_dir) {
            return Ok(());
        }
        let source = current.log_dir.join(LOG_FILE_NAME);
        self.logger.close();
        let moved =
            if source.is_file() { storage::move_file(&source, &destination_dir.join(LOG_FILE_NAME)) } else { Ok(()) };
        if let Err(error) = moved {
            self.reopen_logger(&current.log_dir);
            return Err(error);
        }
        self.reopen_logger(destination_dir);
        self.save_config(AppConfig { log_dir: destination_dir.to_path_buf(), ..current.clone() })?;
        log::info!("moved logs from {} to {}", current.log_dir.display(), destination_dir.display());
        Ok(())
    }

    /// Closes the database and the log file; called on every quit path.
    pub fn shutdown(&mut self) {
        self.close_connection();
        log::info!("shut down");
        self.logger.close();
    }

    fn activate(&mut self, config: AppConfig, connection: Connection) -> Result<()> {
        self.reopen_logger(&config.log_dir);
        self.save_config(config)?;
        self.key = None;
        self.replace_connection(connection)
    }

    /// Closes the database (and forgets its key), logging rather than failing: used on
    /// quit, on lock, and when switching away from a database.
    fn close_connection(&mut self) {
        self.key = None;
        if let Some(connection) = self.connection.take()
            && let Err(error) = db::close(connection)
        {
            log::error!("database did not close cleanly: {error}");
        }
    }

    fn save_config(&mut self, config: AppConfig) -> Result<()> {
        self.store.save(&config)?;
        self.config = Some(config);
        Ok(())
    }

    fn replace_connection(&mut self, connection: Connection) -> Result<()> {
        match self.connection.replace(connection) {
            Some(previous) => db::close(previous),
            None => Ok(()),
        }
    }

    fn reopen_logger(&self, log_dir: &Path) {
        self.logger.close();
        if let Err(error) = self.logger.open(log_dir) {
            log::error!("could not open the log file in {}: {error}", log_dir.display());
        }
    }
}

fn discard_copy(path: &Path) {
    if let Err(error) = db::remove_files(path) {
        log::error!("could not remove the incomplete copy at {}: {error}", path.display());
    }
}

fn require_path(path: &Path, purpose: Folder) -> Result<()> {
    if path.as_os_str().is_empty() {
        return Err(AppError::EmptyPath(purpose));
    }
    Ok(())
}
