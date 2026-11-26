// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:30.908535
//+ Revised: 	2026/11/23 13:00:30.908535
// -----------------------------------------------------------------------------

//! Cells and the summary: values typed into the sheet, Enter moving from cell to cell.

use crate::common::{Fixture, Ui};
use crate::driver::{
    click, click_element, count, find, gradebook, press_enter, replace_text, shows, shows_text, type_into, type_text,
};
use crate::fixtures::{cell_text, name_fields, seeded};
use crate::screens::FIRST_MARK;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, columns, pupils};
use avalio_lite::i18n::Language;
use avalio_lite::ui::Tone;
use slint::Model;

// ---------------------------------------------------------------------------------------------
// Cells and summary

/// When the file changed under the open sheet (a pupil or a column added from elsewhere), an
/// in-place refresh would put values in the wrong rows or columns, so the whole sheet is redrawn.
#[test]
fn a_sheet_that_changed_underneath_is_redrawn_whole() {
    let ui = seeded();
    {
        let state = ui.session.state.borrow();
        let connection = state.connection().unwrap();
        let class = classes::list_classes(connection).unwrap()[0].id;
        pupils::add_new_pupil(connection, class, "Carla", "").unwrap();
    }
    type_into(&ui, "Ana, Exam", "8");
    assert_eq!(gradebook(&ui).get_rows().row_count(), 3, "the new pupil shows");
    {
        let state = ui.session.state.borrow();
        let connection = state.connection().unwrap();
        let class = classes::list_classes(connection).unwrap()[0].id;
        columns::add_column(connection, class, ColumnType::Info, Language::English).unwrap();
    }
    let exam = gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().id;
    gradebook(&ui).invoke_set_weight(exam, "100".into());
    assert_eq!(gradebook(&ui).get_columns().row_count(), 5, "the new column shows");
    assert_eq!(cell_text(&ui, 0, 0), "8", "and the marks stay in their places");
}

#[test]
fn typing_marks_updates_the_final_grade_and_summary() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    type_into(&ui, "Bea, Exam", "3");
    assert_eq!(cell_text(&ui, 0, 1), "8");
    let rows = gradebook(&ui).get_rows();
    assert_eq!(rows.row_data(0).unwrap().cells.row_data(FIRST_MARK + 1).unwrap().tone, Tone::Pass);
    assert_eq!(rows.row_data(1).unwrap().cells.row_data(FIRST_MARK + 1).unwrap().tone, Tone::Fail);
    assert!(gradebook(&ui).get_has_summary());
    assert_eq!(gradebook(&ui).get_class_average(), "5.5");
    assert_eq!((gradebook(&ui).get_passing(), gradebook(&ui).get_failing()), (1, 1));
    assert!(shows_text(&ui, "Class average 5.5"));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 2, "typed marks are saved");
}

#[test]
fn an_out_of_range_mark_is_flagged_and_ignored() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "12");
    let cell = gradebook(&ui).get_rows().row_data(0).unwrap().cells.row_data(FIRST_MARK).unwrap();
    assert!(cell.invalid);
    assert_eq!(cell_text(&ui, 0, 1), "", "the final grade leaves it out, so nothing is marked yet");
    assert!(shows(&ui, "Ana, Exam (not a valid mark)"), "said to a screen reader, not only shaded red");
}

#[test]
fn clearing_a_mark_deletes_it() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    type_into(&ui, "Ana, Exam", "");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells"), 0);
    assert!(!gradebook(&ui).get_has_summary());
}

/// A calculated cell's value sat in a plain Text next to a hidden field, so a screen reader had
/// no cell to read it from.
#[test]
fn regression_calculated_cells_are_readable_with_their_value() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    let value = cell_text(&ui, 0, 1);
    assert!(!value.is_empty());
    assert!(shows_text(&ui, &format!("Ana, Final grade: {value}")));
    assert!(!shows_text(&ui, &format!("Ana, Final grade: {value} (provisional)")));
    click(&ui, "Exam");
    type_into(&ui, "Weight", "50");
    let value = cell_text(&ui, 0, 1);
    assert!(shows_text(&ui, &format!("Ana, Final grade: {value} (provisional)")));
    assert!(shows_text(&ui, "Bea, Final grade"), "an empty calculated cell is still named");
}

// ---------------------------------------------------------------------------------------------
// Enter

fn stored_mark(ui: &Ui, pupil: &str) -> Option<String> {
    let state = ui.session.state.borrow();
    state
        .connection()
        .unwrap()
        .query_row(
            "SELECT c.value FROM cells c JOIN pupils p ON p.id = c.pupil_id
             JOIN sheet_columns sc ON sc.id = c.column_id WHERE p.first_name = ?1 AND sc.title = 'Exam'",
            [pupil],
            |row| row.get(0),
        )
        .ok()
}

#[test]
fn enter_in_a_mark_cell_moves_to_the_cell_below() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "7");
    press_enter(&ui);
    assert_eq!(stored_mark(&ui, "Ana").as_deref(), Some("7"));
    type_text(&ui, "8");
    assert_eq!(stored_mark(&ui, "Bea").as_deref(), Some("8"), "the cell below had focus");
    assert_eq!(stored_mark(&ui, "Ana").as_deref(), Some("7"), "and the cell left kept its mark");
    assert_eq!(cell_text(&ui, 0, 0), "7");
}

#[test]
fn enter_selects_the_cell_below_so_typing_replaces_it() {
    let ui = seeded();
    type_into(&ui, "Bea, Exam", "3");
    type_into(&ui, "Ana, Exam", "7");
    press_enter(&ui);
    type_text(&ui, "9");
    assert_eq!(stored_mark(&ui, "Bea").as_deref(), Some("9"), "the old mark was replaced, not added to");
}

#[test]
fn enter_on_the_last_pupil_leaves_the_sheet_unfocused() {
    let ui = seeded();
    type_into(&ui, "Bea, Exam", "5");
    press_enter(&ui);
    type_text(&ui, "1");
    assert_eq!(stored_mark(&ui, "Bea").as_deref(), Some("5"), "nothing has focus to type into");
    assert_eq!(stored_mark(&ui, "Ana"), None);
    assert_eq!(gradebook(&ui).get_rows().row_count(), 2);
}

#[test]
fn enter_in_an_out_of_range_cell_still_moves_down_and_keeps_it_red() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "12");
    press_enter(&ui);
    assert!(gradebook(&ui).get_rows().row_data(0).unwrap().cells.row_data(FIRST_MARK).unwrap().invalid);
    type_text(&ui, "4");
    assert_eq!(stored_mark(&ui, "Bea").as_deref(), Some("4"));
    assert!(gradebook(&ui).get_rows().row_data(0).unwrap().cells.row_data(FIRST_MARK).unwrap().invalid, "still marked");
}

#[test]
fn enter_in_a_text_column_moves_down_too() {
    let ui = seeded();
    gradebook(&ui).invoke_add_column(avalio_lite::ui::ColumnType::Info);
    type_into(&ui, "Ana, Notes", "Glasses");
    press_enter(&ui);
    type_text(&ui, "Left early");
    assert_eq!(cell_text(&ui, 1, 2), "Left early");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM cells WHERE value = 'Left early'"), 1);
}

#[test]
fn enter_moves_down_again_from_a_cell_reached_by_enter() {
    let ui = with_three_pupils();
    type_into(&ui, "Ana, Exam", "7");
    press_enter(&ui);
    type_text(&ui, "8");
    press_enter(&ui);
    type_text(&ui, "9");
    assert_eq!(stored_mark(&ui, "Carla").as_deref(), Some("9"), "a whole column is typed in one go");
}

#[test]
fn enter_in_a_pupil_name_ends_the_editing() {
    let ui = seeded();
    click_element(&name_fields(&ui, "First name")[0]);
    replace_text(&ui, "Ana María");
    press_enter(&ui);
    type_text(&ui, "x");
    assert_eq!(gradebook(&ui).get_rows().row_data(0).unwrap().full_name, "Ana María", "the field lost focus");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils WHERE first_name = 'Ana María'"), 1, "and the name was saved");
}

#[test]
fn enter_with_a_refused_class_name_puts_it_back_and_ends_the_editing() {
    let ui = seeded();
    type_into(&ui, "Class name", "");
    assert!(!ui.session.window.get_error_message().is_empty(), "an empty name is refused");
    press_enter(&ui);
    assert_eq!(find(&ui, "Class name").accessible_value().as_deref(), Some("1A"), "put back");
    type_text(&ui, "x");
    assert_eq!(find(&ui, "Class name").accessible_value().as_deref(), Some("1A"), "and no longer focused");
    assert_eq!(gradebook(&ui).get_class_name(), "1A");
}

#[test]
fn enter_in_add_pupil_keeps_the_field_focused() {
    let ui = seeded();
    click(&ui, "+ Add pupil");
    type_text(&ui, "Carmen");
    press_enter(&ui);
    type_text(&ui, "Gómez");
    press_enter(&ui);
    type_text(&ui, "Dora");
    press_enter(&ui);
    press_enter(&ui);
    let names: Vec<String> = gradebook(&ui).get_rows().iter().map(|row| row.full_name.to_string()).collect();
    assert!(names.contains(&"Carmen Gómez".to_owned()) && names.contains(&"Dora".to_owned()), "{names:?}");
}

/// Class "1A" with Ana, Bea and Carla, and the test "Exam".
fn with_three_pupils() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        for name in ["Ana", "Bea", "Carla"] {
            pupils::add_new_pupil(connection, class, name, "").unwrap();
        }
        let exam = columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
        columns::rename_column(connection, exam, "Exam").unwrap();
    }
    state.shutdown();
    Ui::open(fixture)
}
