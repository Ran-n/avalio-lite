// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:44.148390
//+ Revised: 	2026/11/03 10:00:16.953445
// -----------------------------------------------------------------------------

//! Locking, unlocking and changing the PIN that encrypts the open database.

use std::fs;
use std::path::Path;

use rusqlite::Connection;

use super::AppState;
use crate::error::{AppError, Result, io_at};
use crate::lock::{Pin, pins_match, validate_new_pin};
use crate::platform::db;
use crate::platform::storage::with_suffix;

const REKEY_SUFFIX: &str = ".rekey";
const BACKUP_SUFFIX: &str = ".previous";

impl AppState {
    /// The database is encrypted with a PIN (whether it is open or locked right now).
    pub fn is_protected(&self) -> bool {
        self.key.is_some() || self.is_locked()
    }

    /// A database is chosen but closed, waiting for its PIN.
    pub fn is_locked(&self) -> bool {
        self.config.is_some() && self.connection.is_none()
    }

    /// Opens the locked database with `pin`. A wrong PIN is `WrongPin` and changes nothing.
    pub fn unlock(&mut self, pin: &str) -> Result<()> {
        let path = self.open_config()?.database_path.clone();
        let connection = db::open_existing(&path, Some(pin))?;
        self.connection = Some(connection);
        self.key = Some(Pin::new(pin));
        log::info!("database unlocked");
        Ok(())
    }

    /// Closes an encrypted database so nothing readable stays open. Returns false (and does
    /// nothing) when the database has no PIN, since there would be nothing to unlock with.
    pub fn lock(&mut self) -> bool {
        if self.key.is_none() {
            return false;
        }
        self.close_connection();
        log::info!("database locked");
        true
    }

    /// Encrypts the open, unencrypted database with a new PIN.
    pub fn set_pin(&mut self, pin: &str, confirmation: &str) -> Result<()> {
        if self.key.is_some() {
            return Err(AppError::NotEditable);
        }
        validate_new_pin(pin, confirmation)?;
        self.rekey(Some(pin))?;
        log::info!("database encrypted with a PIN");
        Ok(())
    }

    /// Re-encrypts the database with a new PIN after checking the current one.
    pub fn change_pin(&mut self, current: &str, pin: &str, confirmation: &str) -> Result<()> {
        self.check_current_pin(current)?;
        validate_new_pin(pin, confirmation)?;
        self.rekey(Some(pin))?;
        log::info!("database PIN changed");
        Ok(())
    }

    /// Decrypts the database after checking the current PIN.
    pub fn remove_pin(&mut self, current: &str) -> Result<()> {
        self.check_current_pin(current)?;
        self.rekey(None)?;
        log::info!("database PIN removed; the file is no longer encrypted");
        Ok(())
    }

    fn check_current_pin(&self, current: &str) -> Result<()> {
        match &self.key {
            Some(key) if pins_match(key.as_str(), current) => Ok(()),
            Some(_) => Err(AppError::WrongPin),
            None => Err(AppError::NotEditable),
        }
    }

    fn rekey(&mut self, new_key: Option<&str>) -> Result<()> {
        self.replace_with(None, new_key)
    }

    /// Writes the contents of `source` (the open database itself when `None`), encrypted with
    /// `new_key`, next to the database, checks the copy opens, then swaps it in. Changing the PIN
    /// and restoring a backup both go through here. The original is renamed aside until the new
    /// file is in place, so a failure at any step leaves one complete, openable database: at the
    /// usual path, or, when another program holds the files so it can't go back, kept aside and
    /// named in the error (`KeptAside`).
    pub(super) fn replace_with(&mut self, source: Option<&Connection>, new_key: Option<&str>) -> Result<()> {
        self.replace_using(source, new_key, &db::open_existing)
    }

    /// `replace_with`, with the way the swapped-in file is opened as a parameter so a test can make
    /// that step fail (only another program grabbing the file can make it fail for real).
    #[doc(hidden)]
    pub fn replace_using(
        &mut self,
        source: Option<&Connection>,
        new_key: Option<&str>,
        open: &dyn Fn(&Path, Option<&str>) -> Result<Connection>,
    ) -> Result<()> {
        let path = self.open_config()?.database_path.clone();
        let own = self.open_connection()?;
        let source = source.unwrap_or(own);
        let fresh = with_suffix(&path, REKEY_SUFFIX);
        db::remove_files(&fresh)?;
        let prepared = db::export(source, &fresh, new_key).and_then(|()| db::verify(&fresh, new_key));
        if let Err(error) = prepared {
            db::discard(&fresh);
            return Err(error);
        }
        let old_key = self.key.clone();
        self.close_connection();
        if let Err(error) = set_aside(&path) {
            db::discard(&fresh);
            // the old file is still at `path`; a failure to open it is already logged
            let _ = self.reopen(&path, old_key.as_ref().map(Pin::as_str), open);
            return Err(error);
        }
        // from here the old file is only at `.previous`, and every failure puts it back
        if let Err(error) = move_in(&path, &fresh) {
            db::discard(&fresh);
            // nothing new is at `path`, and any sidecar files left there are the old file's own
            return Err(self.put_back(&path, old_key.as_ref(), error));
        }
        if let Err(error) = self.reopen(&path, new_key, open) {
            return Err(self.roll_back(&path, old_key.as_ref(), error));
        }
        // the new file is in place and opens, so the old one is only a leftover now; failing to
        // delete it is still reported, since it may be an unencrypted copy of the data
        db::remove_files(&with_suffix(&path, BACKUP_SUFFIX))
    }

    /// The new file would not open: removes it and puts the old one back (`put_back`).
    fn roll_back(&mut self, path: &Path, old_key: Option<&Pin>, cause: AppError) -> AppError {
        match db::remove_files(path) {
            Ok(()) => self.put_back(path, old_key, cause),
            Err(error) => kept_aside(path, &cause, &error),
        }
    }

    /// Puts the old file (kept aside as `.previous`) back at `path` and opens it, so the only
    /// intact copy is never lost. Returns `cause`, the reason the swap failed; if even the old file
    /// can't be put back, it stays as `.previous` and the error (`KeptAside`) says where.
    fn put_back(&mut self, path: &Path, old_key: Option<&Pin>, cause: AppError) -> AppError {
        if let Err(error) = fs::rename(with_suffix(path, BACKUP_SUFFIX), path).map_err(io_at(path)) {
            return kept_aside(path, &cause, &error);
        }
        log::warn!("the new database did not go in or open ({cause}); the previous one was put back");
        // a failure to open it is logged, and the lock screen asks for the PIN again
        let _ = self.reopen(path, old_key.map(Pin::as_str), &db::open_existing);
        cause
    }

    /// Opens `path` with `key` after a re-key or restore. A failure is logged and returned; never
    /// creates the file: if it is gone, an empty database in its place would hide where the data went.
    fn reopen(
        &mut self,
        path: &Path,
        key: Option<&str>,
        open: &dyn Fn(&Path, Option<&str>) -> Result<Connection>,
    ) -> Result<()> {
        match open(path, key) {
            Ok(connection) => {
                self.connection = Some(connection);
                self.key = key.map(Pin::new);
                Ok(())
            }
            Err(error) => {
                log::error!("could not reopen {} after replacing it: {error}", path.display());
                Err(error)
            }
        }
    }
}

/// The old file couldn't go back to `path`: it stays at `.previous`, which the error names.
fn kept_aside(path: &Path, cause: &AppError, error: &AppError) -> AppError {
    let kept = with_suffix(path, BACKUP_SUFFIX);
    log::error!("could not put the previous database back after {cause}, it is kept as {}: {error}", kept.display());
    AppError::KeptAside { kept, path: path.to_path_buf() }
}

/// Renames the (closed) database at `path` aside to `.previous`, where it stays until the new one
/// has opened. On failure it is still at `path`.
fn set_aside(path: &Path) -> Result<()> {
    let previous = with_suffix(path, BACKUP_SUFFIX);
    db::remove_files(&previous)?;
    fs::rename(path, &previous).map_err(io_at(path))
}

/// Puts `fresh` at `path`, which `set_aside` emptied. Sidecar files belong to the old file; left
/// next to the new one they would corrupt it.
fn move_in(path: &Path, fresh: &Path) -> Result<()> {
    db::remove_sidecars(path)?;
    fs::rename(fresh, path).map_err(io_at(fresh))
}
