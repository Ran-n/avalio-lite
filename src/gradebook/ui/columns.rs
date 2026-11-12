// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.012344
//+ Revised: 	2026/11/11 17:10:59.325790
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, Model, SharedString};

use super::items::{from_ui_type, round_sources};
use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::error::AppError;
use crate::gradebook::columns::{self, Direction};
use crate::i18n::{self, Message};
use crate::ui::{AppWindow, Gradebook};
use crate::window::{SharedDialogs, with_window1, with_window2};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_structure(window, state, dialogs);
    wire_settings(window, state);
}

fn wire_structure(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let gradebook = window.global::<Gradebook>();

    let shared = state.clone();
    gradebook.on_add_column(with_window1(window, move |window, column_type| {
        let class_id = current_class_id(window);
        run(window, &shared, "add column", |connection| {
            columns::add_column(connection, class_id, from_ui_type(column_type), i18n::current())?;
            Ok(Refresh::Sheet)
        });
    }));

    let shared = state.clone();
    gradebook.on_move_column(with_window2(window, move |window, column_id, step| {
        let direction = if step < 0 { Direction::Left } else { Direction::Right };
        run(window, &shared, "move column", |connection| {
            columns::move_column(connection, i64::from(column_id), direction)?;
            Ok(Refresh::Sheet)
        });
    }));

    let shared = state.clone();
    gradebook.on_move_column_to(with_window2(window, move |window, column_id, index| {
        run(window, &shared, "move column", |connection| {
            columns::move_column_to(connection, i64::from(column_id), usize::try_from(index).unwrap_or(0))?;
            Ok(Refresh::Sheet)
        });
    }));

    let (shared, dialogs) = (state.clone(), dialogs.clone());
    gradebook.on_delete_column(with_window1(window, move |window, column_id| {
        let title = window
            .global::<Gradebook>()
            .get_columns()
            .iter()
            .find(|column| column.id == column_id)
            .map(|column| column.title)
            .unwrap_or_default();
        if !dialogs.confirm(&i18n::text(i18n::current(), &Message::ConfirmDeleteColumn { column: &title })) {
            return;
        }
        run(window, &shared, "delete column", |connection| {
            columns::delete_column(connection, i64::from(column_id))?;
            Ok(Refresh::Sheet)
        });
    }));
}

fn wire_settings(window: &AppWindow, state: &SharedState) {
    let gradebook = window.global::<Gradebook>();
    wire_widths(window, state);

    let shared = state.clone();
    gradebook.on_set_bonus(with_window2(window, move |window, column_id, bonus| {
        run(window, &shared, "set bonus test", |connection| {
            columns::set_bonus(connection, i64::from(column_id), bonus)?;
            Ok(Refresh::Columns)
        });
    }));

    let shared = state.clone();
    gradebook.on_rename_column(with_window2(window, move |window, column_id, title: SharedString| {
        run(window, &shared, "rename column", |connection| {
            columns::rename_column(connection, i64::from(column_id), &title)?;
            Ok(Refresh::Columns)
        });
    }));

    let shared = state.clone();
    gradebook.on_set_weight(with_window2(window, move |window, column_id, text: SharedString| {
        run(window, &shared, "set weight", |connection| {
            columns::set_weight(connection, i64::from(column_id), &text)?;
            Ok(Refresh::Columns)
        });
    }));

    let shared = state.clone();
    gradebook.on_set_max_score(with_window2(window, move |window, column_id, text: SharedString| {
        run(window, &shared, "set maximum score", |connection| {
            columns::set_max_score(connection, i64::from(column_id), &text)?;
            Ok(Refresh::Columns)
        });
    }));

    let shared = state.clone();
    gradebook.on_set_decimals(with_window2(window, move |window, column_id, text: SharedString| {
        run(window, &shared, "set decimals", |connection| {
            columns::set_decimals(connection, i64::from(column_id), &text)?;
            Ok(Refresh::Columns)
        });
    }));

    let shared = state.clone();
    gradebook.on_set_round_source(with_window2(window, move |window, column_id, index| {
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

/// A width the sheet sends, in logical pixels: 0 (or less) means the default.
fn stored_width(width: i32) -> Option<u16> {
    (width > 0).then(|| u16::try_from(width).unwrap_or(u16::MAX))
}

fn wire_widths(window: &AppWindow, state: &SharedState) {
    let gradebook = window.global::<Gradebook>();

    let shared = state.clone();
    gradebook.on_set_column_width(with_window2(window, move |window, index, width| {
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
