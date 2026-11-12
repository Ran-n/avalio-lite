// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:52.749638
//+ Revised: 	2026/11/12 14:39:52.422681
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, Model, SharedString};

use super::view::Refresh;
use super::{SharedState, run};
use crate::error::AppError;
use crate::gradebook::cells;
use crate::ui::{AppWindow, Gradebook};
use crate::window::with_window3;

pub fn wire(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_cell_edited(with_window3(
        window,
        move |window, row, column, text: SharedString| {
            let gradebook = window.global::<Gradebook>();
            let (row, column) = (usize::try_from(row).ok(), usize::try_from(column).ok());
            let pupil = row.and_then(|row| gradebook.get_rows().row_data(row)).map(|item| item.pupil_id);
            let column_id = column.and_then(|column| gradebook.get_columns().row_data(column)).map(|item| item.id);
            run(window, &shared, "save cell", |connection| {
                // a cell the sheet doesn't have (a stale row or column) is reported, never dropped
                let (Some(row), Some(pupil_id), Some(column_id)) = (row, pupil, column_id) else {
                    return Err(AppError::NotEditable);
                };
                cells::set_cell(connection, i64::from(column_id), i64::from(pupil_id), &text)?;
                Ok(Refresh::Row(row))
            });
        },
    ));
}
