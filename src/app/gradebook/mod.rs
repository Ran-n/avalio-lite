// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.139186
//+ Revised: 	2026/10/29 15:46:51.328417
// -----------------------------------------------------------------------------

mod cells;
mod classes;
mod columns;
mod panel;
mod pupils;
mod spreadsheet;
pub mod view;

use rusqlite::Connection;
use slint::{ComponentHandle, SharedString};

use crate::app::{SharedDialogs, SharedState, shortcuts, with_window};
use crate::error::{AppError, Result};
use crate::i18n;
use crate::ui::{AppWindow, Gradebook};
use view::Refresh;

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    classes::wire(window, state, dialogs);
    pupils::wire(window, state, dialogs);
    columns::wire(window, state, dialogs);
    spreadsheet::wire(window, state, dialogs);
    cells::wire(window, state);
    panel::wire(window, state);
    wire_preferences(window, state);
}

fn wire_preferences(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_toggle_column_settings(with_window(window, move |window| {
        let gradebook = window.global::<Gradebook>();
        let show = !gradebook.get_show_column_settings();
        gradebook.set_status_message(SharedString::new());
        match shared.borrow_mut().set_show_column_settings(show) {
            Ok(()) => {
                gradebook.set_show_column_settings(show);
                gradebook.set_error_message(SharedString::new());
            }
            Err(error) => {
                log::warn!("saving the column settings preference failed: {error}");
                gradebook.set_error_message(i18n::error_message(i18n::current(), &error).into());
            }
        }
    }));
}

/// Redraws the class list and the open sheet in place, e.g. after the language changes.
pub fn redraw(window: &AppWindow, state: &SharedState) {
    redraw_selecting(window, state, None);
}

/// Redraws the class list and opens class `select` (the open class stays when `None`).
pub fn redraw_selecting(window: &AppWindow, state: &SharedState, select: Option<i64>) {
    run(window, state, "redraw sheet", |_| Ok(Refresh::All { select }));
}

/// Loads the classes of a freshly opened database; errors show in the sheet's banner.
pub fn show_database(window: &AppWindow, state: &SharedState) {
    let preferences = state.borrow().preferences();
    window.global::<Gradebook>().set_show_column_settings(preferences.show_column_settings);
    run(window, state, "load classes", |connection| {
        view::show_new_database(window, connection)?;
        Ok(Refresh::Nothing)
    });
    shortcuts::show(window, state);
    // after the load, whose success clears the banner
    shortcuts::report_problems(window, state);
}

/// Runs one database action, redraws what it changed, and shows or clears the error banner.
/// The status of an earlier action is cleared first, so it never sits next to a new error.
/// True when the action succeeded.
fn run(
    window: &AppWindow,
    state: &SharedState,
    action: &str,
    task: impl FnOnce(&Connection) -> Result<Refresh>,
) -> bool {
    window.global::<Gradebook>().set_status_message(SharedString::new());
    let result = {
        let state = state.borrow();
        match state.connection() {
            Some(connection) => task(connection).and_then(|refresh| view::apply(window, connection, refresh)),
            None => Err(AppError::NoDatabaseOpen),
        }
    };
    let gradebook = window.global::<Gradebook>();
    match result {
        Ok(()) => {
            gradebook.set_error_message(SharedString::new());
            true
        }
        Err(error) => {
            log::warn!("{action} failed: {error}");
            gradebook.set_error_message(i18n::error_message(i18n::current(), &error).into());
            false
        }
    }
}

/// The green banner on the sheet, until the next sheet action.
fn show_status(window: &AppWindow, text: String) {
    window.global::<Gradebook>().set_status_message(text.into());
}
