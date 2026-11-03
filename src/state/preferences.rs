// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 22:29:57.630220
//+ Revised: 	2026/11/03 10:00:17.034705
// -----------------------------------------------------------------------------

//! The view preferences and keyboard shortcuts kept in the settings file.

use std::collections::BTreeMap;

use super::AppState;
use crate::error::{AppError, Result};
use crate::i18n::Language;
use crate::platform::config::{AppConfig, Preferences, ThemeChoice};
use crate::shortcuts::{Action, KeyCombo, ShortcutProblem, Shortcuts};

impl AppState {
    pub fn shortcuts(&self) -> &Shortcuts {
        &self.shortcuts
    }

    /// Puts `action` on `combo`; refused (`Shortcut(Clash)`) when another action has those keys.
    pub fn set_shortcut(&mut self, action: Action, combo: KeyCombo) -> Result<()> {
        let changed = self.shortcuts.with_change(action, combo).map_err(AppError::Shortcut)?;
        self.save_shortcuts(changed)
    }

    /// Puts `action` back on its default keys, unless another action has them now.
    pub fn reset_shortcut(&mut self, action: Action) -> Result<()> {
        self.set_shortcut(action, Shortcuts::defaults().combo(action))
    }

    pub fn reset_all_shortcuts(&mut self) -> Result<()> {
        self.save_shortcuts(Shortcuts::defaults())
    }

    /// Writes only the differences from the defaults, and takes the new shortcuts only once they are
    /// saved, so a failed save changes nothing.
    fn save_shortcuts(&mut self, shortcuts: Shortcuts) -> Result<()> {
        // the new shortcuts take effect only once they are saved
        self.update_config(|config| config.shortcuts = shortcuts.to_config())?;
        self.shortcuts = shortcuts;
        Ok(())
    }

    /// The ignored shortcut entries, once: the list is empty afterwards, so the banner shows once.
    pub fn take_shortcut_problems(&mut self) -> Vec<ShortcutProblem> {
        std::mem::take(&mut self.shortcut_problems)
    }

    /// The current view preferences: those of the chosen database's settings, else those last read
    /// from the settings file (e.g. when its database went missing), else the defaults.
    pub fn preferences(&self) -> Preferences {
        self.settings().map(|config| config.preferences).unwrap_or_default()
    }

    /// The `[shortcuts]` entries to keep when the settings are written for a newly chosen database.
    pub(super) fn shortcut_overrides(&self) -> BTreeMap<String, String> {
        self.settings().map(|config| config.shortcuts.clone()).unwrap_or_default()
    }

    pub(super) fn settings(&self) -> Option<&AppConfig> {
        self.config.as_ref().or(self.read.as_ref())
    }

    /// Takes the shortcuts from `config`, logging and keeping (for the banner) each ignored entry.
    pub(super) fn adopt_shortcuts(&mut self, config: &AppConfig) {
        let (shortcuts, problems) = Shortcuts::from_config(&config.shortcuts);
        for problem in &problems {
            log::warn!("ignored keyboard shortcut in the settings file, using the default: {problem}");
        }
        self.shortcuts = shortcuts;
        self.shortcut_problems = problems;
    }

    pub fn set_show_column_settings(&mut self, show: bool) -> Result<()> {
        self.update_preferences(|preferences| preferences.show_column_settings = show)
    }

    pub fn set_theme(&mut self, theme: ThemeChoice) -> Result<()> {
        self.update_preferences(|preferences| preferences.theme = Some(theme))
    }

    pub fn set_language(&mut self, language: Language) -> Result<()> {
        self.update_preferences(|preferences| preferences.language = Some(language))
    }

    pub fn set_sidebar_collapsed(&mut self, collapsed: bool) -> Result<()> {
        self.update_preferences(|preferences| preferences.sidebar_collapsed = collapsed)
    }

    /// Applies `change` to the view preferences and saves them; refused while no database is open.
    pub(super) fn update_preferences(&mut self, change: impl FnOnce(&mut Preferences)) -> Result<()> {
        self.update_config(|config| change(&mut config.preferences))
    }
}
