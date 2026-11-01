// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:44.148390
//+ Revised: 	2026/10/29 14:13:27.568156
// -----------------------------------------------------------------------------

//! Locking, unlocking and changing the PIN that encrypts the open database.

use std::fs;
use std::path::Path;

use rusqlite::Connection;

use super::AppState;
use crate::db;
use crate::error::{AppError, Result, io_at};
use crate::lock::{Pin, pins_match, validate_new_pin};
use crate::storage::with_suffix;

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
        let path = self.config.as_ref().ok_or(AppError::NoDatabaseOpen)?.database_path.clone();
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
    /// file is in place, so a failure at any step leaves one complete, openable database at the
    /// usual path.
    pub(super) fn replace_with(&mut self, source: Option<&Connection>, new_key: Option<&str>) -> Result<()> {
        let path = self.config.as_ref().ok_or(AppError::NoDatabaseOpen)?.database_path.clone();
        let own = self.connection.as_ref().ok_or(AppError::NoDatabaseOpen)?;
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
        if let Err(error) = swap_in(&path, &fresh) {
            db::discard(&fresh);
            self.reopen(&path, old_key.as_ref().map(Pin::as_str));
            return Err(error);
        }
        self.reopen(&path, new_key);
        // the new file is in place, so the old one is only a leftover now; failing to delete it is
        // still reported, since it may be an unencrypted copy of the data
        let leftover = db::remove_files(&with_suffix(&path, BACKUP_SUFFIX));
        if self.connection.is_none() {
            return Err(AppError::NoDatabaseOpen);
        }
        leftover
    }

    /// Opens `path` with `key` after a re-key or restore, logging (not hiding) a failure: the caller then
    /// reports `NoDatabaseOpen` and the lock screen asks for the PIN again. Never creates the
    /// file: if it is gone, an empty database in its place would hide where the data went.
    fn reopen(&mut self, path: &Path, key: Option<&str>) {
        match db::open_existing(path, key) {
            Ok(connection) => {
                self.connection = Some(connection);
                self.key = key.map(Pin::new);
            }
            Err(error) => log::error!("could not reopen {} after replacing it: {error}", path.display()),
        }
    }
}

/// Replaces the (closed) database at `path` with `fresh`, keeping the old file aside (and putting
/// it back on any failure) until the new one is in place. The caller deletes the old file.
fn swap_in(path: &Path, fresh: &Path) -> Result<()> {
    let previous = with_suffix(path, BACKUP_SUFFIX);
    db::remove_files(&previous)?;
    fs::rename(path, &previous).map_err(io_at(path))?;
    // sidecar files belong to the old file; left next to the new one they would corrupt it
    let replaced = db::remove_sidecars(path).and_then(|()| fs::rename(fresh, path).map_err(io_at(fresh)));
    if let Err(error) = replaced {
        fs::rename(&previous, path).map_err(io_at(path))?;
        return Err(error);
    }
    Ok(())
}
