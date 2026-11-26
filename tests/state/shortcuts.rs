// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:37.489482
//+ Revised: 	2026/11/23 13:00:37.489482
// -----------------------------------------------------------------------------

//! Shortcuts kept in the settings: bad entries, changes, resets and the other database.

use crate::common::Fixture;
use avalio_lite::error::AppError;
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::ThemeChoice;
use avalio_lite::shortcuts::{Action, ShortcutError, Shortcuts};
use avalio_lite::state::{SetupReason, Startup};
use std::fs;

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
    let _held = crate::common::hold_exclusively(fixture.store().path());
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
