// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/01 21:11:28.443685
//+ Revised: 	2026/11/23 01:29:27.121456
// -----------------------------------------------------------------------------

//! Connects the window's shortcut handler to the configured keys, fills what Settings and the
//! labels show about them, and records new keys from Settings.

pub mod key_events;

use slint::{ComponentHandle, SharedString};

use crate::error::{AppError, Result};
use crate::i18n::{self, Message, ShortcutsMessage, shortcuts::action_label, shortcuts::display, shortcuts::key_names};
use crate::shortcuts::{Action, Modifiers};
use crate::ui::{self, AppWindow, ShortcutAction, ShortcutItem};
use crate::window::report_failure;
use crate::window::view::model;
use crate::window::{SharedState, view, with_window, with_window1, with_window5};

const NOT_RECORDING: i32 = -1;

/// Wires the shortcut callbacks: the handler's lookup, and the changes made from Settings.
pub fn wire(window: &AppWindow, state: &SharedState) {
    wire_find(window, state);
    wire_record(window, state);
    wire_reset(window, state);
    wire_reset_all(window, state);
}

/// Answers the window's key handler: the action bound to the keys pressed, if any.
fn wire_find(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<ui::Shortcuts>().on_find(move |text, ctrl, shift, alt| {
        key_events::combo(&text, Modifiers { ctrl, shift, alt })
            .and_then(|combo| shared.borrow().shortcuts().action_for(&combo))
            .map_or(ShortcutAction::None, to_ui)
    });
}

/// Records the keys typed on a row that is recording, as that row's action's new keys.
fn wire_record(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    let record = move |window: &AppWindow, row, text: SharedString, ctrl, shift, alt| {
        let Some(action) = action_at(row) else { return };
        let result = key_events::checked_combo(&text, Modifiers { ctrl, shift, alt })
            .map_err(AppError::Shortcut)
            .and_then(|combo| shared.borrow_mut().set_shortcut(action, combo));
        // a refused combination keeps the row recording, so the user can try again or press Esc
        after_change(window, &shared, ("change shortcut", result), |language, shortcuts| {
            changed_text(language, action, shortcuts)
        });
    };
    window.global::<ui::Shortcuts>().on_record(with_window5(window, record));
}

/// Gives the action at a row back its default keys.
fn wire_reset(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<ui::Shortcuts>().on_reset(with_window1(window, move |window, row| {
        let Some(action) = action_at(row) else { return };
        let result = shared.borrow_mut().reset_shortcut(action);
        after_change(window, &shared, ("reset shortcut", result), |language, shortcuts| {
            changed_text(language, action, shortcuts)
        });
    }));
}

/// Gives every action its default keys.
fn wire_reset_all(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<ui::Shortcuts>().on_reset_all(with_window(window, move |window| {
        let result = shared.borrow_mut().reset_all_shortcuts();
        after_change(window, &shared, ("reset all shortcuts", result), |language, _| {
            i18n::text(language, &Message::Shortcuts(ShortcutsMessage::ShortcutsReset))
        });
    }));
}

/// The status line after `action` changed: its name and the keys it has now.
fn changed_text(language: i18n::Language, action: Action, shortcuts: &crate::shortcuts::Shortcuts) -> String {
    let keys = display(language, &shortcuts.combo(action));
    let message = ShortcutsMessage::ShortcutChanged { action: action_label(language, action), keys: &keys };
    i18n::text(language, &Message::Shortcuts(message))
}

/// After a change: on success the recording ends, the card and labels show the new keys and the
/// status says what changed; on failure the error shows and a recording row keeps recording.
fn after_change(
    window: &AppWindow,
    state: &SharedState,
    (action, result): (&str, Result<()>),
    done: impl FnOnce(i18n::Language, &crate::shortcuts::Shortcuts) -> String,
) {
    match result {
        Ok(()) => {
            window.global::<ui::Shortcuts>().set_recording(NOT_RECORDING);
            show(window, state);
            let message = done(i18n::current(), state.borrow().shortcuts());
            view::show_status(window, message);
        }
        Err(error) => {
            report_failure!(window, action, &error);
        }
    }
}

fn action_at(row: i32) -> Option<Action> {
    let action = usize::try_from(row).ok().and_then(|row| Action::ALL.get(row).copied());
    if action.is_none() {
        log::error!("no keyboard shortcut at row {row}");
    }
    action
}

/// The Settings card's rows and the Lock keys in the labels, in the current language.
pub fn show(window: &AppWindow, state: &SharedState) {
    let language = i18n::current();
    let state = state.borrow();
    let shortcuts = state.shortcuts();
    let items: Vec<ShortcutItem> = Action::ALL
        .into_iter()
        .map(|action| {
            let combo = shortcuts.combo(action);
            ShortcutItem {
                label: action_label(language, action).into(),
                keys: model(key_names(language, &combo).into_iter().map(SharedString::from).collect()),
                spoken: display(language, &combo).into(),
                is_default: shortcuts.is_default(action),
            }
        })
        .collect();
    let global = window.global::<ui::Shortcuts>();
    global.set_items(model(items));
    global.set_all_default(Action::ALL.into_iter().all(|action| shortcuts.is_default(action)));
    global.set_lock_keys(display(language, &shortcuts.combo(Action::Lock)).into());
}

/// The ignored `[shortcuts]` entries, once, in the sheet's banner (where the main screen opens).
pub fn report_problems(window: &AppWindow, state: &SharedState) {
    let problems = state.borrow_mut().take_shortcut_problems();
    if !problems.is_empty() {
        view::show_error(window, &AppError::InvalidShortcuts(problems));
    }
}

fn to_ui(action: Action) -> ShortcutAction {
    match action {
        Action::Lock => ShortcutAction::Lock,
        Action::ToggleTheme => ShortcutAction::ToggleTheme,
        Action::ToggleColumnSettings => ShortcutAction::ToggleColumnSettings,
        Action::ToggleSidebar => ShortcutAction::ToggleSidebar,
        Action::OpenSettings => ShortcutAction::OpenSettings,
        Action::NextClass => ShortcutAction::NextClass,
        Action::PreviousClass => ShortcutAction::PreviousClass,
    }
}
