// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:30.755753
//+ Revised: 	2026/11/03 11:13:30.755753
// -----------------------------------------------------------------------------

//! A shortcut's keys (modifiers and one key) and the rules they must follow.

use std::fmt;
use std::str::FromStr;

use super::{Action, Key};

/// Why a shortcut isn't allowed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShortcutError {
    /// Not one of the allowed keys, or not written as modifiers followed by one key.
    UnknownKey,
    /// Only F1 to F12 may be used without Ctrl or Alt; plain letters would break typing in cells.
    NeedsCtrlOrAlt,
    /// AltGr sends Ctrl+Alt on Spanish and Galician keyboards (AltGr+2 types "@").
    CtrlAlt,
    /// Ctrl+A, C, V, X, Y or Z (the letter given): text fields need them, and the window catches
    /// shortcuts before the focused field sees them.
    EditingKey(char),
    /// Another action already has these keys; a change is refused rather than taking them over.
    Clash { keys: KeyCombo, taken_by: Action },
}

/// The letters that edit text with Ctrl: select all, copy, paste, cut, redo, undo.
const EDITING_LETTERS: [char; 6] = ['A', 'C', 'V', 'X', 'Y', 'Z'];

impl fmt::Display for ShortcutError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnknownKey => formatter.write_str("not a key a shortcut can use"),
            Self::NeedsCtrlOrAlt => formatter.write_str("needs Ctrl or Alt (only F1 to F12 can be used alone)"),
            Self::CtrlAlt => formatter.write_str("Ctrl and Alt together are what AltGr sends"),
            Self::EditingKey(letter) => write!(formatter, "Ctrl+{letter} is used for editing text"),
            Self::Clash { keys, taken_by } => write!(formatter, "{keys} is already used by {}", taken_by.config_name()),
        }
    }
}

/// Modifiers and one key, such as Ctrl+Shift+D.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct KeyCombo {
    pub(super) ctrl: bool,
    pub(super) shift: bool,
    pub(super) alt: bool,
    pub(super) key: Key,
}

impl KeyCombo {
    /// The one place the shortcut rules live: used by parsing and by key presses alike.
    pub fn new(ctrl: bool, shift: bool, alt: bool, key: Key) -> Result<Self, ShortcutError> {
        if ctrl && alt {
            return Err(ShortcutError::CtrlAlt);
        }
        if !ctrl && !alt && !matches!(key, Key::Function(_)) {
            return Err(ShortcutError::NeedsCtrlOrAlt);
        }
        if let Key::Letter(letter) = key
            && ctrl
            && !shift
            && EDITING_LETTERS.contains(&letter)
        {
            return Err(ShortcutError::EditingKey(letter));
        }
        Ok(Self { ctrl, shift, alt, key })
    }

    pub fn ctrl(&self) -> bool {
        self.ctrl
    }

    pub fn shift(&self) -> bool {
        self.shift
    }

    pub fn alt(&self) -> bool {
        self.alt
    }

    pub fn key(&self) -> Key {
        self.key
    }
}

/// The canonical English form used in `config.toml`: Ctrl, Shift, Alt in that order, then the key.
impl fmt::Display for KeyCombo {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (held, name) in [(self.ctrl, "Ctrl+"), (self.shift, "Shift+"), (self.alt, "Alt+")] {
            if held {
                formatter.write_str(name)?;
            }
        }
        formatter.write_str(&self.key.name())
    }
}

/// Modifiers in any order and any case, then the key: "ctrl+shift+d", "Shift+Ctrl+D".
impl FromStr for KeyCombo {
    type Err = ShortcutError;

    fn from_str(text: &str) -> Result<Self, Self::Err> {
        let mut parts: Vec<&str> = text.trim().split('+').map(str::trim).collect();
        let key = parts.pop().and_then(Key::from_name).ok_or(ShortcutError::UnknownKey)?;
        let (mut ctrl, mut shift, mut alt) = (false, false, false);
        for part in parts {
            let modifier = match part.to_ascii_lowercase().as_str() {
                "ctrl" => &mut ctrl,
                "shift" => &mut shift,
                "alt" => &mut alt,
                _ => return Err(ShortcutError::UnknownKey),
            };
            if *modifier {
                return Err(ShortcutError::UnknownKey);
            }
            *modifier = true;
        }
        Self::new(ctrl, shift, alt, key)
    }
}
