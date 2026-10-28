// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:16.441672
//+ Revised: 	2026/10/28 10:52:42.135924
// -----------------------------------------------------------------------------

use crate::i18n::{self, Language};
use crate::ui::AppWindow;

/// Switches the whole interface: Slint's bundled translations, the Rust-side texts and the picker.
/// Must run after the window exists, because Slint loads its translations with the first component.
pub fn apply(window: &AppWindow, language: Language) {
    i18n::set_current(language);
    if let Err(error) = slint::select_bundled_translation(language.code()) {
        log::error!("could not switch the interface to \"{}\": {error:?}", language.code());
    }
    window.set_language_code(language.code().into());
    log::info!("interface language: {}", language.code());
}
