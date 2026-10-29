// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.946623
//+ Revised: 	2026/10/29 04:09:17.394948
// -----------------------------------------------------------------------------

use std::path::{Path, PathBuf};

use slint::SharedString;

use crate::app::state::Opened;
use crate::app::{
    SharedDialogs, SharedState, gradebook, language, lock_screen, shortcuts, theme, view, with_window, with_window1,
};
use crate::error::Result;
use crate::i18n::{self, Language, Message};
use crate::ui::AppWindow;

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_setup_browsing(window, dialogs);
    wire_setup_actions(window, state, dialogs);
    wire_settings(window, state, dialogs);
    wire_language(window, state);
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
        let Some(database_path) = dialogs.pick_database_file() else { return };
        let log_dir = as_path(&window.get_setup_log_folder());
        let result = shared.borrow_mut().open_existing(&database_path, &log_dir);
        after_database_opened(window, &shared, "open database", result);
    }));
}

fn wire_settings(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, picker) = (state.clone(), dialogs.clone());
    window.on_move_database(with_window(window, move |window| {
        let start = as_path(&window.get_database_path()).parent().map(Path::to_path_buf).unwrap_or_default();
        let Some(folder) = picker.pick_folder(&start) else { return };
        let result = shared.borrow_mut().move_database(&folder);
        let moved_to = result.as_ref().ok().map(|path| path.display().to_string());
        after_data_change(window, &shared, "move database", result.map(|_| ()));
        if let Some(path) = moved_to {
            view::show_status(window, i18n::text(i18n::current(), &Message::DatabaseMoved { path: &path }));
        }
    }));
    let (shared, picker) = (state.clone(), dialogs.clone());
    window.on_move_logs(with_window(window, move |window| {
        let Some(folder) = picker.pick_folder(&as_path(&window.get_log_folder())) else { return };
        let result = shared.borrow_mut().move_logs(&folder);
        let succeeded = result.is_ok();
        after_data_change(window, &shared, "move logs", result);
        if succeeded {
            let path = folder.display().to_string();
            view::show_status(window, i18n::text(i18n::current(), &Message::LogsMoved { path: &path }));
        }
    }));
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.on_switch_database(with_window(window, move |window| {
        let Some(database_path) = dialogs.pick_database_file() else { return };
        let log_dir = as_path(&window.get_log_folder());
        let result = shared.borrow_mut().open_existing(&database_path, &log_dir);
        after_database_opened(window, &shared, "open database", result);
    }));
}

fn wire_language(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.on_choose_language(with_window1(window, move |window, code: SharedString| {
        let Some(language) = Language::from_code(&code) else {
            log::error!("unknown language code \"{code}\" from the language picker");
            return;
        };
        let result = shared.borrow_mut().set_language(language);
        if let Err(error) = result {
            log::warn!("saving the language failed: {error}");
            view::show_error(window, &error);
            return;
        }
        language::apply(window, language);
        view::clear_messages(window);
        shortcuts::show(window, &shared);
        // numbers in the sheet carry the language's decimal separator, so redraw it
        gradebook::redraw(window, &shared);
    }));

    let shared = state.clone();
    window.on_choose_theme(with_window1(window, move |window, dark| {
        theme::apply(window, dark);
        if let Err(error) = shared.borrow_mut().set_theme(theme::choice(dark)) {
            log::warn!("saving the theme failed: {error}");
            view::show_error(window, &error);
        }
    }));
}

/// Shows the main screen with the new locations, or the error on the current screen. A failure
/// can come after the switch (e.g. the old file couldn't be deleted), so the shown locations are
/// refreshed either way.
fn after_data_change(window: &AppWindow, state: &SharedState, action: &str, result: Result<()>) {
    if let Err(error) = result {
        log::warn!("{action} failed: {error}");
        if let Some(config) = state.borrow().config() {
            view::show_locations(window, config);
        }
        view::show_error(window, &error);
        return;
    }
    if let Some(config) = state.borrow().config() {
        view::show_main(window, config);
    }
}

/// Like `after_data_change`, then shows the newly chosen database: its classes, or the lock
/// screen when it is encrypted and needs its PIN first.
fn after_database_opened(window: &AppWindow, state: &SharedState, action: &str, result: Result<Opened>) {
    let opened = result.as_ref().ok().copied();
    after_data_change(window, state, action, result.map(|_| ()));
    match opened {
        Some(Opened::Ready) => {
            lock_screen::show_unprotected(window);
            gradebook::show_database(window, state);
        }
        Some(Opened::Locked) => lock_screen::show_locked(window),
        None => {}
    }
}

fn as_path(text: &SharedString) -> PathBuf {
    PathBuf::from(text.trim())
}
