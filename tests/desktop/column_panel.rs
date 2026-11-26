// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.263873
//+ Revised: 	2026/11/24 17:54:07.502924
// -----------------------------------------------------------------------------

//! The column panel that opens from a column's header: how it looks, its steps between columns, moving
//! a column with it, Escape closing it, and the window's look while it has lost the focus.

use crate::desktop_support::pixels::Image;
use crate::desktop_support::{self, App};

use super::fixtures::{FIRST_MARK, PANEL_CONTROLS, VK_LEFT, config, with_info_column, with_sample_class};
use super::screens::{MINIMUM, assert_on_screen, every_look, galician_look};

/// The panel's buttons and the Bonus test field, in English, on the screen.
fn assert_panel_controls(app: &mut App, image: &Image) {
    for control in PANEL_CONTROLS.iter().chain(&["Bonus test"]) {
        assert_on_screen(image, app.located(control), control);
    }
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_column_panel_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Exam 1");
    assert_eq!(app.state()["panel_index"], FIRST_MARK);
    every_look(&mut app, "column-panel", |app, image, _, _| assert_panel_controls(app, image));
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_column_panel_in_galician() {
    let _turn = desktop_support::turn();
    // the Galician panel is captured after the Info column was added, as a user would have left it
    let fixture = with_info_column();
    let mut app = App::start(&config(&fixture), "gl");
    app.click("Exam 1");
    let image = galician_look(&mut app, "column-panel");
    for english in PANEL_CONTROLS {
        let label = crate::common::galician(english);
        assert_on_screen(&image, app.located(&label), &label);
    }
    app.click(&crate::common::galician("Close"));
    app.click(&crate::common::galician("+ Add column"));
    let image = galician_look(&mut app, "add-column-menu");
    for english in ["Test", "Info", "Final grade", "Average", "Rounded"] {
        let label = crate::common::galician(english);
        let found = app.locate_in_menu(&label);
        assert_eq!(found.len(), 1, "{label}");
        assert_on_screen(&image, found[0], &label);
    }
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_panel_steps_between_columns() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Exam 1");
    app.click("Next column (Alt+→)");
    assert_eq!(app.state()["panel_index"], FIRST_MARK + 1, "a real click on › showed the next column");
    app.click("Previous column (Alt+←)");
    assert_eq!(app.state()["panel_index"], FIRST_MARK);
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_move_button_moves_the_column_and_ctrl_left_moves_it_back() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Exam 1");
    app.click("Move right (Ctrl+→)");
    let state = app.state();
    assert_eq!(state["columns"][FIRST_MARK + 1], "Exam 1", "a real click moved the column: {state}");
    app.press_with_control(VK_LEFT);
    let state = app.state();
    assert_eq!(state["columns"][FIRST_MARK], "Exam 1", "Ctrl+← moved it back: {state}");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn escape_closes_the_column_panel() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Exam 1");
    app.press_escape();
    assert_eq!(app.state()["panel_index"], -1, "Escape closed the panel");
    app.quit();
}

/// A window that lost the focus (another one came to the front during the run) is still captured
/// as the user sees it while working in it: the open panel's Title field drawn focused.
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_snapshot_shows_the_window_active() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.click("Exam 1");
    // the smallest size, where the field's outline is the biggest share of the screen
    app.resize(MINIMUM.0, MINIMUM.1);
    app.theme(false);
    app.window.lose_focus();
    app.settle();
    assert_eq!(app.state()["active"], false, "WM_KILLFOCUS made the window inactive");
    app.snapshot("column-panel-light-1000x600");
    app.quit();
}
