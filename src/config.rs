// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.062570
//+ Revised: 	2026/10/28 15:53:08.128434
// -----------------------------------------------------------------------------

use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{AppError, Result, io_at};
use crate::i18n::Language;

pub const APP_DIR_NAME: &str = "avalio-lite";
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ThemeChoice {
    Light,
    Dark,
}

impl Default for Preferences {
    fn default() -> Self {
        Self { show_column_settings: shown(), language: None, theme: None }
    }
}

fn shown() -> bool {
    true
}

/// Reads and writes `config.toml` in a fixed folder (`%APPDATA%\avalio-lite` in normal use).
pub struct ConfigStore {
    path: PathBuf,
}

impl ConfigStore {
    pub fn in_dir(dir: &Path) -> Self {
        Self { path: dir.join(CONFIG_FILE_NAME) }
    }

    pub fn default_location() -> Result<Self> {
        Self::location(config_dir_override(), dirs::config_dir())
    }

    /// `override_dir` when given; otherwise the app's folder inside the system `config_dir`.
    pub fn location(override_dir: Option<PathBuf>, config_dir: Option<PathBuf>) -> Result<Self> {
        if let Some(dir) = override_dir {
            return Ok(Self::in_dir(&dir));
        }
        let base = config_dir.ok_or(AppError::NoConfigDir)?;
        Ok(Self::in_dir(&base.join(APP_DIR_NAME)))
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns `None` on a first run, when no settings file exists yet.
    pub fn load(&self) -> Result<Option<AppConfig>> {
        let text = match fs::read_to_string(&self.path) {
            Ok(text) => text,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(io_at(&self.path)(error)),
        };
        toml::from_str(&text).map(Some).map_err(|source| AppError::ConfigParse { path: self.path.clone(), source })
    }

    /// Writes to a temporary file first so a crash mid-write never leaves a truncated settings file.
    pub fn save(&self, config: &AppConfig) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            fs::create_dir_all(dir).map_err(io_at(dir))?;
        }
        let temporary = self.path.with_extension("toml.tmp");
        fs::write(&temporary, toml::to_string_pretty(config)?).map_err(io_at(&temporary))?;
        fs::rename(&temporary, &self.path).map_err(io_at(&self.path))
    }
}

/// Folder suggested on the first-run screen.
pub fn default_data_dir() -> PathBuf {
    config_dir_override()
        .unwrap_or_else(|| dirs::document_dir().or_else(dirs::home_dir).unwrap_or_default().join(APP_DIR_NAME))
}

/// The `CONFIG_DIR_ENV` folder, when set to something non-empty.
fn config_dir_override() -> Option<PathBuf> {
    std::env::var_os(CONFIG_DIR_ENV).filter(|dir| !dir.is_empty()).map(PathBuf::from)
}
