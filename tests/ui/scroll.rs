// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.920452
//+ Revised: 	2026/11/25 11:43:19.440841
// -----------------------------------------------------------------------------

//! Focus scrolling: the focused cell is scrolled whole into view, and long text in a field.

use crate::common::{Fixture, Ui};
use crate::driver::{
    click, click_element, count, elements, gradebook, key, press_enter, replace_text, type_into, type_text,
};
use crate::fixtures::{name_fields, seeded};
use crate::screens::with_modifier;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, columns, pupils};
use avalio_lite::i18n::Language;
use i_slint_backend_testing::ElementHandle;
use slint::ComponentHandle;
use slint::platform::{Key, WindowEvent};

// ---------------------------------------------------------------------------------------------
// Focus scrolls whole cells into view

/// Class "1A" with Ana and Bea, the test "Exam" and a 220px "Notes" column, in a 1000px window: the
/// sheet is wider than its card, so it scrolls sideways.
fn with_a_scrolling_sheet() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        for name in ["Ana", "Bea"] {
            pupils::add_new_pupil(connection, class, name, "").unwrap();
        }
        let exam = columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
        columns::rename_column(connection, exam, "Exam").unwrap();
        let notes = columns::add_column(connection, class, ColumnType::Info, Language::English).unwrap();
        columns::rename_column(connection, notes, "Notes").unwrap();
        // wide enough to start out partly hidden, with its middle still in view to click
        columns::set_column_width(connection, notes, Some(220)).unwrap();
    }
    state.shutdown();
    let ui = Ui::open(fixture);
    ui.session.window.window().set_size(slint::LogicalSize::new(1000.0, 600.0));
    ui
}

/// The sheet's visible area, as (left, right) in window coordinates.
fn sheet_view(ui: &Ui) -> (f32, f32) {
    let view = ElementHandle::find_by_element_id(&ui.session.window, "SheetGrid::sheet-view").next().unwrap();
    (view.absolute_position().x, view.absolute_position().x + view.size().width)
}

/// The cell around the field labelled `label` (the `nth`), as (left, right): a cell's text box
/// sits `padding` inside it on each side.
fn cell_span(ui: &Ui, label: &str, nth: usize, padding: f32) -> (f32, f32) {
    let (_, field) = text_and_box(ui, label, nth);
    (field.0 - padding, field.1 + padding)
}

#[test]
fn a_cell_in_the_last_column_scrolls_whole_into_view_with_its_focus_border() {
    let ui = with_a_scrolling_sheet();
    let (left, right) = sheet_view(&ui);
    let before = cell_span(&ui, "Ana, Notes", 0, 12.0);
    assert!(before.1 > right + 1.0, "the cell starts out partly hidden: {before:?}, view to {right}");
    click(&ui, "Ana, Notes");
    let cell = cell_span(&ui, "Ana, Notes", 0, 12.0);
    assert!(cell.1 <= right + 0.5, "its right edge shows: {cell:?}, view to {right}");
    assert!(cell.0 >= left - 0.5, "and its left edge: {cell:?}, view from {left}");
}

#[test]
fn enter_into_a_hidden_row_cell_scrolls_the_whole_cell_into_view() {
    let ui = with_a_scrolling_sheet();
    click(&ui, "Ana, Notes");
    type_text(&ui, "a");
    press_enter(&ui);
    let (_, right) = sheet_view(&ui);
    let cell = cell_span(&ui, "Bea, Notes", 0, 12.0);
    assert!(cell.1 <= right + 0.5, "Bea's note shows whole: {cell:?}, view to {right}");
}

#[test]
fn going_back_to_a_pupil_name_scrolls_the_whole_name_cell_into_view() {
    let ui = with_a_scrolling_sheet();
    let (left, _) = sheet_view(&ui);
    // the remove button at the start of the cell is the leftmost thing in it; one scrolled out of
    // the sheet's view isn't found
    let remove = || elements(&ui, "Remove from this class").first().map(|button| button.absolute_position().x);
    click(&ui, "Ana, Notes");
    assert_eq!(remove(), None, "the sheet scrolled the name cells' start away");
    // from Ana's note back past her exam and surname to her first name
    for _ in 0..3 {
        with_modifier(&ui, Key::Shift, Key::Tab);
    }
    let shown = remove().expect("the remove button shows again");
    assert!(shown >= left - 0.5, "{shown}, view from {left}");
}

#[test]
fn tabbing_to_add_pupil_scrolls_its_whole_row_start_into_view() {
    let ui = with_a_scrolling_sheet();
    let (left, _) = sheet_view(&ui);
    click(&ui, "Bea, Notes");
    // the last cell of the last row is followed by the new pupil's field
    key(&ui, Key::Tab);
    let (_, field) = text_and_box(&ui, "First name…", 0);
    // past the "+" strip and its grid line (69px), the field sits 10px into its cell and its text
    // 10px into the focused field: the row's start is back in view
    assert!((field.0 - (left + 89.0)).abs() < 1.0, "the text starts at {}, view from {left}", field.0);
}

// ---------------------------------------------------------------------------------------------
// Long text in a field

/// The text of the field labelled `label` (the `nth` of them) and the box it slides in, as
/// (left, right) pairs in window coordinates. Each field's text and box come in the same order
/// along a row, so the box is the one at the text's place among the row's boxes.
fn text_and_box(ui: &Ui, label: &str, nth: usize) -> ((f32, f32), (f32, f32)) {
    let input = &name_fields(ui, label)[nth];
    let (at, size) = (input.absolute_position(), input.size());
    let on_row = |id: &str| -> Vec<(f32, f32)> {
        let mut found: Vec<(f32, f32)> = Vec::new();
        for element in ElementHandle::find_by_element_id(&ui.session.window, id) {
            let (position, element_size) = (element.absolute_position(), element.size());
            let span = (position.x, position.x + element_size.width);
            if (position.y - at.y).abs() < 0.5 && !found.contains(&span) {
                found.push(span);
            }
        }
        found
    };
    let text = (at.x, at.x + size.width);
    let index = on_row("ScrollingTextInput::input").iter().position(|span| *span == text).unwrap();
    (text, on_row("ScrollingTextInput::box")[index])
}

fn is_scrolled(ui: &Ui, label: &str, nth: usize) -> bool {
    let (text, field) = text_and_box(ui, label, nth);
    text.0 < field.0 - 0.5
}

fn shows_its_end(ui: &Ui, label: &str, nth: usize) -> bool {
    let (text, field) = text_and_box(ui, label, nth);
    (text.1 - field.1).abs() < 0.5 && text.0 < field.0 - 0.5
}

const LONG_NAME: &str = "Ana María Fernández de la Iglesia Rodríguez-Castelao y Pérez de Sotomayor e Outra";

#[test]
fn a_long_pupil_name_scrolls_to_the_cursor() {
    let ui = seeded();
    click_element(&name_fields(&ui, "First name")[0]);
    replace_text(&ui, LONG_NAME);
    assert!(LONG_NAME.chars().count() >= 80);
    assert!(shows_its_end(&ui, "First name", 0), "typing at the end keeps the cursor in view");
    key(&ui, Key::Home);
    assert!(!is_scrolled(&ui, "First name", 0), "Home shows the start");
    key(&ui, Key::End);
    assert!(shows_its_end(&ui, "First name", 0), "End shows the end");
    for _ in 0..10 {
        key(&ui, Key::LeftArrow);
    }
    assert!(is_scrolled(&ui, "First name", 0), "a few steps left from the end stays scrolled");
    assert_eq!(count(&ui, &format!("SELECT COUNT(*) FROM pupils WHERE first_name = '{LONG_NAME}'")), 1);
}

#[test]
fn leaving_the_field_shows_its_start_again() {
    let ui = seeded();
    click_element(&name_fields(&ui, "First name")[0]);
    replace_text(&ui, LONG_NAME);
    assert!(is_scrolled(&ui, "First name", 0));
    key(&ui, Key::Tab);
    assert!(!is_scrolled(&ui, "First name", 0), "the resting sheet shows the start of the name");
}

#[test]
fn a_short_name_does_not_move() {
    let ui = seeded();
    click_element(&name_fields(&ui, "First name")[0]);
    key(&ui, Key::End);
    type_text(&ui, " M.");
    let (text, field) = text_and_box(&ui, "First name", 0);
    assert_eq!(text, field, "a text that fits fills its box and never slides");
}

#[test]
fn shift_arrows_select_and_scroll_the_same_way() {
    let ui = seeded();
    click_element(&name_fields(&ui, "First name")[0]);
    replace_text(&ui, LONG_NAME);
    key(&ui, Key::Home);
    assert!(!is_scrolled(&ui, "First name", 0));
    for _ in 0..LONG_NAME.chars().count() {
        with_modifier(&ui, Key::Shift, Key::RightArrow);
    }
    assert!(shows_its_end(&ui, "First name", 0), "selecting to the end follows the cursor");
}

#[test]
fn a_long_note_in_a_text_cell_scrolls() {
    let ui = seeded();
    gradebook(&ui).invoke_add_column(avalio_lite::ui::ColumnType::Info);
    type_into(&ui, "Ana, Notes", LONG_NAME);
    assert!(shows_its_end(&ui, "Ana, Notes", 0));
    key(&ui, Key::Home);
    assert!(!is_scrolled(&ui, "Ana, Notes", 0));
    key(&ui, Key::Tab);
    assert!(!is_scrolled(&ui, "Ana, Notes", 0));
}

/// Was: losing the focus slid the text back to its start, and getting it back didn't slide it to
/// the cursor again, so after switching to another window and back the cursor at the end of a long
/// note was out of sight. Found by the desktop tests' dark captures, taken after a light one.
#[test]
fn regression_the_cursor_shows_again_when_the_window_is_active_again() {
    let ui = seeded();
    gradebook(&ui).invoke_add_column(avalio_lite::ui::ColumnType::Info);
    type_into(&ui, "Ana, Notes", LONG_NAME);
    assert!(shows_its_end(&ui, "Ana, Notes", 0));
    // `changed` handlers run once the event loop gets round to them, as it does between real events
    let active = |active: bool| {
        ui.session.window.window().dispatch_event(WindowEvent::WindowActiveChanged(active));
        slint::platform::update_timers_and_animations();
    };
    active(false);
    assert!(!is_scrolled(&ui, "Ana, Notes", 0), "an inactive window shows the note's start");
    active(true);
    assert!(shows_its_end(&ui, "Ana, Notes", 0), "back at the cursor, at the end");
    key(&ui, Key::Home);
    active(false);
    active(true);
    assert!(!is_scrolled(&ui, "Ana, Notes", 0), "a cursor at the start stays at the start");
}

#[test]
fn a_right_aligned_mark_cell_scrolls_mirrored() {
    let ui = seeded();
    let long = "1234567890123456789012345678901234567890";
    type_into(&ui, "Ana, Exam", long);
    // a number this long is past the test's maximum, so the cell now says it isn't a valid mark
    let flagged = "Ana, Exam (not a valid mark)";
    assert!(shows_its_end(&ui, flagged, 0), "the end of a long mark shows at the right edge");
    key(&ui, Key::Home);
    assert!(!is_scrolled(&ui, flagged, 0));
    type_into(&ui, "Bea, Exam", "7");
    let (text, field) = text_and_box(&ui, "Bea, Exam", 0);
    assert_eq!(text, field, "a short mark doesn't move");
}
