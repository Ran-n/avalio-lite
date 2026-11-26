// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.996600
//+ Revised: 	2026/11/25 11:43:11.105605
// -----------------------------------------------------------------------------

//! The database file: opening and creating it, encryption, export and verification, the migrations,
//! the schema rules and the stored column settings. One module per topic.

mod columns;
#[path = "../common/mod.rs"]
mod common;
mod export_verify;
mod migrations;
mod name_migrations;
mod open;
mod pupil_pictures;
mod round_links;
mod round_migration;
mod schema;
mod support;
