// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:35.519994
//+ Revised: 	2026/11/25 11:43:13.910560
// -----------------------------------------------------------------------------

//! The gradebook model: classes, columns and their settings, cells, names and the storage errors.
//! One module per topic.

mod cells;
mod classes;
mod column_order;
mod column_settings;
mod column_widths;
#[path = "../common/mod.rs"]
mod common;
mod errors;
mod names;
mod pictures;
mod refused_pictures;
mod rounding;
mod stored_pictures;
mod support;
