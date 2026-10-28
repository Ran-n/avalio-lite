// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.139186
//+ Revised: 	2026/10/28 15:53:08.652785
// -----------------------------------------------------------------------------

mod cells;
mod classes;
mod columns;
mod panel;
mod pupils;
pub mod view;

use rusqlite::Connection;
use slint::{ComponentHandle, SharedString};

use crate::app::{SharedDialogs, SharedState};
use crate::error::{AppError, Result};
use crate::i18n;
use crate::ui::{AppWindow, Gradebook};
use view::Refresh;

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    classes::wire(window, state, dialogs);
    pupils::wire(window, state, dialogs);
    columns::wire(window, state, dialogs);
    cells::wire(window, state);
    panel::wire(window, state);
    wire_preferences(window, state);
}

fn wire_preferences(window: &AppWindow, state: &SharedState) {
    let (weak, shared) = (window.as_weak(), state.clone());
    window.global::<Gradebook>().on_toggle_column_settings(move || {
        let Some(window) = weak.upgrade() else { return };
        let gradebook = window.global::<Gradebook>();
        let show = !gradebook.get_show_column_settings();
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
    });
}

/// Redraws the class list and the open sheet in place, e.g. after the language changes.
pub fn redraw(window: &AppWindow, state: &SharedState) {
    run(window, state, "redraw sheet", |_| Ok(Refresh::All { select: None }));
}

/// Loads the classes of a freshly opened database; errors show in the sheet's banner.
pub fn show_database(window: &AppWindow, state: &SharedState) {
    let preferences = state.borrow().preferences();
    window.global::<Gradebook>().set_show_column_settings(preferences.show_column_settings);
    run(window, state, "load classes", |connection| {
        view::show_new_database(window, connection)?;
        Ok(Refresh::Nothing)
    });
}

/// Runs one database action, redraws what it changed, and shows or clears the error banner.
fn run(window: &AppWindow, state: &SharedState, action: &str, task: impl FnOnce(&Connection) -> Result<Refresh>) {
    let result = {
        let state = state.borrow();
        match state.connection() {
            Some(connection) => task(connection).and_then(|refresh| view::apply(window, connection, refresh)),
            None => Err(AppError::NoDatabaseOpen),
        }
    };
    let gradebook = window.global::<Gradebook>();
    match result {
        Ok(()) => gradebook.set_error_message(SharedString::new()),
        Err(error) => {
            log::warn!("{action} failed: {error}");
            gradebook.set_error_message(i18n::error_message(i18n::current(), &error).into());
        }
    }
}
