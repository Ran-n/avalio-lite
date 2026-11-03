// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 04:09:17.087739
//+ Revised: 	2026/11/03 08:27:20.468905
// -----------------------------------------------------------------------------

//! A UI callback that fires after its window is gone does nothing and says so in the debug log.
//! A test binary of its own: the logger reads its level from `LOG_LEVEL` once per process, and
//! the other logging tests run at the default level.

mod common;

use std::cell::Cell;
use std::fs;
use std::rc::Rc;

use avalio_lite::platform::config::LOG_FILE_NAME;
use avalio_lite::platform::logging::{self, LOG_LEVEL_ENV};
use avalio_lite::window::with_window;
use common::Ui;

#[test]
fn a_callback_after_the_window_closed_is_logged_at_debug() {
    // SAFETY: this binary's only test sets it before anything else reads the environment.
    unsafe { std::env::set_var(LOG_LEVEL_ENV, "debug") };
    let logger = logging::install();
    let dir = tempfile::tempdir().unwrap();
    logger.open(dir.path()).unwrap();

    let ran = Rc::new(Cell::new(false));
    let ui = Ui::with_database();
    let mut callback = {
        let ran = ran.clone();
        with_window(&ui.session.window, move |_| ran.set(true))
    };
    callback();
    assert!(ran.get(), "runs while the window is alive");
    ran.set(false);
    drop(ui);
    callback();
    logger.close();

    assert!(!ran.get(), "does nothing once the window is gone");
    let log = fs::read_to_string(dir.path().join(LOG_FILE_NAME)).unwrap();
    assert!(log.contains("[DEBUG   ] avalio_lite::window: callback after the window closed"), "{log}");
}
