// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 21:04:22.178711
//+ Revised: 	2026/11/23 21:04:22.178711
// -----------------------------------------------------------------------------

//! Running Slint's testing event loop until something has happened, for the tests with real threads.
//! Each is a binary of its own with one test: the loop belongs to one thread.

use std::cell::Cell;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// Runs the event loop until `done` says so; fails if that takes more than 10 s.
///
/// It asks the loop to quit only once. Unlike winit's, the testing backend's loop keeps every quit it
/// is asked for, and fires timers between taking the quit off its queue and ending. On a stalled
/// machine the timer was due again by then, and its second quit ended the next loop at once, before
/// the result it waited for.
pub fn run_until(what: &str, done: impl Fn() -> bool + 'static) {
    let deadline = Instant::now() + Duration::from_secs(10);
    let happened = Rc::new(Cell::new(false));
    let timer = slint::Timer::default();
    let seen = happened.clone();
    let quitting = Cell::new(false);
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(5), move || {
        if quitting.get() {
            return;
        }
        seen.set(done());
        if seen.get() || Instant::now() > deadline {
            quitting.set(true);
            slint::quit_event_loop().unwrap();
        }
    });
    slint::run_event_loop().unwrap();
    assert!(happened.get(), "{what} didn't happen within 10 s");
}

/// Runs the event loop for `time`, so anything handed to it meanwhile is dealt with.
pub fn run_for(time: Duration) {
    let end = Instant::now() + time;
    run_until("the wait", move || Instant::now() >= end);
}
