// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.179649
//+ Revised: 	2026/11/24 17:54:08.560069
// -----------------------------------------------------------------------------

//! The app as users run it: the shipped exe, its real folder dialog and the real close button. It is the
//! one test whose window shows on the screen, for the few seconds it runs.

use std::time::Duration;

use crate::common::Fixture;
use crate::desktop_support::{self, RealApp};

/// How long a closed dialog may take to go, and how often it is told to close meanwhile.
const DIALOG_CLOSE_WAIT: Duration = Duration::from_secs(10);

/// The app as users run it: `main`, `app::run` and the real Windows dialogs, which every other test
/// replaces. Tab to the first-run screen's "Browse…", press it, close the folder dialog that opens,
/// then close the window: the app must quit cleanly.
///
/// The one test whose window shows on the screen: it runs the shipped exe, which has no test
/// driver to keep its window hidden (Windows lets only the window's own process cloak it), and
/// the real folder dialog it opens. It is over in a few seconds.
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_real_dialogs_open_and_close() {
    let _turn = desktop_support::turn();
    let fixture = Fixture::new();
    let app = RealApp::start(&fixture.path("config"), fixture.root.path());
    app.click_browse_for_database();
    let dialog = app.folder_dialog().expect("the folder dialog opens");
    assert!(
        app.close_dialog(&dialog, DIALOG_CLOSE_WAIT),
        "closing it cancels it (still open after {DIALOG_CLOSE_WAIT:?})"
    );
    let status = app.close_window();
    assert!(status.success(), "a cancelled dialog and the close button quit cleanly: {status}");
}
