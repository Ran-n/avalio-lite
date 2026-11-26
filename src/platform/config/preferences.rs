// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:15.933016
//+ Revised: 	2026/11/23 11:49:15.933016
// -----------------------------------------------------------------------------

//! The view preferences saved in `config.toml`: what the window and the updater remember between
//! runs, and the rules that read older settings files (renamed keys, dates written as text, a
//! skipped version that isn't one).

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::i18n::Language;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Preferences {
    /// Whether column headers show weights, scales and rounding settings.
    #[serde(default = "shown")]
    pub show_column_settings: bool,
    /// The interface language the user picked; `None` follows the Windows language.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub language: Option<Language>,
    /// Light or dark; None follows the Windows setting.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub theme: Option<ThemeChoice>,
    /// Whether the sidebar is folded to its icon rail; only written when it is.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sidebar_collapsed: bool,
    /// Whether the app looks for new versions by itself (Settings › Updates). Written as
    /// `check_updates` before 0.11.0.
    #[serde(default = "shown", alias = "check_updates")]
    pub check_automatically: bool,
    /// When the last successful check ended, in seconds since 1970, so a restart doesn't check
    /// again within a day. Before 0.11.0 it was `last_update_check`, a date and time.
    #[serde(
        default,
        alias = "last_update_check",
        deserialize_with = "seconds_or_time",
        skip_serializing_if = "Option::is_none"
    )]
    pub last_check: Option<i64>,
    /// The version the user skipped; it and older ones aren't offered. Only a version is kept (at
    /// most 64 characters); anything else reads as none. `skipped_update` before 0.11.0.
    #[serde(
        default,
        alias = "skipped_update",
        deserialize_with = "valid_version",
        skip_serializing_if = "Option::is_none"
    )]
    pub skipped_version: Option<String>,
    /// The version that ran last, so a new one can say "Updated to …" once.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_run_version: Option<String>,
    /// Why an update couldn't be started as the app closed; shown, then cleared, at the next start.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub update_failed: Option<String>,
    /// Where the window was when the app last closed; it opens there again. Last, because TOML
    /// writes it as a table of its own.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub window: Option<WindowPlace>,
}

/// The window's place and size as the app closed (`app::placement`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WindowPlace {
    /// The outer top-left corner, in physical pixels on the whole desktop.
    pub x: i32,
    pub y: i32,
    /// The content size, in logical pixels: the window keeps its size on a screen of another scale.
    pub width: u32,
    pub height: u32,
    /// Maximised; the place and size are then the ones it returns to when restored.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub maximized: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    Light,
    Dark,
}

impl Default for Preferences {
    fn default() -> Self {
        Self {
            show_column_settings: shown(),
            language: None,
            theme: None,
            sidebar_collapsed: false,
            check_automatically: shown(),
            last_check: None,
            skipped_version: None,
            last_run_version: None,
            update_failed: None,
            window: None,
        }
    }
}

fn shown() -> bool {
    true
}

/// The longest skipped version kept.
pub const VERSION_LIMIT: usize = 64;

/// `text` if it is a version of at most `VERSION_LIMIT` characters.
pub fn checked_version(text: &str) -> Option<String> {
    (text.len() <= VERSION_LIMIT && semver::Version::parse(text).is_ok()).then(|| text.to_owned())
}

fn valid_version<'de, D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Option<String>, D::Error> {
    let text = Option::<String>::deserialize(deserializer)?;
    Ok(text.and_then(|text| {
        let checked = checked_version(&text);
        if checked.is_none() {
            log::warn!("the skipped version {text:?} in the settings isn't a version; ignored");
        }
        checked
    }))
}

/// Seconds since 1970, or the date and time 0.10.0 and earlier wrote.
fn seconds_or_time<'de, D: serde::Deserializer<'de>>(deserializer: D) -> std::result::Result<Option<i64>, D::Error> {
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum Written {
        Seconds(i64),
        Time(DateTime<Utc>),
    }
    Ok(Option::<Written>::deserialize(deserializer)?.map(|written| match written {
        Written::Seconds(seconds) => seconds,
        Written::Time(time) => time.timestamp(),
    }))
}
