// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:35.027873
//+ Revised: 	2026/11/12 14:39:55.955439
// -----------------------------------------------------------------------------

//! Work that mustn't hold up the window, such as going online: it runs on its own thread, and its
//! result is handed back to the window on the UI thread. Behind a trait because the headless UI
//! tests have no event loop to hand a result back on; there the work runs at once (decision D7).
//!
//! Only the result crosses threads; what is done with it stays on the UI thread, so it may use the
//! app state and the window freely.

use std::any::Any;
use std::cell::RefCell;
use std::collections::HashMap;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use slint::ComponentHandle;

use crate::ui::AppWindow;

/// A job's result, on its way back to the UI thread.
pub type Payload = Box<dyn Any + Send>;
/// The work itself; runs off the UI thread.
pub type Job = Box<dyn FnOnce() -> Payload + Send>;
/// What to do with the result; runs on the UI thread, with the window.
pub type Done = Box<dyn FnOnce(&AppWindow, Payload)>;

pub trait Worker {
    /// Starts `job`; `Err` when it couldn't be started, and then nothing runs.
    fn start(&self, window: &AppWindow, job: Job, done: Done) -> std::io::Result<()>;
    /// Waits for the work still running, at shutdown, for up to `JOIN_LIMIT`.
    fn join(&self);
}

/// How long shutdown waits for background work: longer than a licence request may take (15 s),
/// far shorter than a download (up to 10 min). Work still running then is left to end with the
/// process; none of it changes anything that a cut-off would leave half done (a cut download is
/// removed at the next start, and the portable exe is only swapped on the UI thread).
pub const JOIN_LIMIT: Duration = Duration::from_secs(20);

pub type SharedWorker = Rc<dyn Worker>;

/// A job that panicked instead of returning; the panic's message is already in the log.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Panicked;

impl Panicked {
    /// Logs the panic `payload` (its message, when it has one) and stands for it.
    fn logged(payload: &(dyn Any + Send)) -> Self {
        let text = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "no message".to_owned());
        log::error!("background work panicked: {text}");
        Self
    }
}

/// Runs `job` off the UI thread, then `done` with its result on the UI thread; `Err(Panicked)`
/// when the job panicked, so whatever waits for it (a busy screen) can end and say so.
pub fn run<T: Send + 'static>(
    worker: &dyn Worker,
    window: &AppWindow,
    job: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(&AppWindow, Result<T, Panicked>) + 'static,
) -> std::io::Result<()> {
    worker.start(
        window,
        Box::new(move || {
            let finished = catch_unwind(AssertUnwindSafe(job)).map_err(|payload| Panicked::logged(payload.as_ref()));
            Box::new(finished) as Payload
        }),
        Box::new(move |window, payload| match payload.downcast::<Result<T, Panicked>>() {
            Ok(result) => done(window, *result),
            // run() pairs each job with its own done, so the types always match
            Err(_) => log::error!("a background result came back as the wrong type"),
        }),
    )
}

thread_local! {
    /// What to do with each running job's result, by job number; only the UI thread uses it.
    static WAITING: RefCell<HashMap<u64, Done>> = RefCell::default();
}

static NEXT_JOB: AtomicU64 = AtomicU64::new(0);

/// The app's worker: one thread per job.
#[derive(Default)]
pub struct ThreadWorker {
    running: RefCell<Vec<JoinHandle<()>>>,
}

impl Worker for ThreadWorker {
    fn start(&self, window: &AppWindow, job: Job, done: Done) -> std::io::Result<()> {
        let id = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
        WAITING.with(|waiting| waiting.borrow_mut().insert(id, done));
        let weak = window.as_weak();
        let spawned = thread::Builder::new().name("background".into()).spawn(move || {
            let payload = job();
            let handed = slint::invoke_from_event_loop(move || {
                let done = WAITING.with(|waiting| waiting.borrow_mut().remove(&id));
                if let (Some(window), Some(done)) = (super::alive(&weak), done) {
                    done(&window, payload);
                }
            });
            if let Err(error) = handed {
                log::info!("a background result arrived after the window closed: {error}");
            }
        });
        match spawned {
            Ok(handle) => {
                let mut running = self.running.borrow_mut();
                running.retain(|handle| !handle.is_finished());
                running.push(handle);
                Ok(())
            }
            Err(error) => {
                WAITING.with(|waiting| waiting.borrow_mut().remove(&id));
                Err(error)
            }
        }
    }

    fn join(&self) {
        let deadline = Instant::now() + JOIN_LIMIT;
        let mut running = self.running.take();
        while !running.is_empty() && Instant::now() < deadline {
            let (finished, left): (Vec<_>, Vec<_>) = running.into_iter().partition(JoinHandle::is_finished);
            for handle in finished {
                if handle.join().is_err() {
                    log::error!("background work panicked");
                }
            }
            running = left;
            if !running.is_empty() {
                thread::sleep(Duration::from_millis(50));
            }
        }
        if !running.is_empty() {
            log::warn!("{} background job(s) still running at shutdown are left to end with the app", running.len());
        }
    }
}

/// Runs each job at once, on the calling thread: for the headless UI tests.
#[derive(Default)]
pub struct InlineWorker;

impl Worker for InlineWorker {
    fn start(&self, window: &AppWindow, job: Job, done: Done) -> std::io::Result<()> {
        done(window, job());
        Ok(())
    }

    fn join(&self) {}
}
