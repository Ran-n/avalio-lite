// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 09:42:12.732824
//+ Revised: 	2026/11/03 11:39:47.263806
// -----------------------------------------------------------------------------

//! `.avalio` backup files: a complete copy of the database, in the same SQLite format and with its
//! schema version, so the tested export, verification and migrations do the work. The extension
//! only says "backup, not the live database". Encrypted with the PIN when the database has one.

mod merge;
mod open;
pub mod ui;

use std::fs;
use std::path::Path;

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::error::{Result, io_at};
use crate::platform::db;
use crate::platform::storage::partial_path;

pub use merge::{AddSummary, add_classes};
pub use open::{BackupContents, OpenedBackup, open_copy};

/// The extension of backup files.
pub const EXTENSION: &str = "avalio";

/// The name the Save dialog proposes, with the date so successive backups don't overwrite each other.
pub fn default_file_name(date: NaiveDate) -> String {
    format!("avalio-lite-backup-{}.{EXTENSION}", date.format("%Y-%m-%d"))
}

/// Saves everything in the open database to `destination`, encrypted with `key` when there is one.
/// The copy is written and checked beside it first, so a failure never leaves a half-written
/// backup nor touches a file already at `destination`; a complete one replaces it.
pub fn export(connection: &Connection, destination: &Path, key: Option<&str>) -> Result<()> {
    let partial = partial_path(destination);
    db::remove_files(&partial)?;
    let written = db::export(connection, &partial, key)
        .and_then(|()| db::verify(&partial, key))
        .and_then(|()| fs::rename(&partial, destination).map_err(io_at(destination)));
    if let Err(error) = written {
        db::discard(&partial);
        return Err(error);
    }
    log::info!("backup saved to {}", destination.display());
    Ok(())
}
