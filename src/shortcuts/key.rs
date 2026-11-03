// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:30.819127
//+ Revised: 	2026/11/03 11:13:30.819127
// -----------------------------------------------------------------------------

//! The keys a shortcut may use, and their names.

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

const NAMED_KEYS: [(Key, &str); 4] =
    [(Key::PageUp, "PageUp"), (Key::PageDown, "PageDown"), (Key::Home, "Home"), (Key::End, "End")];

impl Key {
    /// Every allowed key, for listing and testing.
    pub fn all() -> Vec<Key> {
        let letters = ('A'..='Z').map(Key::Letter);
        let digits = (0..=9).map(Key::Digit);
        let functions = (1..=12).map(Key::Function);
        let named = NAMED_KEYS.iter().map(|(key, _)| *key);
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
            named => {
                NAMED_KEYS.iter().find(|(key, _)| *key == named).map(|(_, name)| (*name).to_owned()).unwrap_or_default()
            }
        }
    }

    /// Reads a key name as `config.toml` writes it, ignoring case.
    pub fn from_name(text: &str) -> Option<Key> {
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
        NAMED_KEYS.iter().find(|(_, name)| name.eq_ignore_ascii_case(text)).map(|(key, _)| *key)
    }
}
