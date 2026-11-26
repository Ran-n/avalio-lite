// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:35.027873
//+ Revised: 	2026/11/24 17:54:06.585743
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

/// Whether shutdown waits for a job.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtQuit {
    /// `join` waits for it (up to `JOIN_LIMIT`)
    Wait,
    /// `join` doesn't wait: an update check, which only reads, or a download, which its owner
    /// cancels and waits for itself
    Abandon,
}

/// The thread a job runs on: its name, and whether shutdown waits for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Named<'a> {
    pub name: &'a str,
    pub at_quit: AtQuit,
}

impl Named<'_> {
    /// The thread most work runs on: "background", waited for at quit.
    pub const BACKGROUND: Named<'static> = Named { name: "background", at_quit: AtQuit::Wait };
}

/// A job and what to do with its result.
pub struct Task {
    pub job: Job,
    pub done: Done,
}

pub trait Worker {
    /// Starts `job`; `Err` when it couldn't be started, and then nothing runs.
    fn start(&self, window: &AppWindow, job: Job, done: Done) -> std::io::Result<()>;
    /// `start` on the thread `thread` names, which shutdown waits for or not.
    fn start_named(&self, thread: Named, window: &AppWindow, task: Task) -> std::io::Result<()> {
        let _ = thread;
        self.start(window, task.job, task.done)
    }
    /// Waits for the work still running, at shutdown, for up to `JOIN_LIMIT` in the app.
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

/// A worker and the thread a job runs on there.
#[derive(Clone, Copy)]
pub struct Background<'a> {
    pub worker: &'a dyn Worker,
    pub thread: Named<'a>,
}

/// Runs `job` off the UI thread, then `done` with its result on the UI thread; `Err(Panicked)`
/// when the job panicked, so whatever waits for it (a busy screen) can end and say so.
pub fn run<T: Send + 'static>(
    worker: &dyn Worker,
    window: &AppWindow,
    job: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(&AppWindow, Result<T, Panicked>) + 'static,
) -> std::io::Result<()> {
    run_named(Background { worker, thread: Named::BACKGROUND }, window, job, done)
}

/// `run` on the thread `on.thread` names, which shutdown waits for or not.
pub fn run_named<T: Send + 'static>(
    on: Background,
    window: &AppWindow,
    job: impl FnOnce() -> T + Send + 'static,
    done: impl FnOnce(&AppWindow, Result<T, Panicked>) + 'static,
) -> std::io::Result<()> {
    let task = Task {
        job: Box::new(move || {
            let finished = catch_unwind(AssertUnwindSafe(job)).map_err(|payload| Panicked::logged(payload.as_ref()));
            Box::new(finished) as Payload
        }),
        done: Box::new(move |window, payload| match payload.downcast::<Result<T, Panicked>>() {
            Ok(result) => done(window, *result),
            // run() pairs each job with its own done, so the types always match
            Err(_) => log::error!("a background result came back as the wrong type"),
        }),
    };
    on.worker.start_named(on.thread, window, task)
}

/// What a waiting screen does when its work ends: `done` with the result, or `failed` when the
/// work couldn't be started or broke off.
pub struct Ending<D, F> {
    pub done: D,
    pub failed: F,
}

/// `run` for work a screen waits for: when the work can't be started (logged, as `what`) or breaks
/// off, `outcome.failed` runs in place of `outcome.done`, so the screen stops waiting and can say so.
pub fn run_or<T: Send + 'static, D, F>(
    worker: &dyn Worker,
    window: &AppWindow,
    (what, job): (&str, impl FnOnce() -> T + Send + 'static),
    outcome: Ending<D, F>,
) where
    D: FnOnce(&AppWindow, T) + 'static,
    F: Fn(&AppWindow) + 'static,
{
    let Ending { done, failed } = outcome;
    let failed = Rc::new(failed);
    let broke_off = failed.clone();
    let started = run(worker, window, job, move |window, finished| match finished {
        Ok(result) => done(window, result),
        Err(Panicked) => broke_off(window),
    });
    if let Err(error) = started {
        log::error!("{what} couldn't be started: {error}");
        failed(window);
    }
}

thread_local! {
    /// What to do with each running job's result, by job number; only the UI thread uses it.
    static WAITING: RefCell<HashMap<u64, Done>> = RefCell::default();
}

static NEXT_JOB: AtomicU64 = AtomicU64::new(0);

/// The app's worker: one thread per job.
pub struct ThreadWorker {
    running: RefCell<Vec<JoinHandle<()>>>,
    /// how long `join` waits for the work still running
    join_limit: Duration,
}

impl Default for ThreadWorker {
    fn default() -> Self {
        Self::waiting_at_most(JOIN_LIMIT)
    }
}

impl ThreadWorker {
    /// A worker whose `join` gives up after `join_limit`; the app's waits for `JOIN_LIMIT`.
    pub fn waiting_at_most(join_limit: Duration) -> Self {
        Self { running: RefCell::default(), join_limit }
    }
}

impl Worker for ThreadWorker {
    fn start(&self, window: &AppWindow, job: Job, done: Done) -> std::io::Result<()> {
        self.start_named(Named::BACKGROUND, window, Task { job, done })
    }

    fn start_named(&self, thread: Named, window: &AppWindow, task: Task) -> std::io::Result<()> {
        let id = NEXT_JOB.fetch_add(1, Ordering::Relaxed);
        WAITING.with(|waiting| waiting.borrow_mut().insert(id, task.done));
        let weak = window.as_weak();
        let job = task.job;
        let spawned = thread::Builder::new().name(thread.name.into()).spawn(move || {
            let payload = job();
            if let Err(error) = slint::invoke_from_event_loop(move || hand_back(id, &weak, payload)) {
                log::info!("a background result arrived after the window closed: {error}");
            }
        });
        match spawned {
            Ok(handle) => {
                let mut running = self.running.borrow_mut();
                running.retain(|handle| !handle.is_finished());
                if thread.at_quit == AtQuit::Wait {
                    running.push(handle);
                }
                Ok(())
            }
            Err(error) => {
                WAITING.with(|waiting| waiting.borrow_mut().remove(&id));
                Err(error)
            }
        }
    }

    fn join(&self) {
        let deadline = Instant::now() + self.join_limit;
        let mut running = self.running.take();
        while !running.is_empty() && Instant::now() < deadline {
            running = join_finished(running);
            if !running.is_empty() {
                thread::sleep(Duration::from_millis(50));
            }
        }
        if !running.is_empty() {
            log::warn!("{} background job(s) still running at shutdown are left to end with the app", running.len());
        }
    }
}

/// On the UI thread: runs job `id`'s `done` with its result, if the window is still open.
fn hand_back(id: u64, weak: &slint::Weak<AppWindow>, payload: Payload) {
    let done = WAITING.with(|waiting| waiting.borrow_mut().remove(&id));
    if let (Some(window), Some(done)) = (super::alive(weak), done) {
        done(&window, payload);
    }
}

/// Joins the threads that have finished (logging any that panicked) and returns the others.
fn join_finished(running: Vec<JoinHandle<()>>) -> Vec<JoinHandle<()>> {
    let (finished, left): (Vec<_>, Vec<_>) = running.into_iter().partition(JoinHandle::is_finished);
    for handle in finished {
        if handle.join().is_err() {
            log::error!("background work panicked");
        }
    }
    left
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
