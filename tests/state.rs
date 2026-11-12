// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:32.383625
//+ Revised: 	2026/11/12 14:39:57.419282
// -----------------------------------------------------------------------------

use std::fs;
use std::path::Path;

mod common;
use common::Fixture;

use avalio_lite::error::{AppError, Folder};
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::{AppConfig, DATABASE_FILE_NAME, LOG_FILE_NAME, Preferences, ThemeChoice};
use avalio_lite::shortcuts::{Action, ShortcutError, Shortcuts};
use avalio_lite::state::{AppState, SetupReason, Startup};

fn pupil_count(state: &AppState) -> i64 {
    state.connection().unwrap().query_row("SELECT COUNT(*) FROM pupils", [], |row| row.get(0)).unwrap()
}

fn add_pupil(state: &AppState) {
    state.connection().unwrap().execute("INSERT INTO pupils (first_name) VALUES ('Ana Pérez')", []).unwrap();
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
        shortcuts: Default::default(),
        last_class: None,
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
    avalio_lite::platform::db::close(avalio_lite::platform::db::open(&other, None).unwrap()).unwrap();
    state.open_existing(&other, &fixture.path("logs")).unwrap();
    assert!(!state.preferences().show_column_settings, "switching database keeps the preference");
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Ready);
    assert!(!restarted.preferences().show_column_settings);
    restarted.shutdown();
}

#[test]
fn folding_the_sidebar_is_remembered_across_restarts_and_database_switches() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    assert!(!state.preferences().sidebar_collapsed, "the sidebar starts expanded");
    state.set_sidebar_collapsed(true).unwrap();
    assert!(fixture.store().load().unwrap().unwrap().preferences.sidebar_collapsed);

    let other = fixture.path("other.db");
    avalio_lite::platform::db::close(avalio_lite::platform::db::open(&other, None).unwrap()).unwrap();
    state.open_existing(&other, &fixture.path("logs")).unwrap();
    assert!(state.preferences().sidebar_collapsed, "switching database keeps the preference");
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Ready);
    assert!(restarted.preferences().sidebar_collapsed);
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
    assert!(matches!(state.set_sidebar_collapsed(true), Err(AppError::NoDatabaseOpen)));
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

/// A folder typed without its drive would put the database wherever the app was started from.
#[test]
fn a_relative_folder_is_rejected_and_nothing_is_saved() {
    let fixture = Fixture::new();
    let mut state = fixture.state();
    let result = state.create_database(Path::new("Marks"), &fixture.path("logs"));
    assert!(matches!(result, Err(AppError::RelativePath(Folder::Database))), "{result:?}");
    let result = state.create_database(&fixture.path("data"), Path::new(r"logs\here"));
    assert!(matches!(result, Err(AppError::RelativePath(Folder::Logs))), "{result:?}");
    assert_eq!(fixture.store().load().unwrap(), None);
    assert!(!Path::new("Marks").exists(), "nothing was made in the current folder");
}

/// A crash mid PIN change can leave only `.previous`, the database gone; making a new database in
/// that folder must not delete it, since it may be the only good copy.
#[test]
fn a_new_database_keeps_the_previous_copy_of_one_that_went_missing() {
    let fixture = Fixture::new();
    std::fs::create_dir_all(fixture.path("data")).unwrap();
    let previous = std::path::PathBuf::from(format!("{}.previous", fixture.database().display()));
    std::fs::write(&previous, b"the only good copy").unwrap();
    let mut state = fixture.state();
    state.create_database(&fixture.path("data"), &fixture.path("logs")).unwrap();
    assert!(previous.exists());
}

/// The same for a database without a PIN: the copies go as soon as it opens at start.
#[test]
fn copies_left_by_an_interrupted_change_go_when_the_database_opens() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let leftover = std::path::PathBuf::from(format!("{}.previous", fixture.database().display()));
    std::fs::write(&leftover, b"old copy").unwrap();
    let mut state = fixture.state();
    assert_eq!(state.start(), Startup::Ready);
    assert!(!leftover.exists());
}

#[test]
fn missing_database_is_reported_and_not_recreated() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let database_path = state.config().unwrap().database_path.clone();
    state.shutdown();
    avalio_lite::platform::db::remove_files(&database_path).unwrap();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::NeedsSetup(SetupReason::DatabaseMissing(database_path.clone())));
    assert!(!database_path.exists());
}

/// Was: with its database gone, the settings file's preferences were dropped, so finishing setup
/// saved the defaults over the chosen language and theme.
#[test]
fn regression_setup_after_a_missing_database_keeps_the_preferences_and_shortcuts() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_language(Language::Galician).unwrap();
    state.set_theme(ThemeChoice::Dark).unwrap();
    let mut config = fixture.store().load().unwrap().unwrap();
    config.shortcuts.insert("lock".into(), "Ctrl+K".into());
    fixture.store().save(&config).unwrap();
    state.shutdown();
    avalio_lite::platform::db::remove_files(&fixture.database()).unwrap();

    let mut restarted = fixture.state();
    assert!(matches!(restarted.start(), Startup::NeedsSetup(SetupReason::DatabaseMissing(_))));
    restarted.create_database(&fixture.path("fresh"), &fixture.path("logs")).unwrap();
    let saved = fixture.store().load().unwrap().unwrap();
    assert_eq!(saved.preferences.language, Some(Language::Galician));
    assert_eq!(saved.preferences.theme, Some(ThemeChoice::Dark));
    assert_eq!(saved.shortcuts.get("lock").map(String::as_str), Some("Ctrl+K"));
    restarted.shutdown();
}

#[test]
fn bad_shortcuts_are_reported_once() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let mut config = fixture.store().load().unwrap().unwrap();
    config.shortcuts.insert("lock".into(), "Ctrl+".into());
    config.shortcuts.insert("next-class".into(), "F8".into());
    fixture.store().save(&config).unwrap();

    let mut state = fixture.state();
    assert_eq!(state.start(), Startup::Ready);
    let problems = state.take_shortcut_problems();
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].entry, "lock = \"Ctrl+\"");
    assert!(state.take_shortcut_problems().is_empty(), "shown once");
    let next = state.shortcuts().combo(avalio_lite::shortcuts::Action::NextClass);
    assert_eq!(next.to_string(), "F8", "the good entry is used");
    state.shutdown();
}

#[test]
fn a_changed_shortcut_survives_a_restart() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_shortcut(Action::Lock, "Ctrl+K".parse().unwrap()).unwrap();
    state.shutdown();
    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Ready);
    assert_eq!(restarted.shortcuts().combo(Action::Lock).to_string(), "Ctrl+K");
    restarted.shutdown();
}

#[test]
fn a_failed_change_leaves_everything_as_it_was() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let before = fixture.store().load().unwrap();
    let result = state.set_shortcut(Action::ToggleTheme, "Ctrl+L".parse().unwrap());
    assert!(matches!(result, Err(AppError::Shortcut(ShortcutError::Clash { taken_by: Action::Lock, .. }))));
    assert_eq!(fixture.store().load().unwrap(), before);
    assert_eq!(state.shortcuts(), &Shortcuts::defaults());
    state.shutdown();
}

#[test]
fn a_change_that_cannot_be_saved_changes_nothing() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    // another program holds the settings file, so it can't be replaced
    let _held = common::hold_exclusively(fixture.store().path());
    assert!(matches!(state.set_shortcut(Action::Lock, "Ctrl+K".parse().unwrap()), Err(AppError::Io { .. })));
    assert_eq!(state.shortcuts(), &Shortcuts::defaults());
    drop(_held);
    state.shutdown();
}

#[test]
fn reset_all_removes_the_table() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_shortcut(Action::Lock, "Ctrl+K".parse().unwrap()).unwrap();
    state.set_shortcut(Action::NextClass, "F8".parse().unwrap()).unwrap();
    assert!(fs::read_to_string(fixture.store().path()).unwrap().contains("[shortcuts]"));
    state.reset_shortcut(Action::NextClass).unwrap();
    assert!(!fs::read_to_string(fixture.store().path()).unwrap().contains("next-class"));
    state.reset_all_shortcuts().unwrap();
    assert!(!fs::read_to_string(fixture.store().path()).unwrap().contains("[shortcuts]"));
    assert_eq!(state.shortcuts(), &Shortcuts::defaults());
    state.shutdown();
}

#[test]
fn shortcuts_cannot_be_changed_before_setup() {
    let fixture = Fixture::new();
    let mut state = fixture.state();
    assert!(matches!(state.set_shortcut(Action::Lock, "Ctrl+K".parse().unwrap()), Err(AppError::NoDatabaseOpen)));
}

#[test]
fn custom_shortcuts_survive_opening_another_database() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let mut config = fixture.store().load().unwrap().unwrap();
    config.shortcuts.insert("lock".into(), "Ctrl+K".into());
    fixture.store().save(&config).unwrap();
    state.shutdown();
    let mut state = fixture.state();
    assert_eq!(state.start(), Startup::Ready);
    let other = fixture.path("other.db");
    avalio_lite::platform::db::close(avalio_lite::platform::db::open(&other, None).unwrap()).unwrap();
    state.open_existing(&other, &fixture.path("logs")).unwrap();
    assert_eq!(fixture.store().load().unwrap().unwrap().shortcuts.get("lock").map(String::as_str), Some("Ctrl+K"));
    state.shutdown();
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
    avalio_lite::platform::db::remove_files(&database_path).unwrap();
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
    avalio_lite::platform::db::close(avalio_lite::platform::db::open(&other, None).unwrap()).unwrap();
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
fn shutdown_releases_database_and_log_files() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let config = state.config().unwrap().clone();
    state.shutdown();
    assert!(state.connection().is_none());
    // Windows refuses to delete files that are still open, so this proves both were released.
    avalio_lite::platform::db::remove_files(&config.database_path).unwrap();
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

fn keeps_working_as_before(state: &AppState, fixture: &Fixture) {
    assert_eq!(state.config().unwrap().database_path, fixture.database(), "the choice was not taken");
    assert_eq!(fixture.store().load().unwrap().unwrap().database_path, fixture.database(), "nothing was saved");
    assert!(state.connection().is_some(), "the current database stays open");
    assert!(!state.is_locked());
}

#[test]
fn regression_an_empty_file_is_not_initialised_as_a_database_when_chosen() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let stray = fixture.path("stray.db");
    fs::write(&stray, b"").unwrap();
    let result = state.open_existing(&stray, &fixture.path("logs"));
    assert!(matches!(result, Err(AppError::NotADatabase(path)) if path == stray));
    assert_eq!(fs::metadata(&stray).unwrap().len(), 0, "the file was not touched");
    keeps_working_as_before(&state, &fixture);
}

#[test]
fn regression_a_stray_file_does_not_make_the_app_ask_for_a_pin() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let stray = fixture.path("stray.db");
    // looks encrypted, but is not whole pages
    fs::write(&stray, vec![7u8; 700]).unwrap();
    assert!(matches!(state.open_existing(&stray, &fixture.path("logs")), Err(AppError::NotADatabase(_))));
    keeps_working_as_before(&state, &fixture);
    fs::write(&stray, b"tiny").unwrap();
    assert!(matches!(state.open_existing(&stray, &fixture.path("logs")), Err(AppError::NotADatabase(_))));
    keeps_working_as_before(&state, &fixture);
}

#[test]
fn a_sqlite_file_that_is_not_avalio_lite_is_refused_untouched() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let foreign = fixture.path("foreign.db");
    let connection = rusqlite::Connection::open(&foreign).unwrap();
    connection.execute("CREATE TABLE notes (text)", []).unwrap();
    connection.close().unwrap();
    let before = fs::read(&foreign).unwrap();
    assert!(matches!(state.open_existing(&foreign, &fixture.path("logs")), Err(AppError::NotADatabase(_))));
    assert_eq!(fs::read(&foreign).unwrap(), before, "no migration ran on it");
    keeps_working_as_before(&state, &fixture);
}

#[test]
fn a_whole_page_file_that_looks_encrypted_is_chosen_locked() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let locked = fixture.path("locked.db");
    fs::write(&locked, vec![0xA5u8; 4096]).unwrap();
    assert_eq!(state.open_existing(&locked, &fixture.path("logs")).unwrap(), avalio_lite::state::Opened::Locked);
    assert!(state.is_locked());
}

#[test]
fn regression_start_does_not_initialise_an_empty_or_foreign_database_file() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let path = state.config().unwrap().database_path.clone();
    state.shutdown();
    avalio_lite::platform::db::remove_files(&path).unwrap();
    for contents in [&b""[..], b"short", b"SQLite format 3\0 but then garbage garbage"] {
        fs::write(&path, contents).unwrap();
        let startup = fixture.state().start();
        assert!(matches!(startup, Startup::NeedsSetup(SetupReason::DatabaseUnusable(_))), "{contents:?}");
        assert_eq!(fs::read(&path).unwrap(), contents, "the file was left as it was");
    }
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
fn a_database_from_a_newer_version_needs_setup_at_startup() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let connection = rusqlite::Connection::open(fixture.database()).unwrap();
    connection.pragma_update(None, "user_version", 999).unwrap();
    drop(connection);
    let mut state = fixture.state();
    let startup = state.start();
    assert!(
        matches!(&startup, Startup::NeedsSetup(SetupReason::DatabaseUnusable(text)) if text.contains("newer version")),
        "{startup:?}"
    );
}

#[test]
fn a_database_another_program_is_writing_to_is_reported_and_not_opened() {
    let fixture = Fixture::new();
    let other_place = fixture.path("other").join(DATABASE_FILE_NAME);
    avalio_lite::platform::db::close(avalio_lite::platform::db::open(&other_place, None).unwrap()).unwrap();
    let mut state = fixture.set_up();
    let writer = rusqlite::Connection::open(&other_place).unwrap();
    writer.execute_batch("PRAGMA journal_mode = DELETE; PRAGMA locking_mode = EXCLUSIVE; BEGIN EXCLUSIVE;").unwrap();
    let result = state.open_existing(&other_place, &fixture.path("logs"));
    assert!(matches!(result, Err(AppError::Database(_))), "{result:?}");
    assert_eq!(state.config().unwrap().database_path, fixture.database(), "still on the database it had");
    writer.execute_batch("ROLLBACK").unwrap();
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
