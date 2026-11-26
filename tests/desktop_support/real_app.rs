// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:05.327359
//+ Revised: 	2026/11/26 12:13:04.406383
// -----------------------------------------------------------------------------

//! The app as users run it: the shipped exe, with no driver, so its real Windows dialogs open and
//! its window is the only handle the test has. Its window's clicks are real, through `window`. Only
//! `the_real_dialogs_open_and_close` uses it: every other test runs through `App` and its driver.
//! The shipped exe's window can't be cloaked, so it shows on the screen for the few seconds it runs.

use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use avalio_lite::platform::config::CONFIG_DIR_ENV;

use super::binaries::app_exe;
use super::window::{AppWindow, OtherWindow, dialog_of};

/// The first-run card is centred; its database "Browse…" sits this far from the window's centre
/// (logical pixels, from the setup baseline).
const BROWSE_FROM_CENTRE: (f64, f64) = (204.0, 33.0);
const LOGICAL_WIDTH: f64 = 1280.0;
/// How long the app may take to show its window, and a dialog to appear after a click.
const WINDOW_TIMEOUT: Duration = Duration::from_secs(45);
const DIALOG_TIMEOUT: Duration = Duration::from_secs(10);
/// How long the picture must stay the same to count as drawn, and how often it is looked at.
const SETTLED_FOR: Duration = Duration::from_millis(400);
const LOOK_EVERY: Duration = Duration::from_millis(100);
/// How often a closing dialog is told to close again: one that has only just appeared can miss a
/// close message, so the close is repeated now and then until it is gone.
const DIALOG_CLOSE_AGAIN: Duration = Duration::from_secs(2);

/// The shipped app, started on a scratch settings folder. Killed if the test ends before it quits,
/// so no window is left behind.
pub struct RealApp {
    child: Child,
    window: AppWindow,
    pid: u32,
}

impl RealApp {
    /// Starts the shipped exe on the settings in `config_dir`, from `working_dir`, and waits until its
    /// window shows the first-run card, drawn and still: a click before that is lost.
    pub fn start(config_dir: &Path, working_dir: &Path) -> Self {
        let child = Command::new(app_exe())
            .env(CONFIG_DIR_ENV, config_dir)
            .current_dir(working_dir)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .expect("the app starts");
        let pid = child.id();
        let window = AppWindow::of_process(pid, WINDOW_TIMEOUT).expect("the app shows its window");
        wait_for_the_card(&window);
        Self { child, window, pid }
    }

    /// A real click on the first-run card's database "Browse…", sized from the window's own size.
    pub fn click_browse_for_database(&self) {
        let (width, height) = self.window.client_size();
        let scale = f64::from(width) / LOGICAL_WIDTH;
        let x = f64::from(width) / 2.0 + BROWSE_FROM_CENTRE.0 * scale;
        let y = f64::from(height) / 2.0 + BROWSE_FROM_CENTRE.1 * scale;
        self.window.click(x as i32, y as i32);
    }

    /// The dialog the app opened, if one appears within the usual time.
    pub fn folder_dialog(&self) -> Option<OtherWindow> {
        dialog_of(self.pid, &self.window, DIALOG_TIMEOUT)
    }

    /// Closes `dialog` the way the user would, again now and then, until it is gone or `within`
    /// has passed. Returns whether it went: Windows' dialog may take a while to go.
    pub fn close_dialog(&self, dialog: &OtherWindow, within: Duration) -> bool {
        let deadline = Instant::now() + within;
        let mut closed_at = Instant::now();
        dialog.close();
        while dialog_of(self.pid, &self.window, Duration::ZERO).is_some() {
            if Instant::now() >= deadline {
                return false;
            }
            if closed_at.elapsed() >= DIALOG_CLOSE_AGAIN {
                dialog.close();
                closed_at = Instant::now();
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        true
    }

    /// Closes the window (as its close button does) and waits for the app to exit, which says how.
    pub fn close_window(mut self) -> ExitStatus {
        self.window.close();
        self.child.wait().expect("the app exits")
    }
}

impl Drop for RealApp {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// Waits until the first-run card is drawn, which makes the window's centre differ from its corner,
/// and the picture has stayed the same for `SETTLED_FOR`. A fixed pause was too short on a busy
/// machine, so the click on "Browse…" landed before the card and opened nothing.
fn wait_for_the_card(window: &AppWindow) {
    let deadline = Instant::now() + WINDOW_TIMEOUT;
    let mut previous = window.capture();
    let mut same_since = Instant::now();
    loop {
        std::thread::sleep(LOOK_EVERY);
        let current = window.capture();
        let resized = (current.width, current.height) != (previous.width, previous.height);
        if resized || current.changed_area(&previous).is_some() {
            same_since = Instant::now();
        }
        let card = current.pixel(current.width / 2, current.height / 2) != current.pixel(2, 2);
        if card && same_since.elapsed() >= SETTLED_FOR {
            return;
        }
        assert!(Instant::now() < deadline, "the first-run card wasn't drawn within {WINDOW_TIMEOUT:?}");
        previous = current;
    }
}
