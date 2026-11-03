// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:39:46.617467
//+ Revised: 	2026/11/03 11:39:46.617467
// -----------------------------------------------------------------------------

//! Writing a checked import plan into the class.

use rusqlite::Connection;

use super::plan::Target;
use super::{ImportPlan, ImportSummary};
use crate::error::Result;
use crate::gradebook::{cells, pupils};

/// Writes the plan in one transaction: any failure leaves the database as it was.
pub fn apply_import(connection: &Connection, class_id: i64, plan: &ImportPlan) -> Result<ImportSummary> {
    let transaction = connection.unchecked_transaction()?;
    for pupil in &plan.pupils {
        let pupil_id = match &pupil.target {
            Target::InClass(pupil_id) => *pupil_id,
            Target::Enroll(pupil_id) => {
                pupils::add_existing_pupil(&transaction, class_id, *pupil_id)?;
                *pupil_id
            }
            Target::New(name) => pupils::insert_new_pupil(&transaction, class_id, name)?,
        };
        for (column_id, value) in &pupil.writes {
            cells::set_cell(&transaction, *column_id, pupil_id, value)?;
        }
    }
    transaction.commit()?;
    let ImportSummary { pupils, new_pupils, marks, .. } = &plan.summary;
    log::info!("imported {pupils} pupils ({new_pupils} new) and {marks} values into class {class_id}");
    Ok(plan.summary.clone())
}
