// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 22:29:57.689707
//+ Revised: 	2026/11/12 14:39:54.781608
// -----------------------------------------------------------------------------

//! Moving the database or the log file to another folder, never leaving the settings pointing
//! at something that isn't there.

use std::path::{Path, PathBuf};

use super::AppState;
use crate::error::{AppError, Result};
use crate::lock::Pin;
use crate::platform::config::{AppConfig, DATABASE_FILE_NAME, LOG_FILE_NAME};
use crate::platform::db;
use crate::platform::logging;
use crate::platform::storage;

/// Where a moved database is now, and the original's path when it couldn't be deleted (in use by
/// another program, say): the move worked, but the old copy is still there for the user to delete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Moved {
    pub path: PathBuf,
    pub left_behind: Option<PathBuf>,
}

impl AppState {
    /// Copies the database to `destination_dir`, verifies the copy, switches to it,
    /// and only then deletes the original.
    pub fn move_database(&mut self, destination_dir: &Path) -> Result<Moved> {
        let current = self.open_config()?.clone();
        let destination = destination_dir.join(DATABASE_FILE_NAME);
        if storage::same_location(&current.database_path, &destination) {
            return Ok(Moved { path: destination, left_behind: None });
        }
        if destination.exists() {
            return Err(AppError::AlreadyExists(destination));
        }
        let connection = self.open_connection()?;
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
                db::discard(&destination);
                return Err(error);
            }
        };
        self.replace_connection(new_connection);
        log::info!("moved database from {} to {}", current.database_path.display(), destination.display());
        // the copy kept before the last update goes along with the database
        let kept = storage::with_suffix(&current.database_path, db::BEFORE_UPDATE_SUFFIX);
        if kept.is_file()
            && let Err(error) = storage::move_file(&kept, &storage::with_suffix(&destination, db::BEFORE_UPDATE_SUFFIX))
        {
            log::warn!("the copy kept before the last update stays at {}: {error}", kept.display());
        }
        // the app already uses the new file: a failed delete doesn't undo the move, it is said
        let left_behind = match db::remove_files(&current.database_path) {
            Ok(()) => None,
            Err(error) => {
                log::error!(
                    "the moved database's original {} couldn't be deleted: {error}",
                    current.database_path.display()
                );
                Some(current.database_path)
            }
        };
        Ok(Moved { path: destination, left_behind })
    }

    pub fn move_logs(&mut self, destination_dir: &Path) -> Result<()> {
        let current = self.open_config()?.clone();
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
        let destination_file = destination_dir.join(LOG_FILE_NAME);
        if let Err(error) = self.save_config(AppConfig { log_dir: destination_dir.to_path_buf(), ..current.clone() }) {
            // the settings still name the old folder, so the log goes back there
            if destination_file.is_file()
                && let Err(undo) = storage::move_file(&destination_file, &source)
            {
                log::error!("could not move the log back to {}: {undo}", source.display());
                self.reopen_logger(destination_dir);
            } else {
                self.reopen_logger(&current.log_dir);
            }
            return Err(error);
        }
        // the old log goes first: reopening sets a large log aside under the same `.1` name
        self.move_rotated_log(&current.log_dir, destination_dir);
        self.reopen_logger(destination_dir);
        log::info!("moved logs from {} to {}", current.log_dir.display(), destination_dir.display());
        Ok(())
    }

    /// Takes the old log (`.1`) along to the new folder. It is only history, so a failure is logged
    /// and leaves it where it was.
    fn move_rotated_log(&self, from: &Path, to: &Path) {
        let (old, moved) = (logging::rotated_path(from), logging::rotated_path(to));
        if old.is_file()
            && let Err(error) = storage::move_file(&old, &moved)
        {
            log::warn!("the old log {} stayed where it was: {error}", old.display());
        }
    }
}
