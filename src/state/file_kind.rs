// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.224174
//+ Revised: 	2026/11/23 11:49:16.224174
// -----------------------------------------------------------------------------

//! What an existing database file is, and whether it may be opened at all. Opening runs the
//! migrations, which would turn an empty or foreign file into an avalio-lite database, so this check
//! comes first.

use std::fs;
use std::path::Path;

use crate::error::{AppError, Result, io_at};
use crate::platform::db;

/// SQLite's plain header is 16 bytes; a shorter file can't be a database of any kind.
const MIN_DATABASE_BYTES: u64 = 16;

/// What an existing database file is, once it is known to be an avalio-lite database.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum FileKind {
    Plain,
    Encrypted,
}

/// Checks that the existing file at `path` can be an avalio-lite database before anything opens
/// it, since opening runs the migrations and would turn any empty or foreign file into one.
/// A plain file needs avalio's schema; an encrypted one can't be read without its PIN, so it
/// must at least be whole pages (as in `backup`). Empty and tiny files are never databases.
pub(super) fn existing_kind(path: &Path) -> Result<FileKind> {
    let not_a_database = || AppError::NotADatabase(path.to_path_buf());
    let length = fs::metadata(path).map_err(io_at(path))?.len();
    if length < MIN_DATABASE_BYTES {
        return Err(not_a_database());
    }
    if db::is_encrypted(path)? {
        return if db::has_whole_pages(path)? { Ok(FileKind::Encrypted) } else { Err(not_a_database()) };
    }
    match db::stored_version(path, None) {
        Ok(0) | Err(AppError::WrongPin) => Err(not_a_database()),
        Ok(_) => Ok(FileKind::Plain),
        Err(error) => Err(error),
    }
}
