// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.825924
//+ Revised: 	2026/11/23 13:00:34.825924
// -----------------------------------------------------------------------------

//! Spreadsheet export and import (.xlsx, .ods, .xls): cell writing and reading, the import plan and the
//! name columns. One module per topic.

#[path = "../common/mod.rs"]
mod common;
mod export;
mod import_cells;
mod import_files;
mod import_names;
mod import_values;
mod support;
