// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.842742
//+ Revised: 	2026/11/15 12:17:29.223312
// -----------------------------------------------------------------------------

//! `app::run_with`, the whole app as `main` starts it: on Slint's headless backend with a real
//! event loop, pointed at a scratch settings folder. One test per file, since that backend can
//! only be set up once per process.

mod common;

use std::fs;
use std::process::ExitCode;
use std::rc::Rc;
use std::time::Duration;

use avalio_lite::app;
use avalio_lite::platform::config::{self, CONFIG_DIR_ENV, LOG_FILE_NAME};
use common::{FakeDialogs, Fixture};

#[test]
fn the_app_starts_on_the_saved_database_and_quits_cleanly() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    fixture.without_update_checks();
    // SAFETY: the only test in this process, and it sets the variable before starting any thread.
    unsafe { std::env::set_var(CONFIG_DIR_ENV, fixture.path("config")) };
    assert_eq!(config::default_data_dir(), fixture.path("config"), "a scratch run suggests the scratch folder");
    i_slint_backend_testing::init_integration_test_with_system_time();
    // closing the window is what ends the event loop in normal use
    let quit = slint::Timer::default();
    quit.start(slint::TimerMode::SingleShot, Duration::from_millis(300), || slint::quit_event_loop().unwrap());

    // a release build has no console: a panic anywhere while the app runs must reach the log,
    // whatever it carries
    let panics = slint::Timer::default();
    panics.start(slint::TimerMode::SingleShot, Duration::from_millis(100), || {
        let broken: [fn(); 3] =
            [|| panic!("a fixed message"), || panic!("a built {}", "message"), || std::panic::panic_any(7)];
        for work in broken {
            assert!(std::thread::spawn(work).join().is_err());
        }
    });

    let dialogs = Rc::new(FakeDialogs::default());
    let exit = app::run_with(dialogs.clone());

    assert_eq!(exit, ExitCode::SUCCESS);
    assert!(dialogs.fatal.borrow().is_empty());
    let log = fs::read_to_string(fixture.path("logs").join(LOG_FILE_NAME)).unwrap();
    assert!(log.trim_end().ends_with("shut down"), "the last thing logged is the shutdown:\n{log}");
    for said in ["a fixed message", "a built message", "no message"] {
        let line = log.lines().find(|line| line.contains("panicked at") && line.ends_with(said));
        let line = line.unwrap_or_else(|| panic!("the panic with {said:?} is in the log:\n{log}"));
        assert!(line.contains("entry_point.rs"), "with where it happened: {line}");
    }
    // the quit path released the database: Windows refuses to delete a file that is still open
    fs::remove_file(fixture.database()).expect("the database is closed after quitting");
}
