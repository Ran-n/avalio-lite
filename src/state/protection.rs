// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:44.148390
//+ Revised: 	2026/11/25 11:43:09.928157
// -----------------------------------------------------------------------------

//! Locking, unlocking and changing the PIN that encrypts the open database. The file swap every
//! PIN change goes through is in `swap`.

mod swap;

use std::path::Path;

use rusqlite::Connection;

use super::AppState;
use crate::error::{AppError, Result};
use crate::lock::{Pin, pins_match, validate_new_pin};
use crate::platform::db;
use crate::platform::storage::with_suffix;

const REKEY_SUFFIX: &str = ".rekey";
const BACKUP_SUFFIX: &str = ".previous";

/// Removes what a crash or power cut in the middle of a PIN change or a backup could leave next to
/// the database at `path`: the half-made copy (`.rekey`), the file set aside (`.previous`) and the
/// copy of a backup (`.restore`). Any of them may be an unencrypted copy of the data. Only called
/// once the database itself is open (unlocked, when it has a PIN): until then `.previous` may be
/// the one good copy, which the error of the failed change named. A failure is logged; the next
/// open tries again.
pub(super) fn clear_leftovers(path: &Path) {
    let mut leftovers: Vec<_> = [REKEY_SUFFIX, BACKUP_SUFFIX, super::backups::RESTORE_SUFFIX]
        .iter()
        .map(|suffix| with_suffix(path, suffix))
        .collect();
    // the copy kept before an update stays, unless it is readable beside a database that has a PIN
    // (a PIN set since, whose clean-up couldn't delete it)
    let kept = with_suffix(path, db::BEFORE_UPDATE_SUFFIX);
    if kept.exists() && db::is_encrypted(path).unwrap_or(false) && !db::is_encrypted(&kept).unwrap_or(true) {
        leftovers.push(kept);
    }
    for leftover in leftovers.iter().filter(|leftover| leftover.exists()) {
        match db::remove_files(leftover) {
            Ok(()) => log::info!("removed {} left by an interrupted change", leftover.display()),
            // a backup open for restoring holds its `.restore` copy until it is answered
            Err(error) => log::warn!("couldn't remove {} yet, the next open tries again: {error}", leftover.display()),
        }
    }
}

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
        if !path.is_file() {
            return Err(AppError::NotFound(path));
        }
        let connection = db::open_upgrading(&path, Some(pin))?;
        self.connection = Some(connection);
        self.key = Some(Pin::new(pin));
        log::info!("database unlocked");
        clear_leftovers(&path);
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

    /// Whether `current` is the PIN the open database is encrypted with; `NotEditable` when it has none.
    pub fn check_current_pin(&self, current: &str) -> Result<()> {
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
}
