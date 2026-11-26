// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:18.861325
//+ Revised: 	2026/11/25 11:43:16.994967
// -----------------------------------------------------------------------------

//! The pupil panel: opening it from a row, choosing, changing and removing the pupil's picture, and
//! keeping up with the sheet.

use crate::common::Ui;
use crate::driver::{click, count, elements, english, gradebook, key, shows, shows_text};
use crate::fixtures::{header, seeded, with_second_class};
use crate::pupil_screens::{sample, sample_photo};
use crate::screens::error_on_screen;
use avalio_lite::i18n::{Message, SheetMessage};
use avalio_lite::ui::{Gradebook, PupilPanelItem};
use avalio_lite::window::dialogs::FileKind;
use slint::Model;
use slint::platform::Key;

fn panel(ui: &Ui) -> Option<PupilPanelItem> {
    gradebook(ui).get_panel_pupil().row_data(0)
}

fn open_panel(ui: &Ui, pupil: &str) {
    click(ui, &format!("Picture of {pupil}"));
    assert_eq!(panel(ui).unwrap().full_name, pupil);
}

fn has_picture(ui: &Ui, row: usize) -> bool {
    gradebook(ui).get_rows().row_data(row).unwrap().avatar.size().width > 0
}

/// Chooses the sample photo for the pupil the panel is open on.
fn choose(ui: &Ui, button: &str) {
    let photo = sample_photo(&ui.fixture);
    ui.dialogs.pick_file_next(Some(&photo));
    click(ui, button);
}

#[test]
fn a_rows_avatar_opens_the_pupils_panel() {
    let ui = sample();
    open_panel(&ui, "Brais Otero");
    let shown = panel(&ui).unwrap();
    assert_eq!((shown.number, shown.initials.as_str()), (3, "BO"));
    assert!(shows_text(&ui, "Number 3 in this class, by surname"));
    assert!(shows(&ui, "Choose a picture…") && !shows(&ui, "Remove the picture"));
    click(&ui, "Close");
    assert!(panel(&ui).is_none());
}

#[test]
fn escape_closes_the_panel() {
    let ui = sample();
    open_panel(&ui, "Ana Pérez");
    key(&ui, Key::Escape);
    assert!(panel(&ui).is_none());
}

#[test]
fn a_picture_is_chosen_changed_and_removed() {
    let ui = seeded();
    open_panel(&ui, "Ana");
    assert!(!has_picture(&ui, 0));
    choose(&ui, "Choose a picture…");
    assert_eq!(*ui.dialogs.file_kinds.borrow(), [FileKind::Picture]);
    assert!(has_picture(&ui, 0) && !has_picture(&ui, 1), "on Ana's row, at once");
    assert!(panel(&ui).unwrap().avatar.size().width > 0, "and in the panel, still open");
    assert!(gradebook(&ui).get_saved(), "the summary line says it was saved");
    choose(&ui, "Change the picture…");
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupil_pictures"), 1);
    ui.dialogs.answer_next(false);
    click(&ui, "Remove the picture");
    assert!(has_picture(&ui, 0), "not without a yes");
    ui.dialogs.answer_next(true);
    click(&ui, "Remove the picture");
    let question = english(&Message::Sheet(SheetMessage::ConfirmRemovePicture { pupil: "Ana" }));
    assert_eq!(*ui.dialogs.questions.borrow(), [question.clone(), question]);
    assert!(!has_picture(&ui, 0));
    assert!(shows(&ui, "Choose a picture…"));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupil_pictures"), 0);
}

#[test]
fn a_cancelled_file_picker_changes_nothing() {
    let ui = seeded();
    open_panel(&ui, "Ana");
    let changes = |ui: &Ui| ui.session.state.borrow().connection().unwrap().total_changes();
    let before = changes(&ui);
    ui.dialogs.pick_file_next(None);
    click(&ui, "Choose a picture…");
    assert_eq!(changes(&ui), before);
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn a_file_that_is_not_a_picture_is_said_and_keeps_the_one_there() {
    let ui = seeded();
    open_panel(&ui, "Ana");
    choose(&ui, "Choose a picture…");
    let text = ui.fixture.path("notes.jpg");
    std::fs::write(&text, "not a picture").unwrap();
    ui.dialogs.pick_file_next(Some(&text));
    click(&ui, "Change the picture…");
    assert!(error_on_screen(&ui).contains("isn't a picture that can be read"), "{}", error_on_screen(&ui));
    assert!(has_picture(&ui, 0));
    assert!(!gradebook(&ui).get_save_failed(), "nothing typed is at stake");
}

/// A pupil in two classes has one picture.
#[test]
fn a_picture_shows_in_every_class_of_the_pupil() {
    let ui = with_second_class();
    open_panel(&ui, "Ana");
    choose(&ui, "Choose a picture…");
    click(&ui, "2B");
    click(&ui, "More actions");
    click(&ui, "Add from another class…");
    click(&ui, "Ana");
    let rows = gradebook(&ui).get_rows();
    let row = rows.iter().find(|row| row.first_name == "Ana").unwrap();
    assert!(row.avatar.size().width > 0);
}

#[test]
fn a_picture_survives_a_restart() {
    let ui = seeded();
    open_panel(&ui, "Ana");
    choose(&ui, "Choose a picture…");
    let ui = ui.restart();
    assert!(has_picture(&ui, 0));
}

#[test]
fn a_damaged_stored_picture_leaves_the_initials() {
    let ui = seeded();
    ui.session
        .state
        .borrow()
        .connection()
        .unwrap()
        .execute_batch("INSERT INTO pupil_pictures VALUES (1, x'00')")
        .unwrap();
    let ui = ui.restart();
    assert!(!has_picture(&ui, 0));
    assert_eq!(gradebook(&ui).get_rows().row_data(0).unwrap().initials, "A");
    assert!(ui.session.window.get_error_message().is_empty(), "the sheet still opens");
}

#[test]
fn the_pupil_panel_and_the_column_panel_take_each_others_place() {
    let ui = seeded();
    click(&ui, "Exam");
    assert_eq!(gradebook(&ui).get_panel_index(), 2);
    open_panel(&ui, "Ana");
    assert_eq!(gradebook(&ui).get_panel_index(), -1, "one panel at a time");
    Gradebook::invoke_open_column_panel(&gradebook(&ui), 2);
    assert!(panel(&ui).is_none());
    assert!(elements(&ui, "Choose a picture…").is_empty());
    assert!(header(&ui, "Exam").size().width > 0.0);
}

#[test]
fn the_panel_follows_a_rename_and_closes_when_its_pupil_leaves() {
    let ui = seeded();
    open_panel(&ui, "Bea");
    assert_eq!(panel(&ui).unwrap().number, 2);
    Gradebook::invoke_set_surname(&gradebook(&ui), panel(&ui).unwrap().id, "Alba".into());
    let shown = panel(&ui).unwrap();
    assert_eq!((shown.full_name.as_str(), shown.initials.as_str(), shown.number), ("Bea Alba", "BA", 1));
    assert_eq!(gradebook(&ui).get_rows().row_data(1).unwrap().initials, "BA", "and the row's avatar");
    ui.dialogs.answer_next(true);
    Gradebook::invoke_remove_pupil(&gradebook(&ui), shown.id);
    assert!(panel(&ui).is_none());
}

#[test]
fn another_class_closes_the_panel() {
    let ui = with_second_class();
    open_panel(&ui, "Ana");
    click(&ui, "2B");
    assert!(panel(&ui).is_none());
}

#[test]
fn a_pupil_of_another_class_opens_no_panel() {
    let ui = with_second_class();
    Gradebook::invoke_open_pupil_panel(&gradebook(&ui), 99);
    assert!(panel(&ui).is_none());
}
