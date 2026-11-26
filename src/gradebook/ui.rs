// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.139186
//+ Revised: 	2026/11/25 11:43:05.550666
// -----------------------------------------------------------------------------

//! Wires the gradebook to the window: one module per part of the screen, and here what they share (running
//! an action on the open database and showing its result or its failure). The rules are in the parent.

pub(crate) mod avatars;
mod cells;
mod classes;
mod column_settings;
mod columns;
pub(crate) mod items;
mod panel;
mod pupil_panel;
mod pupils;
pub mod saved;
pub mod view;

use rusqlite::Connection;
use slint::ComponentHandle;

use crate::error::{AppError, Result};
use crate::i18n::{self, Message, SheetMessage};
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
    column_settings::wire(window, state);
    cells::wire(window, state);
    panel::wire(window, state);
    pupil_panel::wire(window, state, dialogs);
    wire_preferences(window, state);
    window.global::<Gradebook>().on_field_restored(with_window(window, |window| {
        window_view::clear_error(window);
        saved::clear_not_saved(window);
    }));
    window.global::<Gradebook>().on_confirm_saved(with_window(window, |window| {
        window_view::show_status(window, i18n::text(i18n::current(), &Message::Sheet(SheetMessage::EverythingSaved)));
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
    let remembered = state.borrow().last_class();
    let loaded = run(window, state, "load classes", |connection| {
        view::show_new_database(window, connection, remembered)?;
        Ok(Refresh::Nothing)
    });
    shortcuts::ui::show(window, state);
    // after the load, whose success clears the banner
    shortcuts::ui::report_problems(window, state);
    loaded
}

/// Runs one database action, redraws what it changed, and shows or clears the error banner of the
/// page that is open. The status of an earlier action is cleared first, so it never sits next to
/// a new error. An action that wrote to the database is acknowledged on the summary line
/// ("Saved"), and one that failed with typed data at stake is marked there too ("Not saved").
/// True when the action succeeded.
pub(crate) fn run(
    window: &AppWindow,
    state: &SharedState,
    action: &str,
    task: impl FnOnce(&Connection) -> Result<Refresh>,
) -> bool {
    window_view::clear_status(window);
    let (result, wrote) = perform(window, state, task);
    match result {
        Ok(()) => {
            window_view::clear_error(window);
            if wrote {
                saved::mark_saved(window);
            }
            remember_open_class(window, state);
            true
        }
        Err(error) => {
            show_failure(window, action, &error);
            false
        }
    }
}

/// Runs `task` on the open database and redraws what it changed. Also tells whether the database
/// was written to, so the summary line can say "Saved". With no database open, the action fails.
fn perform(
    window: &AppWindow,
    state: &SharedState,
    task: impl FnOnce(&Connection) -> Result<Refresh>,
) -> (Result<()>, bool) {
    let state = state.borrow();
    let Some(connection) = state.connection() else {
        return (Err(AppError::NoDatabaseOpen), false);
    };
    let before = connection.total_changes();
    let result = task(connection).and_then(|refresh| view::apply(window, connection, refresh));
    (result, connection.total_changes() > before)
}

/// The class on screen is remembered for the next start, whichever action opened it. A failure to
/// save that is said, but the action itself worked.
fn remember_open_class(window: &AppWindow, state: &SharedState) {
    let class_id = view::current_class_id(window);
    if class_id >= 0
        && let Err(error) = state.borrow_mut().set_last_class(class_id)
    {
        report_failure!(window, "remembering the open class", &error);
    }
}

/// Says that `action` failed, and marks the summary line "Not saved" when typed data was lost.
fn show_failure(window: &AppWindow, action: &str, error: &AppError) {
    report_failure!(window, action, error);
    if saved::leaves_typed_data_unsaved(error) {
        saved::mark_not_saved(window);
    }
}
