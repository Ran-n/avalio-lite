// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.003607
//+ Revised: 	2026/11/23 13:00:33.003607
// -----------------------------------------------------------------------------

//! Keyboard shortcuts: the defaults, changing them in Settings, and what each does on each screen.

use crate::common::{Fixture, Ui};
use crate::driver::{
    click, click_element, click_nth, elements, find, gradebook, key, press_enter, shows, shows_text, type_into,
    type_text, with_control,
};
use crate::fixtures::{
    encrypted, encrypted_fixture, name_fields, open_pin_settings, open_settings, saved_preferences, seeded, unlock,
    with_keys, with_second_class, with_shortcut_config,
};
use avalio_lite::platform::config::ThemeChoice;
use avalio_lite::ui::{Section, Theme};
use slint::platform::Key;
use slint::{ComponentHandle, Model};

#[test]
fn every_shortcut_works_from_a_focused_cell() {
    let ui = with_second_class();
    let window = &ui.session.window;
    click_element(&name_fields(&ui, "First name")[0]);
    with_keys(&ui, true, "D");
    assert!(window.global::<Theme>().get_dark(), "Ctrl+Shift+D switches to dark");
    assert_eq!(saved_preferences(&ui.fixture).theme, Some(ThemeChoice::Dark), "and saves it");
    with_keys(&ui, false, "h");
    assert!(!gradebook(&ui).get_show_column_settings(), "Ctrl+H hides weights and scales");
    with_keys(&ui, false, ",");
    assert_eq!(window.get_section(), Section::Settings, "Ctrl+, opens Settings");
    with_keys(&ui, false, ",");
    assert_eq!(window.get_section(), Section::Sheet, "and closes it again");
    assert_eq!(gradebook(&ui).get_class_name(), "1A");
    with_keys(&ui, false, Key::PageDown);
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "Ctrl+PageDown: the next class");
    with_keys(&ui, false, Key::PageDown);
    assert_eq!(gradebook(&ui).get_class_name(), "1A", "round to the first again");
    with_keys(&ui, false, ",");
    with_keys(&ui, false, Key::PageUp);
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "Ctrl+PageUp from Settings: the previous class");
    assert_eq!(window.get_section(), Section::Sheet, "shown on the sheet");
}

#[test]
fn the_class_shortcuts_do_nothing_without_classes() {
    let ui = Ui::with_database();
    with_keys(&ui, false, Key::PageDown);
    with_keys(&ui, false, Key::PageUp);
    assert_eq!(gradebook(&ui).get_current_class_id(), -1);
    assert!(ui.session.window.get_error_message().is_empty());
}

#[test]
fn shortcuts_do_nothing_on_the_lock_and_first_run_screens() {
    let ui = encrypted();
    with_keys(&ui, false, ",");
    with_keys(&ui, true, "D");
    assert!(ui.session.window.get_locked());
    assert!(!ui.session.window.global::<Theme>().get_dark());
    type_text(&ui, "12");
    assert_eq!(ui.session.window.get_unlock_pin(), "12", "the PIN field still takes digits");
    drop(ui);

    let ui = Ui::open(Fixture::new());
    with_keys(&ui, false, ",");
    assert!(ui.session.window.get_setup_required());
    assert_eq!(ui.session.window.get_section(), Section::Sheet);
}

#[test]
fn a_configured_lock_key_locks_and_shows_in_the_labels() {
    let fixture = encrypted_fixture();
    with_shortcut_config(&fixture, &[("lock", "Ctrl+K")]);
    let ui = Ui::open(fixture);
    unlock(&ui);
    assert!(shows(&ui, "Lock (Ctrl+K)"));
    open_pin_settings(&ui);
    let pin_text = "The database file is encrypted with your PIN. The app asks for it when it opens; lock it any time with the Lock button or Ctrl+K.";
    assert!(shows_text(&ui, pin_text));
    with_control(&ui, "l");
    assert!(!ui.session.window.get_locked(), "Ctrl+L no longer locks");
    with_control(&ui, "k");
    assert!(ui.session.window.get_locked());
}

#[test]
fn bad_shortcut_entries_are_reported_and_use_the_defaults() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    with_shortcut_config(&fixture, &[("lock", "Ctrl+"), ("toggle-theme", "Ctrl+L")]);
    let ui = Ui::open(fixture);
    let banner = ui.session.window.get_error_message().to_string();
    assert!(banner.contains("lock = \"Ctrl+\"") && banner.contains("toggle-theme = \"Ctrl+L\""), "{banner}");
    with_keys(&ui, true, "D");
    assert!(ui.session.window.global::<Theme>().get_dark(), "the theme keeps its default");
}

#[test]
fn settings_lists_the_shortcuts_in_the_current_language() {
    let ui = seeded();
    open_settings(&ui);
    assert!(shows_text(&ui, "Keyboard shortcuts"));
    let items = ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_items();
    assert_eq!(items.row_count(), 7);
    let theme = items.row_data(1).unwrap();
    assert_eq!(theme.label, "Switch between light and dark");
    assert_eq!(theme.keys.iter().collect::<Vec<_>>(), ["Ctrl", "Shift", "D"]);
    click(&ui, "Galego");
    let theme = ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_items().row_data(1).unwrap();
    assert_eq!(theme.label, "Cambiar entre claro e escuro");
    assert_eq!(theme.keys.iter().collect::<Vec<_>>(), ["Ctrl", "Maiús", "D"]);
    assert!(shows_text(&ui, "Atallos de teclado"));
}

fn shortcut_row(ui: &Ui, row: usize) -> avalio_lite::ui::ShortcutItem {
    ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_items().row_data(row).unwrap()
}

fn recording(ui: &Ui) -> i32 {
    ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_recording()
}

#[test]
fn a_shortcut_is_changed_by_pressing_change_and_the_new_keys() {
    let ui = seeded();
    open_settings(&ui);
    assert!(ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_all_default());
    click_nth(&ui, "Change", 0);
    assert_eq!(recording(&ui), 0, "the Lock row records");
    assert!(shows_text(&ui, "Press the new keys…"));
    with_control(&ui, "k");
    assert_eq!(recording(&ui), -1);
    assert_eq!(shortcut_row(&ui, 0).keys.iter().collect::<Vec<_>>(), ["Ctrl", "K"]);
    assert!(!shortcut_row(&ui, 0).is_default);
    assert_eq!(ui.session.window.get_status_message(), "Lock is now Ctrl+K.");
    assert_eq!(saved_shortcut(&ui, "lock"), Some("Ctrl+K".to_owned()), "saved at once");
    assert!(!ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_all_default(), "Reset all is enabled");
}

fn saved_shortcut(ui: &Ui, name: &str) -> Option<String> {
    ui.fixture.store().load().unwrap().unwrap().shortcuts.get(name).cloned()
}

#[test]
fn refused_keys_keep_the_row_recording_with_the_reason() {
    let ui = seeded();
    open_settings(&ui);
    click_nth(&ui, "Change", 0);
    with_control(&ui, "h");
    assert_eq!(recording(&ui), 0, "still recording after a clash");
    assert_eq!(ui.session.window.get_error_message(), "Ctrl+H is already used by Show or hide weights and scales.");
    key(&ui, "k");
    assert_eq!(ui.session.window.get_error_message(), "Add Ctrl or Alt: a plain letter would type into the cell.");
    with_control(&ui, "c");
    assert_eq!(ui.session.window.get_error_message(), "Ctrl+C is used for editing text.");
    key(&ui, Key::Control);
    assert_eq!(recording(&ui), 0, "Ctrl alone waits for the key");
    key(&ui, Key::Escape);
    assert_eq!(recording(&ui), -1, "Esc cancels");
    assert!(shortcut_row(&ui, 0).is_default, "and the shortcut is unchanged");
    assert_eq!(saved_shortcut(&ui, "lock"), None);
}

#[test]
fn leaving_settings_or_locking_ends_a_recording() {
    let ui = encrypted();
    unlock(&ui);
    type_into(&ui, "New class", "1A");
    press_enter(&ui);
    open_settings(&ui);
    click_nth(&ui, "Change", 1);
    with_control(&ui, "l");
    assert!(!ui.session.window.get_locked(), "while recording, Ctrl+L is recorded (and refused), not run");
    click(&ui, "1A");
    assert_eq!(recording(&ui), -1, "leaving Settings cancels");
    open_settings(&ui);
    click_nth(&ui, "Change", 1);
    click(&ui, "Lock (Ctrl+L)");
    assert_eq!(recording(&ui), -1, "locking cancels");
}

#[test]
fn reset_and_reset_all_bring_back_the_defaults() {
    let ui = seeded();
    open_settings(&ui);
    click_nth(&ui, "Change", 0);
    with_control(&ui, "k");
    click_nth(&ui, "Change", 5);
    key(&ui, Key::F8);
    assert_eq!(saved_shortcut(&ui, "next-class"), Some("F8".to_owned()));
    assert_eq!(elements(&ui, "Reset").len(), 2, "a Reset for each changed row");
    click_nth(&ui, "Reset", 0);
    assert!(shortcut_row(&ui, 0).is_default);
    assert_eq!(saved_shortcut(&ui, "lock"), None);
    find(&ui, "Galego").scroll(0.0, -200.0);
    click(&ui, "Reset all");
    assert_eq!(ui.session.window.get_status_message(), "All keyboard shortcuts are back to their defaults.");
    assert!(ui.fixture.store().load().unwrap().unwrap().shortcuts.is_empty());
    assert!(ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_all_default(), "Reset all is greyed again");
    assert!(!shows(&ui, "Reset"));
}

#[test]
fn a_recording_for_a_row_that_does_not_exist_changes_nothing() {
    let ui = seeded();
    ui.session.window.global::<avalio_lite::ui::Shortcuts>().invoke_record(99, "k".into(), true, false, false);
    ui.session.window.global::<avalio_lite::ui::Shortcuts>().invoke_reset(-1);
    assert!(ui.session.window.global::<avalio_lite::ui::Shortcuts>().get_all_default());
    assert!(ui.fixture.store().load().unwrap().unwrap().shortcuts.is_empty());
}

#[test]
fn a_reset_that_would_clash_names_the_action_with_those_keys() {
    let ui = seeded();
    open_settings(&ui);
    click_nth(&ui, "Change", 0);
    with_control(&ui, "k");
    click_nth(&ui, "Change", 2);
    with_control(&ui, "l");
    assert_eq!(shortcut_row(&ui, 2).keys.iter().collect::<Vec<_>>(), ["Ctrl", "L"]);
    click_nth(&ui, "Reset", 0);
    assert_eq!(ui.session.window.get_error_message(), "Ctrl+L is already used by Show or hide weights and scales.");
}
