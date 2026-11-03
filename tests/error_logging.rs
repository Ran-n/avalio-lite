// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.102653
//+ Revised: 	2026/11/03 08:27:18.274923
// -----------------------------------------------------------------------------

//! Error paths must reach the log at the right level, not just the screen. The app's own logger
//! is installed, pointed at a scratch folder, and read back after each action.

mod common;

use std::fs;
use std::sync::Mutex;

use avalio_lite::app;
use avalio_lite::platform::config::LOG_FILE_NAME;
use avalio_lite::platform::logging::{self, FileLogger};
use avalio_lite::state::Startup;
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
    assert!(has_line(&log, "WARN", "avalio_lite::state", "settings file unreadable"), "{log}");
}

#[test]
fn a_missing_database_is_a_warning() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    fs::remove_file(fixture.database()).unwrap();
    let log = logged(|| {
        fixture.state().start();
    });
    assert!(has_line(&log, "WARN", "avalio_lite::state", "database missing"), "{log}");
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
    assert!(has_line(&log, "ERROR", "avalio_lite::state", "could not read the database"), "{log}");
}

#[test]
fn a_failed_sheet_action_is_a_warning_naming_the_action() {
    let log = logged(|| {
        let ui = Ui::with_database();
        ui.session.window.global::<Gradebook>().invoke_create_class("".into());
    });
    assert!(has_line(&log, "WARN", "avalio_lite::gradebook::ui", "create class failed"), "{log}");
}

#[test]
fn an_unknown_language_from_the_picker_is_an_error() {
    let log = logged(|| {
        let ui = Ui::with_database();
        ui.session.window.invoke_choose_language("xx".into());
    });
    assert!(has_line(&log, "ERROR", "avalio_lite::app::settings", "unknown language code \"xx\""), "{log}");
}

#[test]
fn a_failed_unlock_other_than_a_wrong_pin_is_an_error() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin("2468", "2468").unwrap();
    state.shutdown();
    let log = logged(|| {
        let ui = Ui::open(fixture);
        // another program has the file open exclusively, so it can't be opened at all
        let _held = common::hold_exclusively(&ui.fixture.database());
        ui.session.window.invoke_unlock("2468".into());
        assert!(!ui.session.window.get_unlock_error().is_empty(), "the lock screen says so too");
    });
    assert!(has_line(&log, "ERROR", "avalio_lite::lock::ui", "unlocking failed"), "{log}");
}

#[test]
fn a_locked_database_that_is_gone_is_an_error() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    state.set_pin("2468", "2468").unwrap();
    state.shutdown();
    let log = logged(|| {
        let ui = Ui::open(fixture);
        fs::remove_file(ui.fixture.database()).unwrap();
        ui.session.window.invoke_unlock("2468".into());
        assert!(ui.session.window.get_setup_required());
    });
    assert!(has_line(&log, "ERROR", "avalio_lite::lock::ui", "the locked database is gone"), "{log}");
}

#[test]
fn a_refused_pin_change_is_a_warning() {
    let log = logged(|| {
        let ui = Ui::with_database();
        ui.session.window.set_pin_new("12".into());
        ui.session.window.set_pin_repeat("12".into());
        ui.session.window.invoke_set_pin();
    });
    assert!(has_line(&log, "WARN", "avalio_lite::lock::ui", "PIN change failed"), "{log}");
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
    assert!(has_line(&log, "WARN", "avalio_lite::app::settings", "move logs failed"), "{log}");
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
    assert!(has_line(&log, "INFO", "avalio_lite::state", "database unlocked"), "{log}");
    assert!(!log.contains(FIRST) && !log.contains(SECOND), "{log}");
}

/// The user sees a friendly sentence; the log keeps the system's own details.
#[test]
fn regression_a_friendly_error_still_logs_its_technical_source() {
    let log = logged(|| {
        let ui = Ui::with_database();
        let config_dir = ui.fixture.path("config");
        fs::remove_dir_all(&config_dir).unwrap();
        fs::write(&config_dir, b"in the way").unwrap();
        ui.session.window.invoke_choose_theme(true);
        let shown = ui.session.window.get_error_message();
        assert!(!shown.is_empty() && !shown.contains("os error"), "{shown}");
    });
    let line = log.lines().find(|line| line.contains("saving the theme failed")).expect(&log);
    assert!(line.contains("could not"), "{line}");
}

#[test]
fn a_screen_that_cannot_be_read_is_a_warning_and_the_window_keeps_its_size() {
    let log = logged(|| {
        let ui = Ui::with_database();
        let before = ui.session.window.window().size();
        app::placement::apply_with(&ui.session.window, Err("GetMonitorInfoW failed".to_owned()));
        assert_eq!(ui.session.window.window().size(), before, "Slint's size stays");
        assert!(!ui.session.window.window().is_maximized());
        assert!(!ui.session.window.get_setup_required(), "the app still shows its sheet");
    });
    assert!(has_line(&log, "WARN", "avalio_lite::app::placement", "couldn't read the screen's work area"), "{log}");
    assert!(log.contains("GetMonitorInfoW failed"), "the failing call is named:\n{log}");
}
