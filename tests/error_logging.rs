// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.102653
//+ Revised: 	2026/10/28 22:23:17.332209
// -----------------------------------------------------------------------------

//! Error paths must reach the log at the right level, not just the screen. The app's own logger
//! is installed, pointed at a scratch folder, and read back after each action.

mod common;

use std::fs;
use std::sync::Mutex;

use avalio_lite::app::state::Startup;
use avalio_lite::config::LOG_FILE_NAME;
use avalio_lite::logging::{self, FileLogger};
use avalio_lite::ui::Gradebook;
use common::{Fixture, Ui};
use slint::ComponentHandle;

/// The logger is process-wide, so tests that read it take turns.
static LOG_LOCK: Mutex<()> = Mutex::new(());

/// Runs `action` with the app's logger writing to a fresh folder, and returns what it logged.
fn logged(action: impl FnOnce()) -> String {
    let _turn = LOG_LOCK.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
    let logger: &'static FileLogger = logging::install();
    let dir = tempfile::tempdir().unwrap();
    logger.open(dir.path()).unwrap();
    action();
    logger.close();
    fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap_or_default()
}

fn has_line(log: &str, level: &str, target: &str, text: &str) -> bool {
    log.lines().any(|line| line.contains(&format!("[{level:<8}] {target}:")) && line.contains(text))
}

#[test]
fn an_unreadable_settings_file_is_a_warning() {
    let fixture = Fixture::new();
    fs::create_dir_all(fixture.path("config")).unwrap();
    fs::write(fixture.store().path(), "not = [valid").unwrap();
    let log = logged(|| {
        fixture.state().start();
    });
    assert!(has_line(&log, "WARN", "avalio_lite::app::state", "settings file unreadable"), "{log}");
}

#[test]
fn a_missing_database_is_a_warning() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    fs::remove_file(fixture.database()).unwrap();
    let log = logged(|| {
        fixture.state().start();
    });
    assert!(has_line(&log, "WARN", "avalio_lite::app::state", "database missing"), "{log}");
}

#[test]
fn a_database_that_cannot_be_read_is_an_error() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let _held = common::hold_exclusively(&fixture.database());
    let log = logged(|| {
        let startup = fixture.state().start();
        assert!(matches!(startup, Startup::NeedsSetup(_)));
    });
    assert!(has_line(&log, "ERROR", "avalio_lite::app::state", "could not read the database"), "{log}");
}

#[test]
fn a_failed_sheet_action_is_a_warning_naming_the_action() {
    let log = logged(|| {
        let ui = Ui::with_database();
        ui.session.window.global::<Gradebook>().invoke_create_class("".into());
    });
    assert!(has_line(&log, "WARN", "avalio_lite::app::gradebook", "create class failed"), "{log}");
}

#[test]
fn an_unknown_language_from_the_picker_is_an_error() {
    let log = logged(|| {
        let ui = Ui::with_database();
        ui.session.window.invoke_choose_language("xx".into());
    });
    assert!(has_line(&log, "ERROR", "avalio_lite::app::callbacks", "unknown language code \"xx\""), "{log}");
}

#[test]
fn a_failed_unlock_other_than_a_wrong_pin_is_an_error() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin("2468", "2468").unwrap();
    state.shutdown();
    let log = logged(|| {
        let ui = Ui::open(fixture);
        fs::remove_file(ui.fixture.database()).unwrap();
        ui.session.window.invoke_unlock("2468".into());
        assert!(!ui.session.window.get_unlock_error().is_empty(), "the lock screen says so too");
    });
    assert!(has_line(&log, "ERROR", "avalio_lite::app::lock_screen", "unlocking failed"), "{log}");
}

#[test]
fn a_refused_pin_change_is_a_warning() {
    let log = logged(|| {
        let ui = Ui::with_database();
        ui.session.window.set_pin_new("12".into());
        ui.session.window.set_pin_repeat("12".into());
        ui.session.window.invoke_set_pin();
    });
    assert!(has_line(&log, "WARN", "avalio_lite::app::lock_screen", "PIN change failed"), "{log}");
}

#[test]
fn a_failed_move_is_a_warning() {
    let log = logged(|| {
        let ui = Ui::with_database();
        let in_the_way = ui.fixture.path("taken");
        fs::create_dir_all(&in_the_way).unwrap();
        fs::write(in_the_way.join(LOG_FILE_NAME), b"other").unwrap();
        ui.dialogs.pick_folder_next(Some(&in_the_way));
        ui.session.window.invoke_move_logs();
    });
    assert!(has_line(&log, "WARN", "avalio_lite::app::callbacks", "move logs failed"), "{log}");
}

/// Setting, using, changing and removing a PIN logs what happened, never the PIN itself.
#[test]
fn the_pin_never_reaches_the_log() {
    const FIRST: &str = "583920";
    const SECOND: &str = "746351";
    let log = logged(|| {
        let fixture = Fixture::new();
        let mut state = fixture.set_up();
        state.set_pin(FIRST, FIRST).unwrap();
        assert!(state.lock());
        assert!(state.unlock(SECOND).is_err());
        state.unlock(FIRST).unwrap();
        state.change_pin(FIRST, SECOND, SECOND).unwrap();
        state.remove_pin(SECOND).unwrap();
        state.shutdown();
    });
    assert!(has_line(&log, "INFO", "avalio_lite::app::state", "database unlocked"), "{log}");
    assert!(!log.contains(FIRST) && !log.contains(SECOND), "{log}");
}
