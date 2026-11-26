// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.152677
//+ Revised: 	2026/11/23 13:00:31.152677
// -----------------------------------------------------------------------------

//! Name columns among the other columns, and dragging a column header to move it.

use crate::common::Ui;
use crate::driver::{
    click, click_element, click_nth, count, english, find, gradebook, key, press_enter, shows, shows_text, type_into,
    type_text,
};
use crate::fixtures::{header, seeded};
use crate::screens::{
    FIRST_MARK, column_titles, drag_edge, field_text, four_columns, panel_is_closed, stored_column_width, width_edges,
    with_modifier,
};
use avalio_lite::gradebook::{classes, columns};
use avalio_lite::i18n::{Message, SheetMessage};
use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

// ---------------------------------------------------------------------------------------------
// Name columns among the others, and dragging headers

/// Every column's title in sheet order, the name columns included.
fn all_titles(ui: &Ui) -> Vec<String> {
    gradebook(ui).get_columns().iter().map(|column| column.title.to_string()).collect()
}

fn centre(element: &ElementHandle) -> slint::LogicalPosition {
    let (at, size) = (element.absolute_position(), element.size());
    slint::LogicalPosition::new(at.x + size.width / 2.0, at.y + size.height / 2.0)
}

/// Presses the header titled `title`, moves the pointer sideways to `x` (window coordinates) in a
/// few steps, and releases it there, through real pointer events.
fn drag_header_to(ui: &Ui, title: &str, x: f32) {
    let start = centre(&header(ui, title));
    let window = ui.session.window.window();
    window.dispatch_event(WindowEvent::PointerMoved { position: start });
    window.dispatch_event(WindowEvent::PointerPressed { position: start, button: PointerEventButton::Left });
    for step in 1..=4 {
        let position = slint::LogicalPosition::new(start.x + (x - start.x) * step as f32 / 4.0, start.y);
        window.dispatch_event(WindowEvent::PointerMoved { position });
    }
    let end = slint::LogicalPosition::new(x, start.y);
    window.dispatch_event(WindowEvent::PointerReleased { position: end, button: PointerEventButton::Left });
}

/// The left edge of the header titled `title`, in window coordinates.
fn header_left(ui: &Ui, title: &str) -> f32 {
    header(ui, title).absolute_position().x
}

#[test]
fn a_name_columns_panel_shows_only_the_move_buttons() {
    let ui = seeded();
    click_element(&header(&ui, "Surname"));
    assert_eq!(gradebook(&ui).get_panel_index(), 1);
    assert!(!shows(&ui, "Title"), "the title follows the language");
    assert!(!shows(&ui, "Delete"), "a name column can't be deleted");
    assert!(shows_text(&ui, "Surname: the same in every class the pupil is in"));
    assert!(shows(&ui, "Move left (Ctrl+←)") && shows(&ui, "Move right (Ctrl+→)"));
}

#[test]
fn ctrl_arrow_moves_the_surname_past_a_test() {
    let ui = seeded();
    click_element(&header(&ui, "Surname"));
    with_modifier(&ui, Key::Control, Key::RightArrow);
    assert_eq!(all_titles(&ui), ["First name", "Exam", "Surname", "Final grade"]);
    assert_eq!(gradebook(&ui).get_panel_index(), 2, "the panel follows it");
    assert_eq!(field_text(&ui, "Surname…"), "", "the add row's surname field moved with it");
    assert!(shows(&ui, "First name…") && shows(&ui, "Surname…"));
}

#[test]
fn the_remove_button_works_with_the_names_moved() {
    let ui = seeded();
    click_element(&header(&ui, "First name"));
    click(&ui, "Move right (Ctrl+→)");
    click(&ui, "Move right (Ctrl+→)");
    key(&ui, Key::Escape);
    assert_eq!(all_titles(&ui), ["Surname", "Exam", "First name", "Final grade"]);
    ui.dialogs.answer_next(true);
    click_nth(&ui, "Remove from this class", 0);
    assert_eq!(
        ui.dialogs.questions.borrow()[0],
        english(&Message::Sheet(SheetMessage::ConfirmRemovePupil { pupil: "Ana", class: "1A" }))
    );
    assert_eq!(gradebook(&ui).get_rows().row_count(), 1);
}

#[test]
fn the_add_row_follows_the_moved_name_columns() {
    let ui = seeded();
    click_element(&header(&ui, "Surname"));
    with_modifier(&ui, Key::Control, Key::RightArrow);
    key(&ui, Key::Escape);
    let (first_name, surname) = (find(&ui, "First name…"), find(&ui, "Surname…"));
    assert!((first_name.absolute_position().x - header_left(&ui, "First name")).abs() < 12.0, "under its column");
    assert!((surname.absolute_position().x - header_left(&ui, "Surname")).abs() < 12.0, "under its column");
    type_into(&ui, "First name…", "Carla");
    press_enter(&ui);
    type_text(&ui, "Iglesias");
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_rows().row_data(2).unwrap().full_name, "Carla Iglesias");
}

#[test]
fn dragging_a_header_moves_the_column() {
    let ui = four_columns();
    // the final grade dropped just after the first name
    drag_header_to(&ui, "Final grade", header_left(&ui, "Surname") + 4.0);
    assert_eq!(all_titles(&ui), ["First name", "Final grade", "Surname", "Exam", "Quiz", "Rounded"]);
    assert!(panel_is_closed(&ui), "a drag isn't a click");
    let state = ui.session.state.borrow();
    let connection = state.connection().unwrap();
    let class = classes::list_classes(connection).unwrap()[0].id;
    assert_eq!(columns::list_columns(connection, class).unwrap()[1].title, "Final grade", "saved");
}

#[test]
fn a_small_move_is_still_a_click_that_opens_the_panel() {
    let ui = four_columns();
    let start = header_left(&ui, "Quiz");
    drag_header_to(&ui, "Quiz", centre(&header(&ui, "Quiz")).x + 4.0);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32 + 1);
    assert_eq!(header_left(&ui, "Quiz"), start, "and nothing moved");
}

#[test]
fn dropping_a_column_on_its_own_place_writes_nothing() {
    let ui = four_columns();
    {
        let state = ui.session.state.borrow();
        state
            .connection()
            .unwrap()
            .execute_batch(
                "CREATE TEMP TABLE writes (n INTEGER);
                 CREATE TEMP TRIGGER count_writes AFTER UPDATE OF position ON sheet_columns
                 BEGIN INSERT INTO writes VALUES (1); END;",
            )
            .unwrap();
    }
    let quiz = header(&ui, "Quiz");
    // on its own right half: the gap after it, which is where it already is
    drag_header_to(&ui, "Quiz", quiz.absolute_position().x + quiz.size().width - 10.0);
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM writes"), 0);
    assert_eq!(column_titles(&ui), ["Exam", "Quiz", "Final grade", "Rounded"]);
}

#[test]
fn the_open_panel_follows_a_dragged_column() {
    let ui = four_columns();
    click(&ui, "Exam");
    key(&ui, Key::Escape);
    click(&ui, "Quiz");
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32 + 1);
    drag_header_to(&ui, "Quiz", header_left(&ui, "Exam") + 4.0);
    assert_eq!(column_titles(&ui), ["Quiz", "Exam", "Final grade", "Rounded"]);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32, "still on Quiz");
    assert_eq!(gradebook(&ui).get_panel_column().row_data(0).unwrap().title, "Quiz");
}

#[test]
fn pressing_the_resize_edge_never_starts_a_drag() {
    let ui = seeded();
    drag_edge(&width_edges(&ui)[2], 60.0);
    assert_eq!(all_titles(&ui), ["First name", "Surname", "Exam", "Final grade"], "nothing moved");
    assert_eq!(stored_column_width(&ui, FIRST_MARK), Some(188), "it resized instead");
}

#[test]
fn a_name_column_can_be_dragged() {
    let ui = seeded();
    let final_grade = header(&ui, "Final grade");
    // past the end of the last header: the last gap
    drag_header_to(&ui, "First name", final_grade.absolute_position().x + final_grade.size().width + 40.0);
    assert_eq!(all_titles(&ui), ["Surname", "Exam", "Final grade", "First name"]);
}
