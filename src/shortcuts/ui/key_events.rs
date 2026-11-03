// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 09:38:29.281367
//+ Revised: 	2026/11/03 09:38:29.281367
// -----------------------------------------------------------------------------

//! Turns Slint key events into shortcut keys. The rules themselves (`crate::shortcuts`) know nothing
//! of Slint; only this file changes when Slint's key characters do.

use slint::platform::Key as SlintKey;

use crate::shortcuts::{Key, KeyCombo, ShortcutError};

/// The special keys a shortcut may use, as Slint sends them.
const NAMED_KEYS: [(Key, SlintKey); 4] = [
    (Key::PageUp, SlintKey::PageUp),
    (Key::PageDown, SlintKey::PageDown),
    (Key::Home, SlintKey::Home),
    (Key::End, SlintKey::End),
];

const FUNCTION_KEYS: [SlintKey; 12] = [
    SlintKey::F1,
    SlintKey::F2,
    SlintKey::F3,
    SlintKey::F4,
    SlintKey::F5,
    SlintKey::F6,
    SlintKey::F7,
    SlintKey::F8,
    SlintKey::F9,
    SlintKey::F10,
    SlintKey::F11,
    SlintKey::F12,
];

/// The key a Slint key event's text stands for. Letters arrive as typed ("l", or "L" with Shift);
/// the special keys arrive as Slint's own key characters.
pub fn key(text: &str) -> Option<Key> {
    let mut characters = text.chars();
    let (Some(only), None) = (characters.next(), characters.next()) else { return None };
    if let Some(index) = FUNCTION_KEYS.iter().position(|key| char::from(*key) == only) {
        return Some(Key::Function(index as u8 + 1));
    }
    if let Some((key, _)) = NAMED_KEYS.iter().find(|(_, slint_key)| char::from(*slint_key) == only) {
        return Some(*key);
    }
    Key::from_name(text)
}

/// The combo a key press makes, from Slint's event text and modifiers; `None` when it can't be a
/// shortcut (AltGr, a plain letter, a key no shortcut uses).
pub fn combo(text: &str, ctrl: bool, shift: bool, alt: bool) -> Option<KeyCombo> {
    checked_combo(text, ctrl, shift, alt).ok()
}

/// Like [`combo`], saying why the keys can't be a shortcut; for recording a new one.
pub fn checked_combo(text: &str, ctrl: bool, shift: bool, alt: bool) -> Result<KeyCombo, ShortcutError> {
    KeyCombo::new(ctrl, shift, alt, key(text).ok_or(ShortcutError::UnknownKey)?)
}
