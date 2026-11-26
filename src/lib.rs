// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.332042
//+ Revised: 	2026/11/25 11:43:07.896037
// -----------------------------------------------------------------------------

//! The whole app as a library, so the tests and the example programs use the same code as the exe. `app`
//! puts the features together; `ui` holds the types generated from `ui/*.slint`.

pub mod about;
pub mod app;
pub mod backup;
pub mod error;
pub mod gradebook;
pub mod i18n;
pub mod licence;
pub mod lock;
pub mod platform;
pub mod pupil_view;
pub mod shortcuts;
pub mod spreadsheet;
pub mod state;
pub mod update;
pub mod window;

pub mod ui {
    slint::include_modules!();
}
