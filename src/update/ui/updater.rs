// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 03:55:27.325306
//+ Revised: 	2026/11/23 03:55:27.325306
// -----------------------------------------------------------------------------

//! The updater's state, shared by the other UI modules: what the check found, the offer, the
//! download under way and the program to restart. Quitting and the restart note are here too.

use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use chrono::{DateTime, Local};
use ed25519_dalek::VerifyingKey;

use crate::update::check::{Offer, Status};
use crate::update::download::Limits;
use crate::update::release::UpdateError;
use crate::update::source::{Http, SharedSource};
use crate::update::target::Build;
use crate::window::SharedState;
use crate::window::worker::SharedWorker;

/// How long quitting waits for a cancelled download's thread.
pub const QUIT_WAIT: Duration = Duration::from_secs(5);

/// Where updates come from and go to.
pub struct Updates {
    /// GitHub first, then GitLab
    pub sources: Vec<SharedSource>,
    /// the client the proof and the download use
    pub http: Http,
    /// the release keys this build trusts
    pub keys: Vec<VerifyingKey>,
    pub build: Build,
    /// the running program, read at start; replaced by an update
    pub exe: Option<PathBuf>,
    pub downloads: PathBuf,
    pub limits: Limits,
    /// automatic checks; always off under the UI test driver
    pub automatic: bool,
    /// runs the check and the download off the UI thread (at once, in the headless tests)
    pub worker: SharedWorker,
    /// puts the verified download in place of the program: `install::replace` in the app, a plain
    /// move of scratch files in the tests (never the test's own exe)
    pub replace: fn(&std::path::Path, &std::path::Path) -> Result<(), UpdateError>,
}

/// A download under way.
pub(super) struct Running {
    /// the version being downloaded
    pub(super) version: semver::Version,
    /// set by Cancel or quit; the download stops at its next check
    pub(super) cancel: Arc<AtomicBool>,
    /// set by the download's thread when it has ended
    pub(super) finished: Arc<AtomicBool>,
}

/// Everything the updater remembers while the window is open.
#[derive(Default)]
pub(super) struct Inner {
    /// what the last check of this run found, and when it ended
    pub(super) status: Option<(Status, DateTime<Local>)>,
    /// the newer version on offer; a failed check never removes it
    pub(super) offer: Option<Offer>,
    /// × hid the banner for this version until the next start
    pub(super) dismissed: Option<semver::Version>,
    /// whether a check is running
    pub(super) checking: bool,
    /// the download under way, if any
    pub(super) download: Option<Running>,
    /// counts schedule changes, so a timer from before one does nothing
    pub(super) generation: u64,
    /// the timer for the next automatic check
    pub(super) timer: Option<slint::Timer>,
    /// the program to start once the app has shut down: the update is in place
    pub(super) restart: Option<PathBuf>,
}

/// The updater as the window holds it: the state, the settings and the mutable part behind a cell.
#[derive(Clone)]
pub struct Updater {
    /// the app's saved state, where the update settings and the last check are kept
    pub(super) state: SharedState,
    /// the fixed inputs of the updater
    pub(super) updates: Rc<Updates>,
    /// the mutable part
    pub(super) inner: Rc<RefCell<Inner>>,
}

impl Updater {
    /// An updater for the window's state, with nothing checked or downloaded yet.
    pub fn new(state: &SharedState, updates: Updates) -> Self {
        Self { state: state.clone(), updates: Rc::new(updates), inner: Rc::default() }
    }

    /// The program to start once the app has shut down, when an update was put in place.
    pub fn restart(&self) -> Option<PathBuf> {
        self.inner.borrow().restart.clone()
    }

    /// The newer version on offer.
    pub fn offer(&self) -> Option<Offer> {
        self.inner.borrow().offer.clone()
    }

    /// As the app quits: no more timers; a download is cancelled (its file deleted) and waited for
    /// up to `QUIT_WAIT`, so the app never exits halfway through it. A check is abandoned: it only
    /// reads.
    pub fn quit(&self) {
        let mut inner = self.inner.borrow_mut();
        inner.generation += 1;
        inner.timer = None;
        if let Some(running) = &inner.download {
            running.cancel.store(true, Ordering::SeqCst);
            let deadline = Instant::now() + QUIT_WAIT;
            while !running.finished.load(Ordering::SeqCst) && Instant::now() < deadline {
                std::thread::sleep(Duration::from_millis(20));
            }
            if !running.finished.load(Ordering::SeqCst) {
                log::warn!(target: "avalio_lite::update", "the cancelled download didn't end within {QUIT_WAIT:?}");
            }
        }
    }

    /// Keeps that the new version couldn't be started, for the next start to say.
    pub fn not_restarted(&self, reason: &str) {
        log::error!(target: "avalio_lite::update", "the new version couldn't be started: {reason}");
        if let Err(error) = self.state.borrow_mut().set_update_failed(Some(reason.to_owned())) {
            log::error!("couldn't keep that the update didn't restart ({reason}): {error}");
        }
    }
}
