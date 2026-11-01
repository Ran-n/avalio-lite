// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.468773
//+ Revised: 	2026/10/29 15:46:51.461785
// -----------------------------------------------------------------------------

use rusqlite::{Connection, params};

use super::model::{Pupil, clean_name};
use super::update_one;
use crate::error::Result;

/// Pupils of a class in sheet row order.
pub fn list_class_pupils(connection: &Connection, class_id: i64) -> Result<Vec<Pupil>> {
    query_pupils(
        connection,
        "SELECT p.id, p.name FROM pupils p JOIN class_pupils cp ON cp.pupil_id = p.id
         WHERE cp.class_id = ?1 ORDER BY cp.position",
        class_id,
    )
}

/// Pupils that exist (in other classes, or in none) but aren't in this class yet.
pub fn pupils_not_in_class(connection: &Connection, class_id: i64) -> Result<Vec<Pupil>> {
    query_pupils(
        connection,
        "SELECT id, name FROM pupils
         WHERE id NOT IN (SELECT pupil_id FROM class_pupils WHERE class_id = ?1)
         ORDER BY name COLLATE NOCASE",
        class_id,
    )
}

pub fn add_new_pupil(connection: &Connection, class_id: i64, name: &str) -> Result<i64> {
    let transaction = connection.unchecked_transaction()?;
    let pupil_id = insert_new_pupil(&transaction, class_id, name)?;
    transaction.commit()?;
    Ok(pupil_id)
}

/// `add_new_pupil` without a transaction of its own, for callers already inside one: SQLite
/// can't nest them.
pub fn insert_new_pupil(connection: &Connection, class_id: i64, name: &str) -> Result<i64> {
    let name = clean_name(name)?;
    connection.execute("INSERT INTO pupils (name) VALUES (?1)", [&name])?;
    let pupil_id = connection.last_insert_rowid();
    enroll(connection, class_id, pupil_id)?;
    log::info!("added new pupil {pupil_id} to class {class_id}");
    Ok(pupil_id)
}

/// Every pupil called `name`, ignoring case and outer spaces.
pub fn pupils_named(connection: &Connection, name: &str) -> Result<Vec<Pupil>> {
    let mut statement = connection.prepare("SELECT id, name FROM pupils WHERE name = ?1 COLLATE NOCASE ORDER BY id")?;
    let pupils = statement
        .query_map([name.trim()], |row| Ok(Pupil { id: row.get(0)?, name: row.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(pupils)
}

pub fn add_existing_pupil(connection: &Connection, class_id: i64, pupil_id: i64) -> Result<()> {
    enroll(connection, class_id, pupil_id)?;
    log::info!("added pupil {pupil_id} to class {class_id}");
    Ok(())
}

pub fn pupil_name(connection: &Connection, pupil_id: i64) -> Result<String> {
    Ok(connection.query_row("SELECT name FROM pupils WHERE id = ?1", [pupil_id], |row| row.get(0))?)
}

/// Renames the pupil everywhere: pupils are shared between classes.
pub fn rename_pupil(connection: &Connection, pupil_id: i64, name: &str) -> Result<()> {
    let name = clean_name(name)?;
    update_one(connection, "UPDATE pupils SET name = ?2 WHERE id = ?1", params![pupil_id, name])
}

/// Takes the pupil out of one class (their marks there go too); they stay in other classes.
pub fn remove_from_class(connection: &Connection, class_id: i64, pupil_id: i64) -> Result<()> {
    connection
        .execute("DELETE FROM class_pupils WHERE class_id = ?1 AND pupil_id = ?2", params![class_id, pupil_id])?;
    log::info!("removed pupil {pupil_id} from class {class_id}");
    Ok(())
}

fn enroll(connection: &Connection, class_id: i64, pupil_id: i64) -> Result<()> {
    connection.execute(
        "INSERT INTO class_pupils (class_id, pupil_id, position)
         VALUES (?1, ?2, (SELECT COALESCE(MAX(position), 0) + 1 FROM class_pupils WHERE class_id = ?1))",
        params![class_id, pupil_id],
    )?;
    Ok(())
}

fn query_pupils(connection: &Connection, sql: &str, class_id: i64) -> Result<Vec<Pupil>> {
    let mut statement = connection.prepare(sql)?;
    let pupils = statement
        .query_map([class_id], |row| Ok(Pupil { id: row.get(0)?, name: row.get(1)? }))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(pupils)
}
