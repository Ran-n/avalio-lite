// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.872870
//+ Revised: 	2026/11/25 11:43:12.507833
// -----------------------------------------------------------------------------

//! The real app in its real window: started through the driver (`examples/app_driver/`) on scratch
//! data, clicked and typed into through Windows messages, and checked pixel by pixel.
//!
//! These tests open windows, so they are left out of the default run:
//!   cargo test --test desktop -- --ignored
//! Every screen is checked in light and dark, at the normal and the minimum window size, and in
//! Galician (the longer language) at the minimum size, against reviewed baselines in
//! `tests/desktop/baselines/`. After a deliberate change to the look:
//!   $env:AVALIO_UPDATE_BASELINES = "1"; cargo test --test desktop -- --ignored
//! then open every rewritten baseline, check nothing is clipped, wrapped or cut off, and commit.
//!
//! One module per feature. `fixtures` (the settings and the sample class a test starts from) and
//! `screens` (the looks a screen is captured in) are shared; the page moves and the driving are in
//! `desktop_support`. A test that needs another's leftovers doesn't exist: each sets up its own.

#[path = "../common/mod.rs"]
mod common;
#[path = "../desktop_support/mod.rs"]
mod desktop_support;

mod about;
mod add_column;
mod backup;
mod classes;
mod column_panel;
mod first_run;
mod fixtures;
mod headers;
mod keyboard;
mod licence;
mod lock;
mod menus;
mod placement;
mod pupil_menu;
mod pupil_view;
mod real_dialogs;
mod screens;
mod settings;
mod sheet;
mod sheet_editing;
mod sheet_menu;
mod sidebar;
mod small_screens;
mod test_window;
mod update_banner;
mod update_install;
