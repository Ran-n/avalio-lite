// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:32.250028
//+ Revised: 	2026/11/02 14:00:10.121611
// -----------------------------------------------------------------------------

use std::fs;

use avalio_lite::config::LOG_FILE_NAME;
use avalio_lite::logging::{FileLogger, format_record, level_from};
use log::{Level, LevelFilter, Log, Record};

fn record_for<'a>(target: &'a str, level: Level, args: std::fmt::Arguments<'a>) -> Record<'a> {
    Record::builder().target(target).level(level).args(args).build()
}

#[test]
fn level_defaults_to_info_and_parses_env_values() {
    assert_eq!(level_from(None), LevelFilter::Info);
    assert_eq!(level_from(Some("garbage")), LevelFilter::Info);
    assert_eq!(level_from(Some(" debug ")), LevelFilter::Debug);
    assert_eq!(level_from(Some("WARN")), LevelFilter::Warn);
}

#[test]
fn format_matches_house_style() {
    let line = format_record(&record_for("avalio_lite::db", Level::Warn, format_args!("hello")));
    let (timestamp, rest) = line.split_at(19);
    assert!(chrono::NaiveDateTime::parse_from_str(timestamp, "%Y-%m-%d %H:%M:%S").is_ok());
    assert_eq!(rest, " [WARN    ] avalio_lite::db: hello\n");
}

#[test]
fn writes_app_records_at_or_above_level_to_the_file() {
    let dir = tempfile::tempdir().unwrap();
    let logger = FileLogger::new(LevelFilter::Info);
    logger.open(dir.path()).unwrap();
    logger.log(&record_for("avalio_lite::app", Level::Info, format_args!("kept")));
    logger.log(&record_for("avalio_lite::app", Level::Debug, format_args!("too verbose")));
    logger.log(&record_for("winit::window", Level::Error, format_args!("third party")));
    logger.close();
    let text = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
    assert!(text.contains("kept"));
    assert!(!text.contains("too verbose"));
    assert!(!text.contains("third party"));
}

#[test]
fn reopening_in_another_folder_switches_output_and_appends() {
    let (first, second) = (tempfile::tempdir().unwrap(), tempfile::tempdir().unwrap());
    let logger = FileLogger::new(LevelFilter::Info);
    logger.open(first.path()).unwrap();
    logger.log(&record_for("avalio_lite", Level::Info, format_args!("one")));
    logger.close();
    logger.open(second.path()).unwrap();
    logger.log(&record_for("avalio_lite", Level::Info, format_args!("two")));
    logger.open(second.path()).unwrap();
    logger.log(&record_for("avalio_lite", Level::Info, format_args!("three")));
    logger.close();
    let first_text = fs::read_to_string(first.path().join(LOG_FILE_NAME)).unwrap();
    let second_text = fs::read_to_string(second.path().join(LOG_FILE_NAME)).unwrap();
    assert!(first_text.contains("one") && !first_text.contains("two"));
    assert!(second_text.contains("two") && second_text.contains("three"));
}

#[test]
fn open_reports_an_unusable_folder() {
    let dir = tempfile::tempdir().unwrap();
    let file_in_the_way = dir.path().join("not-a-folder");
    fs::write(&file_in_the_way, b"x").unwrap();
    assert!(FileLogger::new(LevelFilter::Info).open(&file_in_the_way).is_err());
}

#[test]
fn records_before_a_folder_is_chosen_are_not_lost_in_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let logger = FileLogger::new(LevelFilter::Info);
    // no folder yet: the record goes to stderr, and no file appears anywhere
    logger.log(&record_for("avalio_lite::app", Level::Warn, format_args!("early")));
    logger.flush();
    logger.open(dir.path()).unwrap();
    logger.log(&record_for("avalio_lite::app", Level::Info, format_args!("later")));
    logger.close();
    let text = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
    assert!(text.contains("later") && !text.contains("early"));
}

#[test]
fn flush_leaves_the_file_open_with_everything_written() {
    let dir = tempfile::tempdir().unwrap();
    let logger = FileLogger::new(LevelFilter::Info);
    logger.open(dir.path()).unwrap();
    logger.log(&record_for("avalio_lite::app", Level::Info, format_args!("first")));
    logger.flush();
    assert!(fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap().contains("first"));
    logger.log(&record_for("avalio_lite::app", Level::Info, format_args!("second")));
    logger.close();
    logger.close();
    assert!(fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap().contains("second"), "still open after flush");
}

#[test]
fn regression_a_log_over_the_limit_is_set_aside_at_startup() {
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join(LOG_FILE_NAME);
    fs::write(&file, vec![b'x'; 100]).unwrap();
    let logger = FileLogger::new(LevelFilter::Info);
    logger.open_with_limit(dir.path(), 100).unwrap();
    logger.close();
    assert_eq!(fs::metadata(&file).unwrap().len(), 100, "at the limit it is kept and appended to");
    fs::write(&file, vec![b'y'; 101]).unwrap();
    logger.open_with_limit(dir.path(), 100).unwrap();
    logger.log(&record_for("avalio_lite", Level::Info, format_args!("fresh")));
    logger.close();
    let rotated = avalio_lite::logging::rotated_path(dir.path());
    assert_eq!(fs::read(&rotated).unwrap(), vec![b'y'; 101]);
    assert!(fs::read_to_string(&file).unwrap().contains("fresh"));
    // only one old file is kept: the next rotation replaces it
    fs::write(&file, vec![b'z'; 150]).unwrap();
    logger.open_with_limit(dir.path(), 100).unwrap();
    logger.close();
    assert_eq!(fs::read(&rotated).unwrap(), vec![b'z'; 150]);
}

struct Failing;

impl std::io::Write for Failing {
    fn write(&mut self, _: &[u8]) -> std::io::Result<usize> {
        Err(std::io::Error::other("disk full"))
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn lost_lines_are_counted_and_reported_once_the_file_works_again() {
    use std::sync::atomic::{AtomicU64, Ordering};

    use avalio_lite::logging::write_line;
    let lost = AtomicU64::new(0);
    assert!(!write_line(&mut Failing, &lost, "one\n"));
    assert!(!write_line(&mut Failing, &lost, "two\n"));
    assert_eq!(lost.load(Ordering::Relaxed), 2);
    let mut file = Vec::new();
    assert!(write_line(&mut file, &lost, "three\n"));
    let text = String::from_utf8(file).unwrap();
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 2, "the note is a line of its own: {text:?}");
    assert!(lines[0].ends_with("[WARNING ] avalio_lite::logging: 2 log line(s) could not be written"), "{text:?}");
    assert_eq!(lines[1], "three");
    assert_eq!(lost.load(Ordering::Relaxed), 0);
}

#[test]
fn a_large_log_that_cannot_be_set_aside_keeps_growing() {
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join(LOG_FILE_NAME);
    fs::write(&log, "x".repeat(200)).unwrap();
    // a folder where the old log would go makes setting it aside fail
    let rotated = avalio_lite::logging::rotated_path(dir.path());
    fs::create_dir(&rotated).unwrap();
    fs::write(rotated.join("keep"), b"").unwrap();
    let logger = FileLogger::new(LevelFilter::Info);
    logger.open_with_limit(dir.path(), 100).unwrap();
    assert_eq!(fs::read_to_string(&log).unwrap(), "x".repeat(200), "the log is kept and still used");
    assert!(rotated.join("keep").exists());
    logger.close();
}
