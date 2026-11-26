// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.062570
//+ Revised: 	2026/11/23 11:49:20.015481
// -----------------------------------------------------------------------------

//! `config.toml`: where the data lives and the view preferences. `preferences` holds the settings
//! themselves, `store` reads and writes the file, `place` finds the folder it lives in, and
//! `legacy` moves a settings file from the folder 0.10.0 used.

mod legacy;
mod place;
mod preferences;
mod store;

pub use legacy::move_legacy_settings;
pub use place::default_data_dir;
pub use preferences::{Preferences, ThemeChoice, VERSION_LIMIT, WindowPlace, checked_version};
pub use store::ConfigStore;

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const APP_DIR_NAME: &str = "avalio-lite";
/// The publisher, whose folder holds the app's own: `%APPDATA%\Breren\avalio-lite`.
pub const VENDOR_DIR_NAME: &str = "Breren";
pub const CONFIG_FILE_NAME: &str = "config.toml";
pub const DATABASE_FILE_NAME: &str = "avalio-lite.db";
pub const LOG_FILE_NAME: &str = "avalio-lite.log";
/// Moves the settings folder elsewhere, e.g. to run the app against a scratch copy. While it is
/// set, the first-run screen also suggests that folder for the data, so nothing lands in Documents.
pub const CONFIG_DIR_ENV: &str = "AVALIO_LITE_CONFIG_DIR";

/// Where the user chose to keep their data, plus their view preferences.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AppConfig {
    pub database_path: PathBuf,
    pub log_dir: PathBuf,
    /// Missing in settings files written before preferences existed; those get the defaults.
    #[serde(default)]
    pub preferences: Preferences,
    /// Keyboard shortcuts changed from their defaults, by action name (`lock = "Ctrl+K"`); checked
    /// by `shortcuts::Shortcuts::from_config`. Kept outside `Preferences` so that stays `Copy`.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub shortcuts: BTreeMap<String, String>,
    /// The class open when the app last showed this database, opened again at the next start.
    /// It belongs to `database_path`: choosing another database starts without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_class: Option<i64>,
}
