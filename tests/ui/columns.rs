// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.070703
//+ Revised: 	2026/11/23 13:00:31.070703
// -----------------------------------------------------------------------------

//! Columns: adding them from the menu, opening their settings, weights and moving them.

use crate::driver::{click, click_element, click_menu_option, find, gradebook, shows, shows_text, type_into};
use crate::fixtures::{header, seeded};
use crate::screens::{FIRST_MARK, column_titles};
use slint::Model;

// ---------------------------------------------------------------------------------------------
// Columns

#[test]
fn every_column_type_can_be_added_from_the_menu() {
    let ui = seeded();
    for (option, title) in [("Info", "Notes"), ("Average", "Average"), ("Rounded", "Rounded"), ("Test", "Test 2")] {
        click(&ui, "+ Add column");
        click_menu_option(&ui, option);
        assert!(gradebook(&ui).get_columns().iter().any(|column| column.title == title), "{title}");
    }
    click(&ui, "+ Add column");
    click_menu_option(&ui, "Final grade");
    assert_eq!(gradebook(&ui).get_columns().row_count(), 9);
}

#[test]
fn clicking_a_header_opens_its_settings_panel() {
    let ui = seeded();
    click(&ui, "Exam");
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32);
    assert!(shows(&ui, "Weight"));
    click(&ui, "Close");
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert!(!shows(&ui, "Weight"));
}

#[test]
fn the_panel_edits_a_tests_settings() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Title", "Exam 1");
    type_into(&ui, "Weight", "60");
    type_into(&ui, "Out of", "5");
    click(&ui, "Bonus test");
    let exam = gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap();
    assert_eq!(exam.title, "Exam 1");
    assert_eq!(exam.weight, "60");
    assert_eq!(exam.max_score, "5");
    assert!(exam.bonus);
    assert!(!gradebook(&ui).get_weights_complete(), "a bonus test leaves the normal tests at 0%");
}

#[test]
fn an_invalid_weight_is_reported_in_the_banner() {
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "lots");
    assert!(!ui.session.window.get_error_message().is_empty());
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().weight, "100", "the weight is unchanged");
}

#[test]
fn weights_that_dont_add_up_warn_and_grey_the_final_grade() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "8");
    click(&ui, "Exam");
    type_into(&ui, "Weight", "50");
    assert!(!gradebook(&ui).get_weights_complete());
    assert!(shows_text(&ui, "Test weights add up to 50%, not 100%"));
    let final_grade = gradebook(&ui).get_rows().row_data(0).unwrap().cells.row_data(FIRST_MARK + 1).unwrap();
    assert!(final_grade.provisional);
}

#[test]
fn columns_move_and_the_ends_are_disabled() {
    let ui = seeded();
    click_element(&header(&ui, "First name"));
    assert_eq!(find(&ui, "Move left (Ctrl+←)").accessible_enabled(), Some(false), "nothing before the first column");
    click(&ui, "Final grade");
    assert_eq!(find(&ui, "Move right (Ctrl+→)").accessible_enabled(), Some(false), "nor after the last");
    click(&ui, "Exam");
    click(&ui, "Move right (Ctrl+→)");
    assert_eq!(column_titles(&ui), ["Final grade", "Exam"]);
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32 + 1, "the panel follows the column");
    click(&ui, "Move left (Ctrl+←)");
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().title, "Exam");
}
