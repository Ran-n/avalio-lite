// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.490928
//+ Revised: 	2026/11/24 17:54:07.823293
// -----------------------------------------------------------------------------

//! Dragging a column's header to another place: the drop line while it is held, the new order after
//! the drop, the order kept after a restart, and the moved sheet's names in English and Galician.

use crate::desktop_support::{self, App};

use super::fixtures::{MOVED, column_titles, config, with_sample_class};
use super::screens::{NORMAL, assert_on_screen, every_look, galician_look};

/// Surname held by its header and dragged past the middle of Exam 1, then let go: the drop.
/// Each drag starts from the same sheet, since a drop moves the column for good.
fn drag_surname_past_exam_1(app: &mut App) {
    // from the middle of Surname (210px wide) to past the middle of Exam 1 (128px), just after it
    let at = app.hold_drag_nth("Surname", 0, 105.0 + 100.0);
    app.release(at);
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_header_moves_only_on_the_drop() {
    let _turn = desktop_support::turn();
    for dark in [false, true] {
        let fixture = with_sample_class();
        let mut app = App::start(&config(&fixture), "en");
        app.resize(NORMAL.0, NORMAL.1);
        app.theme(dark);
        // from the middle of Surname (210px wide) to past the middle of Exam 1 (128px), just after it
        let at = app.hold_drag_nth("Surname", 0, 105.0 + 100.0);
        let theme = if dark { "dark" } else { "light" };
        app.snapshot_holding(&format!("sheet-drag-{theme}-{}x{}", NORMAL.0, NORMAL.1));
        assert_eq!(column_titles(&app.state())[1], "Surname", "nothing moves before the drop");
        app.release(at);
        let titles = column_titles(&app.state());
        assert_eq!(titles[..3], MOVED[..3], "the drop moved Surname after Exam 1");
        app.quit();
    }
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_dropped_header_moves_its_column() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    drag_surname_past_exam_1(&mut app);
    assert_eq!(column_titles(&app.state()), MOVED, "a real drag moved Surname");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_moved_names_look_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    drag_surname_past_exam_1(&mut app);
    every_look(&mut app, "sheet-moved-names", |app, image, _, _| {
        for header in ["First name", "Exam 1", "Surname"] {
            let found = app.located(header);
            assert_on_screen(image, found, header);
        }
        let first = app.located("First name");
        let exam = app.located("Exam 1");
        let surname = app.located("Surname");
        assert!(first.x < exam.x && exam.x < surname.x, "drawn in their new order: {first:?} {exam:?} {surname:?}");
    });
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_moved_order_survives_a_restart() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    drag_surname_past_exam_1(&mut app);
    app.quit();
    let mut reopened = App::start(&config(&fixture), "en");
    assert_eq!(column_titles(&reopened.state()), MOVED, "the order is kept after a restart");
    reopened.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_moved_names_in_galician() {
    let _turn = desktop_support::turn();
    // the Galician sheet is captured after the column was dropped, as a user would have left it
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    drag_surname_past_exam_1(&mut app);
    app.quit();
    let mut app = App::start(&config(&fixture), "gl");
    let image = galician_look(&mut app, "sheet-moved-names");
    for header in ["Nome", "Apelidos", "Nome…", "Apelidos…"] {
        let found = app.locate(header);
        assert!(!found.is_empty(), "{header}");
        assert_on_screen(&image, found[0], header);
    }
    app.quit();
}
