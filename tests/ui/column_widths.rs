// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.403817
//+ Revised: 	2026/11/23 13:00:31.403817
// -----------------------------------------------------------------------------

//! Column widths: the stored width, dragging an edge, double-clicking it, and the width keys.

use crate::common::Ui;
use crate::driver::{click, click_element, find, gradebook, key};
use crate::fixtures::{seeded, with_second_class};
use crate::screens::{FIRST_MARK, drag_edge, panel_is_closed, stored_column_width, width_edges, with_modifier};
use avalio_lite::ui::ColumnWidths;
use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, Model};

/// Two quick clicks in the middle of `element`, as a double-click arrives.
fn double_click(ui: &Ui, element: &ElementHandle) {
    let start = element.absolute_position();
    let position =
        slint::LogicalPosition::new(start.x + element.size().width / 2.0, start.y + element.size().height / 2.0);
    let window = ui.session.window.window();
    window.dispatch_event(WindowEvent::PointerMoved { position });
    for _ in 0..2 {
        window.dispatch_event(WindowEvent::PointerPressed { position, button: PointerEventButton::Left });
        window.dispatch_event(WindowEvent::PointerReleased { position, button: PointerEventButton::Left });
    }
}

#[test]
fn the_sheet_width_limits_match_the_stored_ones() {
    use avalio_lite::gradebook::model::{MAX_COLUMN_WIDTH, MIN_COLUMN_WIDTH};
    let ui = seeded();
    let widths = ui.session.window.global::<ColumnWidths>();
    assert_eq!(widths.get_column_min(), f32::from(MIN_COLUMN_WIDTH));
    assert_eq!(widths.get_max(), f32::from(MAX_COLUMN_WIDTH));
}

#[test]
fn dragging_a_column_edge_resizes_and_saves_it() {
    let ui = seeded();
    let edges = width_edges(&ui);
    assert_eq!(edges.len(), 4, "both name columns and both mark columns have an edge");
    assert_eq!(edges[0].accessible_value().as_deref(), Some("160"), "the first name starts at its default");
    assert_eq!(edges[1].accessible_value().as_deref(), Some("210"), "and the surname at its own");
    assert_eq!(edges[2].accessible_value().as_deref(), Some("128"), "a test column starts at its default");
    drag_edge(&edges[2], 80.0);
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().width, 208);
    assert_eq!(stored_column_width(&ui, FIRST_MARK), Some(208));
    assert_eq!(find(&ui, "Ana, Exam").size().width, 208.0 - 24.0, "the cells follow the header");
    assert_eq!(stored_column_width(&ui, FIRST_MARK + 1), None, "the other column keeps its default");
    assert!(panel_is_closed(&ui), "a drag doesn't open the column panel");
}

#[test]
fn a_drag_is_clamped_to_the_limits() {
    let ui = seeded();
    drag_edge(&width_edges(&ui)[2], -500.0);
    assert_eq!(stored_column_width(&ui, FIRST_MARK), Some(72), "no narrower than a pill");
    drag_edge(&width_edges(&ui)[2], 1500.0);
    assert_eq!(stored_column_width(&ui, FIRST_MARK), Some(640));
    drag_edge(&width_edges(&ui)[0], -500.0);
    assert_eq!(stored_column_width(&ui, 0), Some(72), "the first name has the same limits as any column");
}

#[test]
fn double_clicking_the_edge_restores_the_default() {
    let ui = seeded();
    drag_edge(&width_edges(&ui)[2], 80.0);
    assert_eq!(stored_column_width(&ui, FIRST_MARK), Some(208));
    double_click(&ui, &width_edges(&ui)[2]);
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().width, 0);
    assert_eq!(stored_column_width(&ui, FIRST_MARK), None, "the default is stored as no width at all");
    assert_eq!(width_edges(&ui)[2].accessible_value().as_deref(), Some("128"));
}

#[test]
fn shift_arrow_on_a_focused_header_changes_its_width() {
    let ui = seeded();
    click(&ui, "Class name");
    // Tab on from the class name until a Shift+→ widens the first column: its header has focus
    let mut reached = false;
    for _ in 0..20 {
        key(&ui, Key::Tab);
        with_modifier(&ui, Key::Shift, Key::RightArrow);
        if stored_column_width(&ui, 0).is_some() {
            reached = true;
            break;
        }
    }
    assert!(reached, "Tab reaches the first-name header");
    assert_eq!(stored_column_width(&ui, 0), Some(176), "16px wider");
    with_modifier(&ui, Key::Shift, Key::LeftArrow);
    with_modifier(&ui, Key::Shift, Key::LeftArrow);
    assert_eq!(stored_column_width(&ui, 0), Some(144), "16px narrower each time");
    assert!(panel_is_closed(&ui), "Shift+arrow doesn't open the panel");
}

#[test]
fn a_screen_reader_can_step_the_width() {
    let ui = seeded();
    width_edges(&ui)[2].invoke_accessible_increment_action();
    assert_eq!(stored_column_width(&ui, FIRST_MARK), Some(144));
    width_edges(&ui)[2].invoke_accessible_decrement_action();
    assert_eq!(stored_column_width(&ui, FIRST_MARK), Some(128));
}

#[test]
fn the_name_column_widths_are_kept_per_class() {
    let ui = with_second_class();
    let name_width = |ui: &Ui, index: usize| gradebook(ui).get_columns().row_data(index).unwrap().width;
    drag_edge(&width_edges(&ui)[0], 90.0);
    drag_edge(&width_edges(&ui)[1], -60.0);
    assert_eq!((name_width(&ui, 0), name_width(&ui, 1)), (250, 150));
    click(&ui, "2B");
    assert_eq!((name_width(&ui, 0), name_width(&ui, 1)), (0, 0), "the other class keeps its defaults");
    click(&ui, "1A");
    assert_eq!((name_width(&ui, 0), name_width(&ui, 1)), (250, 150), "and this one keeps its own");
    assert_eq!(width_edges(&ui)[0].accessible_value().as_deref(), Some("250"));
}

/// Was: with no columns the header row had no height, so the pupil column's edge couldn't be grabbed.
#[test]
fn regression_a_class_without_columns_still_has_a_pupil_edge_to_drag() {
    let ui = with_second_class();
    let edges = width_edges(&ui);
    assert_eq!(edges.len(), 2, "only the name columns");
    assert!(edges.iter().all(|edge| edge.size().height >= 44.0), "a whole row high: {:?}", edges[0].size());
}

#[test]
fn a_click_on_the_edge_does_not_open_the_column_panel() {
    let ui = seeded();
    click_element(&width_edges(&ui)[2]);
    assert!(panel_is_closed(&ui));
    assert_eq!(stored_column_width(&ui, FIRST_MARK), None, "a click without a drag saves nothing");
    click(&ui, "Exam");
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32, "the rest of the header still opens the panel");
}

#[test]
fn resizing_the_column_in_the_panel_keeps_the_panel_open_on_it() {
    let ui = seeded();
    click(&ui, "Exam");
    drag_edge(&width_edges(&ui)[2], 60.0);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32);
    assert_eq!(stored_column_width(&ui, FIRST_MARK), Some(188));
}

#[test]
fn a_failed_width_save_shows_the_error() {
    let ui = seeded();
    {
        let state = ui.session.state.borrow();
        state
            .connection()
            .unwrap()
            .execute_batch(
                "CREATE TEMP TRIGGER no_widths BEFORE UPDATE OF width ON sheet_columns
                 BEGIN SELECT RAISE(ABORT, 'test'); END",
            )
            .unwrap();
    }
    drag_edge(&width_edges(&ui)[2], 80.0);
    assert!(!ui.session.window.get_error_message().is_empty(), "the failure is on screen");
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().width, 0, "the column snaps back");
    assert_eq!(width_edges(&ui)[2].accessible_value().as_deref(), Some("128"));
}
