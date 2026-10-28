// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.826999
//+ Revised: 	2026/10/28 10:52:45.713666
// -----------------------------------------------------------------------------

//! Interface language. Slint strings are translated from `lang/<code>/LC_MESSAGES/avalio-lite.po`;
//! text produced in Rust (errors, questions, default titles) is translated here, one `match`
//! arm per language so a missing translation is a compile error.

mod errors;
mod messages;

use std::sync::atomic::{AtomicU8, Ordering};

use serde::{Deserialize, Serialize};

pub use errors::error_message;
pub use messages::{Message, default_column_title, text};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "en")]
    English,
    #[serde(rename = "gl")]
    Galician,
}

pub const LANGUAGES: [Language; 2] = [Language::English, Language::Galician];

static CURRENT: AtomicU8 = AtomicU8::new(0);

impl Language {
    /// The locale code, which is also the folder name under `lang/`.
    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::Galician => "gl",
        }
    }

    pub fn from_code(code: &str) -> Option<Self> {
        LANGUAGES.into_iter().find(|language| language.code() == code)
    }

    /// The language's own name, as shown in the language picker.
    pub fn native_name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Galician => "Galego",
        }
    }

    /// Galician when Windows is set to Galician (`gl`, `gl-ES`), English otherwise.
    pub fn from_locale(locale: Option<&str>) -> Self {
        match locale {
            Some(locale) if locale.to_ascii_lowercase().starts_with("gl") => Self::Galician,
            _ => Self::English,
        }
    }

    pub fn from_system() -> Self {
        Self::from_locale(sys_locale::get_locale().as_deref())
    }

    pub fn decimal_separator(self) -> char {
        match self {
            Self::English => '.',
            Self::Galician => ',',
        }
    }

    fn index(self) -> u8 {
        match self {
            Self::English => 0,
            Self::Galician => 1,
        }
    }
}

/// The language the interface is currently shown in. A process-wide setting, like the logger.
pub fn current() -> Language {
    let index = CURRENT.load(Ordering::Relaxed);
    LANGUAGES.into_iter().find(|language| language.index() == index).unwrap_or_default()
}

pub fn set_current(language: Language) {
    CURRENT.store(language.index(), Ordering::Relaxed);
}

/// Swaps the decimal point of a number formatted by `compute` for the language's separator.
pub fn localize_number(language: Language, number: &str) -> String {
    number.replace('.', &language.decimal_separator().to_string())
}
