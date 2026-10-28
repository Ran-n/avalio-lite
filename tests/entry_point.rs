// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.842742
//+ Revised: 	2026/10/28 15:53:06.842742
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
use avalio_lite::config::{self, CONFIG_DIR_ENV, LOG_FILE_NAME};
use common::{FakeDialogs, Fixture};

#[test]
fn the_app_starts_on_the_saved_database_and_quits_cleanly() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    // SAFETY: the only test in this process, and it sets the variable before starting any thread.
    unsafe { std::env::set_var(CONFIG_DIR_ENV, fixture.path("config")) };
    assert_eq!(config::default_data_dir(), fixture.path("config"), "a scratch run suggests the scratch folder");
    i_slint_backend_testing::init_integration_test_with_system_time();
    // closing the window is what ends the event loop in normal use
    let quit = slint::Timer::default();
    quit.start(slint::TimerMode::SingleShot, Duration::from_millis(300), || slint::quit_event_loop().unwrap());

    let dialogs = Rc::new(FakeDialogs::default());
    let exit = app::run_with(dialogs.clone());

    assert_eq!(exit, ExitCode::SUCCESS);
    assert!(dialogs.fatal.borrow().is_empty());
    let log = fs::read_to_string(fixture.path("logs").join(LOG_FILE_NAME)).unwrap();
    assert!(log.trim_end().ends_with("shut down"), "the last thing logged is the shutdown:\n{log}");
    // the quit path released the database: Windows refuses to delete a file that is still open
    fs::remove_file(fixture.database()).expect("the database is closed after quitting");
}
