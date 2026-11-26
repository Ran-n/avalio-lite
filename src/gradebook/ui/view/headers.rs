// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 01:29:26.313662
//+ Revised: 	2026/11/24 17:54:05.144334
// -----------------------------------------------------------------------------

//! The sheet's column headers, updated in place: a column's title, tags and width change without the
//! rows being rebuilt. They change with the columns' settings, not with the cells, so they are kept
//! apart from the rest of the sheet's redraw.

use rusqlite::Connection;
use slint::{ComponentHandle, Model};

use super::{current_class_id, show_sheet};
use crate::error::Result;
use crate::gradebook::columns;
use crate::gradebook::model::Column;
use crate::gradebook::ui::items::column_item;
use crate::ui::{AppWindow, Gradebook};

/// Updates the headers' widths in place; the rows aren't rebuilt.
pub fn refresh_widths(window: &AppWindow, connection: &Connection) -> Result<()> {
    refresh_headers(window, connection).map(|_| ())
}

/// Puts each column's current item in the header model, changing only the items that differ. The
/// columns, or `None` when their number changed and the whole sheet was reloaded instead.
pub fn refresh_headers(window: &AppWindow, connection: &Connection) -> Result<Option<Vec<Column>>> {
    let columns = columns::list_columns(connection, current_class_id(window))?;
    let items = window.global::<Gradebook>().get_columns();
    if items.row_count() != columns.len() {
        show_sheet(window, connection, current_class_id(window))?;
        return Ok(None);
    }
    for (index, column) in columns.iter().enumerate() {
        let item = column_item(column, &columns);
        if items.row_data(index).as_ref() != Some(&item) {
            items.set_row_data(index, item);
        }
    }
    Ok(Some(columns))
}
