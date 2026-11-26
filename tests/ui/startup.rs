// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.175774
//+ Revised: 	2026/11/23 13:00:33.175774
// -----------------------------------------------------------------------------

//! Startup and first-run setup: which screen opens, and what the setup does with folders
//! and existing databases.

use crate::common::{Fixture, Ui};
use crate::driver::{click, error_text, gradebook, shows, type_into};
use crate::fixtures::{encrypted, encrypted_fixture, seeded, unlock};
use avalio_lite::error::{AppError, Folder};
use avalio_lite::gradebook::classes;
use avalio_lite::i18n::{self, Language};
use avalio_lite::platform::config::{LOG_FILE_NAME, ThemeChoice};
use avalio_lite::ui::{SetupNotice, Theme};
use slint::{ComponentHandle, Model};
use std::fs;

/// Starts like an SQLite file, so it isn't taken for an encrypted one, but the rest is garbage.
fn corrupt_database() -> Vec<u8> {
    let mut bytes = b"SQLite format 3\0".to_vec();
    bytes.extend(std::iter::repeat_n(0xA5, 4096));
    bytes
}

// ---------------------------------------------------------------------------------------------
// Startup

#[test]
fn first_run_shows_setup_with_the_suggested_folder() {
    let ui = Ui::open(Fixture::new());
    let window = &ui.session.window;
    assert!(window.get_setup_required());
    assert_eq!(window.get_setup_notice(), SetupNotice::FirstRun);
    assert!(!window.get_setup_database_folder().is_empty());
    assert_eq!(window.get_setup_database_folder(), window.get_setup_log_folder());
    assert!(shows(&ui, "Create database"));
}

#[test]
fn an_existing_database_opens_on_its_first_class() {
    let ui = seeded();
    assert!(!ui.session.window.get_setup_required());
    assert!(!ui.session.window.get_locked());
    assert_eq!(gradebook(&ui).get_class_name(), "1A");
    assert!(shows(&ui, "Exam"));
    assert_eq!(gradebook(&ui).get_rows().row_count(), 2);
}

/// The class that was open is open again at the next start; one deleted meanwhile falls back to
/// the first class, and another database starts on its own first class.
#[test]
fn the_open_class_is_open_again_at_the_next_start() {
    let ui = seeded();
    let second = {
        let state = ui.session.state.borrow();
        classes::create_class(state.connection().unwrap(), "2B").unwrap()
    };
    gradebook(&ui).invoke_select_class(i32::try_from(second).unwrap());
    assert_eq!(gradebook(&ui).get_class_name(), "2B");
    assert_eq!(ui.fixture.store().load().unwrap().unwrap().last_class, Some(second), "saved at once");

    let ui = ui.restart();
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "opened again");

    classes::delete_class(ui.session.state.borrow().connection().unwrap(), second).unwrap();
    let ui = ui.restart();
    assert_eq!(gradebook(&ui).get_class_name(), "1A", "the remembered class is gone: the first one");
}

#[test]
fn a_missing_database_asks_where_it_went() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    fs::remove_file(fixture.database()).unwrap();
    let ui = Ui::open(fixture);
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_setup_notice(), SetupNotice::DatabaseMissing);
    assert_eq!(ui.session.window.get_setup_notice_detail(), ui.fixture.database().display().to_string());
}

#[test]
fn unreadable_settings_ask_for_setup_again() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.path("config")).unwrap();
    fs::write(fixture.store().path(), "not = [valid").unwrap();
    let ui = Ui::open(fixture);
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_setup_notice(), SetupNotice::ConfigUnreadable);
}

#[test]
fn an_unusable_database_asks_for_another() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    fs::write(fixture.database(), corrupt_database()).unwrap();
    let ui = Ui::open(fixture);
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_setup_notice(), SetupNotice::DatabaseUnusable);
}

#[test]
fn an_encrypted_database_starts_on_the_lock_screen() {
    let ui = encrypted();
    assert!(ui.session.window.get_locked());
    assert!(ui.session.window.get_pin_set());
    assert!(shows(&ui, "Unlock"));
    assert!(ui.session.state.borrow().connection().is_none(), "nothing is readable before the PIN");
}

#[test]
fn the_saved_language_and_theme_are_applied_at_startup() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_language(Language::Galician).unwrap();
    state.set_theme(ThemeChoice::Dark).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    assert_eq!(ui.session.window.get_language_code(), "gl");
    assert_eq!(i18n::current(), Language::Galician);
    assert!(ui.session.window.global::<Theme>().get_dark());
    assert!(shows(&ui, "Crear clase"), "the interface is in Galician");
}

#[test]
fn a_saved_light_theme_wins_over_the_windows_setting() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_theme(ThemeChoice::Light).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    assert!(!ui.session.window.global::<Theme>().get_dark());
}

#[test]
fn the_windows_language_is_used_until_one_is_chosen() {
    let ui = Ui::open_in(Fixture::new(), Language::Galician);
    assert_eq!(ui.session.window.get_language_code(), "gl");
    assert!(shows(&ui, "Crear base de datos"));
}

// ---------------------------------------------------------------------------------------------
// First-run setup

#[test]
fn first_run_creates_the_database_in_the_chosen_folders() {
    let ui = Ui::open(Fixture::new());
    type_into(&ui, "Database folder", &ui.fixture.path("data").display().to_string());
    type_into(&ui, "Log folder", &ui.fixture.path("logs").display().to_string());
    click(&ui, "Create database");
    assert!(!ui.session.window.get_setup_required());
    assert!(ui.fixture.database().is_file());
    assert!(ui.fixture.path("logs").join(LOG_FILE_NAME).is_file());
    assert_eq!(ui.session.window.get_database_path(), ui.fixture.database().display().to_string());
    assert!(shows(&ui, "Create class"));
}

#[test]
fn browse_fills_each_folder_field_from_the_picker() {
    let ui = Ui::open(Fixture::new());
    let data = ui.fixture.path("data");
    let logs = ui.fixture.path("logs");
    let suggested = ui.session.window.get_setup_database_folder().to_string();
    ui.dialogs.pick_folder_next(Some(&data));
    ui.dialogs.pick_folder_next(Some(&logs));
    // each says which folder it is for, so a screen reader tells the two apart
    click(&ui, "Browse… (Database folder)");
    click(&ui, "Browse… (Log folder)");
    assert_eq!(ui.session.window.get_setup_database_folder(), data.display().to_string());
    assert_eq!(ui.session.window.get_setup_log_folder(), logs.display().to_string());
    assert_eq!(ui.dialogs.folder_starts.borrow()[0].display().to_string(), suggested, "opens at the current folder");
}

#[test]
fn a_cancelled_browse_keeps_the_folder() {
    let ui = Ui::open(Fixture::new());
    let before = ui.session.window.get_setup_database_folder();
    ui.dialogs.pick_folder_next(None);
    ui.dialogs.pick_folder_next(None);
    click(&ui, "Browse… (Database folder)");
    click(&ui, "Browse… (Log folder)");
    assert_eq!(ui.session.window.get_setup_database_folder(), before);
    assert_eq!(ui.session.window.get_setup_log_folder(), before);
}

#[test]
fn an_empty_folder_is_refused_on_the_setup_screen() {
    let ui = Ui::open(Fixture::new());
    type_into(&ui, "Database folder", "");
    click(&ui, "Create database");
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::EmptyPath(Folder::Database)));
    assert!(ui.fixture.store().load().unwrap().is_none(), "nothing is saved");
}

#[test]
fn setup_can_open_an_existing_database() {
    let other = Fixture::new();
    other.set_up().shutdown();
    let ui = Ui::open(Fixture::new());
    type_into(&ui, "Log folder", &ui.fixture.path("logs").display().to_string());
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open an existing database…");
    assert!(!ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_database_path(), other.database().display().to_string());
}

#[test]
fn a_cancelled_open_stays_on_setup() {
    let ui = Ui::open(Fixture::new());
    ui.dialogs.pick_file_next(None);
    click(&ui, "Open an existing database…");
    assert!(ui.session.window.get_setup_required());
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn a_file_that_isnt_a_database_is_reported_on_setup() {
    let ui = Ui::open(Fixture::new());
    let not_a_database = ui.fixture.path("broken.db");
    fs::write(&not_a_database, corrupt_database()).unwrap();
    type_into(&ui, "Log folder", &ui.fixture.path("logs").display().to_string());
    ui.dialogs.pick_file_next(Some(&not_a_database));
    click(&ui, "Open an existing database…");
    assert!(ui.session.window.get_setup_required());
    assert!(!ui.session.window.get_error_message().is_empty());
    assert!(ui.fixture.store().load().unwrap().is_none(), "nothing is saved");
}

#[test]
fn opening_an_encrypted_database_from_setup_asks_for_its_pin() {
    let other = encrypted_fixture();
    let ui = Ui::open(Fixture::new());
    ui.dialogs.pick_file_next(Some(&other.database()));
    click(&ui, "Open an existing database…");
    assert!(ui.session.window.get_locked());
    unlock(&ui);
}

// ---------------------------------------------------------------------------------------------
// Actions without an open database

#[test]
fn sheet_actions_while_locked_report_no_database() {
    let ui = encrypted();
    gradebook(&ui).invoke_create_class("1A".into());
    assert_eq!(ui.session.window.get_error_message(), error_text(&AppError::NoDatabaseOpen));
}
