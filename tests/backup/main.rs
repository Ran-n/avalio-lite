// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:36.769629
//+ Revised: 	2026/11/25 11:43:10.529466
// -----------------------------------------------------------------------------

//! Backups: making one, opening a backup file, restoring it and adding its classes to the open database.
//! One module per topic.

mod add_classes;
mod backup;
#[path = "../common/mod.rs"]
mod common;
mod open_backup;
mod pictures;
mod restore;
mod support;
