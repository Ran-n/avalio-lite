// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.139186
//+ Revised: 	2026/11/03 11:13:31.539517
// -----------------------------------------------------------------------------

mod cells;
mod classes;
mod columns;
mod items;
mod panel;
mod pupils;
pub mod view;

use rusqlite::Connection;
use slint::ComponentHandle;

use crate::error::{AppError, Result};
use crate::shortcuts;
use crate::state::AppState;
use crate::ui::{AppWindow, Gradebook};
use crate::window::report_failure;
use crate::window::{SharedDialogs, SharedState, view as window_view, with_window};
use view::Refresh;

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    classes::wire(window, state, dialogs);
    pupils::wire(window, state, dialogs);
    columns::wire(window, state, dialogs);
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
        let toggle = Toggle {
            what: "column settings",
            get: |gradebook| gradebook.get_show_column_settings(),
            set: |gradebook, show| gradebook.set_show_column_settings(show),
            save: AppState::set_show_column_settings,
            // the headers change only once the choice is saved
            shown_before_saving: false,
        };
        toggle.run(window, &shared);
    }));
    let shared = state.clone();
    window.global::<Gradebook>().on_toggle_sidebar(with_window(window, move |window| {
        let toggle = Toggle {
            what: "sidebar",
            get: |gradebook| gradebook.get_sidebar_collapsed(),
            set: |gradebook, collapsed| gradebook.set_sidebar_collapsed(collapsed),
            save: AppState::set_sidebar_collapsed,
            // the sidebar still folds when only remembering the choice failed
            shown_before_saving: true,
        };
        toggle.run(window, &shared);
    }));
}

/// A view preference flipped from the window and saved in the settings file.
struct Toggle {
    what: &'static str,
    get: fn(&Gradebook) -> bool,
    set: fn(&Gradebook, bool),
    save: fn(&mut AppState, bool) -> Result<()>,
    /// Whether the window shows the new value even if saving it fails.
    shown_before_saving: bool,
}

impl Toggle {
    fn run(&self, window: &AppWindow, state: &SharedState) {
        let gradebook = window.global::<Gradebook>();
        let value = !(self.get)(&gradebook);
        window_view::clear_messages(window);
        if self.shown_before_saving {
            (self.set)(&gradebook, value);
        }
        match (self.save)(&mut state.borrow_mut(), value) {
            Ok(()) => (self.set)(&gradebook, value),
            Err(error) => {
                report_failure!(window, format!("saving the {} preference", self.what), &error);
            }
        }
    }
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
    window.global::<Gradebook>().set_sidebar_collapsed(preferences.sidebar_collapsed);
    let loaded = run(window, state, "load classes", |connection| {
        view::show_new_database(window, connection)?;
        Ok(Refresh::Nothing)
    });
    shortcuts::ui::show(window, state);
    // after the load, whose success clears the banner
    shortcuts::ui::report_problems(window, state);
    loaded
}

/// Runs one database action, redraws what it changed, and shows or clears the error banner of the
/// page that is open. The status of an earlier action is cleared first, so it never sits next to
/// a new error. True when the action succeeded.
pub(crate) fn run(
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
            report_failure!(window, action, &error);
            false
        }
    }
}
