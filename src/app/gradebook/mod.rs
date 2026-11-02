// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.139186
//+ Revised: 	2026/11/02 08:27:49.021281
// -----------------------------------------------------------------------------

mod cells;
mod classes;
mod columns;
mod panel;
mod pupils;
mod spreadsheet;
pub mod view;

use rusqlite::Connection;
use slint::ComponentHandle;

use crate::app::{SharedDialogs, SharedState, shortcuts, view as window_view, with_window};
use crate::error::{AppError, Result};
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
    window.global::<Gradebook>().on_field_restored(with_window(window, |window| {
        window_view::clear_error(window);
    }));
}

fn wire_preferences(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_toggle_column_settings(with_window(window, move |window| {
        let gradebook = window.global::<Gradebook>();
        let show = !gradebook.get_show_column_settings();
        window_view::clear_messages(window);
        match shared.borrow_mut().set_show_column_settings(show) {
            Ok(()) => gradebook.set_show_column_settings(show),
            Err(error) => {
                log::warn!("saving the column settings preference failed: {error}");
                window_view::show_error(window, &error);
            }
        }
    }));
}

/// Redraws the class list and the open sheet in place, e.g. after the language changes.
pub fn redraw(window: &AppWindow, state: &SharedState) {
    redraw_selecting(window, state, None);
}

/// Redraws the class list and opens class `select` (the open class stays when `None`). True when
/// it worked; otherwise the error is on screen.
pub fn redraw_selecting(window: &AppWindow, state: &SharedState, select: Option<i64>) -> bool {
    run(window, state, "redraw sheet", |_| Ok(Refresh::All { select }))
}

/// Loads the classes of a freshly opened database; an error shows in the message banner. True
/// when it worked.
pub fn show_database(window: &AppWindow, state: &SharedState) -> bool {
    let preferences = state.borrow().preferences();
    window.global::<Gradebook>().set_show_column_settings(preferences.show_column_settings);
    let loaded = run(window, state, "load classes", |connection| {
        view::show_new_database(window, connection)?;
        Ok(Refresh::Nothing)
    });
    shortcuts::show(window, state);
    // after the load, whose success clears the banner
    shortcuts::report_problems(window, state);
    loaded
}

/// Runs one database action, redraws what it changed, and shows or clears the error banner of the
/// page that is open. The status of an earlier action is cleared first, so it never sits next to
/// a new error. True when the action succeeded.
fn run(
    window: &AppWindow,
    state: &SharedState,
    action: &str,
    task: impl FnOnce(&Connection) -> Result<Refresh>,
) -> bool {
    window_view::clear_status(window);
    let result = {
        let state = state.borrow();
        match state.connection() {
            Some(connection) => task(connection).and_then(|refresh| view::apply(window, connection, refresh)),
            None => Err(AppError::NoDatabaseOpen),
        }
    };
    match result {
        Ok(()) => {
            window_view::clear_error(window);
            true
        }
        Err(error) => {
            log::warn!("{action} failed: {error}");
            window_view::show_error(window, &error);
            false
        }
    }
}
