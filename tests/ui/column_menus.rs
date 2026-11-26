// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.236032
//+ Revised: 	2026/11/25 11:43:16.309501
// -----------------------------------------------------------------------------

//! The column menus and the column panel from the keyboard, and the panel closing with the page.

use crate::common::Fixture;
use crate::driver::{click, gradebook, key, press_enter, replace_text, shows, type_into};
use crate::fixtures::{open_folder_settings, seeded};
use crate::screens::FIRST_MARK;
use avalio_lite::gradebook::model::ColumnType;
use avalio_lite::gradebook::{classes, columns};
use avalio_lite::i18n::Language;
use slint::Model;
use slint::platform::Key;

/// The panel is matched to its column by row id, and a new database has columns with the same ids.
#[test]
fn regression_opening_another_database_closes_the_column_panel() {
    let other = Fixture::new();
    let mut state = other.set_up();
    {
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "Other class").unwrap();
        columns::add_column(connection, class, ColumnType::Test, Language::English).unwrap();
    }
    state.shutdown();
    let ui = seeded();
    click(&ui, "Exam");
    assert_eq!(gradebook(&ui).get_panel_index(), FIRST_MARK as i32);
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open another database…");
    assert_eq!(gradebook(&ui).get_class_name(), "Other class");
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert_eq!(gradebook(&ui).get_panel_column().row_count(), 0);
}

#[test]
fn regression_deleting_the_last_class_closes_the_column_panel() {
    let ui = seeded();
    click(&ui, "Exam");
    ui.dialogs.answer_next(true);
    gradebook(&ui).invoke_delete_class();
    assert_eq!(gradebook(&ui).get_current_class_id(), -1);
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert_eq!(gradebook(&ui).get_panel_column().row_count(), 0);
}

#[test]
fn regression_the_add_column_menu_works_from_the_keyboard() {
    let ui = seeded();
    click(&ui, "+ Add column");
    key(&ui, Key::DownArrow);
    press_enter(&ui);
    let columns = gradebook(&ui).get_columns();
    assert_eq!(columns.row_count(), 5);
    assert_eq!(columns.row_data(FIRST_MARK + 2).unwrap().kind, avalio_lite::ui::ColumnType::Info, "Down moved to Info");
    assert!(!shows(&ui, "Rounded"), "choosing closes the menu");

    click(&ui, "+ Add column");
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_columns().row_count(), 6, "the first option is the one on");
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK + 3).unwrap().kind, avalio_lite::ui::ColumnType::Test);

    click(&ui, "+ Add column");
    key(&ui, Key::UpArrow);
    key(&ui, Key::Escape);
    assert!(!shows(&ui, "Rounded"), "Escape closes the menu");
    assert_eq!(gradebook(&ui).get_columns().row_count(), 6, "and adds nothing");
}

#[test]
fn regression_the_more_menu_works_from_the_keyboard() {
    let ui = seeded();
    click(&ui, "More actions");
    key(&ui, Key::DownArrow);
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_class_name(), "1A (2)", "the second option duplicates the class");
    // back on 1A, whose pupils are all in it, so "Add from another class" isn't offered
    click(&ui, "1A");
    click(&ui, "More actions");
    for _ in 0..4 {
        key(&ui, Key::DownArrow);
    }
    press_enter(&ui);
    assert!(!gradebook(&ui).get_show_column_settings(), "the fifth option toggles the weights and scales");
    click(&ui, "More actions");
    key(&ui, Key::UpArrow);
    key(&ui, Key::Escape);
    assert!(!shows(&ui, "Delete class"), "Escape closes the menu");
    assert_eq!(gradebook(&ui).get_classes().row_count(), 2, "nothing more was added or deleted");
}

#[test]
fn regression_the_column_panel_starts_on_its_title_and_escape_closes_it() {
    let ui = seeded();
    click(&ui, "Exam");
    replace_text(&ui, "Quiz");
    assert_eq!(gradebook(&ui).get_columns().row_data(FIRST_MARK).unwrap().title, "Quiz", "typing went to the title");
    key(&ui, Key::Escape);
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert!(!shows(&ui, "Weight"));
}

#[test]
fn regression_closing_the_panel_takes_a_refused_value_error_with_it() {
    // Escape on a field holding a refused weight closed the panel but left its error showing
    let ui = seeded();
    click(&ui, "Exam");
    type_into(&ui, "Weight", "x");
    assert!(!ui.session.window.get_error_message().is_empty(), "the refused weight is reported");
    key(&ui, Key::Escape);
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert!(ui.session.window.get_error_message().is_empty(), "{}", ui.session.window.get_error_message());
}

#[test]
fn a_stale_column_index_opens_no_panel() {
    // a click that lands after the column went (a reload in between) must not open anything
    let ui = seeded();
    gradebook(&ui).invoke_open_column_panel(99);
    assert_eq!(gradebook(&ui).get_panel_index(), -1);
    assert!(ui.session.window.get_error_message().is_empty());
}
