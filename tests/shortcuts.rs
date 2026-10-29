// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:54:47.030126
//+ Revised: 	2026/10/29 02:54:48.792409
// -----------------------------------------------------------------------------

//! The shortcut rules, their text form in config.toml, and how config entries are applied.

use std::collections::BTreeMap;

use avalio_lite::shortcuts::{Action, Key, KeyCombo, ProblemKind, ShortcutError, Shortcuts};
use slint::platform::Key as SlintKey;

fn combo(text: &str) -> KeyCombo {
    text.parse().unwrap_or_else(|error| panic!("{text}: {error}"))
}

fn overrides(entries: &[(&str, &str)]) -> BTreeMap<String, String> {
    entries.iter().map(|(name, keys)| ((*name).to_owned(), (*keys).to_owned())).collect()
}

#[test]
fn defaults_never_clash() {
    let defaults = Shortcuts::defaults();
    for (index, first) in Action::ALL.iter().enumerate() {
        for second in &Action::ALL[index + 1..] {
            assert_ne!(defaults.combo(*first), defaults.combo(*second), "{first:?} and {second:?}");
        }
    }
}

#[test]
fn the_defaults_are_the_documented_keys() {
    let shown: Vec<String> =
        Action::ALL.iter().map(|action| Shortcuts::defaults().combo(*action).to_string()).collect();
    assert_eq!(shown, ["Ctrl+L", "Ctrl+Shift+D", "Ctrl+H", "Ctrl+,", "Ctrl+PageDown", "Ctrl+PageUp"]);
}

#[test]
fn action_names_round_trip() {
    for action in Action::ALL {
        assert_eq!(Action::from_config_name(action.config_name()), Some(action));
    }
    assert_eq!(Action::from_config_name("Lock"), None, "names are exact");
}

#[test]
fn parsing_is_case_insensitive_and_canonical() {
    assert_eq!(combo("ctrl+shift+d").to_string(), "Ctrl+Shift+D");
    assert_eq!(combo("Shift+Ctrl+D"), combo("Ctrl+Shift+D"));
    assert_eq!(combo(" Ctrl + pagedown ").to_string(), "Ctrl+PageDown");
    assert_eq!(combo("alt+f4").to_string(), "Alt+F4");
}

#[test]
fn plain_letters_and_altgr_are_refused() {
    assert_eq!("L".parse::<KeyCombo>(), Err(ShortcutError::NeedsCtrlOrAlt));
    assert_eq!("Shift+L".parse::<KeyCombo>(), Err(ShortcutError::NeedsCtrlOrAlt));
    assert_eq!("Ctrl+Alt+2".parse::<KeyCombo>(), Err(ShortcutError::CtrlAlt));
    assert!("F5".parse::<KeyCombo>().is_ok());
    assert!("Shift+F5".parse::<KeyCombo>().is_ok());
    assert!("Alt+L".parse::<KeyCombo>().is_ok());
}

#[test]
fn unknown_keys_are_refused() {
    for text in ["Ctrl+", "Ctrl+Space", "Ctrl+F13", "Ctrl+F0", "", "Ctrl+Ctrl+L", "Win+L", "Ctrl+LL", "Ctrl+-"] {
        assert_eq!(text.parse::<KeyCombo>(), Err(ShortcutError::UnknownKey), "{text:?}");
    }
}

#[test]
fn config_overrides_replace_defaults() {
    let (shortcuts, problems) = Shortcuts::from_config(&overrides(&[("lock", "Ctrl+K")]));
    assert!(problems.is_empty(), "{problems:?}");
    assert_eq!(shortcuts.combo(Action::Lock), combo("Ctrl+K"));
    for action in &Action::ALL[1..] {
        assert_eq!(shortcuts.combo(*action), Shortcuts::defaults().combo(*action));
    }
}

#[test]
fn bad_entries_fall_back_and_are_reported() {
    let (shortcuts, problems) = Shortcuts::from_config(&overrides(&[("lokc", "Ctrl+K"), ("lock", "Ctrl+")]));
    assert_eq!(shortcuts, Shortcuts::defaults());
    let kinds: Vec<(&str, &ProblemKind)> =
        problems.iter().map(|problem| (problem.entry.as_str(), &problem.kind)).collect();
    assert_eq!(
        kinds,
        [
            ("lock = \"Ctrl+\"", &ProblemKind::BadKeys(ShortcutError::UnknownKey)),
            ("lokc = \"Ctrl+K\"", &ProblemKind::UnknownAction),
        ]
    );
}

#[test]
fn clashing_overrides_both_revert() {
    let (shortcuts, problems) = Shortcuts::from_config(&overrides(&[("toggle-theme", "Ctrl+L")]));
    assert_eq!(shortcuts, Shortcuts::defaults(), "the theme goes back to Ctrl+Shift+D; Lock keeps Ctrl+L");
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].kind, ProblemKind::Clash);

    let (shortcuts, problems) = Shortcuts::from_config(&overrides(&[("next-class", "F8"), ("previous-class", "f8")]));
    assert_eq!(shortcuts, Shortcuts::defaults());
    assert_eq!(problems.iter().filter(|problem| problem.kind == ProblemKind::Clash).count(), 2);
}

/// A clash whose revert lands on another override's keys makes that one revert too.
#[test]
fn reverting_a_clash_can_revert_another_override() {
    let (shortcuts, problems) = Shortcuts::from_config(&overrides(&[
        ("lock", "Ctrl+H"),
        ("toggle-column-settings", "Ctrl+L"),
        ("next-class", "Ctrl+H"),
    ]));
    assert_eq!(shortcuts, Shortcuts::defaults());
    assert_eq!(problems.len(), 3);
}

#[test]
fn key_presses_map_to_actions() {
    let defaults = Shortcuts::defaults();
    let pressed = |text: &str, ctrl, shift, alt| KeyCombo::from_key_press(text, ctrl, shift, alt);
    assert_eq!(pressed("l", true, false, false), Some(combo("Ctrl+L")));
    assert_eq!(pressed("L", true, true, false), Some(combo("Ctrl+Shift+L")));
    assert_eq!(pressed("D", true, true, false).and_then(|keys| defaults.action_for(&keys)), Some(Action::ToggleTheme));
    let page_down = char::from(SlintKey::PageDown).to_string();
    assert_eq!(
        pressed(&page_down, true, false, false).and_then(|keys| defaults.action_for(&keys)),
        Some(Action::NextClass)
    );
    assert_eq!(pressed(&char::from(SlintKey::F5).to_string(), false, false, false), Some(combo("F5")));
    assert_eq!(
        pressed(",", true, false, false).and_then(|keys| defaults.action_for(&keys)),
        Some(Action::OpenSettings)
    );
    assert_eq!(pressed("2", true, false, true), None, "AltGr+2 types @, it is no shortcut");
    assert_eq!(pressed("l", false, false, false), None, "a plain letter is typing");
    assert_eq!(pressed("ñ", true, false, false), None);
}

fn any_combo() -> impl proptest::strategy::Strategy<Value = KeyCombo> {
    use proptest::prelude::*;
    (proptest::sample::select(Key::all()), any::<bool>(), any::<bool>(), any::<bool>())
        .prop_filter_map("not an allowed shortcut", |(key, ctrl, shift, alt)| KeyCombo::new(ctrl, shift, alt, key).ok())
}

proptest::proptest! {
    #[test]
    fn combos_round_trip_through_text(combo in any_combo()) {
        proptest::prop_assert_eq!(combo.to_string().parse::<KeyCombo>(), Ok(combo));
    }
}
