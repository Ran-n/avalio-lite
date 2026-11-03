// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.393948
//+ Revised: 	2026/11/03 11:13:31.475894
// -----------------------------------------------------------------------------

use rusqlite::Connection;
use slint::{ComponentHandle, Model, SharedString};

use super::items::{cell_items, column_item, number, round_sources, row_item, short_class_name};
use super::panel;
use crate::error::Result;
use crate::gradebook::compute::{bonus_weight_total, grade_summary, grades_are_provisional, round_to, weight_total};
use crate::gradebook::model::{ClassSummary, Column, Sheet};
use crate::gradebook::{classes, columns, pupils, sheet};
use crate::ui::{AddablePupil, AppWindow, ClassItem, Gradebook};
use crate::window::view::model;

const NO_CLASS: i32 = -1;

/// What has to be redrawn after a successful action. In-place refreshes keep the
/// cell being typed in alive, so typing is never interrupted.
pub enum Refresh {
    Nothing,
    /// the open class was renamed: the sidebar, and the name confirmations quote
    ClassName,
    /// a pupil was renamed: their row's name, which confirmations and screen readers use
    PupilName(i64),
    Row(usize),
    /// column settings changed: recalculate cells and update the headers' tags in place
    Columns,
    Sheet,
    All {
        select: Option<i64>,
    },
}

pub fn apply(window: &AppWindow, connection: &Connection, refresh: Refresh) -> Result<()> {
    match refresh {
        Refresh::Nothing => Ok(()),
        Refresh::ClassName => show_class_name(window, connection),
        Refresh::PupilName(pupil_id) => show_pupil_name(window, connection, pupil_id),
        Refresh::Row(row) => refresh_rows(window, connection, Some(row)),
        Refresh::Columns => refresh_columns(window, connection),
        Refresh::Sheet => show_sheet(window, connection, current_class_id(window)),
        Refresh::All { select } => show_all(window, connection, select),
    }
}

/// Called when a database is opened: forgets the class selected in the previous one.
pub fn show_new_database(window: &AppWindow, connection: &Connection) -> Result<()> {
    // the panel belongs to a column of the previous database; the same row id there is another column
    panel::close(window);
    window.global::<Gradebook>().set_current_class_id(NO_CLASS);
    show_all(window, connection, None)
}

pub fn current_class_id(window: &AppWindow) -> i64 {
    i64::from(window.global::<Gradebook>().get_current_class_id())
}

pub fn ui_id(id: i64) -> i32 {
    i32::try_from(id).expect("row ids stay far below i32::MAX")
}

fn show_all(window: &AppWindow, connection: &Connection, select: Option<i64>) -> Result<()> {
    let class_list = show_class_list(window, connection)?;
    let wanted = select.unwrap_or_else(|| current_class_id(window));
    let current =
        class_list.iter().find(|class| class.id == wanted).or_else(|| class_list.first()).map(|class| class.id);
    match current {
        Some(class_id) => show_sheet(window, connection, class_id),
        None => {
            panel::close(window);
            window.global::<Gradebook>().set_current_class_id(NO_CLASS);
            Ok(())
        }
    }
}

fn show_class_list(window: &AppWindow, connection: &Connection) -> Result<Vec<ClassSummary>> {
    let class_list = classes::list_classes(connection)?;
    let items: Vec<ClassItem> = class_list
        .iter()
        .map(|class| ClassItem {
            id: ui_id(class.id),
            name: class.name.as_str().into(),
            short_name: short_class_name(&class.name).into(),
            pupil_count: i32::try_from(class.pupil_count).unwrap_or(i32::MAX),
        })
        .collect();
    window.global::<Gradebook>().set_classes(model(items));
    Ok(class_list)
}

fn show_sheet(window: &AppWindow, connection: &Connection, class_id: i64) -> Result<()> {
    let loaded = sheet::load_sheet(connection, class_id)?;
    let no_class: Vec<i64> = pupils::pupils_in_no_class(connection)?.into_iter().map(|pupil| pupil.id).collect();
    let addable: Vec<AddablePupil> = pupils::pupils_not_in_class(connection, class_id)?
        .into_iter()
        .map(|pupil| AddablePupil { no_class: no_class.contains(&pupil.id), name: pupil.name.into() })
        .collect();
    let gradebook = window.global::<Gradebook>();
    gradebook.set_current_class_id(ui_id(class_id));
    gradebook.set_class_name(classes::class_name(connection, class_id)?.into());
    gradebook.set_columns(model(loaded.columns.iter().map(|column| column_item(column, &loaded.columns)).collect()));
    gradebook.set_rows(model(loaded.rows.iter().map(|row| row_item(row, &loaded.columns)).collect()));
    gradebook.set_addable_pupils(model(addable));
    show_round_sources(window, &loaded.columns);
    show_weight_total(window, &loaded.columns);
    show_summary(window, &loaded);
    panel::keep_in_sync(window, &loaded.columns);
    Ok(())
}

/// The typed-in name field already shows the new name; this updates what the rest of the window
/// knows it by. (Typing broke the field's binding, so this never moves its caret.)
fn show_class_name(window: &AppWindow, connection: &Connection) -> Result<()> {
    show_class_list(window, connection)?;
    let name = classes::class_name(connection, current_class_id(window))?;
    window.global::<Gradebook>().set_class_name(name.into());
    Ok(())
}

fn show_pupil_name(window: &AppWindow, connection: &Connection, pupil_id: i64) -> Result<()> {
    let name: SharedString = pupils::pupil_name(connection, pupil_id)?.into();
    let rows = window.global::<Gradebook>().get_rows();
    let found = rows.iter().enumerate().find(|(_, row)| i64::from(row.pupil_id) == pupil_id);
    if let Some((index, mut row)) = found
        && row.name != name
    {
        row.name = name;
        rows.set_row_data(index, row);
    }
    Ok(())
}

fn show_round_sources(window: &AppWindow, columns: &[Column]) {
    let titles: Vec<SharedString> = round_sources(columns).map(|column| column.title.as_str().into()).collect();
    window.global::<Gradebook>().set_round_sources(model(titles));
}

/// Recalculates cells in place (one row, or all) without rebuilding the sheet.
fn refresh_rows(window: &AppWindow, connection: &Connection, only: Option<usize>) -> Result<()> {
    let loaded = sheet::load_sheet(connection, current_class_id(window))?;
    let rows = window.global::<Gradebook>().get_rows();
    if rows.row_count() != loaded.rows.len() {
        return show_sheet(window, connection, current_class_id(window));
    }
    for (index, row) in loaded.rows.iter().enumerate().filter(|(index, _)| only.is_none_or(|row| row == *index)) {
        let Some(item) = rows.row_data(index) else { continue };
        for (column, cell) in cell_items(row, &loaded.columns).into_iter().enumerate() {
            if item.cells.row_data(column).as_ref() != Some(&cell) {
                item.cells.set_row_data(column, cell);
            }
        }
    }
    show_weight_total(window, &loaded.columns);
    show_summary(window, &loaded);
    Ok(())
}

/// Updates each header's title and tags in place, then recalculates the cells.
fn refresh_columns(window: &AppWindow, connection: &Connection) -> Result<()> {
    let columns = columns::list_columns(connection, current_class_id(window))?;
    let items = window.global::<Gradebook>().get_columns();
    if items.row_count() != columns.len() {
        return show_sheet(window, connection, current_class_id(window));
    }
    for (index, column) in columns.iter().enumerate() {
        let item = column_item(column, &columns);
        if items.row_data(index).as_ref() != Some(&item) {
            items.set_row_data(index, item);
        }
    }
    show_round_sources(window, &columns);
    refresh_rows(window, connection, None)
}

/// The class average and pass / fail counts under the class name.
fn show_summary(window: &AppWindow, loaded: &Sheet) {
    let values: Vec<Vec<String>> = loaded.rows.iter().map(|row| row.values.clone()).collect();
    let summary = grade_summary(&loaded.columns, &values);
    let gradebook = window.global::<Gradebook>();
    gradebook.set_has_summary(summary.is_some());
    if let Some(summary) = summary {
        gradebook.set_class_average(number(round_to(summary.average, 2)).into());
        gradebook.set_passing(i32::try_from(summary.passing).unwrap_or(i32::MAX));
        gradebook.set_failing(i32::try_from(summary.failing).unwrap_or(i32::MAX));
    }
}

fn show_weight_total(window: &AppWindow, columns: &[Column]) {
    let gradebook = window.global::<Gradebook>();
    gradebook.set_weight_total(number(weight_total(columns)).into());
    gradebook.set_bonus_total(number(bonus_weight_total(columns)).into());
    gradebook.set_weights_complete(!grades_are_provisional(columns));
}
