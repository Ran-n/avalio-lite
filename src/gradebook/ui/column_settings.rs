// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 01:29:26.244187
//+ Revised: 	2026/11/24 17:54:05.040697
// -----------------------------------------------------------------------------

//! Wires the callbacks that change a column's settings: its width, bonus, title, weight, maximum
//! score, decimals and the column it rounds. Adding, moving and deleting columns are in `columns`.

use slint::{ComponentHandle, Model, SharedString};

use super::items::round_sources;
use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::error::AppError;
use crate::gradebook::columns;
use crate::ui::{AppWindow, Gradebook};
use crate::window::with_window2;

/// Wires the column settings callbacks, in the order they were always wired.
pub fn wire(window: &AppWindow, state: &SharedState) {
    wire_set_column_width(window, state);
    wire_set_bonus(window, state);
    wire_rename_column(window, state);
    wire_set_weight(window, state);
    wire_set_max_score(window, state);
    wire_set_decimals(window, state);
    wire_set_round_source(window, state);
}

/// A width the sheet sends, in logical pixels: 0 (or less) means the default.
fn stored_width(width: i32) -> Option<u16> {
    (width > 0).then(|| u16::try_from(width).unwrap_or(u16::MAX))
}

/// Stores a column's width as the sheet dragged it; only the widths change on screen.
fn wire_set_column_width(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_set_column_width(with_window2(window, move |window, index, width| {
        let column_id = usize::try_from(index)
            .ok()
            .and_then(|index| window.global::<Gradebook>().get_columns().row_data(index))
            .map(|column| column.id);
        run(window, &shared, "set column width", |connection| {
            let Some(column_id) = column_id else { return Err(AppError::NotEditable) };
            columns::set_column_width(connection, i64::from(column_id), stored_width(width))?;
            Ok(Refresh::Widths)
        });
    }));
}

/// Sets a column's bonus flag.
fn wire_set_bonus(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_set_bonus(with_window2(window, move |window, column_id, bonus| {
        run(window, &shared, "set bonus test", |connection| {
            columns::set_bonus(connection, i64::from(column_id), bonus)?;
            Ok(Refresh::Columns)
        });
    }));
}

/// Renames a column as typed.
fn wire_rename_column(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_rename_column(with_window2(
        window,
        move |window, column_id, title: SharedString| {
            run(window, &shared, "rename column", |connection| {
                columns::rename_column(connection, i64::from(column_id), &title)?;
                Ok(Refresh::Columns)
            });
        },
    ));
}

/// Sets a column's weight as typed.
fn wire_set_weight(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_set_weight(with_window2(window, move |window, column_id, text: SharedString| {
        run(window, &shared, "set weight", |connection| {
            columns::set_weight(connection, i64::from(column_id), &text)?;
            Ok(Refresh::Columns)
        });
    }));
}

/// Sets a column's maximum score as typed.
fn wire_set_max_score(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_set_max_score(with_window2(
        window,
        move |window, column_id, text: SharedString| {
            run(window, &shared, "set maximum score", |connection| {
                columns::set_max_score(connection, i64::from(column_id), &text)?;
                Ok(Refresh::Columns)
            });
        },
    ));
}

/// Sets how many decimals a column shows, as typed.
fn wire_set_decimals(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_set_decimals(with_window2(window, move |window, column_id, text: SharedString| {
        run(window, &shared, "set decimals", |connection| {
            columns::set_decimals(connection, i64::from(column_id), &text)?;
            Ok(Refresh::Columns)
        });
    }));
}

/// Picks the column a rounded column takes its values from, by its place among the sources.
fn wire_set_round_source(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_set_round_source(with_window2(window, move |window, column_id, index| {
        let class_id = current_class_id(window);
        run(window, &shared, "set rounded column source", |connection| {
            let all = columns::list_columns(connection, class_id)?;
            let source =
                usize::try_from(index).ok().and_then(|index| round_sources(&all).nth(index)).map(|column| column.id);
            columns::set_round_source(connection, i64::from(column_id), source)?;
            Ok(Refresh::Columns)
        });
    }));
}

#[cfg(test)]
mod tests {
    use super::stored_width;

    #[test]
    fn sheet_widths_become_stored_widths() {
        assert_eq!(stored_width(0), None, "0 is the default");
        assert_eq!(stored_width(-5), None);
        assert_eq!(stored_width(208), Some(208));
        assert_eq!(stored_width(100_000), Some(u16::MAX), "too wide is clamped later, not dropped");
    }
}
