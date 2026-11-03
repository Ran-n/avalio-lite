// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 22:29:57.566701
//+ Revised: 	2026/11/03 10:00:16.876321
// -----------------------------------------------------------------------------

//! Backups of the open database: exporting one, and opening one to restore or add classes from.

use std::path::Path;

use super::AppState;
use crate::backup::{self, AddSummary, OpenedBackup};
use crate::error::Result;
use crate::lock::Pin;
use crate::platform::storage;

/// A backup being restored or added from is copied next to the database under this suffix.
const RESTORE_SUFFIX: &str = ".restore";

impl AppState {
    /// Saves everything to a backup file, encrypted with the PIN when the database has one; the
    /// key never leaves this struct.
    pub fn export_backup(&self, destination: &Path) -> Result<()> {
        let connection = self.open_connection()?;
        backup::export(connection, destination, self.key.as_ref().map(Pin::as_str))
    }

    /// Opens a copy of the backup `file` (never the file itself) next to the database. With a typed
    /// `pin`, only that is tried; otherwise the app's own PIN, since a backup of this database has
    /// it. A backup that needs another PIN is `WrongPin`.
    pub fn open_backup(&self, file: &Path, pin: Option<&str>) -> Result<OpenedBackup> {
        let database = &self.open_config()?.database_path;
        let own_key = self.key.as_ref().map(Pin::as_str);
        let keys = match pin {
            Some(pin) => [Some(pin)],
            None => [own_key],
        };
        backup::open_copy(file, &storage::with_suffix(database, RESTORE_SUFFIX), &keys)
    }

    /// Replaces everything with the backup's contents, encrypted with the app's current PIN (or
    /// not, if it has none): the PIN belongs to this computer's copy, so a restore never changes
    /// it. A failure leaves the current database complete and in use. The backup's copy is deleted
    /// either way.
    pub fn restore_backup(&mut self, backup: OpenedBackup) -> Result<()> {
        let key = self.key.clone();
        self.replace_with(Some(backup.connection()), key.as_ref().map(Pin::as_str))?;
        log::info!("restored the backup {}", backup.file_name());
        Ok(())
    }

    /// Adds every class of the backup to the open database, keeping all that is there. The PIN and
    /// encryption don't change. The backup's copy is deleted either way.
    pub fn add_backup_classes(&self, backup: OpenedBackup) -> Result<AddSummary> {
        let connection = self.open_connection()?;
        backup::add_classes(connection, backup.connection())
    }
}
