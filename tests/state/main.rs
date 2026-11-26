// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:37.174421
//+ Revised: 	2026/11/23 13:00:37.174421
// -----------------------------------------------------------------------------

//! App state kept between runs: setup and the remembered preferences, shortcuts, moving the database and
//! the logs, and the copies kept beside the database. One module per topic.

#[path = "../common/mod.rs"]
mod common;
mod database_files;
mod moves;
mod preferences;
mod running;
mod shortcuts;
mod support;
