// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 21:04:22.334497
//+ Revised: 	2026/11/23 23:19:32.108325
// -----------------------------------------------------------------------------

//! The automatic update checks: when the timers fire, what plans them again and what drops them.
//! Slint's mock clock is moved on; nothing waits.

use crate::common::{Fixture, Ui};
use crate::screens::{next_version, release, update};
use avalio_lite::ui::UpdateBanner;
use avalio_lite::update::check::{AFTER_FAILURE, AFTER_SUCCESS, FIRST_CHECK_DELAY};
use avalio_lite::update::ui::{after_start, start};
use chrono::Utc;
use i_slint_core::animations::CURRENT_ANIMATION_DRIVER;
use i_slint_core::timers::TimerList;
use std::time::Duration;

/// The app with a database, checking by itself, as the real start leaves it.
fn started() -> Ui {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let ui = Ui::open_checking(fixture);
    start(&ui.session.window, &ui.session.updater);
    ui
}

/// Moves Slint's mock clock on by `time` and fires the timers due by then: the testing backend's own
/// `mock_elapsed_time`, which is only in builds from Slint's repository.
fn pass(time: Duration) {
    let now = CURRENT_ANIMATION_DRIVER.with(|driver| {
        let now = driver.current_tick() + time;
        driver.update_animations(now);
        now
    });
    TimerList::maybe_activate_timers(now);
}

const MOMENT: Duration = Duration::from_secs(1);

#[test]
fn the_first_check_comes_a_minute_after_the_start_and_the_next_a_day_later() {
    let ui = started();
    ui.releases.set(Ok(release(&next_version())));
    pass(FIRST_CHECK_DELAY - MOMENT);
    assert_eq!(ui.releases.calls(), 0, "not yet");
    pass(MOMENT * 2);
    assert_eq!(ui.releases.calls(), 1);
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
    pass(AFTER_SUCCESS - MOMENT * 2);
    assert_eq!(ui.releases.calls(), 1, "once a day");
    pass(MOMENT * 3);
    assert_eq!(ui.releases.calls(), 2);
}

#[test]
fn after_a_failed_check_the_next_comes_an_hour_later() {
    let ui = started();
    pass(FIRST_CHECK_DELAY + MOMENT);
    assert_eq!(ui.releases.calls(), 1, "it failed: nothing is set to answer");
    pass(AFTER_FAILURE + MOMENT);
    assert_eq!(ui.releases.calls(), 2);
}

#[test]
fn a_recent_check_puts_the_first_one_off() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let ui = Ui::open_checking(fixture);
    ui.session.state.borrow_mut().set_last_check(Utc::now()).unwrap();
    start(&ui.session.window, &ui.session.updater);
    pass(FIRST_CHECK_DELAY * 2);
    assert_eq!(ui.releases.calls(), 0);
}

#[test]
fn switching_automatic_checks_off_drops_the_timer_and_on_plans_it_again() {
    let ui = started();
    update(&ui).invoke_set_automatic(false);
    pass(FIRST_CHECK_DELAY * 2);
    assert_eq!(ui.releases.calls(), 0, "off");
    update(&ui).invoke_set_automatic(true);
    pass(FIRST_CHECK_DELAY + MOMENT);
    assert_eq!(ui.releases.calls(), 1, "on again");
}

#[test]
fn quitting_drops_the_timer() {
    let ui = started();
    ui.session.updater.quit();
    pass(FIRST_CHECK_DELAY * 2);
    assert_eq!(ui.releases.calls(), 0);
}

#[test]
fn a_build_from_source_plans_no_check() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let ui = Ui::open_from_source(fixture);
    start(&ui.session.window, &ui.session.updater);
    pass(AFTER_SUCCESS * 2);
    assert_eq!(ui.releases.calls(), 0);
}

#[test]
fn a_check_under_way_is_not_started_again() {
    let ui = started();
    ui.releases.set(Ok(release(&next_version())));
    ui.worker.holding.set(true);
    update(&ui).invoke_check_now();
    assert!(update(&ui).get_checking());
    update(&ui).invoke_check_now();
    assert_eq!(ui.releases.calls(), 1, "the second ask starts nothing new");
    ui.worker.release(&ui.session.window);
    assert!(!update(&ui).get_checking());
    assert_eq!(update(&ui).get_banner(), UpdateBanner::Offer);
}

/// `after_start` looks at this user's uninstall entry; a copy in a scratch folder isn't the
/// installed one, so it stays as it is.
#[test]
fn a_release_copy_that_was_not_installed_starts_without_an_error() {
    let fixture = Fixture::new();
    fixture.set_up().shutdown();
    let exe = fixture.path("avalio-lite.exe");
    std::fs::write(&exe, b"this version").unwrap();
    let ui = Ui::open_release(fixture, exe);
    after_start(&ui.session.window, &ui.session.updater);
    assert!(ui.session.window.get_error_message().is_empty());
}
