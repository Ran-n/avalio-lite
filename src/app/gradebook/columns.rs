// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.012344
//+ Revised: 	2026/10/28 15:53:08.784561
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, Model};

use super::view::{Refresh, current_class_id, from_ui_type, round_sources};
use super::{SharedState, run};
use crate::app::SharedDialogs;
use crate::gradebook::columns::{self, Direction};
use crate::i18n::{self, Message};
use crate::ui::{AppWindow, Gradebook};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_structure(window, state, dialogs);
    wire_settings(window, state);
}

fn wire_structure(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let gradebook = window.global::<Gradebook>();

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_add_column(move |column_type| {
        let Some(window) = weak.upgrade() else { return };
        let class_id = current_class_id(&window);
        run(&window, &shared, "add column", |connection| {
            columns::add_column(connection, class_id, from_ui_type(column_type), i18n::current())?;
            Ok(Refresh::Sheet)
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_move_column(move |column_id, step| {
        let Some(window) = weak.upgrade() else { return };
        let direction = if step < 0 { Direction::Left } else { Direction::Right };
        run(&window, &shared, "move column", |connection| {
            columns::move_column(connection, i64::from(column_id), direction)?;
            Ok(Refresh::Sheet)
        });
    });

    let (weak, shared, dialogs) = (window.as_weak(), state.clone(), dialogs.clone());
    gradebook.on_delete_column(move |column_id| {
        let Some(window) = weak.upgrade() else { return };
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
        run(&window, &shared, "delete column", |connection| {
            columns::delete_column(connection, i64::from(column_id))?;
            Ok(Refresh::Sheet)
        });
    });
}

fn wire_settings(window: &AppWindow, state: &SharedState) {
    let gradebook = window.global::<Gradebook>();

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_set_bonus(move |column_id, bonus| {
        let Some(window) = weak.upgrade() else { return };
        run(&window, &shared, "set bonus test", |connection| {
            columns::set_bonus(connection, i64::from(column_id), bonus)?;
            Ok(Refresh::Columns)
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_rename_column(move |column_id, title| {
        let Some(window) = weak.upgrade() else { return };
        run(&window, &shared, "rename column", |connection| {
            columns::rename_column(connection, i64::from(column_id), &title)?;
            Ok(Refresh::Columns)
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_set_weight(move |column_id, text| {
        let Some(window) = weak.upgrade() else { return };
        run(&window, &shared, "set weight", |connection| {
            columns::set_weight(connection, i64::from(column_id), &text)?;
            Ok(Refresh::Columns)
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_set_max_score(move |column_id, text| {
        let Some(window) = weak.upgrade() else { return };
        run(&window, &shared, "set maximum score", |connection| {
            columns::set_max_score(connection, i64::from(column_id), &text)?;
            Ok(Refresh::Columns)
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_set_decimals(move |column_id, text| {
        let Some(window) = weak.upgrade() else { return };
        run(&window, &shared, "set decimals", |connection| {
            columns::set_decimals(connection, i64::from(column_id), &text)?;
            Ok(Refresh::Columns)
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_set_round_source(move |column_id, index| {
        let Some(window) = weak.upgrade() else { return };
        let class_id = current_class_id(&window);
        run(&window, &shared, "set rounded column source", |connection| {
            let all = columns::list_columns(connection, class_id)?;
            let source =
                usize::try_from(index).ok().and_then(|index| round_sources(&all).nth(index)).map(|column| column.id);
            columns::set_round_source(connection, i64::from(column_id), source)?;
            Ok(Refresh::Columns)
        });
    });
}
