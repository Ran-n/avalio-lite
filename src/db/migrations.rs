// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.660497
//+ Revised: 	2026/10/29 00:10:41.297655
// -----------------------------------------------------------------------------

use rusqlite::Connection;

use crate::error::{AppError, Result};

/// Schema migrations, applied in order. The database's `user_version` pragma
/// records how many have run. Never edit a shipped migration; add a new one.
const MIGRATIONS: &[&str] = &[
    include_str!("../../migrations/001_initial.sql"),
    include_str!("../../migrations/002_bonus_tests.sql"),
    include_str!("../../migrations/003_enforce_class_membership.sql"),
    include_str!("../../migrations/004_cells_only_in_typed_columns.sql"),
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
    for (index, sql) in MIGRATIONS.iter().enumerate().take(target).skip(current) {
        let version = index + 1;
        let transaction = connection.transaction()?;
        transaction.execute_batch(sql)?;
        transaction.pragma_update(None, "user_version", version as i64)?;
        transaction.commit()?;
        log::info!("applied schema migration {version}");
    }
    Ok(())
}
