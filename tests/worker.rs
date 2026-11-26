// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 12:17:33.982538
//+ Revised: 	2026/11/24 17:54:11.896491
// -----------------------------------------------------------------------------

//! The app's real worker and its daily licence check: work runs on its own thread, and its result
//! comes back to the window through the event loop. In a binary of its own, with one test: it
//! needs Slint's testing backend with an event loop, which belongs to one thread, and the headless
//! UI tests (tests/ui/) run without one.

mod common;

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::thread;
use std::time::Duration;

use avalio_lite::app::{self, Ports, Session};
use avalio_lite::i18n::Language;
use avalio_lite::licence;
use avalio_lite::ui::AppWindow;
use avalio_lite::ui::Licence;
use avalio_lite::update::source::FixedRelease;
use avalio_lite::update::target::Build;
use avalio_lite::window::worker::{self, Ending, Panicked, SharedWorker, ThreadWorker, Worker};
use chrono::TimeDelta;
use clavren_client::Client;
use common::event_loop::{run_for, run_until};
use common::licence::{FakeLicence, Mode};
use common::{FakeDialogs, Fixture, leaked_logger};
use slint::ComponentHandle;

/// The app on `fixture`'s database, its background work on `worker` and its licence from `client`.
fn open(fixture: &Fixture, worker: SharedWorker, client: Arc<Client>) -> Session {
    let updates = common::releases::updates(
        vec![Arc::new(FixedRelease::default())],
        common::releases::Running { build: Build::Release(common::releases::TARGET), exe: None, automatic: false },
        fixture.path("downloads"),
    );
    let ports = Ports { dialogs: Rc::new(FakeDialogs::default()), licence: client, worker, updates };
    app::open(fixture.store(), leaked_logger(), ports, Language::English).unwrap()
}

/// Starts `job` on `worker` and runs the event loop until its result is back.
fn result_of<T: Send + 'static>(
    worker: &ThreadWorker,
    session: &Session,
    job: impl FnOnce() -> T + Send + 'static,
) -> Result<T, Panicked> {
    let result = Rc::new(RefCell::new(None));
    let slot = result.clone();
    worker::run(worker, &session.window, job, move |_, finished| *slot.borrow_mut() = Some(finished)).unwrap();
    let waiting = result.clone();
    run_until("the result coming back", move || waiting.borrow().is_some());
    result.take().unwrap()
}

/// `run_until`'s own rule: a loop whose timer is held up leaves no quit behind for the next one.
fn a_held_up_wait_does_not_cut_the_next_one_short(worker: &ThreadWorker, session: &Session) {
    run_until("the held-up wait", || {
        thread::sleep(Duration::from_millis(20));
        true
    });
    let job = || thread::sleep(Duration::from_millis(50));
    assert_eq!(result_of(worker, session, job), Ok(()));
}

fn a_result_comes_back_from_another_thread(worker: &ThreadWorker, session: &Session) {
    let ui_thread = thread::current().id();
    let ran_on = result_of(worker, session, || (thread::current().id(), thread::current().name().map(str::to_owned)));
    let (thread, name) = ran_on.unwrap();
    assert_ne!(thread, ui_thread, "the work ran off the UI thread");
    assert_eq!(name.as_deref(), Some("background"));
    assert_eq!(result_of(worker, session, || 6 * 7), Ok(42));
}

fn work_that_panics_comes_back_as_panicked(worker: &ThreadWorker, session: &Session) {
    assert_eq!(result_of(worker, session, || -> i32 { panic!("a fixed message") }), Err(Panicked));
    let built = result_of(worker, session, || -> i32 { panic!("a built {}", "message") });
    assert_eq!(built, Err(Panicked));
    assert_eq!(result_of(worker, session, || -> i32 { std::panic::panic_any(7) }), Err(Panicked));
}

/// `run_or`: what a waiting screen uses, with one place to end the wait.
fn a_screen_is_told_when_its_work_broke_off(worker: &ThreadWorker, session: &Session) {
    let told = Rc::new(RefCell::new(Vec::new()));
    let (done, failed) = (told.clone(), told.clone());
    worker::run_or(
        worker,
        &session.window,
        ("the sum", || 1 + 1),
        Ending {
            done: move |_: &AppWindow, sum| done.borrow_mut().push(format!("done {sum}")),
            failed: move |_: &AppWindow| failed.borrow_mut().push("failed".to_owned()),
        },
    );
    let (done, failed) = (told.clone(), told.clone());
    worker::run_or(
        worker,
        &session.window,
        ("the sum", || -> i32 { panic!("it broke") }),
        Ending {
            done: move |_: &AppWindow, sum| done.borrow_mut().push(format!("done {sum}")),
            failed: move |_: &AppWindow| failed.borrow_mut().push("failed".to_owned()),
        },
    );
    let waiting = told.clone();
    run_until("both answers", move || waiting.borrow().len() == 2);
    told.borrow_mut().sort();
    assert_eq!(*told.borrow(), ["done 2", "failed"]);
}

fn shutdown_waits_for_work_under_way(worker: &ThreadWorker, session: &Session) {
    let finished = Arc::new(AtomicBool::new(false));
    let reached = Rc::new(Cell::new(false));
    let (flag, back) = (finished.clone(), reached.clone());
    let job = move || {
        thread::sleep(Duration::from_millis(200));
        flag.store(true, Ordering::SeqCst);
    };
    worker::run(worker, &session.window, job, move |_, _| back.set(true)).unwrap();
    worker.join();
    assert!(finished.load(Ordering::SeqCst), "join returned only once the work was done");
    worker.join();
    // its result was handed over before the thread ended, and is still dealt with
    let back = reached.clone();
    run_until("the result of the joined work", move || back.get());
}

fn shutdown_gives_up_on_work_that_takes_too_long(session: &Session) {
    let impatient = ThreadWorker::waiting_at_most(Duration::from_millis(60));
    let finished = Arc::new(AtomicBool::new(false));
    let flag = finished.clone();
    let (release, held) = mpsc::channel();
    // ends only when told to, so `join` returning at all means it gave up
    let job = move || {
        held.recv().unwrap();
        flag.store(true, Ordering::SeqCst);
    };
    worker::run(&impatient, &session.window, job, |_, _| {}).unwrap();
    impatient.join();
    assert!(!finished.load(Ordering::SeqCst), "the work is left to end by itself");
    release.send(()).unwrap();
    let done = finished.clone();
    run_until("the slow work ending", move || done.load(Ordering::SeqCst));
}

fn a_revocation_found_by_the_daily_check_reaches_the_window() {
    let computer = FakeLicence::activated();
    let client = computer.client_checking_every(Duration::from_millis(10));
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let session = open(&fixture, Rc::new(ThreadWorker::default()), client.clone());
    assert!(session.window.global::<Licence>().get_licensed());
    let checks = licence::ui::start_checks(&session.window, &client).unwrap();
    computer.server.set_mode(Mode::Refuse("licence_revoked", 403));
    computer.advance(TimeDelta::hours(25));
    let window = session.window.as_weak();
    run_until("the revocation showing", move || !window.unwrap().global::<Licence>().get_licensed());
    checks.stop();
    session.state.borrow_mut().shutdown();
}

fn a_result_arriving_after_the_window_closed_is_dropped(worker: &ThreadWorker) {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let session = open(&fixture, Rc::new(ThreadWorker::default()), FakeLicence::activated().client());
    let finished = Arc::new(AtomicBool::new(false));
    let reached = Rc::new(Cell::new(false));
    let (flag, back) = (finished.clone(), reached.clone());
    let job = move || {
        thread::sleep(Duration::from_millis(100));
        flag.store(true, Ordering::SeqCst);
    };
    worker::run(worker, &session.window, job, move |_, _| back.set(true)).unwrap();
    session.state.borrow_mut().shutdown();
    drop(session);
    let done = finished.clone();
    run_until("the work ending", move || done.load(Ordering::SeqCst));
    run_for(Duration::from_millis(100));
    assert!(!reached.get(), "nothing is done with a result once the window is gone");
}

#[test]
fn the_real_worker_hands_its_results_to_the_window_through_the_event_loop() {
    i_slint_backend_testing::init_integration_test_with_system_time();
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let session = open(&fixture, Rc::new(ThreadWorker::default()), FakeLicence::activated().client());
    let worker = ThreadWorker::default();

    a_held_up_wait_does_not_cut_the_next_one_short(&worker, &session);
    a_result_comes_back_from_another_thread(&worker, &session);
    work_that_panics_comes_back_as_panicked(&worker, &session);
    a_screen_is_told_when_its_work_broke_off(&worker, &session);
    shutdown_waits_for_work_under_way(&worker, &session);
    shutdown_gives_up_on_work_that_takes_too_long(&session);
    a_revocation_found_by_the_daily_check_reaches_the_window();
    a_result_arriving_after_the_window_closed_is_dropped(&worker);

    session.state.borrow_mut().shutdown();
}
