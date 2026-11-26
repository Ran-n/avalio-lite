// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:37.339413
//+ Revised: 	2026/11/24 14:49:43.426336
// -----------------------------------------------------------------------------

//! Setup and the remembered preferences: folders, columns, the sidebar, language and theme.

use crate::common::Fixture;
use avalio_lite::error::{AppError, Folder};
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::{
    AppConfig, DATABASE_FILE_NAME, LOG_FILE_NAME, Preferences, ThemeChoice, WindowPlace,
};
use avalio_lite::state::{SetupReason, Startup};
use std::path::Path;

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
fn the_update_choices_are_remembered_across_restarts() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let (checked_at, skipped) = (chrono::Utc::now(), semver::Version::new(9, 8, 7));
    assert!(state.preferences().check_automatically, "on until the user turns it off");
    state.set_check_automatically(false).unwrap();
    state.set_last_check(checked_at).unwrap();
    state.set_skipped_version(Some(&skipped)).unwrap();
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Ready);
    let kept = restarted.preferences();
    assert!(!kept.check_automatically);
    assert_eq!(kept.last_check, Some(checked_at.timestamp()));
    assert_eq!(kept.skipped_version.as_deref(), Some("9.8.7"));
    restarted.set_skipped_version(None).unwrap();
    assert_eq!(restarted.preferences().skipped_version, None, "nothing is skipped any more");
    restarted.shutdown();
}

#[test]
fn the_window_place_is_remembered_across_restarts() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let first = WindowPlace { x: 10, y: 20, width: 1100, height: 700, maximized: false };
    let moved = WindowPlace { x: 11, ..first };
    for place in [first, moved] {
        state.set_window_place(place).unwrap();
        assert_eq!(fixture.store().load().unwrap().unwrap().preferences.window, Some(place));
    }
    state.shutdown();

    let mut restarted = fixture.state();
    assert_eq!(restarted.start(), Startup::Ready);
    assert_eq!(restarted.preferences().window, Some(moved));
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
