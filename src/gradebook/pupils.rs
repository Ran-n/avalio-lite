// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:54.468773
//+ Revised: 	2026/11/11 17:10:58.522309
// -----------------------------------------------------------------------------

use rusqlite::{Connection, Params, params};

use super::model::{Pupil, clean_name, clean_surname, name_match_key, name_sort_key};
use super::update_one;
use crate::error::Result;

/// Pupils of a class in sheet row order.
pub fn list_class_pupils(connection: &Connection, class_id: i64) -> Result<Vec<Pupil>> {
    query_pupils(
        connection,
        "SELECT p.id, p.first_name, p.surname FROM pupils p JOIN class_pupils cp ON cp.pupil_id = p.id
         WHERE cp.class_id = ?1 ORDER BY cp.position",
        [class_id],
    )
}

/// Pupils that exist (in other classes, or in none) but aren't in this class yet.
pub fn pupils_not_in_class(connection: &Connection, class_id: i64) -> Result<Vec<Pupil>> {
    query_pupils(
        connection,
        "SELECT id, first_name, surname FROM pupils
         WHERE id NOT IN (SELECT pupil_id FROM class_pupils WHERE class_id = ?1)
         ORDER BY id",
        [class_id],
    )
    .map(|mut pupils| {
        // sorted here: SQLite's NOCASE only folds A to Z, which put "Álvaro" after "Zoe"
        pupils.sort_by_cached_key(|pupil| name_sort_key(&pupil.listed_name()));
        pupils
    })
}

/// Pupils in no class at all: left their last class, or their only class was deleted.
pub fn pupils_in_no_class(connection: &Connection) -> Result<Vec<Pupil>> {
    let mut pupils = query_pupils(
        connection,
        "SELECT id, first_name, surname FROM pupils WHERE id NOT IN (SELECT pupil_id FROM class_pupils) ORDER BY id",
        [],
    )?;
    pupils.sort_by_cached_key(|pupil| name_sort_key(&pupil.listed_name()));
    Ok(pupils)
}

/// Deletes a pupil who is in no class, name and all. A pupil still in a class is refused
/// (`NotEditable`): they leave their classes first, which is where their marks go.
pub fn delete_pupil_in_no_class(connection: &Connection, pupil_id: i64) -> Result<()> {
    update_one(
        connection,
        "DELETE FROM pupils WHERE id = ?1 AND NOT EXISTS (SELECT 1 FROM class_pupils WHERE pupil_id = ?1)",
        [pupil_id],
    )?;
    log::info!("deleted pupil {pupil_id}, who was in no class");
    Ok(())
}

/// Makes a pupil and puts them at the end of the class. The first name is required (`EmptyName`
/// otherwise); the surname may be empty. Both are trimmed.
pub fn add_new_pupil(connection: &Connection, class_id: i64, first_name: &str, surname: &str) -> Result<i64> {
    let transaction = connection.unchecked_transaction()?;
    let pupil_id = insert_new_pupil(&transaction, class_id, first_name, surname)?;
    transaction.commit()?;
    Ok(pupil_id)
}

/// `add_new_pupil` without a transaction of its own, for callers already inside one: SQLite
/// can't nest them.
pub fn insert_new_pupil(connection: &Connection, class_id: i64, first_name: &str, surname: &str) -> Result<i64> {
    let first_name = clean_name(first_name)?;
    connection.execute(
        "INSERT INTO pupils (first_name, surname) VALUES (?1, ?2)",
        params![first_name, clean_surname(surname)],
    )?;
    let pupil_id = connection.last_insert_rowid();
    enroll(connection, class_id, pupil_id)?;
    log::info!("added new pupil {pupil_id} to class {class_id}");
    Ok(pupil_id)
}

/// Every pupil whose full name (first name and surname) is `full_name`, ignoring case (in any
/// alphabet, unlike SQLite's NOCASE) and outer spaces: "Ana Pérez" finds Ana / Pérez, and also a
/// pupil whose whole name is still in the first name.
pub fn pupils_named(connection: &Connection, full_name: &str) -> Result<Vec<Pupil>> {
    let key = name_match_key(full_name);
    let mut pupils = query_pupils(connection, "SELECT id, first_name, surname FROM pupils ORDER BY id", [])?;
    pupils.retain(|pupil| name_match_key(&pupil.full_name()) == key);
    Ok(pupils)
}

pub fn add_existing_pupil(connection: &Connection, class_id: i64, pupil_id: i64) -> Result<()> {
    enroll(connection, class_id, pupil_id)?;
    log::info!("added pupil {pupil_id} to class {class_id}");
    Ok(())
}

/// The pupil as stored, with both names.
pub fn pupil_name(connection: &Connection, pupil_id: i64) -> Result<Pupil> {
    Ok(connection.query_row("SELECT id, first_name, surname FROM pupils WHERE id = ?1", [pupil_id], pupil_from_row)?)
}

/// Sets the pupil's first name everywhere (pupils are shared between classes); an empty one is
/// refused with `EmptyName`.
pub fn set_first_name(connection: &Connection, pupil_id: i64, text: &str) -> Result<()> {
    let first_name = clean_name(text)?;
    update_one(connection, "UPDATE pupils SET first_name = ?2 WHERE id = ?1", params![pupil_id, first_name])?;
    log::info!("changed the first name of pupil {pupil_id}");
    Ok(())
}

/// Sets the pupil's surname everywhere; an empty text clears it.
pub fn set_surname(connection: &Connection, pupil_id: i64, text: &str) -> Result<()> {
    update_one(connection, "UPDATE pupils SET surname = ?2 WHERE id = ?1", params![pupil_id, clean_surname(text)])?;
    log::info!("changed the surname of pupil {pupil_id}");
    Ok(())
}

/// Takes the pupil out of one class (their marks there go too); they stay in other classes.
pub fn remove_from_class(connection: &Connection, class_id: i64, pupil_id: i64) -> Result<()> {
    update_one(
        connection,
        "DELETE FROM class_pupils WHERE class_id = ?1 AND pupil_id = ?2",
        params![class_id, pupil_id],
    )?;
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

fn query_pupils(connection: &Connection, sql: &str, parameters: impl Params) -> Result<Vec<Pupil>> {
    let mut statement = connection.prepare(sql)?;
    let pupils = statement.query_map(parameters, pupil_from_row)?.collect::<rusqlite::Result<_>>()?;
    Ok(pupils)
}

/// A pupil from a row of `id, first_name, surname`.
fn pupil_from_row(row: &rusqlite::Row) -> rusqlite::Result<Pupil> {
    Ok(Pupil { id: row.get(0)?, first_name: row.get(1)?, surname: row.get(2)? })
}
