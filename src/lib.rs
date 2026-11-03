// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.332042
//+ Revised: 	2026/11/03 08:27:16.084962
// -----------------------------------------------------------------------------

pub mod app;
pub mod backup;
pub mod error;
pub mod gradebook;
pub mod i18n;
pub mod lock;
pub mod platform;
pub mod shortcuts;
pub mod spreadsheet;
pub mod state;
pub mod window;

pub mod ui {
    slint::include_modules!();
}
