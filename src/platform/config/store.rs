// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.008960
//+ Revised: 	2026/11/23 11:49:16.008960
// -----------------------------------------------------------------------------

//! Reading and writing `config.toml` safely: the save goes through a temporary file, and a settings
//! file that failed to load is kept as a copy before it is replaced.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use super::{AppConfig, CONFIG_FILE_NAME};
use crate::error::{AppError, Result, io_at};
use crate::platform::storage;

/// Reads and writes `config.toml` in a fixed folder (`%APPDATA%\Breren\avalio-lite` in normal use).
pub struct ConfigStore {
    path: PathBuf,
    /// The file on disk failed to load, so it may hold settings the user still wants (the
    /// database path, shortcuts): the first save keeps a copy of it instead of just replacing it.
    unreadable: AtomicBool,
}

impl ConfigStore {
    pub fn in_dir(dir: &Path) -> Self {
        Self { path: dir.join(CONFIG_FILE_NAME), unreadable: AtomicBool::new(false) }
    }

    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Returns `None` on a first run, when no settings file exists yet.
    pub fn load(&self) -> Result<Option<AppConfig>> {
        let loaded = match fs::read_to_string(&self.path) {
            Ok(text) => toml::from_str(&text)
                .map(Some)
                .map_err(|source| AppError::ConfigParse { path: self.path.clone(), source }),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(io_at(&self.path)(error)),
        };
        self.unreadable.store(loaded.is_err(), Ordering::Relaxed);
        loaded
    }

    /// Writes to a temporary file, flushed to the disk, then renames it over the settings file, so
    /// a crash or power cut leaves either the old file or the new one, never a truncated one. A
    /// failed save leaves no temporary file behind. A file that failed to load is copied aside
    /// (`config.toml.bad`) before it is replaced; if that copy can't be made, nothing is written.
    pub fn save(&self, config: &AppConfig) -> Result<()> {
        storage::ensure_parent(&self.path)?;
        if self.unreadable.load(Ordering::Relaxed) {
            self.keep_unreadable_copy()?;
        }
        let text = toml::to_string_pretty(config)?;
        let temporary = self.path.with_extension("toml.tmp");
        let written = write_flushed(&temporary, text.as_bytes())
            .map_err(io_at(&temporary))
            .and_then(|()| fs::rename(&temporary, &self.path).map_err(io_at(&self.path)));
        if written.is_err()
            && let Err(removal) = fs::remove_file(&temporary)
            && removal.kind() != std::io::ErrorKind::NotFound
        {
            log::error!("could not remove {}: {removal}", temporary.display());
        }
        written
    }

    /// Copies the settings file that failed to load to `config.toml.bad` (with a timestamp added
    /// when that name is taken), once; the file is absent when it was never there to copy.
    fn keep_unreadable_copy(&self) -> Result<()> {
        if self.path.is_file() {
            let mut copy = storage::with_suffix(&self.path, ".bad");
            if copy.exists() {
                copy = storage::with_suffix(
                    &self.path,
                    &format!(".bad-{}", chrono::Local::now().format("%Y%m%d-%H%M%S%3f")),
                );
            }
            fs::copy(&self.path, &copy).map_err(io_at(&copy))?;
            log::warn!("the unreadable settings file was kept as {}", copy.display());
        }
        self.unreadable.store(false, Ordering::Relaxed);
        Ok(())
    }
}

/// Writes `bytes` to a new `path` and waits until they are on the disk.
pub(super) fn write_flushed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let mut file = fs::File::create(path)?;
    std::io::Write::write_all(&mut file, bytes)?;
    file.sync_all()
}
