// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.365284
//+ Revised: 	2026/11/12 14:39:54.897358
// -----------------------------------------------------------------------------

//! The app's core: the settings, the open database and its PIN, data moves and backups. No UI
//! code lives here; the UI layer wires it to the window.

mod backups;
mod locations;
mod preferences;
mod protection;
mod startup;

pub use locations::Moved;
pub use startup::{Opened, SetupReason, Startup};

use std::path::Path;

use rusqlite::Connection;

use crate::error::{AppError, Result};
use crate::lock::Pin;
use crate::platform::config::{AppConfig, ConfigStore};
use crate::platform::db;
use crate::platform::logging::FileLogger;
use crate::shortcuts::{ShortcutProblem, Shortcuts};

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
    /// The settings file as last read, even when it pointed at a database that is gone: its
    /// preferences and shortcuts carry over to whatever database is chosen next.
    read: Option<AppConfig>,
    shortcuts: Shortcuts,
    /// Shortcut entries from the settings file that were ignored, until they have been shown.
    shortcut_problems: Vec<ShortcutProblem>,
}

impl AppState {
    pub fn new(store: ConfigStore, logger: &'static FileLogger) -> Self {
        Self {
            store,
            logger,
            config: None,
            connection: None,
            key: None,
            read: None,
            shortcuts: Shortcuts::defaults(),
            shortcut_problems: Vec::new(),
        }
    }

    pub fn config(&self) -> Option<&AppConfig> {
        self.config.as_ref()
    }

    pub fn connection(&self) -> Option<&Connection> {
        self.connection.as_ref()
    }

    /// Closes the database and the log file; called on every quit path.
    pub fn shutdown(&mut self) {
        self.close_connection();
        log::info!("shut down");
        self.logger.close();
    }

    fn activate(&mut self, config: AppConfig, connection: Connection) -> Result<()> {
        let log_dir = config.log_dir.clone();
        self.save_config(config)?;
        self.reopen_logger(&log_dir);
        self.key = None;
        self.replace_connection(connection);
        Ok(())
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

    /// The chosen database's settings; `NoDatabaseOpen` before one is chosen.
    fn open_config(&self) -> Result<&AppConfig> {
        self.config.as_ref().ok_or(AppError::NoDatabaseOpen)
    }

    /// The open database; `NoDatabaseOpen` when none is open (or it is locked).
    fn open_connection(&self) -> Result<&Connection> {
        self.connection.as_ref().ok_or(AppError::NoDatabaseOpen)
    }

    /// Changes a copy of the settings and saves it; `NoDatabaseOpen` before a database is chosen.
    fn update_config(&mut self, change: impl FnOnce(&mut AppConfig)) -> Result<()> {
        let mut config = self.open_config()?.clone();
        change(&mut config);
        self.save_config(config)
    }

    fn save_config(&mut self, config: AppConfig) -> Result<()> {
        self.store.save(&config)?;
        self.config = Some(config);
        Ok(())
    }

    /// Switches to `connection`. The previous one is closed; a failure to close it is logged, not
    /// returned, since the switch has happened and the new database is the one in use.
    fn replace_connection(&mut self, connection: Connection) {
        if let Some(previous) = self.connection.replace(connection)
            && let Err(error) = db::close(previous)
        {
            log::error!("the previous database did not close cleanly: {error}");
        }
    }

    fn reopen_logger(&self, log_dir: &Path) {
        self.logger.close();
        if let Err(error) = self.logger.open(log_dir) {
            log::error!("could not open the log file in {}: {error}", log_dir.display());
        }
    }
}
