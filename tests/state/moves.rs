// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:37.259198
//+ Revised: 	2026/11/23 13:00:37.259198
// -----------------------------------------------------------------------------

//! Moving the database and the logs: where they go, what is refused, and what is kept when a move fails.

use crate::common::Fixture;
use crate::support::{add_pupil, pupil_count};
use avalio_lite::error::AppError;
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::{DATABASE_FILE_NAME, LOG_FILE_NAME, ThemeChoice};
use avalio_lite::shortcuts::Action;
use avalio_lite::state::Startup;
use std::fs;

#[test]
fn move_database_keeps_data_updates_settings_and_removes_original() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    add_pupil(&state);
    let original = state.config().unwrap().database_path.clone();

    let moved = state.move_database(&fixture.path("moved")).unwrap();
    assert_eq!(moved.left_behind, None);
    let moved = moved.path;
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
    assert_eq!(state.move_database(&fixture.path("data")).unwrap().path, original);
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
fn a_new_database_with_an_unusable_log_folder_still_opens() {
    let fixture = Fixture::new();
    let blocked = fixture.path("blocked");
    fs::write(&blocked, b"a file where the folder should be").unwrap();
    let mut state = fixture.state();
    state.create_database(&fixture.path("data"), &blocked).unwrap();
    assert!(state.connection().is_some(), "logging falls back to stderr; the data still opens");
    assert_eq!(fixture.store().load().unwrap().unwrap().log_dir, blocked);
    state.shutdown();
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

#[test]
fn regression_a_failed_settings_save_does_not_leave_the_log_moved_away_from_the_settings() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    // a directory where the temporary settings file goes makes saving fail
    fs::create_dir(fixture.path("config").join("config.toml.tmp")).unwrap();
    let result = state.move_logs(&fixture.path("new-logs"));
    assert!(matches!(result, Err(AppError::Io { .. })));
    assert_eq!(state.config().unwrap().log_dir, fixture.path("logs"));
    assert_eq!(fixture.store().load().unwrap().unwrap().log_dir, fixture.path("logs"));
    assert!(fixture.path("logs").join(LOG_FILE_NAME).is_file(), "the log is back where the settings say");
    assert!(!fixture.path("new-logs").join(LOG_FILE_NAME).exists());
}

#[test]
fn moving_the_logs_takes_the_old_rotated_log_along() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let old = avalio_lite::platform::logging::rotated_path(&fixture.path("logs"));
    fs::write(&old, b"history").unwrap();
    state.move_logs(&fixture.path("new-logs")).unwrap();
    assert!(!old.exists());
    assert_eq!(fs::read(avalio_lite::platform::logging::rotated_path(&fixture.path("new-logs"))).unwrap(), b"history");
}

#[test]
fn an_old_log_that_cannot_be_taken_along_stays_behind() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let old = avalio_lite::platform::logging::rotated_path(&fixture.path("logs"));
    fs::write(&old, b"history").unwrap();
    // something already has the old log's name in the new folder
    let taken = avalio_lite::platform::logging::rotated_path(&fixture.path("new-logs"));
    fs::create_dir_all(&taken).unwrap();
    state.move_logs(&fixture.path("new-logs")).unwrap();
    assert_eq!(fs::read(&old).unwrap(), b"history", "the history stays where it was");
    assert_eq!(state.config().unwrap().log_dir, fixture.path("new-logs"), "the move itself went through");
}

#[test]
fn regression_a_large_log_moved_with_its_old_log_strands_nothing() {
    // the moved log, past the size limit after a long session, was set aside as `.1` on reopening
    // before the old `.1` was taken along, which then found its name taken and stayed behind
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let logs = fixture.path("logs");
    fs::write(avalio_lite::platform::logging::rotated_path(&logs), b"history").unwrap();
    let mut log = fs::OpenOptions::new().append(true).open(logs.join(LOG_FILE_NAME)).unwrap();
    let limit = usize::try_from(avalio_lite::platform::logging::MAX_LOG_BYTES).unwrap();
    std::io::Write::write_all(&mut log, &vec![b'x'; limit + 1]).unwrap();
    drop(log);
    state.move_logs(&fixture.path("new-logs")).unwrap();
    assert!(!avalio_lite::platform::logging::rotated_path(&logs).exists(), "nothing is left in the old folder");
    assert!(avalio_lite::platform::logging::rotated_path(&fixture.path("new-logs")).is_file());
}

#[test]
fn preference_changes_without_a_database_are_refused() {
    let fixture = Fixture::new();
    let mut state = fixture.state();
    assert!(matches!(state.set_theme(ThemeChoice::Dark), Err(AppError::NoDatabaseOpen)));
    assert!(matches!(state.set_language(Language::Galician), Err(AppError::NoDatabaseOpen)));
    assert!(matches!(state.set_show_column_settings(false), Err(AppError::NoDatabaseOpen)));
    assert!(matches!(state.set_shortcut(Action::Lock, "Ctrl+K".parse().unwrap()), Err(AppError::NoDatabaseOpen)));
    assert_eq!(fixture.store().load().unwrap(), None, "no settings file is written");
}
