// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.332042
//+ Revised: 	2026/10/28 10:52:41.240933
// -----------------------------------------------------------------------------

pub mod app;
pub mod config;
pub mod db;
pub mod error;
pub mod gradebook;
pub mod i18n;
pub mod lock;
pub mod logging;
pub mod storage;

pub mod ui {
    slint::include_modules!();
}
