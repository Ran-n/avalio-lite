// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.093362
//+ Revised: 	2026/11/23 13:00:33.093362
// -----------------------------------------------------------------------------

//! The sidebar: collapsing and expanding it, and remembering that across starts.

use crate::common::{Fixture, Ui};
use crate::driver::{PIN, click, gradebook, key, press_enter, shows, shows_text, type_text};
use crate::fixtures::{saved_preferences, seeded, unlock, with_keys};
use avalio_lite::gradebook::classes;
use avalio_lite::ui::Theme;
use slint::platform::Key;
use slint::{ComponentHandle, Model};
use std::fs;

#[test]
fn the_sidebar_collapses_and_expands_and_remembers_it() {
    let ui = seeded();
    click(&ui, "Collapse the sidebar");
    assert!(gradebook(&ui).get_sidebar_collapsed());
    assert!(saved_preferences(&ui.fixture).sidebar_collapsed, "the choice is saved");
    assert!(shows(&ui, "Expand the sidebar") && !shows(&ui, "Collapse the sidebar"));
    click(&ui, "Ana, Exam");
    with_keys(&ui, false, "b");
    assert!(!gradebook(&ui).get_sidebar_collapsed(), "Ctrl+B expands it, even from a cell");
    assert!(!saved_preferences(&ui.fixture).sidebar_collapsed);
    with_keys(&ui, false, "b");
    assert!(gradebook(&ui).get_sidebar_collapsed(), "and folds it again");
}

#[test]
fn the_app_opens_with_the_sidebar_as_it_was_left() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_sidebar_collapsed(true).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    assert!(gradebook(&ui).get_sidebar_collapsed(), "a restart opens it folded");
    assert!(shows(&ui, "Expand the sidebar"));
}

#[test]
fn the_collapsed_plus_expands_and_focuses_the_new_class_field() {
    let ui = seeded();
    click(&ui, "Collapse the sidebar");
    click(&ui, "New class");
    assert!(!gradebook(&ui).get_sidebar_collapsed(), "the + opens the sidebar");
    type_text(&ui, "2B");
    press_enter(&ui);
    assert_eq!(gradebook(&ui).get_class_name(), "2B", "the typing went to the New class field");
}

#[test]
fn collapsed_entries_keep_full_accessible_labels() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    classes::create_class(state.connection().unwrap(), "1A").unwrap();
    state.set_pin(PIN, PIN).unwrap();
    state.shutdown();
    let ui = Ui::open(fixture);
    unlock(&ui);
    click(&ui, "Collapse the sidebar");
    for label in ["1A", "Lock (Ctrl+L)", "Dark mode", "Settings", "New class", "Expand the sidebar"] {
        assert!(shows(&ui, label), "{label} keeps its whole label folded");
    }
    let short: Vec<String> = gradebook(&ui).get_classes().iter().map(|class| class.short_name.to_string()).collect();
    assert_eq!(short, ["1A"]);
}

#[test]
fn a_failed_save_still_toggles_the_sidebar_and_shows_the_error() {
    let ui = seeded();
    let settings = ui.fixture.store().path().to_path_buf();
    let mut permissions = fs::metadata(&settings).unwrap().permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&settings, permissions.clone()).unwrap();

    click(&ui, "Collapse the sidebar");
    assert!(gradebook(&ui).get_sidebar_collapsed(), "the sidebar folds for this session");
    assert!(!ui.session.window.get_error_message().is_empty(), "the error is shown");
    assert!(!saved_preferences(&ui.fixture).sidebar_collapsed, "nothing was remembered");

    #[expect(
        clippy::permissions_set_readonly_false,
        reason = "undoes the test's own read-only flag on a scratch file; Windows has no world-writable bit"
    )]
    permissions.set_readonly(false);
    fs::set_permissions(&settings, permissions).unwrap();
}

#[test]
fn tab_reaches_the_folded_rail_and_enter_or_space_presses_it() {
    let ui = seeded();
    click(&ui, "Collapse the sidebar");
    let theme = ui.session.window.global::<Theme>();
    // the rail in order: expand, the class, New class, the theme, Settings
    for _ in 0..4 {
        key(&ui, Key::Tab);
    }
    press_enter(&ui);
    assert!(theme.get_dark(), "Tab went through the rail to the theme button");
    key(&ui, " ");
    assert!(!theme.get_dark(), "Space pressed it again");
    key(&ui, Key::Tab);
    press_enter(&ui);
    assert!(shows_text(&ui, "Appearance"), "the next Tab stop is Settings");
}

#[test]
fn tab_reaches_the_expand_button_and_enter_opens_the_sidebar() {
    let ui = seeded();
    click(&ui, "Collapse the sidebar");
    key(&ui, Key::Tab);
    press_enter(&ui);
    assert!(!gradebook(&ui).get_sidebar_collapsed(), "the first Tab stop is the expand button");
}
