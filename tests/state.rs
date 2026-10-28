// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:32.383625
//+ Revised: 	2026/10/28 15:53:09.060231
// -----------------------------------------------------------------------------

use std::fs;
use std::path::Path;

mod common;
use common::Fixture;

use avalio_lite::app::state::{AppState, SetupReason, Startup};
use avalio_lite::config::{AppConfig, DATABASE_FILE_NAME, LOG_FILE_NAME, Preferences, ThemeChoice};
use avalio_lite::error::{AppError, Folder};
use avalio_lite::i18n::Language;

fn pupil_count(state: &AppState) -> i64 {
    state.connection().unwrap().query_row("SELECT COUNT(*) FROM pupils", [], |row| row.get(0)).unwrap()
}

fn add_pupil(state: &AppState) {
    state.connection().unwrap().execute("INSERT INTO pupils (name) VALUES ('Ana Pérez')", []).unwrap();
}

#[test]
fn first_run_asks_for_setup() {
    let fixture = Fixture::new();
    assert_eq!(fixture.state().start(), Startup::NeedsSetup(SetupReason::FirstRun));
}

#[test]
fn create_database_saves_choice_and_next_start_is_ready() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let expected = AppConfig {
        database_path: fixture.path("data").join(DATABASE_FILE_NAME),
        log_dir: fixture.path("logs"),
        preferences: Preferences::default(),
    };
    assert_eq!(state.config(), Some(&expected));
    assert_eq!(fixture.store().load().unwrap(), Some(expected));
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Ready);
    assert!(fixture.path("logs").join(LOG_FILE_NAME).is_file());
    restarted.shutdown();
}

#[test]
fn hiding_column_settings_is_remembered_across_restarts_and_database_switches() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    assert!(state.preferences().show_column_settings);
    state.set_show_column_settings(false).unwrap();
    assert!(!fixture.store().load().unwrap().unwrap().preferences.show_column_settings);

    let other = fixture.path("other.db");
    avalio_lite::db::close(avalio_lite::db::open(&other, None).unwrap()).unwrap();
    state.open_existing(&other, &fixture.path("logs")).unwrap();
    assert!(!state.preferences().show_column_settings, "switching database keeps the preference");
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Ready);
    assert!(!restarted.preferences().show_column_settings);
    restarted.shutdown();
}

#[test]
fn chosen_language_is_remembered_across_restarts() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    assert_eq!(state.preferences().language, None, "follows Windows until the user picks one");
    state.set_language(Language::Galician).unwrap();
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Ready);
    assert_eq!(restarted.preferences().language, Some(Language::Galician));
    restarted.shutdown();
}

#[test]
fn preferences_cannot_be_saved_before_setup() {
    let fixture = Fixture::new();
    let mut state = fixture.state();
    assert!(matches!(state.set_language(Language::Galician), Err(AppError::NoDatabaseOpen)));
    assert!(matches!(state.set_show_column_settings(false), Err(AppError::NoDatabaseOpen)));
    assert_eq!(fixture.store().load().unwrap(), None);
}

#[test]
fn empty_folder_is_rejected_and_nothing_is_saved() {
    let fixture = Fixture::new();
    let mut state = fixture.state();
    let result = state.create_database(Path::new(""), &fixture.path("logs"));
    assert!(matches!(result, Err(AppError::EmptyPath(Folder::Database))));
    assert_eq!(fixture.store().load().unwrap(), None);
    assert!(state.config().is_none());
}

#[test]
fn missing_database_is_reported_and_not_recreated() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let database_path = state.config().unwrap().database_path.clone();
    state.shutdown();
    avalio_lite::db::remove_files(&database_path).unwrap();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::NeedsSetup(SetupReason::DatabaseMissing(database_path.clone())));
    assert!(!database_path.exists());
}

#[test]
fn unreadable_settings_ask_for_setup() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.path("config")).unwrap();
    fs::write(fixture.store().path(), "not = = toml").unwrap();
    assert!(matches!(fixture.state().start(), Startup::NeedsSetup(SetupReason::ConfigUnreadable(_))));
}

#[test]
fn corrupt_database_asks_for_setup() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let database_path = state.config().unwrap().database_path.clone();
    state.shutdown();
    avalio_lite::db::remove_files(&database_path).unwrap();
    fs::write(&database_path, b"not a database").unwrap();
    assert!(matches!(fixture.state().start(), Startup::NeedsSetup(SetupReason::DatabaseUnusable(_))));
}

#[test]
fn open_existing_switches_database_and_refuses_missing_file() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let missing = fixture.path("nowhere.db");
    assert!(matches!(state.open_existing(&missing, &fixture.path("logs")), Err(AppError::NotFound(_))));

    let other = fixture.path("other.db");
    avalio_lite::db::close(avalio_lite::db::open(&other, None).unwrap()).unwrap();
    state.open_existing(&other, &fixture.path("logs")).unwrap();
    assert_eq!(fixture.store().load().unwrap().unwrap().database_path, other);
}

#[test]
fn move_database_keeps_data_updates_settings_and_removes_original() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    add_pupil(&state);
    let original = state.config().unwrap().database_path.clone();

    let moved = state.move_database(&fixture.path("moved")).unwrap();
    assert_eq!(moved, fixture.path("moved").join(DATABASE_FILE_NAME));
    assert!(!original.exists());
    assert_eq!(pupil_count(&state), 1);
    assert_eq!(fixture.store().load().unwrap().unwrap().database_path, moved);
}

#[test]
fn move_database_to_same_folder_does_nothing() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let original = state.config().unwrap().database_path.clone();
    assert_eq!(state.move_database(&fixture.path("data")).unwrap(), original);
    assert!(original.is_file());
}

#[test]
fn move_database_refuses_existing_file_and_leaves_both_untouched() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    add_pupil(&state);
    let blocker = fixture.path("taken").join(DATABASE_FILE_NAME);
    fs::create_dir_all(blocker.parent().unwrap()).unwrap();
    fs::write(&blocker, b"someone else's file").unwrap();

    assert!(matches!(state.move_database(&fixture.path("taken")), Err(AppError::AlreadyExists(_))));
    assert_eq!(fs::read(&blocker).unwrap(), b"someone else's file");
    assert_eq!(pupil_count(&state), 1);
}

#[test]
fn move_logs_moves_file_and_updates_settings() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.move_logs(&fixture.path("new-logs")).unwrap();
    assert!(!fixture.path("logs").join(LOG_FILE_NAME).exists());
    assert!(fixture.path("new-logs").join(LOG_FILE_NAME).is_file());
    assert_eq!(fixture.store().load().unwrap().unwrap().log_dir, fixture.path("new-logs"));
}

#[test]
fn move_logs_refuses_existing_file_and_keeps_old_folder() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    fs::create_dir_all(fixture.path("busy")).unwrap();
    fs::write(fixture.path("busy").join(LOG_FILE_NAME), b"other").unwrap();
    assert!(matches!(state.move_logs(&fixture.path("busy")), Err(AppError::AlreadyExists(_))));
    assert_eq!(state.config().unwrap().log_dir, fixture.path("logs"));
}

#[test]
fn moves_before_setup_are_refused() {
    let fixture = Fixture::new();
    let mut state = fixture.state();
    assert!(matches!(state.move_database(&fixture.path("x")), Err(AppError::NoDatabaseOpen)));
    assert!(matches!(state.move_logs(&fixture.path("x")), Err(AppError::NoDatabaseOpen)));
}

#[test]
fn shutdown_releases_database_and_log_files() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let config = state.config().unwrap().clone();
    state.shutdown();
    assert!(state.connection().is_none());
    // Windows refuses to delete files that are still open, so this proves both were released.
    avalio_lite::db::remove_files(&config.database_path).unwrap();
    fs::remove_file(config.log_dir.join(LOG_FILE_NAME)).unwrap();
}

#[test]
fn the_theme_choice_is_saved() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_theme(ThemeChoice::Dark).unwrap();
    assert_eq!(fixture.store().load().unwrap().unwrap().preferences.theme, Some(ThemeChoice::Dark));
}

#[test]
fn a_theme_before_setup_is_refused() {
    let fixture = Fixture::new();
    assert!(matches!(fixture.state().set_theme(ThemeChoice::Dark), Err(AppError::NoDatabaseOpen)));
}

#[test]
fn a_database_held_by_another_program_asks_for_setup() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let _held = common::hold_exclusively(&fixture.database());
    let startup = fixture.state().start();
    assert!(matches!(startup, Startup::NeedsSetup(SetupReason::DatabaseUnusable(_))), "{startup:?}");
}

#[test]
fn an_unusable_log_folder_still_opens_the_database_and_logs_can_move_later() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let mut config = fixture.store().load().unwrap().unwrap();
    // the log folder became a file, so there is no log file to write or move
    config.log_dir = fixture.path("blocked");
    fs::write(&config.log_dir, b"in the way").unwrap();
    fixture.store().save(&config).unwrap();

    let mut state = fixture.state();
    assert_eq!(state.start(), Startup::Ready);
    state.move_logs(&fixture.path("logs-again")).unwrap();
    assert!(fixture.path("logs-again").join(LOG_FILE_NAME).is_file(), "logging starts in the new folder");
}

#[test]
fn moving_logs_to_the_same_folder_does_nothing() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let logs = fixture.path("logs");
    state.move_logs(&logs).unwrap();
    assert!(logs.join(LOG_FILE_NAME).is_file());
    assert_eq!(state.config().unwrap().log_dir, logs);
}

#[test]
fn a_database_move_that_fails_midway_leaves_no_copy_and_keeps_the_original() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    add_pupil(&state);
    let destination = fixture.path("moved");
    // the settings can't be saved, so the move fails after the copy was made and checked
    let config_dir = fixture.path("config");
    fs::remove_dir_all(&config_dir).unwrap();
    fs::write(&config_dir, b"in the way").unwrap();
    assert!(state.move_database(&destination).is_err());
    assert!(!destination.join(DATABASE_FILE_NAME).exists(), "the copy is removed");
    assert!(fixture.database().is_file());
    assert_eq!(pupil_count(&state), 1, "the original stays open and intact");
}
