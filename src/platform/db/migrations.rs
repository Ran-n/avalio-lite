// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.660497
//+ Revised: 	2026/11/12 14:39:53.658906
// -----------------------------------------------------------------------------

use rusqlite::{Connection, OptionalExtension};

use crate::error::{AppError, Result};

/// Schema migrations, applied in order. The database's `user_version` pragma
/// records how many have run. Never edit a shipped migration; add a new one.
const MIGRATIONS: &[&str] = &[
    include_str!("../../../migrations/001_initial.sql"),
    include_str!("../../../migrations/002_bonus_tests.sql"),
    include_str!("../../../migrations/003_enforce_class_membership.sql"),
    include_str!("../../../migrations/004_cells_only_in_typed_columns.sql"),
    include_str!("../../../migrations/005_column_widths.sql"),
    include_str!("../../../migrations/006_pupil_surnames.sql"),
    include_str!("../../../migrations/007_name_columns.sql"),
    include_str!("../../../migrations/008_split_comma_names.sql"),
    include_str!("../../../migrations/009_column_limits.sql"),
];

pub fn latest_version() -> usize {
    MIGRATIONS.len()
}

pub fn schema_version(connection: &Connection) -> Result<usize> {
    let version: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    Ok(usize::try_from(version).unwrap_or(usize::MAX))
}

pub fn migrate(connection: &mut Connection) -> Result<()> {
    migrate_to(connection, latest_version())
}

/// Applies the migrations up to and including `target`; never goes back down.
pub fn migrate_to(connection: &mut Connection, target: usize) -> Result<()> {
    let current = schema_version(connection)?;
    if current > latest_version() {
        return Err(AppError::NewerSchema { found: current, supported: latest_version() });
    }
    let pending: Vec<(usize, &str)> =
        MIGRATIONS.iter().enumerate().take(target).skip(current).map(|(index, sql)| (index + 1, *sql)).collect();
    apply(connection, &pending)
}

/// Runs `sql` as migration `version` through the same checks as the real ones, without recording
/// it; for the tests of the migration runner itself.
#[doc(hidden)]
pub fn apply_unrecorded(connection: &mut Connection, version: usize, sql: &str) -> Result<()> {
    with_foreign_keys_off(connection, |connection| apply_one(connection, version, sql, false))
}

fn apply(connection: &mut Connection, pending: &[(usize, &str)]) -> Result<()> {
    if pending.is_empty() {
        return Ok(());
    }
    with_foreign_keys_off(connection, |connection| {
        pending.iter().try_for_each(|(version, sql)| apply_one(connection, *version, sql, true))
    })
}

/// Runs `task` with foreign keys off, then puts them back as they were, on every path.
///
/// A migration that rebuilds a table (SQLite's way to change a constraint) drops the old one, and
/// with foreign keys on that would delete every row pointing at it through ON DELETE CASCADE.
/// SQLite ignores the pragma inside a transaction, so it is set here, around the migrations.
fn with_foreign_keys_off(connection: &mut Connection, task: impl FnOnce(&mut Connection) -> Result<()>) -> Result<()> {
    let enforced: bool = connection.pragma_query_value(None, "foreign_keys", |row| row.get(0))?;
    connection.pragma_update(None, "foreign_keys", false)?;
    let done = task(connection);
    let restored = connection.pragma_update(None, "foreign_keys", enforced);
    done?;
    restored?;
    Ok(())
}

/// One migration in its own transaction, committed only when `foreign_key_check` finds no
/// dangling reference (foreign keys being off meanwhile, nothing else would catch one).
fn apply_one(connection: &mut Connection, version: usize, sql: &str, record: bool) -> Result<()> {
    let transaction = connection.transaction()?;
    let before = transaction.total_changes();
    transaction.execute_batch(sql)?;
    let changed = transaction.total_changes() - before;
    let broken: Option<String> = transaction
        .query_row("SELECT \"table\" FROM pragma_foreign_key_check LIMIT 1", [], |row| row.get(0))
        .optional()?;
    if let Some(table) = broken {
        // dropping the transaction rolls the migration back
        log::error!("schema migration {version} left a dangling reference in table {table}; rolled back");
        let code = rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_CONSTRAINT_FOREIGNKEY);
        let message = format!("schema migration {version} left a dangling reference in table {table}");
        return Err(AppError::Database(rusqlite::Error::SqliteFailure(code, Some(message))));
    }
    if record {
        transaction.pragma_update(None, "user_version", version as i64)?;
    }
    transaction.commit()?;
    log::info!("applied schema migration {version} ({changed} rows changed)");
    Ok(())
}
