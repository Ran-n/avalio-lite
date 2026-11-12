// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.853714
//+ Revised: 	2026/11/11 17:10:57.889217
// -----------------------------------------------------------------------------

//! Adding a backup's classes to the open database, keeping all that is there.

use std::collections::{HashMap, HashSet};

use rusqlite::Connection;

use crate::error::Result;
use crate::gradebook::columns::copy_columns;
use crate::gradebook::model::{Pupil, Sheet};
use crate::gradebook::{cells, classes, pupils, sheet};

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
/// pupil already here with the same full name; with none or several, a new pupil is made. A pupil the
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
    /// Adds the class and returns its id here.
    fn add_class(&mut self, connection: &Connection, name: &str, sheet: &Sheet) -> Result<i64> {
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
                if column.kind.holds_cells() && !value.is_empty() {
                    cells::set_cell(connection, columns[&column.id], pupil_id, value)?;
                }
            }
        }
        Ok(class_id)
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
        let here: Vec<Pupil> = pupils::pupils_named(connection, &pupil.full_name())?
            .into_iter()
            .filter(|found| !self.claimed.contains(&found.id))
            .collect();
        let id = match here.as_slice() {
            [only] => {
                pupils::add_existing_pupil(connection, class_id, only.id)?;
                self.summary.existing_pupils += 1;
                only.id
            }
            _ => pupils::insert_new_pupil(connection, class_id, &pupil.first_name, &pupil.surname)?,
        };
        self.pupils.insert(pupil.id, id);
        self.claimed.insert(id);
        Ok(id)
    }
}
