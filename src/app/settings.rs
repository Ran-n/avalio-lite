// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 22:53:31.368093
//+ Revised: 	2026/11/12 14:39:51.581038
// -----------------------------------------------------------------------------

//! The Settings screen: language, theme, moving the data, switching database.

use std::path::{Path, PathBuf};
use std::rc::Rc;

use slint::SharedString;

use crate::error::Result;
use crate::i18n::{self, Language, Message};
use crate::state::Opened;
use crate::ui::AppWindow;
use crate::window::dialogs::FileKind;
use crate::window::report_failure;
use crate::window::{SharedDialogs, SharedState, language, theme, view, with_window, with_window1};
use crate::{gradebook, lock, shortcuts};

/// Rewords, in the language just chosen, texts the Rust side built in the old one and still shows
/// (the update bar, the licence card); passed in by `app`, which owns those parts.
pub type Reword = Rc<dyn Fn(&AppWindow)>;

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs, reword: Reword) {
    wire_settings(window, state, dialogs);
    wire_language(window, state, reword);
}

fn wire_settings(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, picker) = (state.clone(), dialogs.clone());
    window.on_move_database(with_window(window, move |window| {
        let start = as_path(&window.get_database_path()).parent().map(Path::to_path_buf).unwrap_or_default();
        let Some(folder) = picker.pick_folder(&start) else { return };
        let result = shared.borrow_mut().move_database(&folder);
        let moved = result.as_ref().ok().cloned();
        after_data_change(window, &shared, "move database", result.map(|_| ()));
        if let Some(moved) = moved {
            let path = moved.path.display().to_string();
            match moved.left_behind {
                None => view::show_status(window, i18n::text(i18n::current(), &Message::DatabaseMoved { path: &path })),
                Some(old) => {
                    let old = old.display().to_string();
                    let text = i18n::text(i18n::current(), &Message::DatabaseMovedOldKept { path: &path, old: &old });
                    view::show_error_text(window, text);
                }
            }
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
        let Some(database_path) = dialogs.pick_file(FileKind::Database) else { return };
        let log_dir = as_path(&window.get_log_folder());
        let result = shared.borrow_mut().open_existing(&database_path, &log_dir);
        after_database_opened(window, &shared, "open database", result);
    }));
}

fn wire_language(window: &AppWindow, state: &SharedState, reword: Reword) {
    let shared = state.clone();
    window.on_choose_language(with_window1(window, move |window, code: SharedString| {
        let Some(language) = Language::from_code(&code) else {
            log::error!("unknown language code \"{code}\" from the language picker");
            return;
        };
        let result = shared.borrow_mut().set_language(language);
        if let Err(error) = result {
            report_failure!(window, "saving the language", &error);
            return;
        }
        language::apply(window, language);
        view::clear_messages(window);
        shortcuts::ui::show(window, &shared);
        // numbers in the sheet carry the language's decimal separator, so redraw it
        gradebook::ui::redraw(window, &shared);
        reword(window);
    }));

    let shared = state.clone();
    window.on_choose_theme(with_window1(window, move |window, dark| {
        // saved first, like the language: a theme that couldn't be saved would be gone at the next
        // start, so the screen keeps the one that is saved and says why
        if let Err(error) = shared.borrow_mut().set_theme(theme::choice(dark)) {
            report_failure!(window, "saving the theme", &error);
            return;
        }
        theme::apply(window, dark);
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
pub(super) fn after_database_opened(window: &AppWindow, state: &SharedState, action: &str, result: Result<Opened>) {
    let opened = result.as_ref().ok().copied();
    after_data_change(window, state, action, result.map(|_| ()));
    match opened {
        Some(Opened::Ready) => {
            lock::ui::show_unprotected(window);
            gradebook::ui::show_database(window, state);
        }
        Some(Opened::Locked) => lock::ui::show_locked(window),
        None => {}
    }
}

pub(super) fn as_path(text: &SharedString) -> PathBuf {
    PathBuf::from(text.trim())
}
