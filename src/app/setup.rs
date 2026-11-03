// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 22:53:31.261954
//+ Revised: 	2026/11/02 22:53:31.261954
// -----------------------------------------------------------------------------

//! The first-run screen: choosing the folders, creating or opening a database.

use super::settings::{after_database_opened, as_path};
use crate::ui::AppWindow;
use crate::window::dialogs::FileKind;
use crate::window::{SharedDialogs, SharedState, view, with_window};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_setup_browsing(window, dialogs);
    wire_setup_actions(window, state, dialogs);
}

fn wire_setup_browsing(window: &AppWindow, dialogs: &SharedDialogs) {
    let picker = dialogs.clone();
    window.on_browse_setup_database_folder(with_window(window, move |window| {
        if let Some(folder) = picker.pick_folder(&as_path(&window.get_setup_database_folder())) {
            window.set_setup_database_folder(view::display(&folder));
        }
    }));
    let dialogs = dialogs.clone();
    window.on_browse_setup_log_folder(with_window(window, move |window| {
        if let Some(folder) = dialogs.pick_folder(&as_path(&window.get_setup_log_folder())) {
            window.set_setup_log_folder(view::display(&folder));
        }
    }));
}

fn wire_setup_actions(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let shared = state.clone();
    window.on_create_database(with_window(window, move |window| {
        let database_dir = as_path(&window.get_setup_database_folder());
        let log_dir = as_path(&window.get_setup_log_folder());
        let result = shared.borrow_mut().create_database(&database_dir, &log_dir);
        after_database_opened(window, &shared, "create database", result);
    }));
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.on_open_existing_database(with_window(window, move |window| {
        let Some(database_path) = dialogs.pick_file(FileKind::Database) else { return };
        let log_dir = as_path(&window.get_setup_log_folder());
        let result = shared.borrow_mut().open_existing(&database_path, &log_dir);
        after_database_opened(window, &shared, "open database", result);
    }));
}
