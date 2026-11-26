// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:30.989644
//+ Revised: 	2026/11/23 13:00:30.989644
// -----------------------------------------------------------------------------

//! The class list: creating, renaming, deleting and duplicating classes.

use crate::common::Ui;
use crate::driver::{
    click, click_menu_option, count, english, error_text, gradebook, press_enter, shows, shows_text, type_into,
};
use crate::fixtures::{seeded, with_second_class};
use avalio_lite::error::AppError;
use avalio_lite::i18n::{Message, SheetMessage};
use slint::Model;

// ---------------------------------------------------------------------------------------------
// Classes

#[test]
fn a_class_is_created_from_the_empty_page() {
    let ui = Ui::with_database();
    assert!(shows_text(&ui, "Start with a class"));
    type_into(&ui, "Class name, e.g. 2º ESO A", "2º ESO A");
    click(&ui, "Create class");
    assert_eq!(gradebook(&ui).get_class_name(), "2º ESO A");
    assert!(shows(&ui, "2º ESO A"), "listed in the sidebar");
}

#[test]
fn an_empty_class_name_is_refused() {
    let ui = Ui::with_database();
    click(&ui, "Create class");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyName));
    assert_eq!(gradebook(&ui).get_classes().row_count(), 0);
}

#[test]
fn a_class_is_created_from_the_sidebar_and_selected() {
    let ui = seeded();
    type_into(&ui, "New class", "3C");
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_class_name(), "3C");
    assert_eq!(gradebook(&ui).get_classes().row_count(), 2);
}

#[test]
fn a_duplicate_class_name_is_refused() {
    let ui = seeded();
    type_into(&ui, "New class", "1A");
    press_enter(&ui);
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::DuplicateClassName("1A".into())));
}

#[test]
fn clicking_a_class_opens_its_sheet() {
    let ui = with_second_class();
    click(&ui, "2B");
    assert_eq!(gradebook(&ui).get_class_name(), "2B");
    assert_eq!(gradebook(&ui).get_rows().row_data(0).unwrap().full_name, "Carla");
}

#[test]
fn the_class_is_renamed_in_place() {
    let ui = seeded();
    type_into(&ui, "Class name", "1A bis");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM classes WHERE name = '1A bis'"), 1);
    assert!(shows(&ui, "1A bis"), "the sidebar follows");
}

#[test]
fn deleting_a_class_asks_first() {
    let ui = with_second_class();
    click(&ui, "More actions");
    ui.dialogs.answer_next(false);
    click(&ui, "Delete class");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM classes"), 2, "no keeps it");
    assert_eq!(
        ui.dialogs.questions.borrow()[0],
        english(&Message::Sheet(SheetMessage::ConfirmDeleteClass { class: "1A" }))
    );

    click(&ui, "More actions");
    ui.dialogs.answer_next(true);
    click(&ui, "Delete class");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM classes"), 1);
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "the next class opens");
}

#[test]
fn each_duplicate_option_creates_and_selects_the_copy() {
    let ui = seeded();
    let options = [
        ("Duplicate class", "1A (2)", 0),
        ("Duplicate with pupils", "1A (3)", 2),
        ("Duplicate with pupils and marks", "1A (4)", 2),
    ];
    for (index, (option, copy, rows)) in options.into_iter().enumerate() {
        click(&ui, "1A");
        click(&ui, "More actions");
        click_menu_option(&ui, option);
        assert_eq!(gradebook(&ui).get_class_name(), copy, "{option}: the copy is selected");
        assert_eq!(gradebook(&ui).get_rows().row_count(), rows, "{option}");
        assert_eq!(gradebook(&ui).get_classes().row_count(), index + 2, "{option}: listed");
        assert!(shows(&ui, copy), "{option}: in the sidebar");
        assert!(ui.session.window.get_error_message().is_empty());
    }
}

#[test]
fn a_failed_duplicate_shows_the_banner() {
    let ui = seeded();
    ui.session
        .state
        .borrow()
        .connection()
        .unwrap()
        .execute_batch(
            "CREATE TEMP TRIGGER no_cells BEFORE INSERT ON sheet_columns BEGIN SELECT RAISE(ABORT, 'test'); END",
        )
        .unwrap();
    click(&ui, "More actions");
    click_menu_option(&ui, "Duplicate with pupils and marks");
    assert!(!ui.session.window.get_error_message().is_empty(), "the error is shown");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM classes"), 1, "nothing was added");
    assert_eq!(gradebook(&ui).get_class_name(), "1A", "the original stays open");
}
