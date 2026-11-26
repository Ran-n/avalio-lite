// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.299626
//+ Revised: 	2026/11/23 11:49:16.299626
// -----------------------------------------------------------------------------

//! The file swap behind every PIN change and backup restore: the new file is made beside the
//! database and checked, the old one is set aside, the new one moved in and opened. Each failure
//! puts the old file back, so one complete database is always left.

use std::fs;
use std::path::Path;

use rusqlite::Connection;

use super::{BACKUP_SUFFIX, REKEY_SUFFIX};
use crate::error::{AppError, Result, io_at};
use crate::lock::Pin;
use crate::platform::db;
use crate::platform::storage::with_suffix;
use crate::state::AppState;

impl AppState {
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
        prepare_copy(source, &fresh, new_key)?;
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
        remove_after_change(&path);
        Ok(())
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

/// Writes the contents of the source encrypted with `new_key` to `fresh` and checks that it opens.
/// A failure removes `fresh`, so no half-made copy is left beside the database.
fn prepare_copy(source: &Connection, fresh: &Path, new_key: Option<&str>) -> Result<()> {
    let prepared = db::export(source, fresh, new_key).and_then(|()| db::verify(fresh, new_key));
    if let Err(error) = prepared {
        db::discard(fresh);
        return Err(error);
    }
    Ok(())
}

/// Once the new file is in place and opens, the change worked: the old file is only a leftover now,
/// and so is the copy kept before the last update (under the old PIN, or none). A failure to delete
/// either doesn't undo the change; it is logged, and the next open of the database tries again
/// (`clear_leftovers`).
fn remove_after_change(path: &Path) {
    for leftover in [BACKUP_SUFFIX, db::BEFORE_UPDATE_SUFFIX] {
        let leftover = with_suffix(path, leftover);
        if let Err(error) = db::remove_files(&leftover) {
            log::error!("couldn't remove {} after the PIN change: {error}", leftover.display());
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
