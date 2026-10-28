// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:32.250028
//+ Revised: 	2026/10/28 15:53:09.517557
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
