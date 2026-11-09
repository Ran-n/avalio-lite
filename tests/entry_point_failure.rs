// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:06.973308
//+ Revised: 	2026/11/08 04:36:59.096624
// -----------------------------------------------------------------------------

//! `app::run_with` when the window can't run: Slint's headless backend without an event loop
//! makes `run` fail, which must end in the fatal-error dialog and a failure exit code.

mod common;

use std::process::ExitCode;
use std::rc::Rc;

use avalio_lite::app;
use avalio_lite::i18n::{self, Message};
use avalio_lite::platform::config::CONFIG_DIR_ENV;
use common::{FakeDialogs, Fixture};

#[test]
fn a_window_that_cannot_run_shows_the_fatal_error_and_fails() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    fixture.without_update_checks();
    // SAFETY: the only test in this process, and it sets the variable before starting any thread.
    unsafe { std::env::set_var(CONFIG_DIR_ENV, fixture.path("config")) };
    i_slint_backend_testing::init_no_event_loop();

    let dialogs = Rc::new(FakeDialogs::default());
    let exit = app::run_with(dialogs.clone());

    assert_eq!(exit, ExitCode::FAILURE);
    let fatal = dialogs.fatal.borrow();
    assert_eq!(fatal.len(), 1);
    let prefix = i18n::text(i18n::current(), &Message::WindowFailed);
    // a fixed sentence: the window system's own error text is only in the log
    assert_eq!(fatal[0], prefix);
    // even a failed run shuts down: the database can be deleted
    std::fs::remove_file(fixture.database()).expect("the database is closed");
}
