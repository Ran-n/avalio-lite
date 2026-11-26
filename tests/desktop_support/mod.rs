// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.485818
//+ Revised: 	2026/11/24 17:54:10.149409
// -----------------------------------------------------------------------------

//! The desktop test harness: the real app started through the driver (`examples/app_driver/`), its
//! window driven through Win32 (`window`), and its pixels checked against theme colours and reviewed
//! baselines (`pixels`, `capture`). `App` is what the tests hold: its process and replies (`app`),
//! its window setup (`setup`), clicks and keys (`input`), finders (`locate`) and page-level moves
//! (`pages`). `real_app` runs the shipped exe with its own dialogs, for the one test that needs them.

mod app;
mod binaries;
mod capture;
mod input;
mod locate;
mod pages;
pub mod pixels;
mod real_app;
mod setup;
mod theme;
pub mod window;

use std::path::PathBuf;

pub use app::App;
pub use binaries::turn;
pub use real_app::RealApp;
pub use theme::ThemeColours;

/// The package root (the folder holding Cargo.toml): where `tests/`, the baselines and the failure output sit.
fn manifest_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}
