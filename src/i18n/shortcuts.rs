// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:54:46.754890
//+ Revised: 	2026/10/29 02:54:48.647043
// -----------------------------------------------------------------------------

use super::Language;
use crate::shortcuts::{Action, KeyCombo};

/// What the shortcut does, as Settings lists it.
pub fn action_label(language: Language, action: Action) -> &'static str {
    match language {
        Language::English => match action {
            Action::Lock => "Lock",
            Action::ToggleTheme => "Switch between light and dark",
            Action::ToggleColumnSettings => "Show or hide weights and scales",
            Action::OpenSettings => "Open Settings",
            Action::NextClass => "Next class",
            Action::PreviousClass => "Previous class",
        },
        Language::Galician => match action {
            Action::Lock => "Bloquear",
            Action::ToggleTheme => "Cambiar entre claro e escuro",
            Action::ToggleColumnSettings => "Amosar ou agochar pesos e escalas",
            Action::OpenSettings => "Abrir os axustes",
            Action::NextClass => "Clase seguinte",
            Action::PreviousClass => "Clase anterior",
        },
    }
}

/// One name per key, as on a keyboard: Shift is "Maiús" on Galician keyboards; Ctrl, Alt and the
/// keys themselves are the same in both languages.
pub fn key_names(language: Language, combo: &KeyCombo) -> Vec<String> {
    let shift = match language {
        Language::English => "Shift",
        Language::Galician => "Maiús",
    };
    [(combo.ctrl(), "Ctrl"), (combo.shift(), shift), (combo.alt(), "Alt")]
        .into_iter()
        .filter(|(held, _)| *held)
        .map(|(_, name)| name.to_owned())
        .chain([combo.key().name()])
        .collect()
}

/// The keys joined with "+", for labels such as "Lock (Ctrl+L)".
pub fn display(language: Language, combo: &KeyCombo) -> String {
    key_names(language, combo).join("+")
}
