// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:32.195618
//+ Revised: 	2026/11/23 13:00:32.195618
// -----------------------------------------------------------------------------

//! Settings › data locations: moving the database and the logs, and opening another database.

use crate::common::Fixture;
use crate::driver::{click, click_nth, english, error_text, gradebook};
use crate::fixtures::{encrypted_fixture, open_folder_settings, seeded};
use avalio_lite::error::AppError;
use avalio_lite::gradebook::classes;
use avalio_lite::i18n::{DataMessage, Message};
use avalio_lite::platform::config::{DATABASE_FILE_NAME, LOG_FILE_NAME};
use std::fs;

// ---------------------------------------------------------------------------------------------
// Settings: data locations

#[test]
fn moving_the_database_shows_where_it_went() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("moved");
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 0);
    let moved = destination.join(DATABASE_FILE_NAME);
    assert!(moved.is_file());
    assert!(!ui.fixture.database().exists());
    assert_eq!(ui.session.window.get_database_path(), moved.display().to_string());
    let path = moved.display().to_string();
    assert_eq!(
        ui.session.window.get_status_message(),
        english(&Message::Data(DataMessage::DatabaseMoved { path: &path }))
    );
    assert_eq!(ui.dialogs.folder_starts.borrow()[0], ui.fixture.path("data"), "opens where the database is");
}

#[test]
fn regression_a_move_that_cannot_delete_the_original_still_shows_the_new_location() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("moved");
    let _held = crate::common::hold_against_removal(&ui.fixture.database());
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 0);
    let moved = destination.join(DATABASE_FILE_NAME);
    assert!(moved.is_file(), "the app switched to the copy");
    assert_eq!(ui.session.window.get_database_path(), moved.display().to_string());
    assert!(!ui.session.window.get_error_message().is_empty(), "the leftover original is reported");
}

#[test]
fn moving_the_database_onto_another_file_is_refused() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("taken");
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join(DATABASE_FILE_NAME), b"someone else's").unwrap();
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 0);
    let error = AppError::AlreadyExists(destination.join(DATABASE_FILE_NAME));
    assert_eq!(ui.session.window.get_error_message(), error_text(&error));
    assert!(ui.fixture.database().is_file(), "the database stays where it was");
}

#[test]
fn cancelled_moves_change_nothing() {
    let ui = seeded();
    open_folder_settings(&ui);
    ui.dialogs.pick_folder_next(None);
    ui.dialogs.pick_folder_next(None);
    click_nth(&ui, "Move…", 0);
    click_nth(&ui, "Move…", 1);
    assert!(ui.fixture.database().is_file());
    assert!(ui.session.window.get_status_message().is_empty());
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn moving_the_logs_shows_where_they_went() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("new-logs");
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 1);
    assert!(destination.join(LOG_FILE_NAME).is_file());
    assert_eq!(ui.session.window.get_log_folder(), destination.display().to_string());
    let path = destination.display().to_string();
    assert_eq!(ui.session.window.get_status_message(), english(&Message::Data(DataMessage::LogsMoved { path: &path })));
}

#[test]
fn moving_the_logs_onto_another_log_file_is_refused() {
    let ui = seeded();
    open_folder_settings(&ui);
    let destination = ui.fixture.path("taken-logs");
    fs::create_dir_all(&destination).unwrap();
    fs::write(destination.join(LOG_FILE_NAME), b"other log").unwrap();
    ui.dialogs.pick_folder_next(Some(&destination));
    click_nth(&ui, "Move…", 1);
    let error = AppError::AlreadyExists(destination.join(LOG_FILE_NAME));
    assert_eq!(ui.session.window.get_error_message(), error_text(&error));
    assert_eq!(ui.session.window.get_log_folder(), ui.fixture.path("logs").display().to_string());
}

#[test]
fn another_database_can_be_opened_from_settings() {
    let other = Fixture::new();
    let mut state = other.set_up();
    classes::create_class(state.connection().unwrap(), "Other class").unwrap();
    state.shutdown();
    let ui = seeded();
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open another database…");
    assert_eq!(ui.session.window.get_database_path(), other.database().display().to_string());
    assert_eq!(gradebook(&ui).get_class_name(), "Other class");
}

#[test]
fn opening_a_missing_file_is_reported() {
    let ui = seeded();
    open_folder_settings(&ui);
    let missing = ui.fixture.path("gone.db");
    ui.dialogs.pick_file_next(Some(&missing));
    click(&ui, "Open another database…");
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NotFound(missing)));
    assert_eq!(gradebook(&ui).get_class_name(), "1A", "the open database stays");
}

#[test]
fn a_cancelled_switch_keeps_the_database() {
    let ui = seeded();
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(None);
    click(&ui, "Open another database…");
    assert_eq!(ui.session.window.get_database_path(), ui.fixture.database().display().to_string());
}

#[test]
fn switching_to_an_encrypted_database_locks() {
    let other = encrypted_fixture();
    let ui = seeded();
    open_folder_settings(&ui);
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open another database…");
    assert!(ui.session.window.get_locked());
}
