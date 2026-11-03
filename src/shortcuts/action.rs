// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:30.688332
//+ Revised: 	2026/11/03 11:13:30.688332
// -----------------------------------------------------------------------------

//! The actions a shortcut can run, with their names in `config.toml`.

use super::{Key, KeyCombo};

/// What a shortcut does, in the order Settings lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Lock,
    ToggleTheme,
    ToggleColumnSettings,
    ToggleSidebar,
    OpenSettings,
    NextClass,
    PreviousClass,
}

impl Action {
    pub const ALL: [Action; 7] = [
        Self::Lock,
        Self::ToggleTheme,
        Self::ToggleColumnSettings,
        Self::ToggleSidebar,
        Self::OpenSettings,
        Self::NextClass,
        Self::PreviousClass,
    ];

    /// The name used in the `[shortcuts]` table of `config.toml`.
    pub fn config_name(self) -> &'static str {
        match self {
            Self::Lock => "lock",
            Self::ToggleTheme => "toggle-theme",
            Self::ToggleColumnSettings => "toggle-column-settings",
            Self::ToggleSidebar => "toggle-sidebar",
            Self::OpenSettings => "open-settings",
            Self::NextClass => "next-class",
            Self::PreviousClass => "previous-class",
        }
    }

    pub fn from_config_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.config_name() == name)
    }

    pub(super) fn index(self) -> usize {
        Self::ALL.iter().position(|action| *action == self).unwrap_or_default()
    }

    /// Ctrl+Shift+D rather than Ctrl+D for the theme, so typing never hits it by accident; Ctrl+, is
    /// the usual Windows key for settings; Ctrl+PageDown/PageUp switch sheets in Excel; Ctrl+B folds
    /// the sidebar, as in VS Code and Edge.
    pub(super) fn default_combo(self) -> KeyCombo {
        let ctrl = |shift, key| KeyCombo { ctrl: true, shift, alt: false, key };
        match self {
            Self::Lock => ctrl(false, Key::Letter('L')),
            Self::ToggleTheme => ctrl(true, Key::Letter('D')),
            Self::ToggleColumnSettings => ctrl(false, Key::Letter('H')),
            Self::ToggleSidebar => ctrl(false, Key::Letter('B')),
            Self::OpenSettings => ctrl(false, Key::Comma),
            Self::NextClass => ctrl(false, Key::PageDown),
            Self::PreviousClass => ctrl(false, Key::PageUp),
        }
    }
}
