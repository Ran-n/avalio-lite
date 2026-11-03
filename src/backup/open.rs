// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.777280
//+ Revised: 	2026/11/03 11:39:46.777280
// -----------------------------------------------------------------------------

//! Opening a backup: a checked copy beside the database, never the file itself.

use std::fs;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{AppError, Result, io_at};
use crate::platform::db;

/// A backup opened for reading: a copy of the chosen file (which is never changed), upgraded by the
/// migrations if it is older. The copy is deleted when this is dropped, whatever happened.
pub struct OpenedBackup {
    connection: Option<Connection>,
    contents: BackupContents,
    file: PathBuf,
    copy: PathBuf,
}

impl OpenedBackup {
    pub fn connection(&self) -> &Connection {
        self.connection.as_ref().expect("the connection is only taken when dropped")
    }

    /// How many classes and pupils it holds, counted when it was opened.
    pub fn contents(&self) -> BackupContents {
        self.contents
    }

    /// The chosen file's name, as the messages quote it.
    pub fn file_name(&self) -> String {
        self.file
            .file_name()
            .map_or_else(|| self.file.display().to_string(), |name| name.to_string_lossy().into_owned())
    }
}

impl Drop for OpenedBackup {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take()
            && let Err(error) = db::close(connection)
        {
            log::error!("the backup copy did not close cleanly: {error}");
        }
        db::discard(&self.copy);
    }
}

/// What a backup holds, for the question before restoring it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BackupContents {
    pub classes: usize,
    pub pupils: usize,
}

/// Copies `file` to `copy` and opens the copy, trying each of `keys` when it is encrypted.
/// `WrongPin` only when none of them opens it; any error deletes the copy.
pub fn open_copy(file: &Path, copy: &Path, keys: &[Option<&str>]) -> Result<OpenedBackup> {
    db::remove_files(copy)?;
    if let Err(error) = fs::copy(file, copy) {
        // a copy cut short (a full disk, say) may still hold readable data
        db::discard(copy);
        return Err(io_at(file)(error));
    }
    // counted here, so an unreadable backup fails while opening, like every other bad file
    let opened = open_checked_copy(file, copy, keys)
        .and_then(|connection| count_contents(&connection).map(|contents| (connection, contents)));
    match opened {
        Ok((connection, contents)) => {
            log::info!("opened the backup {}", file.display());
            let (file, copy) = (file.to_path_buf(), copy.to_path_buf());
            Ok(OpenedBackup { connection: Some(connection), contents, file, copy })
        }
        Err(error) => {
            db::discard(copy);
            Err(error)
        }
    }
}

/// A plain file is opened without a key; only an encrypted one is tried with the keys. A file
/// SQLite can't read isn't a database at all, which is told apart from a wrong PIN where it can
/// be: a plain one fails without a key, and an encrypted one is made of whole pages.
fn open_checked_copy(file: &Path, copy: &Path, keys: &[Option<&str>]) -> Result<Connection> {
    let not_a_database = || AppError::NotADatabase(file.to_path_buf());
    if !db::is_encrypted(copy)? {
        return open_with(file, copy, None).map_err(|error| match error {
            AppError::WrongPin => not_a_database(),
            other => other,
        });
    }
    if !db::has_whole_pages(copy)? {
        return Err(not_a_database());
    }
    for key in keys.iter().flatten() {
        match open_with(file, copy, Some(*key)) {
            Err(AppError::WrongPin) => {}
            opened => return opened,
        }
    }
    Err(AppError::WrongPin)
}

/// Checks the copy is a whole avalio-lite database before the migrations touch it. Errors name
/// the chosen file, not the copy.
fn open_with(file: &Path, copy: &Path, key: Option<&str>) -> Result<Connection> {
    db::verify(copy, key).map_err(|error| match error {
        AppError::IntegrityCheckFailed(_) => AppError::IntegrityCheckFailed(file.to_path_buf()),
        other => other,
    })?;
    if db::stored_version(copy, key)? == 0 {
        return Err(AppError::NotADatabase(file.to_path_buf()));
    }
    db::open(copy, key)
}

/// What the backup holds, for the question before restoring it.
fn count_contents(connection: &Connection) -> Result<BackupContents> {
    let count = |table: &str| -> Result<usize> {
        let count: i64 = connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))?;
        Ok(usize::try_from(count).unwrap_or_default())
    };
    Ok(BackupContents { classes: count("classes")?, pupils: count("pupils")? })
}
