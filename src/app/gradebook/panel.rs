// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:43.679671
//+ Revised: 	2026/10/28 10:52:43.679671
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, Model};

use super::view::{column_item, model};
use super::{SharedState, run, view::Refresh};
use crate::gradebook::columns::list_columns;
use crate::gradebook::model::Column;
use crate::ui::{AppWindow, ColumnItem, Gradebook};

const CLOSED: i32 = -1;

pub fn wire(window: &AppWindow, state: &SharedState) {
    let gradebook = window.global::<Gradebook>();

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_open_column_panel(move |index| {
        let Some(window) = weak.upgrade() else { return };
        let class_id = super::view::current_class_id(&window);
        run(&window, &shared, "open column settings", |connection| {
            let columns = list_columns(connection, class_id)?;
            if let Some(column) = usize::try_from(index).ok().and_then(|index| columns.get(index)) {
                show(&window, column, &columns, index);
            }
            Ok(Refresh::Nothing)
        });
    });

    let weak = window.as_weak();
    gradebook.on_close_column_panel(move || {
        if let Some(window) = weak.upgrade() {
            close(&window);
        }
    });
}

/// Opens the panel for `column`. The one-item model is replaced each time, so the panel is
/// rebuilt and its fields show this column's values rather than what was typed for another.
fn show(window: &AppWindow, column: &Column, columns: &[Column], index: i32) {
    let gradebook = window.global::<Gradebook>();
    gradebook.set_panel_index(index);
    gradebook.set_panel_column(model(vec![column_item(column, columns)]));
}

pub fn close(window: &AppWindow) {
    let gradebook = window.global::<Gradebook>();
    gradebook.set_panel_index(CLOSED);
    gradebook.set_panel_column(model(Vec::<ColumnItem>::new()));
}

/// After the sheet is reloaded (a column moved, added or deleted), keeps the panel on the same
/// column at its new position, or closes it when that column is gone.
pub fn keep_in_sync(window: &AppWindow, columns: &[Column]) {
    let gradebook = window.global::<Gradebook>();
    let Some(open) = gradebook.get_panel_column().row_data(0) else { return };
    match columns.iter().position(|column| super::view::ui_id(column.id) == open.id) {
        Some(index) => show(window, &columns[index], columns, i32::try_from(index).unwrap_or(CLOSED)),
        None => close(window),
    }
}
