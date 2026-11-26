// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.079217
//+ Revised: 	2026/11/23 11:49:16.079217
// -----------------------------------------------------------------------------

//! Where the settings and the data are: the settings folder (`%APPDATA%\Breren\avalio-lite`, or the
//! `CONFIG_DIR_ENV` folder), and the folder the first-run screen suggests for the data.

use std::path::PathBuf;

use super::legacy::move_legacy_settings;
use super::store::ConfigStore;
use super::{APP_DIR_NAME, CONFIG_DIR_ENV, VENDOR_DIR_NAME};
use crate::error::{AppError, Result};

impl ConfigStore {
    pub fn default_location() -> Result<Self> {
        Self::location(config_dir_override(), dirs::config_dir())
    }

    /// `override_dir` when given; otherwise `<config_dir>\Breren\avalio-lite`, the publisher's folder,
    /// where a settings file from 0.10.0 and earlier (`<config_dir>\avalio-lite`) is moved first.
    pub fn location(override_dir: Option<PathBuf>, config_dir: Option<PathBuf>) -> Result<Self> {
        if let Some(dir) = override_dir {
            return Ok(Self::in_dir(&dir));
        }
        let base = config_dir.ok_or(AppError::NoConfigDir)?;
        let dir = base.join(VENDOR_DIR_NAME).join(APP_DIR_NAME);
        Ok(Self::in_dir(&move_legacy_settings(&base.join(APP_DIR_NAME), &dir)))
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
