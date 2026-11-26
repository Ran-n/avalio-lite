// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.660352
//+ Revised: 	2026/11/23 13:00:32.660352
// -----------------------------------------------------------------------------

//! Settings › backup: restoring a backup, and adding the classes of a backup.

use crate::common::{Fixture, Ui};
use crate::driver::{click, count, english, error_text, gradebook, press_enter, shows_text, type_into, with_control};
use crate::fixtures::{encrypted, seeded, unlock};
use crate::screens::{backup_file, open_backup_settings};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::{classes, pupils};
use avalio_lite::i18n::{DataMessage, Message};
use avalio_lite::window::dialogs::FileKind;
use slint::{ComponentHandle, Model};
use std::fs;

fn backup_global(ui: &Ui) -> avalio_lite::ui::Backup<'_> {
    ui.session.window.global::<avalio_lite::ui::Backup>()
}

fn restore_copy(ui: &Ui) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}.restore", ui.fixture.database().display()))
}

fn sidebar_classes(ui: &Ui) -> Vec<String> {
    gradebook(ui).get_classes().iter().map(|class| class.name.to_string()).collect()
}

#[test]
fn restore_asks_once_and_replaces_the_sheet() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(true);
    click(&ui, "Restore from a backup…");

    let question = english(&Message::Data(DataMessage::ConfirmRestore {
        file: "avalio-lite-backup-2026-10-28.avalio",
        classes: 2,
        pupils: 2,
    }));
    assert_eq!(*ui.dialogs.questions.borrow(), [question]);
    assert_eq!(*ui.dialogs.file_kinds.borrow(), [FileKind::Backup]);
    assert_eq!(sidebar_classes(&ui), ["7A", "7B"]);
    assert_eq!(gradebook(&ui).get_class_name(), "7A", "the first class is selected");
    let status = english(&Message::Data(DataMessage::BackupRestored { file: "avalio-lite-backup-2026-10-28.avalio" }));
    assert_eq!(ui.session.window.get_status_message(), status);
    assert!(!restore_copy(&ui).exists());
}

#[test]
fn no_to_the_restore_question_changes_nothing() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(false);
    click(&ui, "Restore from a backup…");
    assert_eq!(sidebar_classes(&ui), ["1A"]);
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 2);
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(!restore_copy(&ui).exists());
}

#[test]
fn a_cancelled_restore_picker_does_nothing() {
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(None);
    click(&ui, "Restore from a backup…");
    assert!(ui.dialogs.questions.borrow().is_empty());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
}

#[test]
fn the_open_database_chosen_as_a_backup_shows_the_error() {
    for button in ["Restore from a backup…", "Add classes from a backup…"] {
        let ui = seeded();
        open_backup_settings(&ui);
        let own = ui.fixture.database();
        ui.dialogs.pick_file_next(Some(&own));
        click(&ui, button);
        assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::OpenDatabaseAsBackup(own)), "{button}");
        assert!(ui.dialogs.questions.borrow().is_empty(), "nothing is asked: {button}");
        assert!(!backup_global(&ui).get_pin_needed());
        assert_eq!(sidebar_classes(&ui), ["1A"], "{button}");
    }
}

#[test]
fn a_file_that_is_not_a_backup_shows_the_error() {
    let ui = seeded();
    open_backup_settings(&ui);
    let file = ui.fixture.path("notes.avalio");
    fs::write(&file, b"hello").unwrap();
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NotADatabase(file)));
    assert!(!backup_global(&ui).get_pin_needed());
    assert!(ui.dialogs.questions.borrow().is_empty());
}

#[test]
fn an_encrypted_backup_asks_for_its_pin_inline() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    assert!(backup_global(&ui).get_pin_needed());
    assert!(shows_text(&ui, "This backup is encrypted. Type the PIN it was made with."));
    assert!(backup_global(&ui).get_pin_error().is_empty(), "no error before a PIN is typed");
    assert!(ui.dialogs.questions.borrow().is_empty(), "no question before the backup opens");

    ui.dialogs.answer_next(true);
    type_into(&ui, "Backup PIN", "9753");
    press_enter(&ui);
    assert!(!backup_global(&ui).get_pin_needed(), "the prompt closes");
    assert_eq!(ui.dialogs.questions.borrow().len(), 1);
    assert_eq!(sidebar_classes(&ui), ["7A", "7B"]);
    assert!(!ui.session.state.borrow().is_protected(), "the app keeps having no PIN");
}

#[test]
fn a_wrong_backup_pin_keeps_the_prompt() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    type_into(&ui, "Backup PIN", "1111");
    click(&ui, "Continue");
    assert!(backup_global(&ui).get_pin_needed());
    assert_eq!(backup_global(&ui).get_pin_error(), error_text(&AppError::WrongPin));
    assert!(backup_global(&ui).get_pin().is_empty(), "the wrong PIN is cleared");
    assert!(!restore_copy(&ui).exists());

    click(&ui, "Cancel");
    assert!(!backup_global(&ui).get_pin_needed());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
}

#[test]
fn leaving_settings_closes_the_backup_prompt() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    assert!(backup_global(&ui).get_pin_needed());
    click(&ui, "1A");
    assert!(!backup_global(&ui).get_pin_needed());
    // nothing is left waiting: a stray Continue does nothing
    backup_global(&ui).invoke_pin_entered();
    assert_eq!(sidebar_classes(&ui), ["1A"]);
}

#[test]
fn locking_closes_the_backup_prompt() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = encrypted();
    unlock(&ui);
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Restore from a backup…");
    assert!(backup_global(&ui).get_pin_needed());
    with_control(&ui, "l");
    assert!(ui.session.window.get_locked());
    assert!(!backup_global(&ui).get_pin_needed());
    assert!(!restore_copy(&ui).exists());
}

#[test]
fn a_backup_of_this_database_restores_without_asking_for_its_pin() {
    let ui = encrypted();
    unlock(&ui);
    open_backup_settings(&ui);
    let file = ui.fixture.path("mine.avalio");
    ui.dialogs.save_file_next(Some(&file));
    click(&ui, "Export everything…");
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(true);
    click(&ui, "Restore from a backup…");
    assert!(!backup_global(&ui).get_pin_needed());
    assert_eq!(ui.dialogs.questions.borrow().len(), 1);
    assert!(ui.session.state.borrow().is_protected(), "still encrypted with the app's PIN");
}

#[test]
fn a_failed_restore_shows_the_error_and_keeps_the_sheet() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    ui.dialogs.answer_next(true);
    let held = crate::common::hold_against_removal(&ui.fixture.database());
    click(&ui, "Restore from a backup…");
    drop(held);
    assert!(!ui.session.window.get_error_message().is_empty());
    assert!(ui.session.window.get_status_message().is_empty());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
    assert!(!ui.session.window.get_locked());
}

#[test]
fn add_classes_from_a_backup_lists_them_and_reports() {
    let (other, file) = backup_file(None);
    // the backup also has a 1A, so the one added is renamed; and Ana, who is already here
    {
        let mut state = other.state();
        state.start();
        let connection = state.connection().unwrap();
        let class = classes::create_class(connection, "1A").unwrap();
        pupils::add_new_pupil(connection, class, "Ana", "").unwrap();
        fs::remove_file(&file).unwrap();
        state.export_backup(&file).unwrap();
        state.shutdown();
    }
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");

    assert!(ui.dialogs.questions.borrow().is_empty(), "nothing is replaced, so nothing is asked");
    assert_eq!(sidebar_classes(&ui), ["1A", "1A (2)", "7A", "7B"]);
    assert_eq!(gradebook(&ui).get_class_name(), "1A (2)", "the first class added is open");
    let added = english(&Message::Data(DataMessage::BackupClassesAdded {
        classes: 3,
        pupils: 3,
        existing: 1,
        file: "avalio-lite-backup-2026-10-28.avalio",
    }));
    let renamed =
        english(&Message::Data(DataMessage::BackupClassesRenamed { pairs: &[("1A".to_owned(), "1A (2)".to_owned())] }));
    assert_eq!(ui.session.window.get_status_message(), format!("{added} {renamed}"));
    assert_eq!(count(&ui, "SELECT COUNT(*) FROM pupils"), 4, "Ana joins; Xoán and Iria are new");
    assert!(!restore_copy(&ui).exists());
}

#[test]
fn adding_from_an_encrypted_backup_asks_for_its_pin() {
    let (_other, file) = backup_file(Some("9753"));
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");
    assert!(backup_global(&ui).get_pin_needed());
    type_into(&ui, "Backup PIN", "9753");
    click(&ui, "Continue");
    assert!(!backup_global(&ui).get_pin_needed());
    assert_eq!(sidebar_classes(&ui), ["1A", "7A", "7B"]);
    assert!(!ui.session.state.borrow().is_protected(), "the app's own encryption doesn't change");
}

#[test]
fn adding_from_an_empty_backup_says_so() {
    let other = Fixture::new();
    let file = other.path("empty.avalio");
    let mut state = other.set_up();
    state.export_backup(&file).unwrap();
    state.shutdown();
    let ui = seeded();
    open_backup_settings(&ui);
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");
    assert_eq!(ui.session.window.get_status_message(), english(&Message::Data(DataMessage::BackupHasNoClasses)));
    assert!(ui.session.window.get_error_message().is_empty());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
}

#[test]
fn a_failed_add_shows_the_error_and_adds_nothing() {
    let (_other, file) = backup_file(None);
    let ui = seeded();
    open_backup_settings(&ui);
    // the open database refuses new classes from here on
    ui.session
        .state
        .borrow()
        .connection()
        .unwrap()
        .execute_batch("CREATE TEMP TRIGGER refuse BEFORE INSERT ON classes BEGIN SELECT RAISE(ABORT, 'test'); END")
        .unwrap();
    ui.dialogs.pick_file_next(Some(&file));
    click(&ui, "Add classes from a backup…");
    assert!(!ui.session.window.get_error_message().is_empty());
    assert_eq!(sidebar_classes(&ui), ["1A"]);
    assert!(!restore_copy(&ui).exists());
}
