// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 10:52:42.488112
//+ Revised: 	2026/11/23 16:34:35.212397
// -----------------------------------------------------------------------------

//! The light or dark look: the saved choice, or the one Windows uses. The colours are in `ui/theme.slint`.

use crate::platform::config::{Preferences, ThemeChoice};
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

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the yes/no value being saved or shown, not a switch between two behaviours"
)]
pub fn apply(window: &AppWindow, dark: bool) {
    window.invoke_apply_theme(dark);
    log::info!("theme: {}", if dark { "dark" } else { "light" });
}

#[expect(
    clippy::fn_params_excessive_bools,
    reason = "the yes/no value being saved or shown, not a switch between two behaviours"
)]
pub fn choice(dark: bool) -> ThemeChoice {
    if dark { ThemeChoice::Dark } else { ThemeChoice::Light }
}
