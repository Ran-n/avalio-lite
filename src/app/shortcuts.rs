// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:54:46.881854
//+ Revised: 	2026/10/29 02:54:48.576386
// -----------------------------------------------------------------------------

//! Connects the window's shortcut handler to the configured keys, and fills what Settings and the
//! labels show about them.

use slint::{ComponentHandle, SharedString};

use crate::app::SharedState;
use crate::app::gradebook::view::model;
use crate::error::AppError;
use crate::i18n::{self, shortcuts::action_label, shortcuts::display, shortcuts::key_names};
use crate::shortcuts::{Action, KeyCombo};
use crate::ui::{self, AppWindow, Gradebook, ShortcutAction, ShortcutItem};

pub fn wire(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<ui::Shortcuts>().on_find(move |text, ctrl, shift, alt| {
        KeyCombo::from_key_press(&text, ctrl, shift, alt)
            .and_then(|combo| shared.borrow().shortcuts().action_for(&combo))
            .map_or(ShortcutAction::None, to_ui)
    });
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
            }
        })
        .collect();
    let global = window.global::<ui::Shortcuts>();
    global.set_items(model(items));
    global.set_lock_keys(display(language, &shortcuts.combo(Action::Lock)).into());
}

/// The ignored `[shortcuts]` entries, once, in the sheet's banner (where the main screen opens).
pub fn report_problems(window: &AppWindow, state: &SharedState) {
    let problems = state.borrow_mut().take_shortcut_problems();
    if !problems.is_empty() {
        let message = i18n::error_message(i18n::current(), &AppError::InvalidShortcuts(problems));
        window.global::<Gradebook>().set_error_message(message.into());
    }
}

fn to_ui(action: Action) -> ShortcutAction {
    match action {
        Action::Lock => ShortcutAction::Lock,
        Action::ToggleTheme => ShortcutAction::ToggleTheme,
        Action::ToggleColumnSettings => ShortcutAction::ToggleColumnSettings,
        Action::OpenSettings => ShortcutAction::OpenSettings,
        Action::NextClass => ShortcutAction::NextClass,
        Action::PreviousClass => ShortcutAction::PreviousClass,
    }
}
