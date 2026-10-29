// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:54:46.632108
//+ Revised: 	2026/10/29 03:07:05.820040
// -----------------------------------------------------------------------------

//! Keyboard shortcuts: which keys run which action, their text form in `config.toml`, and the
//! rules a shortcut must follow. No UI or file access, so the rules can be tested on their own.

use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

use slint::platform::Key as SlintKey;

/// What a shortcut does, in the order Settings lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Action {
    Lock,
    ToggleTheme,
    ToggleColumnSettings,
    OpenSettings,
    NextClass,
    PreviousClass,
}

impl Action {
    pub const ALL: [Action; 6] = [
        Self::Lock,
        Self::ToggleTheme,
        Self::ToggleColumnSettings,
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
            Self::OpenSettings => "open-settings",
            Self::NextClass => "next-class",
            Self::PreviousClass => "previous-class",
        }
    }

    pub fn from_config_name(name: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|action| action.config_name() == name)
    }

    fn index(self) -> usize {
        Self::ALL.iter().position(|action| *action == self).unwrap_or_default()
    }

    /// Ctrl+Shift+D rather than Ctrl+D for the theme, so typing never hits it by accident; Ctrl+, is
    /// the usual Windows key for settings; Ctrl+PageDown/PageUp switch sheets in Excel.
    fn default_combo(self) -> KeyCombo {
        let ctrl = |shift, key| KeyCombo { ctrl: true, shift, alt: false, key };
        match self {
            Self::Lock => ctrl(false, Key::Letter('L')),
            Self::ToggleTheme => ctrl(true, Key::Letter('D')),
            Self::ToggleColumnSettings => ctrl(false, Key::Letter('H')),
            Self::OpenSettings => ctrl(false, Key::Comma),
            Self::NextClass => ctrl(false, Key::PageDown),
            Self::PreviousClass => ctrl(false, Key::PageUp),
        }
    }
}

/// The keys a shortcut may use.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Key {
    /// A to Z, always upper case.
    Letter(char),
    /// 0 to 9.
    Digit(u8),
    Comma,
    Period,
    /// F1 to F12.
    Function(u8),
    PageUp,
    PageDown,
    Home,
    End,
}

const NAMED_KEYS: [(Key, &str, SlintKey); 4] = [
    (Key::PageUp, "PageUp", SlintKey::PageUp),
    (Key::PageDown, "PageDown", SlintKey::PageDown),
    (Key::Home, "Home", SlintKey::Home),
    (Key::End, "End", SlintKey::End),
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

impl Key {
    /// Every allowed key, for listing and testing.
    pub fn all() -> Vec<Key> {
        let letters = ('A'..='Z').map(Key::Letter);
        let digits = (0..=9).map(Key::Digit);
        let functions = (1..=12).map(Key::Function);
        let named = NAMED_KEYS.iter().map(|(key, _, _)| *key);
        letters.chain(digits).chain([Key::Comma, Key::Period]).chain(functions).chain(named).collect()
    }

    /// The key's name, the same in every language and in `config.toml`.
    pub fn name(self) -> String {
        match self {
            Self::Letter(letter) => letter.to_string(),
            Self::Digit(digit) => digit.to_string(),
            Self::Comma => ",".to_owned(),
            Self::Period => ".".to_owned(),
            Self::Function(number) => format!("F{number}"),
            named => NAMED_KEYS
                .iter()
                .find(|(key, _, _)| *key == named)
                .map(|(_, name, _)| (*name).to_owned())
                .unwrap_or_default(),
        }
    }

    /// Reads a key name, ignoring case.
    fn parse(text: &str) -> Option<Key> {
        let mut characters = text.chars();
        if let (Some(only), None) = (characters.next(), characters.next()) {
            return match only {
                'a'..='z' | 'A'..='Z' => Some(Key::Letter(only.to_ascii_uppercase())),
                '0'..='9' => only.to_digit(10).map(|digit| Key::Digit(digit as u8)),
                ',' => Some(Key::Comma),
                '.' => Some(Key::Period),
                _ => None,
            };
        }
        if let Some(number) = text.strip_prefix(['F', 'f']).and_then(|rest| rest.parse::<u8>().ok()) {
            return (1..=12).contains(&number).then_some(Key::Function(number));
        }
        NAMED_KEYS.iter().find(|(_, name, _)| name.eq_ignore_ascii_case(text)).map(|(key, _, _)| *key)
    }

    /// The key a Slint key event's text stands for. Letters arrive as typed ("l", or "L" with Shift);
    /// the special keys arrive as Slint's own key characters.
    fn from_event_text(text: &str) -> Option<Key> {
        let mut characters = text.chars();
        let (Some(only), None) = (characters.next(), characters.next()) else { return None };
        if let Some(index) = FUNCTION_KEYS.iter().position(|key| char::from(*key) == only) {
            return Some(Key::Function(index as u8 + 1));
        }
        if let Some((key, _, _)) = NAMED_KEYS.iter().find(|(_, _, slint_key)| char::from(*slint_key) == only) {
            return Some(*key);
        }
        Key::parse(text)
    }
}

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
    ctrl: bool,
    shift: bool,
    alt: bool,
    key: Key,
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

    /// The combo a key press makes, from Slint's event text and modifiers; `None` when it can't be
    /// a shortcut (AltGr, a plain letter, a key no shortcut uses).
    pub fn from_key_press(text: &str, ctrl: bool, shift: bool, alt: bool) -> Option<Self> {
        Self::checked_key_press(text, ctrl, shift, alt).ok()
    }

    /// Like `from_key_press`, saying why the keys can't be a shortcut; for recording a new one.
    pub fn checked_key_press(text: &str, ctrl: bool, shift: bool, alt: bool) -> Result<Self, ShortcutError> {
        Self::new(ctrl, shift, alt, Key::from_event_text(text).ok_or(ShortcutError::UnknownKey)?)
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
        let key = parts.pop().and_then(Key::parse).ok_or(ShortcutError::UnknownKey)?;
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

/// What was wrong with a `[shortcuts]` entry.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProblemKind {
    UnknownAction,
    BadKeys(ShortcutError),
    /// Another action uses the same keys.
    Clash,
}

/// A `[shortcuts]` entry that was ignored, with the reason.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortcutProblem {
    /// The entry as written: `lock = "Ctrl+"`.
    pub entry: String,
    pub kind: ProblemKind,
}

impl fmt::Display for ShortcutProblem {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match &self.kind {
            ProblemKind::UnknownAction => write!(formatter, "{}: no such action", self.entry),
            ProblemKind::BadKeys(error) => write!(formatter, "{}: {error}", self.entry),
            ProblemKind::Clash => write!(formatter, "{}: the same keys as another action", self.entry),
        }
    }
}

/// The keys of every action.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shortcuts {
    combos: [KeyCombo; Action::ALL.len()],
}

impl Default for Shortcuts {
    fn default() -> Self {
        Self::defaults()
    }
}

impl Shortcuts {
    pub fn defaults() -> Self {
        Self { combos: Action::ALL.map(Action::default_combo) }
    }

    /// The defaults with the `[shortcuts]` overrides applied. Entries that name no action or don't
    /// parse are reported and ignored. Actions left on the same keys all go back to their defaults,
    /// repeatedly, until none share keys; that ends because the defaults never clash.
    pub fn from_config(overrides: &BTreeMap<String, String>) -> (Self, Vec<ShortcutProblem>) {
        let mut shortcuts = Self::defaults();
        let mut problems = Vec::new();
        let mut entries: BTreeMap<usize, String> = BTreeMap::new();
        for (name, keys) in overrides {
            let entry = format!("{name} = {keys:?}");
            let Some(action) = Action::from_config_name(name) else {
                problems.push(ShortcutProblem { entry, kind: ProblemKind::UnknownAction });
                continue;
            };
            match keys.parse::<KeyCombo>() {
                Ok(combo) => {
                    shortcuts.combos[action.index()] = combo;
                    entries.insert(action.index(), entry);
                }
                Err(error) => problems.push(ShortcutProblem { entry, kind: ProblemKind::BadKeys(error) }),
            }
        }
        loop {
            let clashing: Vec<usize> = entries
                .keys()
                .copied()
                .filter(|&index| {
                    shortcuts
                        .combos
                        .iter()
                        .enumerate()
                        .any(|(other, combo)| other != index && *combo == shortcuts.combos[index])
                })
                .collect();
            if clashing.is_empty() {
                break;
            }
            for index in clashing {
                shortcuts.combos[index] = Action::ALL[index].default_combo();
                if let Some(entry) = entries.remove(&index) {
                    problems.push(ShortcutProblem { entry, kind: ProblemKind::Clash });
                }
            }
        }
        (shortcuts, problems)
    }

    pub fn combo(&self, action: Action) -> KeyCombo {
        self.combos[action.index()]
    }

    pub fn action_for(&self, combo: &KeyCombo) -> Option<Action> {
        Action::ALL.into_iter().find(|action| self.combo(*action) == *combo)
    }

    /// These shortcuts with `action` on `combo`, or `Clash` when another action has those keys.
    pub fn with_change(&self, action: Action, combo: KeyCombo) -> Result<Shortcuts, ShortcutError> {
        if let Some(other) = self.action_for(&combo).filter(|other| *other != action) {
            return Err(ShortcutError::Clash { keys: combo, taken_by: other });
        }
        let mut changed = self.clone();
        changed.combos[action.index()] = combo;
        Ok(changed)
    }

    pub fn is_default(&self, action: Action) -> bool {
        self.combo(action) == action.default_combo()
    }

    /// The `[shortcuts]` table for `config.toml`: only the actions that differ from their defaults.
    pub fn to_config(&self) -> BTreeMap<String, String> {
        Action::ALL
            .into_iter()
            .filter(|action| !self.is_default(*action))
            .map(|action| (action.config_name().to_owned(), self.combo(action).to_string()))
            .collect()
    }
}
