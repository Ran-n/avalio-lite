// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:52.749638
//+ Revised: 	2026/10/28 10:52:42.723121
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, Model};

use super::view::Refresh;
use super::{SharedState, run};
use crate::error::AppError;
use crate::gradebook::cells;
use crate::ui::{AppWindow, Gradebook};

pub fn wire(window: &AppWindow, state: &SharedState) {
    let (weak, shared) = (window.as_weak(), state.clone());
    window.global::<Gradebook>().on_cell_edited(move |row, column, text| {
        let Some(window) = weak.upgrade() else { return };
        let gradebook = window.global::<Gradebook>();
        let (Ok(row), Ok(column)) = (usize::try_from(row), usize::try_from(column)) else { return };
        let pupil = gradebook.get_rows().row_data(row).map(|item| item.pupil_id);
        let column_id = gradebook.get_columns().row_data(column).map(|item| item.id);
        run(&window, &shared, "save cell", |connection| {
            let (Some(pupil_id), Some(column_id)) = (pupil, column_id) else { return Err(AppError::NotEditable) };
            cells::set_cell(connection, i64::from(column_id), i64::from(pupil_id), &text)?;
            Ok(Refresh::Row(row))
        });
    });
}
