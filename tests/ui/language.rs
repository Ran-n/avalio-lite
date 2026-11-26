// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.847939
//+ Revised: 	2026/11/23 13:00:31.847939
// -----------------------------------------------------------------------------

//! Settings › language and theme, and the keyboard and screen-reader reach of those controls.

use crate::driver::{PIN, click, find, gradebook, key, press_enter, shows, shows_text, type_into};
use crate::fixtures::{cell_text, encrypted, open_settings, saved_preferences, seeded};
use avalio_lite::i18n::Language;
use avalio_lite::platform::config::ThemeChoice;
use avalio_lite::ui::Theme;
use slint::ComponentHandle;
use slint::platform::Key;
use std::fs;

// ---------------------------------------------------------------------------------------------
// Settings: language and theme

#[test]
fn choosing_galician_switches_the_interface_and_numbers() {
    let ui = seeded();
    type_into(&ui, "Ana, Exam", "7.5");
    open_settings(&ui);
    click(&ui, "Galego");
    assert_eq!(ui.session.window.get_language_code(), "gl");
    assert_eq!(saved_preferences(&ui.fixture).language, Some(Language::Galician));
    assert!(shows(&ui, "Escuro"), "the settings page is in Galician");
    assert_eq!(cell_text(&ui, 0, 1), "7,5", "calculated numbers use a decimal comma");
    assert_eq!(cell_text(&ui, 0, 0), "7.5", "typed text stays as typed");
    click(&ui, "English");
    assert_eq!(cell_text(&ui, 0, 1), "7.5");
    assert_eq!(saved_preferences(&ui.fixture).language, Some(Language::English));
}

#[test]
fn the_theme_buttons_and_sidebar_toggle_switch_and_save() {
    let ui = seeded();
    let theme = ui.session.window.global::<Theme>();
    open_settings(&ui);
    click(&ui, "Dark");
    assert!(theme.get_dark());
    assert_eq!(saved_preferences(&ui.fixture).theme, Some(ThemeChoice::Dark));
    click(&ui, "Light");
    assert!(!theme.get_dark());
    assert_eq!(saved_preferences(&ui.fixture).theme, Some(ThemeChoice::Light));
    click(&ui, "Dark mode");
    assert!(theme.get_dark());
    click(&ui, "Light mode");
    assert!(!theme.get_dark());
    assert_eq!(saved_preferences(&ui.fixture).theme, Some(ThemeChoice::Light));
}

#[test]
fn tab_reaches_the_sidebar_buttons_and_enter_or_space_presses_them() {
    let ui = seeded();
    let theme = ui.session.window.global::<Theme>();
    click(&ui, "New class");
    key(&ui, Key::Tab);
    press_enter(&ui);
    assert!(theme.get_dark(), "Tab moved from the field to the theme button and Enter pressed it");
    key(&ui, " ");
    assert!(!theme.get_dark(), "Space pressed it again");
    key(&ui, Key::Tab);
    press_enter(&ui);
    assert!(shows_text(&ui, "Appearance"), "the next Tab stop is Settings");
}

#[test]
fn screen_readers_can_press_the_custom_controls() {
    let ui = seeded();
    find(&ui, "Settings").invoke_accessible_default_action();
    assert!(shows_text(&ui, "Appearance"));
    find(&ui, "Dark").invoke_accessible_default_action();
    assert!(ui.session.window.global::<Theme>().get_dark());
}

#[test]
fn screen_readers_can_press_the_lock_screen_keys() {
    let ui = encrypted();
    find(&ui, "Letters").invoke_accessible_default_action();
    find(&ui, "q").invoke_accessible_default_action();
    find(&ui, "Numbers").invoke_accessible_default_action();
    find(&ui, "1").invoke_accessible_default_action();
    assert_eq!(ui.session.window.get_unlock_pin(), "q1");
    find(&ui, "Erase").invoke_accessible_default_action();
    find(&ui, "Erase").invoke_accessible_default_action();
    for digit in PIN.chars() {
        find(&ui, &digit.to_string()).invoke_accessible_default_action();
    }
    find(&ui, "Unlock").invoke_accessible_default_action();
    assert!(!ui.session.window.get_locked());
}

#[test]
fn preferences_that_cannot_be_saved_are_reported() {
    let ui = seeded();
    open_settings(&ui);
    // the settings folder turns into a file, so config.toml can't be written
    let config_dir = ui.fixture.path("config");
    fs::remove_dir_all(&config_dir).unwrap();
    fs::write(&config_dir, b"in the way").unwrap();
    click(&ui, "Dark");
    assert!(!ui.session.window.get_error_message().is_empty(), "theme");
    click(&ui, "Galego");
    assert!(!ui.session.window.get_error_message().is_empty(), "language");
    assert_eq!(ui.session.window.get_language_code(), "en", "the language stays when it can't be saved");
    click(&ui, "1A");
    click(&ui, "More actions");
    click(&ui, "Hide weights & scales");
    assert!(gradebook(&ui).get_show_column_settings(), "the column settings stay shown");
    assert!(!ui.session.window.get_error_message().is_empty());
}
