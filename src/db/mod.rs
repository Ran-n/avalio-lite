// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.812550
//+ Revised: 	2026/10/28 22:23:17.008109
// -----------------------------------------------------------------------------

//! The database file. SQLite through SQLCipher: without a key it is a plain SQLite file;
//! with one (the user's PIN) every page is encrypted on disk.

pub mod migrations;

use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use rusqlite::{Connection, ErrorCode, OpenFlags};
use zeroize::Zeroizing;

use crate::error::{AppError, Result, io_at};

const SIDECAR_SUFFIXES: &[&str] = &["-wal", "-shm"];
/// Every unencrypted SQLite file starts with these 16 bytes; an encrypted one starts with random salt.
const PLAINTEXT_HEADER: &[u8; 16] = b"SQLite format 3\0";
/// SQLCipher's name for "no encryption" when attaching another database.
const NO_KEY: &str = "";

/// Opens (creating if needed) the database at `path`, unlocking it with `key` when it is
/// encrypted, and brings its schema up to date. A wrong or missing key is `WrongPin`.
pub fn open(path: &Path, key: Option<&str>) -> Result<Connection> {
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    let mut connection = Connection::open(path)?;
    apply_key(&connection, key)?;
    connection.pragma_update(None, "foreign_keys", true)?;
    connection.pragma_update(None, "journal_mode", "WAL")?;
    migrations::migrate(&mut connection)?;
    log::info!("opened database {} ({})", path.display(), if key.is_some() { "encrypted" } else { "not encrypted" });
    Ok(connection)
}

/// Opens a database that must already exist; never creates an empty one in its place.
pub fn open_existing(path: &Path, key: Option<&str>) -> Result<Connection> {
    if !path.is_file() {
        return Err(AppError::NotFound(path.to_path_buf()));
    }
    open(path, key)
}

/// True when the file is encrypted (it doesn't start with SQLite's plain header).
/// A missing or still empty file is not encrypted.
pub fn is_encrypted(path: &Path) -> Result<bool> {
    let mut header = [0u8; 16];
    let read = match File::open(path) {
        Ok(mut file) => file.read(&mut header).map_err(io_at(path))?,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(false),
        Err(error) => return Err(io_at(path)(error)),
    };
    Ok(read == header.len() && &header != PLAINTEXT_HEADER)
}

/// Writes a complete copy of the open database to `destination`, encrypted with `key`
/// (or not encrypted when `key` is `None`). This is how a database is encrypted, decrypted,
/// re-keyed and moved: SQLCipher's export, which copies every table, index and trigger.
pub fn export(connection: &Connection, destination: &Path, key: Option<&str>) -> Result<()> {
    if destination.exists() {
        return Err(AppError::AlreadyExists(destination.to_path_buf()));
    }
    if let Some(dir) = destination.parent() {
        fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    let version = migrations::schema_version(connection)?;
    connection.execute(
        "ATTACH DATABASE ?1 AS export KEY ?2",
        [destination.to_string_lossy().as_ref(), key.unwrap_or(NO_KEY)],
    )?;
    let exported = connection
        .query_row("SELECT sqlcipher_export('export')", [], |_| Ok(()))
        // the export doesn't carry the schema version, which the migrations rely on
        .and_then(|()| connection.execute_batch(&format!("PRAGMA export.user_version = {version}")));
    let detached = connection.execute("DETACH DATABASE export", []);
    exported?;
    detached?;
    Ok(())
}

/// Checks a database file opens with `key` and passes SQLite's integrity check.
pub fn verify(path: &Path, key: Option<&str>) -> Result<()> {
    let connection = Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
    apply_key(&connection, key)?;
    let result: String = connection.query_row("PRAGMA integrity_check", [], |row| row.get(0))?;
    if result != "ok" {
        return Err(AppError::IntegrityCheckFailed(path.to_path_buf()));
    }
    Ok(())
}

pub fn close(connection: Connection) -> Result<()> {
    connection.close().map_err(|(_, error)| AppError::Database(error))
}

/// Deletes a closed database file together with its WAL sidecar files.
pub fn remove_files(path: &Path) -> Result<()> {
    remove_if_present(path)?;
    remove_sidecars(path)
}

/// Deletes only the WAL sidecar files (`-wal`, `-shm`) of a closed database.
pub fn remove_sidecars(path: &Path) -> Result<()> {
    sidecar_paths(path).try_for_each(|file| remove_if_present(&file))
}

fn remove_if_present(file: &Path) -> Result<()> {
    match fs::remove_file(file) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_at(file)(error)),
    }
}

/// Sets the key, then reads once so a wrong key shows up now rather than on some later query.
/// SQLCipher reports a wrong (or missing) key as "file is not a database", either when the key
/// is set or on that first read, so both steps translate it to `WrongPin`.
fn apply_key(connection: &Connection, key: Option<&str>) -> Result<()> {
    let wrong_key_or = |error: rusqlite::Error| match error.sqlite_error_code() {
        Some(ErrorCode::NotADatabase) => AppError::WrongPin,
        _ => AppError::Database(error),
    };
    // SQLCipher then overwrites the memory it frees, so the key it derives and its copies of
    // the PIN don't linger in the process after a lock or a re-key.
    connection.execute_batch("PRAGMA cipher_memory_security = ON")?;
    if let Some(key) = key {
        // A plain statement on purpose: rusqlite's pragma_update doesn't key SQLCipher correctly
        // (the file then reads as "not a database" even with the right key).
        connection.execute_batch(&key_statement(key)).map_err(wrong_key_or)?;
    }
    connection.query_row("SELECT count(*) FROM sqlite_master", [], |_| Ok(())).map_err(wrong_key_or)
}

/// `PRAGMA key = '…'` with quotes doubled. It holds the PIN, so it is built in one allocation
/// big enough from the start (growing would leave unwiped copies behind) and wiped when dropped.
pub fn key_statement(key: &str) -> Zeroizing<String> {
    const START: &str = "PRAGMA key = '";
    let mut statement = Zeroizing::new(String::with_capacity(START.len() + 2 * key.len() + 1));
    statement.push_str(START);
    for character in key.chars() {
        statement.push(character);
        if character == '\'' {
            statement.push(character);
        }
    }
    statement.push('\'');
    statement
}

fn sidecar_paths(path: &Path) -> impl Iterator<Item = PathBuf> + '_ {
    SIDECAR_SUFFIXES.iter().map(move |suffix| {
        let mut name = path.as_os_str().to_owned();
        name.push(suffix);
        PathBuf::from(name)
    })
}
