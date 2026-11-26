// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.370989
//+ Revised: 	2026/11/25 11:43:16.681868
// -----------------------------------------------------------------------------

//! The real window, wired as in the app, driven on Slint's headless backend: elements are found
//! by their accessible labels and clicked with pointer events, and text is typed as key events.
//! Dialogs are answered by `FakeDialogs`; data is checked in the database the app has open.
//!
//! One module per feature; `driver`, `fixtures`, `screens` and `pupil_screens` hold the helpers they
//! share.

mod about;
mod background;
mod backup;
mod cells;
mod classes;
mod column_drag;
mod column_menus;
mod column_panel;
mod column_widths;
mod columns;
#[path = "../common/mod.rs"]
mod common;
mod driver;
mod excel;
mod fields;
mod fixtures;
mod language;
mod layout;
mod licence_running;
mod licence_screen;
mod locations;
mod lock;
mod messages;
mod pupil_panel;
mod pupil_screens;
mod pupil_view;
mod pupil_view_meanwhile;
mod pupil_view_menu;
mod pupil_view_pin;
mod pupil_view_shortcuts;
mod pupil_view_typing;
mod pupils;
mod restore;
mod saved;
mod screens;
mod scroll;
mod shortcuts;
mod sidebar;
mod startup;
mod update_banner;
mod update_restart;
mod update_schedule;
mod update_under_way;
