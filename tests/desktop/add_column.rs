// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:02.040143
//+ Revised: 	2026/11/23 15:53:48.892216
// -----------------------------------------------------------------------------

//! The add-column menu from the sheet's "+ Add column" button: how it looks at each size, a click on
//! an option, and the keyboard choosing an option, closing the menu or taking the first option.

use crate::desktop_support::{self, App};

use super::fixtures::{FIRST_MARK, VK_LEFT, config, with_sample_class};
use super::screens::{MINIMUM, assert_on_screen, every_look_after};

/// The sheet as the menu's baselines show it: scrolled to Exam 1 by the column panel at the minimum
/// size, with a change just saved, as a user who moved a column there and back leaves it.
fn move_exam_there_and_back(app: &mut App) {
    app.resize(MINIMUM.0, MINIMUM.1);
    app.theme(true);
    app.click("Exam 1");
    app.click("Next column (Alt+→)");
    app.click("Previous column (Alt+←)");
    app.click("Move right (Ctrl+→)");
    app.press_with_control(VK_LEFT);
    app.press_escape();
}

/// How many columns the sheet has now.
fn column_count(app: &mut App) -> usize {
    app.state()["columns"].as_array().unwrap().len()
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_add_column_menu_looks_right() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    move_exam_there_and_back(&mut app);
    // resizing closes an open menu, so it opens again at each size
    every_look_after(
        &mut app,
        "add-column-menu",
        |app| app.open_add_column_menu(),
        |app, image, _, _| {
            for option in ["Test", "Info", "Final grade", "Average", "Rounded"] {
                let found = app.locate_in_menu(option);
                assert_eq!(found.len(), 1, "{option}");
                assert_on_screen(image, found[0], option);
            }
        },
    );
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn a_click_on_an_option_adds_a_column() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    app.open_add_column_menu();
    app.click_in_menu("Info");
    assert_eq!(app.state()["columns"].as_array().unwrap().len(), FIRST_MARK + 7, "a real click added a column");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn down_then_enter_picks_the_second_option() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    let before = column_count(&mut app);
    app.open_add_column_menu();
    app.press_down();
    app.press_enter();
    assert_eq!(column_count(&mut app), before + 1, "Down then Enter added the second option, Info");
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn escape_closes_the_add_column_menu_without_adding() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    let before = column_count(&mut app);
    app.open_add_column_menu();
    app.press_escape();
    assert!(app.locate("Rounded").is_empty(), "Escape closed the menu");
    assert_eq!(column_count(&mut app), before);
    app.quit();
}

#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn enter_alone_picks_the_first_option() {
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let mut app = App::start(&config(&fixture), "en");
    let before = column_count(&mut app);
    app.open_add_column_menu();
    app.press_enter();
    assert_eq!(column_count(&mut app), before + 1, "the first option is the one on");
    app.quit();
}
