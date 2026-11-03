// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:54:47.030126
//+ Revised: 	2026/11/03 09:38:29.865629
// -----------------------------------------------------------------------------

//! The shortcut rules, their text form in config.toml, and how config entries are applied.

use std::collections::BTreeMap;

use avalio_lite::shortcuts::ui::key_events;
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
    assert_eq!(shown, ["Ctrl+L", "Ctrl+Shift+D", "Ctrl+H", "Ctrl+B", "Ctrl+,", "Ctrl+PageDown", "Ctrl+PageUp"]);
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

#[test]
fn the_sidebar_shortcut_reads_from_config_and_defaults_to_ctrl_b() {
    assert_eq!(Action::from_config_name("toggle-sidebar"), Some(Action::ToggleSidebar));
    assert_eq!(Shortcuts::defaults().combo(Action::ToggleSidebar), combo("Ctrl+B"));
    let (shortcuts, problems) = Shortcuts::from_config(&overrides(&[("toggle-sidebar", "Ctrl+Shift+B")]));
    assert_eq!(shortcuts.combo(Action::ToggleSidebar), combo("Ctrl+Shift+B"));
    assert!(problems.is_empty());
}

/// Ctrl+B is a new default: a settings file that already gave it to another action reports the
/// clash and both go back to their defaults (regression for the new default meeting user keys).
#[test]
fn a_user_key_on_the_new_ctrl_b_default_is_reported() {
    let (shortcuts, problems) = Shortcuts::from_config(&overrides(&[("lock", "Ctrl+B")]));
    assert_eq!(shortcuts, Shortcuts::defaults());
    assert_eq!(problems.len(), 1);
    assert_eq!(problems[0].kind, ProblemKind::Clash);
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
    let pressed = |text: &str, ctrl, shift, alt| key_events::combo(text, ctrl, shift, alt);
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

#[test]
fn editing_keys_are_refused() {
    for letter in ['A', 'C', 'V', 'X', 'Y', 'Z'] {
        let text = format!("Ctrl+{letter}");
        assert_eq!(text.parse::<KeyCombo>(), Err(ShortcutError::EditingKey(letter)), "{text}");
    }
    assert!("Ctrl+Shift+Z".parse::<KeyCombo>().is_ok(), "only the plain editing keys are taken");
    assert!("Alt+C".parse::<KeyCombo>().is_ok());
    let (shortcuts, problems) = Shortcuts::from_config(&overrides(&[("lock", "ctrl+v")]));
    assert_eq!(shortcuts, Shortcuts::defaults(), "a hand-edited config can't set them either");
    assert_eq!(problems[0].kind, ProblemKind::BadKeys(ShortcutError::EditingKey('V')));
}

#[test]
fn a_change_that_clashes_names_the_other_action() {
    let result = Shortcuts::defaults().with_change(Action::ToggleTheme, combo("Ctrl+L"));
    assert_eq!(result, Err(ShortcutError::Clash { keys: combo("Ctrl+L"), taken_by: Action::Lock }));
    assert!(Shortcuts::defaults().with_change(Action::Lock, combo("Ctrl+L")).is_ok(), "its own keys are no clash");
}

#[test]
fn only_differences_are_saved() {
    let changed = Shortcuts::defaults().with_change(Action::Lock, combo("Ctrl+K")).unwrap();
    assert_eq!(changed.to_config(), overrides(&[("lock", "Ctrl+K")]));
    assert!(!changed.is_default(Action::Lock));
    let back = changed.with_change(Action::Lock, combo("Ctrl+L")).unwrap();
    assert!(back.to_config().is_empty());
    assert_eq!(Shortcuts::from_config(&changed.to_config()).0, changed, "what is saved reads back the same");
}

#[test]
fn reset_restores_the_default() {
    let changed = Shortcuts::defaults().with_change(Action::NextClass, combo("F8")).unwrap();
    let default = Shortcuts::defaults().combo(Action::NextClass);
    assert_eq!(changed.with_change(Action::NextClass, default).unwrap(), Shortcuts::defaults());
}

#[test]
fn recorded_presses_say_why_they_are_refused() {
    assert_eq!(key_events::checked_combo("k", false, false, false), Err(ShortcutError::NeedsCtrlOrAlt));
    assert_eq!(key_events::checked_combo("2", true, false, true), Err(ShortcutError::CtrlAlt));
    assert_eq!(key_events::checked_combo("c", true, false, false), Err(ShortcutError::EditingKey('C')));
    assert_eq!(key_events::checked_combo(" ", true, false, false), Err(ShortcutError::UnknownKey));
    assert_eq!(key_events::checked_combo("k", true, false, false), Ok(combo("Ctrl+K")));
}

/// The English texts the log gets for refused shortcuts and ignored config entries.
#[test]
fn refusals_and_ignored_entries_read_well_in_the_log() {
    let texts = [
        (ShortcutError::UnknownKey, "not a key a shortcut can use"),
        (ShortcutError::NeedsCtrlOrAlt, "needs Ctrl or Alt (only F1 to F12 can be used alone)"),
        (ShortcutError::CtrlAlt, "Ctrl and Alt together are what AltGr sends"),
        (ShortcutError::EditingKey('C'), "Ctrl+C is used for editing text"),
        (ShortcutError::Clash { keys: combo("Ctrl+H"), taken_by: Action::Lock }, "Ctrl+H is already used by lock"),
    ];
    for (error, text) in texts {
        assert_eq!(error.to_string(), text);
    }
    let (_, problems) =
        Shortcuts::from_config(&overrides(&[("lokc", "Ctrl+K"), ("lock", "Ctrl+"), ("next-class", "Ctrl+L")]));
    let logged: Vec<String> = problems.iter().map(ToString::to_string).collect();
    assert_eq!(
        logged,
        [
            "lock = \"Ctrl+\": not a key a shortcut can use",
            "lokc = \"Ctrl+K\": no such action",
            "next-class = \"Ctrl+L\": the same keys as another action",
        ]
    );
    let error = avalio_lite::error::AppError::InvalidShortcuts(problems);
    assert!(error.to_string().contains("lokc = \"Ctrl+K\": no such action; "), "{error}");
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

#[test]
fn every_allowed_key_round_trips_through_its_name() {
    for key in Key::all() {
        assert_eq!(Key::from_name(&key.name()), Some(key), "{key:?}");
        assert_eq!(Key::from_name(&key.name().to_lowercase()), Some(key), "{key:?} lower-cased");
    }
}

#[test]
fn slint_special_keys_map_to_their_keys() {
    assert_eq!(key_events::key(&char::from(SlintKey::F5).to_string()), Some(Key::Function(5)));
    assert_eq!(key_events::key(&char::from(SlintKey::Home).to_string()), Some(Key::Home));
    assert_eq!(key_events::key("q"), Some(Key::Letter('Q')));
    assert_eq!(key_events::key(" "), None);
}
