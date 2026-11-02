// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 09:42:12.732824
//+ Revised: 	2026/11/02 08:27:49.708329
// -----------------------------------------------------------------------------

//! `.avalio` backup files: a complete copy of the database, in the same SQLite format and with its
//! schema version, so the tested export, verification and migrations do the work. The extension
//! only says "backup, not the live database". Encrypted with the PIN when the database has one.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

use chrono::NaiveDate;
use rusqlite::Connection;

use crate::db;
use crate::error::{AppError, Result, io_at};
use crate::gradebook::columns::{insert_column, set_round_source};
use crate::gradebook::model::{Column, ColumnKind, Pupil, Sheet};
use crate::gradebook::{cells, classes, pupils, sheet};
use crate::storage::partial_path;

pub const EXTENSION: &str = "avalio";
/// The backup is written under this suffix first and renamed once it is complete and checked.
/// SQLite's smallest page size; every database file, encrypted or not, is a whole number of pages.
const SMALLEST_PAGE: u64 = 512;

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
    let length = fs::metadata(copy).map_err(io_at(copy))?.len();
    if length % SMALLEST_PAGE != 0 {
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

/// What adding a backup's classes did, for the status line.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AddSummary {
    /// the new classes, in the order they were added
    pub class_ids: Vec<i64>,
    /// pupils of the backup now in the new classes, counting those who were already here
    pub pupils: usize,
    /// of those, the ones matched to a pupil already here
    pub existing_pupils: usize,
    /// classes whose name was taken: (name in the backup, name given here)
    pub renamed: Vec<(String, String)>,
}

/// Copies every class of `source` (columns, pupils and typed marks) into `target`, keeping all
/// that is there. A taken class name gets the lowest free " (n)". A backup pupil joins the one
/// pupil already here with the same name; with none or several, a new pupil is made. A pupil the
/// backup shares between classes stays one pupil. All or nothing: one transaction.
pub fn add_classes(target: &Connection, source: &Connection) -> Result<AddSummary> {
    let transaction = target.unchecked_transaction()?;
    let mut adding = Adding::default();
    for class in classes::list_classes(source)? {
        adding.add_class(&transaction, &class.name, &sheet::load_sheet(source, class.id)?)?;
    }
    transaction.commit()?;
    let summary = adding.summary;
    let (classes, pupils, existing) = (summary.class_ids.len(), summary.pupils, summary.existing_pupils);
    log::info!("added {classes} classes and {pupils} pupils ({existing} already here) from a backup");
    Ok(summary)
}

#[derive(Default)]
struct Adding {
    summary: AddSummary,
    /// backup pupil id -> pupil id here, so a shared pupil is matched once
    pupils: HashMap<i64, i64>,
    /// pupils here that already stand for a backup pupil, never matched by name again
    claimed: HashSet<i64>,
}

impl Adding {
    fn add_class(&mut self, connection: &Connection, name: &str, sheet: &Sheet) -> Result<()> {
        let free = classes::free_class_name(connection, name)?;
        if free != name {
            self.summary.renamed.push((name.to_owned(), free.clone()));
        }
        let class_id = classes::create_class(connection, &free)?;
        self.summary.class_ids.push(class_id);
        let columns = copy_columns(connection, class_id, &sheet.columns)?;
        for row in &sheet.rows {
            let pupil_id = self.pupil(connection, class_id, &row.pupil)?;
            for (column, value) in sheet.columns.iter().zip(&row.values) {
                if !column.kind.is_computed() && !value.is_empty() {
                    cells::set_cell(connection, columns[&column.id], pupil_id, value)?;
                }
            }
        }
        Ok(())
    }

    /// The pupil here for a backup pupil, put into the new class.
    fn pupil(&mut self, connection: &Connection, class_id: i64, pupil: &Pupil) -> Result<i64> {
        if let Some(&id) = self.pupils.get(&pupil.id) {
            pupils::add_existing_pupil(connection, class_id, id)?;
            return Ok(id);
        }
        self.summary.pupils += 1;
        // a pupil here already stands for another backup pupil (one made during this add, or
        // the namesake joined for an earlier one), so two backup pupils never become one
        let here: Vec<Pupil> = pupils::pupils_named(connection, &pupil.name)?
            .into_iter()
            .filter(|found| !self.claimed.contains(&found.id))
            .collect();
        let id = match here.as_slice() {
            [only] => {
                pupils::add_existing_pupil(connection, class_id, only.id)?;
                self.summary.existing_pupils += 1;
                only.id
            }
            _ => pupils::insert_new_pupil(connection, class_id, &pupil.name)?,
        };
        self.pupils.insert(pupil.id, id);
        self.claimed.insert(id);
        Ok(id)
    }
}

/// Copies the columns in order, with every setting. A rounded column may round a column to its
/// right, so sources are set once every column exists. Returns backup column id -> new id.
fn copy_columns(connection: &Connection, class_id: i64, columns: &[Column]) -> Result<HashMap<i64, i64>> {
    let mut ids = HashMap::new();
    for column in columns {
        let kind = match column.kind {
            ColumnKind::Round { decimals, .. } => ColumnKind::Round { source: None, decimals },
            kind => kind,
        };
        ids.insert(column.id, insert_column(connection, class_id, &column.title, kind)?);
    }
    for column in columns {
        if let ColumnKind::Round { source: Some(source), .. } = column.kind {
            set_round_source(connection, ids[&column.id], ids.get(&source).copied())?;
        }
    }
    Ok(ids)
}

/// What the backup holds, for the question before restoring it.
fn count_contents(connection: &Connection) -> Result<BackupContents> {
    let count = |table: &str| -> Result<usize> {
        let count: i64 = connection.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |row| row.get(0))?;
        Ok(usize::try_from(count).unwrap_or_default())
    };
    Ok(BackupContents { classes: count("classes")?, pupils: count("pupils")? })
}
