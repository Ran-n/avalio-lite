// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:42.488112
//+ Revised: 	2026/10/28 10:52:42.488112
// -----------------------------------------------------------------------------

use crate::config::{Preferences, ThemeChoice};
use crate::ui::AppWindow;

/// The saved choice, or the Windows setting when the user hasn't picked one.
pub fn apply_preference(window: &AppWindow, preferences: &Preferences) {
    let dark = match preferences.theme {
        Some(ThemeChoice::Dark) => true,
        Some(ThemeChoice::Light) => false,
        None => window.invoke_system_prefers_dark(),
    };
    apply(window, dark);
}

pub fn apply(window: &AppWindow, dark: bool) {
    window.invoke_apply_theme(dark);
    log::info!("theme: {}", if dark { "dark" } else { "light" });
}

pub fn choice(dark: bool) -> ThemeChoice {
    if dark { ThemeChoice::Dark } else { ThemeChoice::Light }
}
