// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.012344
//+ Revised: 	2026/11/23 16:34:34.366191
// -----------------------------------------------------------------------------

//! Adding, moving and deleting columns from the sheet's header. A column's own settings are in
//! `column_settings`, the panel that shows them in `panel`, the rules in `gradebook::columns`.

use slint::{ComponentHandle, Model};

use super::items::from_ui_type;
use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::error::AppError;
use crate::gradebook::columns::{self, Direction};
use crate::i18n::{self, Message, SheetMessage};
use crate::ui::{AppWindow, Gradebook};
use crate::window::{SharedDialogs, with_window1, with_window2};

/// Wires the column structure callbacks: adding, moving and deleting columns. The settings of a
/// column are wired by `column_settings`.
pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_add_column(window, state);
    wire_move_column(window, state);
    wire_move_column_to(window, state);
    wire_delete_column(window, state, dialogs);
}

/// Adds a column of the type picked to the open class.
fn wire_add_column(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_add_column(with_window1(window, move |window, column_type| {
        let class_id = current_class_id(window);
        run(window, &shared, "add column", |connection| {
            columns::add_column(connection, class_id, from_ui_type(column_type), i18n::current())?;
            Ok(Refresh::Sheet)
        });
    }));
}

/// Moves a column one place left or right, by the sign of `step`.
fn wire_move_column(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_move_column(with_window2(window, move |window, column_id, step| {
        let direction = if step < 0 { Direction::Left } else { Direction::Right };
        run(window, &shared, "move column", |connection| {
            columns::move_column(connection, i64::from(column_id), direction)?;
            Ok(Refresh::Sheet)
        });
    }));
}

/// Moves a column to the place it was dropped on.
fn wire_move_column_to(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_move_column_to(with_window2(window, move |window, column_id, index| {
        run(window, &shared, "move column", |connection| {
            // a place before the first can't come from the sheet; it is reported, not moved to the front
            let index = usize::try_from(index).map_err(|_| AppError::NotEditable)?;
            columns::move_column_to(connection, i64::from(column_id), index)?;
            Ok(Refresh::Sheet)
        });
    }));
}

/// Deletes a column, once confirmed.
fn wire_delete_column(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.global::<Gradebook>().on_delete_column(with_window1(window, move |window, column_id| {
        let title = window
            .global::<Gradebook>()
            .get_columns()
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.title);
        // a column the sheet no longer shows: said, rather than asking about a nameless one
        let Some(title) = title else {
            run(window, &shared, "delete column", |_| Err(AppError::NotEditable));
            return;
        };
        if !dialogs.confirm(&i18n::text(
            i18n::current(),
            &Message::Sheet(SheetMessage::ConfirmDeleteColumn { column: &title }),
        )) {
            return;
        }
        run(window, &shared, "delete column", |connection| {
            columns::delete_column(connection, i64::from(column_id))?;
            Ok(Refresh::Sheet)
        });
    }));
}
