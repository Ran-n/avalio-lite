// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.566425
//+ Revised: 	2026/11/23 13:00:32.566425
// -----------------------------------------------------------------------------

//! Pupils: adding, renaming and removing them, and pupils from other classes or with no class.

use crate::common::{Fixture, Ui};
use crate::driver::{
    click, click_element, click_nth, count, english, error_text, gradebook, key, press_enter, replace_text, shows,
    shows_text, type_into, type_text,
};
use crate::fixtures::{name_fields, seeded, with_second_class};
use crate::screens::{error_on_screen, field_text};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::{classes, pupils};
use avalio_lite::i18n::{Message, SheetMessage};
use slint::Model;
use slint::platform::Key;

// ---------------------------------------------------------------------------------------------
// Pupils

#[test]
fn add_pupil_focuses_the_new_pupil_field() {
    let ui = seeded();
    click(&ui, "+ Add pupil");
    type_text(&ui, "Carmen");
    press_enter(&ui);
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_rows().row_count(), 3);
    assert!(gradebook(&ui).get_rows().iter().any(|row| row.full_name == "Carmen"));
}

#[test]
fn a_pupil_is_renamed_in_place() {
    let ui = seeded();
    click_element(&name_fields(&ui, "First name")[0]);
    replace_text(&ui, "Ana María");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils WHERE first_name = 'Ana María'"), 1);
}

/// Was: the row kept the old name, so the removal question asked about "Ana".
#[test]
fn regression_a_renamed_pupil_is_asked_about_by_the_new_name() {
    let ui = seeded();
    click_element(&name_fields(&ui, "First name")[0]);
    replace_text(&ui, "Ana María");
    assert_eq!(gradebook(&ui).get_rows().row_data(0).unwrap().full_name, "Ana María");
    ui.dialogs.answer_next(false);
    click_nth(&ui, "Remove from this class", 0);
    let question = english(&Message::Sheet(SheetMessage::ConfirmRemovePupil { pupil: "Ana María", class: "1A" }));
    assert_eq!(ui.dialogs.questions.borrow()[0], question);
}

/// Was: the delete question still named the class by its old name.
#[test]
fn regression_a_renamed_class_is_asked_about_by_the_new_name() {
    let ui = seeded();
    type_into(&ui, "Class name", "1A bis ");
    assert_eq!(gradebook(&ui).get_class_name(), "1A bis", "saved and shown trimmed");
    click(&ui, "More actions");
    ui.dialogs.answer_next(false);
    click(&ui, "Delete class");
    assert_eq!(
        ui.dialogs.questions.borrow()[0],
        english(&Message::Sheet(SheetMessage::ConfirmDeleteClass { class: "1A bis" }))
    );
}

#[test]
fn removing_a_pupil_asks_first() {
    let ui = seeded();
    ui.dialogs.answer_next(false);
    click_nth(&ui, "Remove from this class", 0);
    assert_eq!(gradebook(&ui).get_rows().row_count(), 2);
    let question = english(&Message::Sheet(SheetMessage::ConfirmRemovePupil { pupil: "Ana", class: "1A" }));
    assert_eq!(ui.dialogs.questions.borrow()[0], question);

    ui.dialogs.answer_next(true);
    click_nth(&ui, "Remove from this class", 0);
    assert_eq!(gradebook(&ui).get_rows().row_count(), 1);
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "the pupil itself stays");
}

#[test]
fn a_pupil_from_another_class_is_added_from_the_menu() {
    let ui = with_second_class();
    click(&ui, "More actions");
    click(&ui, "Add from another class…");
    click(&ui, "Carla");
    let names: Vec<String> = gradebook(&ui).get_rows().iter().map(|row| row.full_name.to_string()).collect();
    assert_eq!(names, ["Ana", "Carla"]);
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "the same pupil, not a copy");
}

// ---------------------------------------------------------------------------------------------
// Pupils in no class

/// Class "1A" with Ana, and Carla, who left her only class: she is in none now.
fn with_a_pupil_in_no_class() -> Ui {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let first = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, first, "Ana", "").unwrap();
        let second = classes::create_class(connection, "2B").unwrap();
        let carla = pupils::add_new_pupil(connection, second, "Carla", "").unwrap();
        pupils::remove_from_class(connection, second, carla).unwrap();
        classes::delete_class(connection, second).unwrap();
    }
    state.shutdown();
    Ui::open(fixture)
}

fn open_the_pupil_menu(ui: &Ui) {
    click(ui, "More actions");
    click(ui, "Add from another class…");
}

#[test]
fn a_pupil_in_no_class_is_marked_and_deleted_after_asking() {
    let ui = with_a_pupil_in_no_class();
    open_the_pupil_menu(&ui);
    assert!(shows_text(&ui, "no class"), "marked in the list");
    ui.dialogs.answer_next(false);
    click(&ui, "Delete Carla");
    assert_eq!(
        ui.dialogs.questions.borrow()[0],
        english(&Message::Sheet(SheetMessage::ConfirmDeletePupil { pupil: "Carla" }))
    );
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "a cancelled question deletes nobody");
    assert!(shows(&ui, "Delete Carla"));
    ui.dialogs.answer_next(true);
    click(&ui, "Delete Carla");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 1);
    assert!(gradebook(&ui).get_addable_pupils().row_count() == 0);
    assert!(!shows(&ui, "Delete Carla"), "and the emptied menu closes");
}

#[test]
fn a_pupil_in_another_class_cannot_be_deleted_from_the_menu() {
    let ui = with_second_class();
    open_the_pupil_menu(&ui);
    assert!(shows(&ui, "Carla"));
    assert!(!shows(&ui, "Delete Carla"));
    assert!(!shows_text(&ui, "no class"));
}

#[test]
fn deleting_a_pupil_who_has_joined_a_class_meanwhile_is_refused_and_shown() {
    let ui = with_a_pupil_in_no_class();
    open_the_pupil_menu(&ui);
    {
        let state = ui.session.state.borrow();
        let connection = state.connection().unwrap();
        let carla = pupils::pupils_in_no_class(connection).unwrap()[0].id;
        let class = classes::list_classes(connection).unwrap()[0].id;
        pupils::add_existing_pupil(connection, class, carla).unwrap();
    }
    ui.dialogs.answer_next(true);
    click(&ui, "Delete Carla");
    assert_eq!(error_on_screen(&ui), error_text(&AppError::NotEditable));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2, "nobody was deleted");
}

// ---------------------------------------------------------------------------------------------
// First name and surname

/// The pupil `pupil_id` as stored: first name, surname.
fn stored_names(ui: &Ui, pupil_id: i32) -> (String, String) {
    let state = ui.session.state.borrow();
    let pupil = pupils::pupil_name(state.connection().unwrap(), i64::from(pupil_id)).unwrap();
    (pupil.first_name, pupil.surname)
}

/// Which of the add row's fields has the keyboard focus. The testing backend can't tell, so a key
/// is typed and taken back again: the field it lands in is the focused one.
fn focused_label(ui: &Ui) -> Option<String> {
    let before = ["First name…", "Surname…"].map(|label| field_text(ui, label));
    type_text(ui, "§");
    let after = ["First name…", "Surname…"].map(|label| field_text(ui, label));
    let found = (0..2).find(|index| before[*index] != after[*index]);
    if found.is_some() {
        key(ui, Key::Backspace);
    }
    found.map(|index| ["First name…", "Surname…"][index].to_owned())
}

#[test]
fn enter_in_the_first_name_moves_to_the_surname() {
    let ui = seeded();
    type_into(&ui, "First name…", "Ana");
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_rows().row_count(), 2, "nothing added yet");
    assert_eq!(focused_label(&ui).as_deref(), Some("Surname…"));
    assert_eq!(field_text(&ui, "First name…"), "Ana", "the first name waits for the surname");
}

#[test]
fn enter_in_the_surname_adds_the_pupil_and_refocuses_the_first_name() {
    let ui = seeded();
    type_into(&ui, "First name…", "Ana");
    press_enter(&ui);
    type_text(&ui, "Pérez López");
    press_enter(&ui);
    let rows = gradebook(&ui).get_rows();
    assert_eq!(rows.row_count(), 3);
    let added = rows.row_data(2).unwrap();
    assert_eq!((added.first_name.as_str(), added.surname.as_str()), ("Ana", "Pérez López"));
    assert_eq!(added.full_name, "Ana Pérez López");
    assert_eq!(stored_names(&ui, added.pupil_id), ("Ana".to_owned(), "Pérez López".to_owned()));
    assert_eq!((field_text(&ui, "First name…"), field_text(&ui, "Surname…")), (String::new(), String::new()));
    assert_eq!(focused_label(&ui).as_deref(), Some("First name…"), "ready for the next pupil");
}

#[test]
fn a_new_pupil_without_a_first_name_is_refused_and_the_surname_stays() {
    let ui = seeded();
    type_into(&ui, "Surname…", "Otero");
    press_enter(&ui);
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyName));
    assert_eq!(gradebook(&ui).get_rows().row_count(), 2, "nobody added");
    assert_eq!(field_text(&ui, "Surname…"), "Otero", "the typed surname is kept");
    assert_eq!(focused_label(&ui).as_deref(), Some("First name…"), "where the missing name goes");
    type_text(&ui, "Brais");
    press_enter(&ui);
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_rows().row_data(2).unwrap().full_name, "Brais Otero");
}

#[test]
fn a_pupil_without_a_surname_can_be_added() {
    let ui = seeded();
    type_into(&ui, "First name…", "Carmen");
    press_enter(&ui);
    press_enter(&ui);
    let added = gradebook(&ui).get_rows().row_data(2).unwrap();
    assert_eq!((added.first_name.as_str(), added.surname.as_str(), added.full_name.as_str()), ("Carmen", "", "Carmen"));
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn first_name_and_surname_are_renamed_in_place() {
    let ui = seeded();
    let ana = gradebook(&ui).get_rows().row_data(0).unwrap().pupil_id;
    click_element(&name_fields(&ui, "First name")[0]);
    replace_text(&ui, "Ana María");
    click_element(&name_fields(&ui, "Surname")[0]);
    type_text(&ui, "Pérez");
    assert_eq!(stored_names(&ui, ana), ("Ana María".to_owned(), "Pérez".to_owned()));
    let row = gradebook(&ui).get_rows().row_data(0).unwrap();
    assert_eq!(row.full_name, "Ana María Pérez", "what confirmations and screen readers use");
    assert!(shows(&ui, "Ana María Pérez, Exam"), "the cells are spoken with the full name");
    replace_text(&ui, "");
    assert!(ui.session.window.get_error_message().is_empty(), "a surname may be emptied");
    assert_eq!(stored_names(&ui, ana), ("Ana María".to_owned(), String::new()));
    press_enter(&ui);
    type_text(&ui, "x");
    assert_eq!(stored_names(&ui, ana).1, "", "Enter ended the editing: nothing takes the typing");
}

#[test]
fn removing_a_pupil_quotes_the_full_name() {
    let ui = seeded();
    click_element(&name_fields(&ui, "Surname")[0]);
    type_text(&ui, "Pérez");
    ui.dialogs.answer_next(false);
    click_nth(&ui, "Remove from this class", 0);
    let question = english(&Message::Sheet(SheetMessage::ConfirmRemovePupil { pupil: "Ana Pérez", class: "1A" }));
    assert_eq!(ui.dialogs.questions.borrow()[0], question);
}

#[test]
fn pupils_from_another_class_are_listed_by_surname() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    {
        let connection = state.connection().unwrap();
        let first = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, first, "Ana", "").unwrap();
        let second = classes::create_class(connection, "2B").unwrap();
        pupils::add_new_pupil(connection, second, "Ana", "Pérez López").unwrap();
        pupils::add_new_pupil(connection, second, "Carla", "").unwrap();
    }
    state.shutdown();
    let ui = Ui::open(fixture);
    let listed: Vec<String> = gradebook(&ui).get_addable_pupils().iter().map(|pupil| pupil.name.to_string()).collect();
    assert_eq!(listed, ["Carla", "Pérez López, Ana"]);
    click(&ui, "More actions");
    click(&ui, "Add from another class…");
    click(&ui, "Pérez López, Ana");
    assert_eq!(gradebook(&ui).get_rows().iter().last().unwrap().full_name, "Ana Pérez López");
}
