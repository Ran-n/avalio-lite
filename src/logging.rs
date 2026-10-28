// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.523539
//+ Revised: 	2026/10/28 15:53:08.261084
// -----------------------------------------------------------------------------

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::Path;
use std::str::FromStr;
use std::sync::{Mutex, OnceLock};

use log::{LevelFilter, Log, Metadata, Record};

use crate::config::LOG_FILE_NAME;
use crate::error::{Result, io_at};

pub const LOG_LEVEL_ENV: &str = "LOG_LEVEL";
const DEFAULT_LEVEL: LevelFilter = LevelFilter::Info;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const APP_TARGET_PREFIX: &str = "avalio_lite";

static LOGGER: OnceLock<FileLogger> = OnceLock::new();

/// Logger whose output file can be swapped at runtime, because the user picks
/// (and can later move) the log folder after the app has started.
/// Until a folder is set, records go to stderr.
pub struct FileLogger {
    level: LevelFilter,
    file: Mutex<Option<File>>,
}

impl FileLogger {
    pub fn new(level: LevelFilter) -> Self {
        Self { level, file: Mutex::new(None) }
    }

    pub fn open(&self, log_dir: &Path) -> Result<()> {
        fs::create_dir_all(log_dir).map_err(io_at(log_dir))?;
        let path = log_dir.join(LOG_FILE_NAME);
        let file = OpenOptions::new().create(true).append(true).open(&path).map_err(io_at(&path))?;
        *self.lock() = Some(file);
        Ok(())
    }

    /// Releases the log file so it can be moved or the app can exit.
    pub fn close(&self) {
        if let Some(mut file) = self.lock().take() {
            let _ = file.flush();
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Option<File>> {
        // A panic while logging must not disable logging for the rest of the session.
        self.file.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

impl Log for FileLogger {
    fn enabled(&self, metadata: &Metadata) -> bool {
        metadata.level() <= self.level && metadata.target().starts_with(APP_TARGET_PREFIX)
    }

    fn log(&self, record: &Record) {
        if !self.enabled(record.metadata()) {
            return;
        }
        let line = format_record(record);
        let written = match self.lock().as_mut() {
            Some(file) => file.write_all(line.as_bytes()).is_ok(),
            None => false,
        };
        // no log file yet, or it can't be written: stderr is the last place left to say it
        if !written {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
    }

    fn flush(&self) {
        if let Some(file) = self.lock().as_mut() {
            let _ = file.flush();
        }
    }
}

pub fn format_record(record: &Record) -> String {
    format!(
        "{} [{:<8}] {}: {}\n",
        chrono::Local::now().format(TIMESTAMP_FORMAT),
        record.level(),
        record.target(),
        record.args()
    )
}

pub fn level_from(value: Option<&str>) -> LevelFilter {
    value.and_then(|text| LevelFilter::from_str(text.trim()).ok()).unwrap_or(DEFAULT_LEVEL)
}

/// Installs the global logger once; later calls return the same instance.
pub fn install() -> &'static FileLogger {
    let logger = LOGGER.get_or_init(|| FileLogger::new(level_from(std::env::var(LOG_LEVEL_ENV).ok().as_deref())));
    if log::set_logger(logger).is_ok() {
        log::set_max_level(logger.level);
    }
    logger
}
