// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.332042
//+ Revised: 	2026/11/07 20:25:36.353950
// -----------------------------------------------------------------------------

pub mod app;
pub mod backup;
pub mod error;
pub mod gradebook;
pub mod i18n;
pub mod licence;
pub mod lock;
pub mod platform;
pub mod shortcuts;
pub mod spreadsheet;
pub mod state;
pub mod update;
pub mod window;

pub mod ui {
    slint::include_modules!();
}
