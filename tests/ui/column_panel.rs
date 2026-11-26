// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.318860
//+ Revised: 	2026/11/23 13:00:31.318860
// -----------------------------------------------------------------------------

//! The column settings panel: Escape, stepping through the columns, moving and deleting them.

use crate::driver::{click, click_element, click_menu_option, english, find, gradebook, key, type_into};
use crate::fixtures::{cell_text, header, saved_preferences, seeded};
use crate::screens::{FIRST_MARK, column_titles, four_columns, panel_is_closed, with_modifier};
use avalio_lite::gradebook::{classes, columns};
use avalio_lite::i18n::{Message, SheetMessage};
use i_slint_backend_testing::ElementHandle;
use slint::Model;
use slint::platform::Key;

#[test]
fn escape_closes_the_panel_from_the_weight_field() {
    let ui = four_columns();
    click(&ui, "Exam");
    click(&ui, "Weight");
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui));
}

#[test]
fn escape_closes_the_panel_from_the_out_of_field() {
    let ui = four_columns();
    click(&ui, "Exam");
    click(&ui, "Out of");
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui));
}

#[test]
fn escape_closes_the_panel_after_clicking_the_bonus_toggle() {
    let ui = four_columns();
    click(&ui, "Exam");
    click(&ui, "Bonus test");
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui));
}

#[test]
fn escape_closes_the_panel_after_moving_the_column() {
    let ui = four_columns();
    click(&ui, "Exam");
    click(&ui, "Move right (Ctrl+→)");
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui));
}

#[test]
fn escape_closes_the_panel_after_clicking_the_panels_blank_area() {
    let ui = four_columns();
    click(&ui, "Exam");
    // a plain text of the panel: the click lands on the panel's background
    click_element(&ElementHandle::find_by_accessible_label(&ui.session.window, "Type").next().unwrap());
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui));
}

#[test]
fn escape_closes_the_panel_from_the_decimals_field() {
    let ui = four_columns();
    click(&ui, "Rounded");
    click(&ui, "Decimals");
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui));
}

#[test]
fn escape_closes_the_panel_from_the_rounds_combo_box() {
    let ui = four_columns();
    click(&ui, "Rounded");
    click(&ui, "Decimals");
    // back from Decimals to the combo box, which takes the focus without opening its list
    with_modifier(&ui, Key::Shift, Key::Tab);
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui));
}

#[test]
fn escape_with_the_rounds_list_open_closes_only_the_list() {
    let ui = four_columns();
    click(&ui, "Rounded");
    let rounds = ElementHandle::find_by_element_id(&ui.session.window, "ColumnPanel::rounds").next().unwrap();
    click_element(&rounds);
    key(&ui, Key::Escape);
    assert!(!panel_is_closed(&ui), "the first Escape closes only the list");
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui), "the second closes the panel");
}

#[test]
fn escape_with_a_refused_value_restores_it_and_closes() {
    let ui = four_columns();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "abc");
    key(&ui, Key::Escape);
    assert!(panel_is_closed(&ui));
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().weight, "100", "the saved weight stays");
    assert!(ui.session.window.get_error_message().is_empty(), "no error is left showing");
}

#[test]
fn escape_in_a_sheet_cell_leaves_the_panel_open() {
    let ui = four_columns();
    click(&ui, "Exam");
    click(&ui, "Ana, Quiz");
    key(&ui, Key::Escape);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32, "Escape belongs to the cell");
}

#[test]
fn the_arrow_buttons_step_the_panel_through_the_columns() {
    let ui = four_columns();
    click(&ui, "Exam");
    click(&ui, "Next column (Alt+→)");
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32 + 1);
    assert_eq!(gradebook(&ui).get_panel_column().row_data(0).unwrap().title, "Quiz");
    click(&ui, "Previous column (Alt+←)");
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32);
    assert_eq!(gradebook(&ui).get_panel_column().row_data(0).unwrap().title, "Exam");
}

#[test]
fn alt_arrows_step_the_panel() {
    let ui = four_columns();
    click(&ui, "Exam");
    click(&ui, "Weight");
    with_modifier(&ui, Key::Alt, Key::RightArrow);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32 + 1, "Alt+→ from inside the Weight field");
    with_modifier(&ui, Key::Alt, Key::LeftArrow);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32);
}

#[test]
fn the_ends_dim_the_arrow_and_ignore_the_key() {
    let ui = four_columns();
    click_element(&header(&ui, "First name"));
    assert_eq!(find(&ui, "Previous column (Alt+←)").accessible_enabled(), Some(false));
    with_modifier(&ui, Key::Alt, Key::LeftArrow);
    assert_eq!(gradebook(&ui).get_panel_index(), 0);
    // the open panel covers the last header; Escape closes it
    key(&ui, Key::Escape);
    click(&ui, "Rounded");
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32 + 3);
    assert_eq!(find(&ui, "Next column (Alt+→)").accessible_enabled(), Some(false));
    with_modifier(&ui, Key::Alt, Key::RightArrow);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32 + 3);
}

#[test]
fn stepping_puts_a_refused_value_back() {
    let ui = four_columns();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "abc");
    with_modifier(&ui, Key::Alt, Key::RightArrow);
    with_modifier(&ui, Key::Alt, Key::LeftArrow);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32);
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().weight, "100");
    assert!(ui.session.window.get_error_message().is_empty(), "no error is left showing");
}

#[test]
fn alt_arrows_dont_move_the_column() {
    let ui = four_columns();
    click(&ui, "Exam");
    with_modifier(&ui, Key::Alt, Key::RightArrow);
    assert_eq!(column_titles(&ui), ["Exam", "Quiz", "Final grade", "Rounded"]);
}

#[test]
fn ctrl_arrows_move_the_column_and_keep_the_panel_on_it() {
    let ui = four_columns();
    click(&ui, "Exam");
    with_modifier(&ui, Key::Control, Key::RightArrow);
    with_modifier(&ui, Key::Control, Key::RightArrow);
    assert_eq!(column_titles(&ui), ["Quiz", "Final grade", "Exam", "Rounded"]);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32 + 2, "the panel stays on the column");
    let state = ui.session.state.borrow();
    let connection = state.connection().unwrap();
    let class = classes::list_classes(connection).unwrap()[0].id;
    let saved: Vec<String> = columns::list_columns(connection, class)
        .unwrap()
        .into_iter()
        .filter(|column| !column.kind.is_name())
        .map(|column| column.title)
        .collect();
    assert_eq!(saved, ["Quiz", "Final grade", "Exam", "Rounded"], "the order is saved");
}

#[test]
fn ctrl_arrows_work_from_inside_a_text_field() {
    let ui = four_columns();
    click(&ui, "Quiz");
    click(&ui, "Title");
    with_modifier(&ui, Key::Control, Key::LeftArrow);
    assert_eq!(column_titles(&ui), ["Quiz", "Exam", "Final grade", "Rounded"]);
    assert_eq!(gradebook(&ui).get_panel_column().row_data(0).unwrap().title, "Quiz", "the title is unchanged");
}

#[test]
fn ctrl_arrows_stop_at_the_ends() {
    let ui = four_columns();
    click(&ui, "Exam");
    with_modifier(&ui, Key::Control, Key::LeftArrow);
    click(&ui, "Rounded");
    with_modifier(&ui, Key::Control, Key::RightArrow);
    assert_eq!(column_titles(&ui), ["Exam", "Quiz", "Final grade", "Rounded"]);
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn ctrl_arrows_outside_the_panel_dont_move_columns() {
    let ui = four_columns();
    click(&ui, "Ana, Exam");
    with_modifier(&ui, Key::Control, Key::RightArrow);
    assert_eq!(column_titles(&ui), ["Exam", "Quiz", "Final grade", "Rounded"]);
}

#[test]
fn deleting_a_column_asks_first() {
    let ui = seeded();
    click(&ui, "Exam");
    ui.dialogs.answer_next(false);
    click(&ui, "Delete");
    assert_eq!(gradebook(&ui).get_columns().row_count(), 4);
    assert_eq!(
        ui.dialogs.questions.borrow()[0],
        english(&Message::Sheet(SheetMessage::ConfirmDeleteColumn { column: "Exam" }))
    );
    ui.dialogs.answer_next(true);
    click(&ui, "Delete");
    assert_eq!(gradebook(&ui).get_columns().row_count(), 3);
    assert_eq!(gradebook(&ui).get_panel_index(), -1, "the panel closes with its column");
}

#[test]
fn a_rounded_column_rounds_its_source() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "7.46");
    click(&ui, "+ Add column");
    click_menu_option(&ui, "Rounded");
    let rounded = gradebook(&ui).get_columns().row_data(FIRST_MARK + 2).unwrap();
    assert_eq!(rounded.source_index, 1, "rounds the last numeric column, the final grade");
    click(&ui, "Rounded");
    type_into(&ui, "Decimals", "1");
    assert_eq!(cell_text(&ui, 0, 2), "7.5");
    // the source picker's indexes refer to the numeric columns: 0 is the test
    gradebook(&ui).invoke_set_round_source(rounded.id, 0);
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK + 2).unwrap().source_index, 0);
    assert_eq!(cell_text(&ui, 0, 2), "7.5");
}

#[test]
fn hiding_weights_and_scales_is_saved() {
    let ui = seeded();
    click(&ui, "More actions");
    click(&ui, "Hide weights & scales");
    assert!(!gradebook(&ui).get_show_column_settings());
    assert!(!saved_preferences(&ui.fixture).show_column_settings);
    click(&ui, "More actions");
    click(&ui, "Show weights & scales");
    assert!(gradebook(&ui).get_show_column_settings());
}
