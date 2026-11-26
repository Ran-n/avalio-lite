// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 08:27:15.129951
//+ Revised: 	2026/11/22 21:06:22.927461
// -----------------------------------------------------------------------------

//! Plumbing every feature uses: the settings file, the database, the log file, safe file moves.

pub mod config;
pub mod db;
pub mod logging;
// the desktop tests' guard against personal data on screen; never in a release build
#[cfg(debug_assertions)]
pub mod personal;
pub mod storage;
