// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.523539
//+ Revised: 	2026/11/03 08:42:01.692243
// -----------------------------------------------------------------------------

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};

use log::{LevelFilter, Log, Metadata, Record};

use crate::error::{Result, io_at};
use crate::platform::config::LOG_FILE_NAME;

pub const LOG_LEVEL_ENV: &str = "LOG_LEVEL";
const DEFAULT_LEVEL: LevelFilter = LevelFilter::Info;
const TIMESTAMP_FORMAT: &str = "%Y-%m-%d %H:%M:%S";
const APP_TARGET_PREFIX: &str = "avalio_lite";
/// A log file bigger than this when it is opened is set aside as the one old file (`.1`).
pub const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const ROTATED_SUFFIX: &str = ".1";

static LOGGER: OnceLock<FileLogger> = OnceLock::new();

/// Logger whose output file can be swapped at runtime, because the user picks
/// (and can later move) the log folder after the app has started.
/// Until a folder is set, records go to stderr.
pub struct FileLogger {
    level: LevelFilter,
    file: Mutex<Option<File>>,
    /// Lines that could not be written to the file since the last one that could.
    lost: AtomicU64,
}

impl FileLogger {
    pub fn new(level: LevelFilter) -> Self {
        Self { level, file: Mutex::new(None), lost: AtomicU64::new(0) }
    }

    pub fn open(&self, log_dir: &Path) -> Result<()> {
        self.open_with_limit(log_dir, MAX_LOG_BYTES)
    }

    /// `open`, setting the existing log aside first when it is over `max_bytes`. Only one old log
    /// is kept: the previous `.1` is replaced.
    pub fn open_with_limit(&self, log_dir: &Path, max_bytes: u64) -> Result<()> {
        fs::create_dir_all(log_dir).map_err(io_at(log_dir))?;
        let path = log_dir.join(LOG_FILE_NAME);
        let rotation = rotate(&path, max_bytes);
        let file = OpenOptions::new().create(true).append(true).open(&path).map_err(io_at(&path))?;
        *self.lock() = Some(file);
        if let Err(error) = rotation {
            log::warn!("could not set the large log file aside, it keeps growing: {error}");
        }
        Ok(())
    }

    /// Releases the log file so it can be moved or the app can exit.
    pub fn close(&self) {
        // a `File` keeps no buffer of its own (each write goes straight to the system), so there
        // is nothing to flush that could fail: dropping it is all closing takes
        drop(self.lock().take());
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
            Some(file) => write_line(file, &self.lost, &line),
            None => false,
        };
        // no log file yet, or it can't be written: stderr is the last place left to say it. A
        // windowed app has none, so there is truly nowhere else to report a failure of this write;
        // the lost lines are counted and reported in the file as soon as it works again.
        if !written {
            let _ = std::io::stderr().write_all(line.as_bytes());
        }
    }

    /// Nothing to do: see `close`.
    fn flush(&self) {}
}

/// Writes `line`, preceded by a note of how many lines were lost since the last successful write.
/// Returns false (and counts the line as lost) when the write fails.
pub fn write_line(file: &mut dyn Write, lost: &AtomicU64, line: &str) -> bool {
    let missed = lost.load(Ordering::Relaxed);
    let note = if missed > 0 {
        format!(
            "{} [WARNING ] {}: {missed} log line(s) could not be written\n",
            chrono::Local::now().format(TIMESTAMP_FORMAT),
            module_path!()
        )
    } else {
        String::new()
    };
    match file.write_all(format!("{note}{line}").as_bytes()) {
        Ok(()) => {
            lost.store(0, Ordering::Relaxed);
            true
        }
        Err(_) => {
            lost.fetch_add(1, Ordering::Relaxed);
            false
        }
    }
}

/// Where `open` puts a log it sets aside: `avalio-lite.log` gives `avalio-lite.log.1`.
pub fn rotated_path(log_dir: &Path) -> PathBuf {
    crate::platform::storage::with_suffix(&log_dir.join(LOG_FILE_NAME), ROTATED_SUFFIX)
}

fn rotate(path: &Path, max_bytes: u64) -> Result<()> {
    match fs::metadata(path) {
        Ok(meta) if meta.len() > max_bytes => {
            let old = crate::platform::storage::with_suffix(path, ROTATED_SUFFIX);
            // renaming over an existing file replaces it on every platform std supports
            fs::rename(path, &old).map_err(io_at(path))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(io_at(path)(error)),
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
