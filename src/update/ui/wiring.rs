// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 03:55:27.603056
//+ Revised: 	2026/11/23 03:55:27.603056
// -----------------------------------------------------------------------------

//! Wires the Update global's callbacks to the updater: one `wire_<callback>` per callback, each
//! doing what its button or switch asks. The callbacks run on the UI thread, in the order below.

use std::sync::atomic::Ordering;

use chrono::Local;
use slint::ComponentHandle;

use crate::ui::{AppWindow, Update};
use crate::update::check::Status;
use crate::window::{report_failure, with_window, with_window1};

use super::installing::install;
use super::schedule::{Asked, check_now, schedule};
use super::updater::Updater;
use super::view::show;

/// Wires every callback of the Update global to the updater.
pub fn wire(window: &AppWindow, updater: &Updater) {
    wire_check_now(window, updater);
    wire_set_automatic(window, updater);
    wire_install(window, updater);
    wire_cancel(window, updater);
    wire_skip(window, updater);
    wire_stop_skipping(window, updater);
    wire_dismiss(window, updater);
}

/// Check now: a check by the user, whose failure is shown.
fn wire_check_now(window: &AppWindow, updater: &Updater) {
    let shared = updater.clone();
    let callback = with_window(window, move |window| check_now(window, &shared, Asked::ByTheUser));
    window.global::<Update>().on_check_now(callback);
}

/// The automatic-check switch: saved, then the timer planned again and the view redrawn.
fn wire_set_automatic(window: &AppWindow, updater: &Updater) {
    let shared = updater.clone();
    let callback = with_window1(window, move |window, on: bool| {
        let saved = shared.state.borrow_mut().set_check_automatically(on);
        if let Err(error) = saved {
            report_failure!(window, "saving the update setting", &error);
        }
        schedule(window, &shared, None);
        show(window, &shared);
    });
    window.global::<Update>().on_set_automatic(callback);
}

/// Install: the download and the restart.
fn wire_install(window: &AppWindow, updater: &Updater) {
    let shared = updater.clone();
    let callback = with_window(window, move |window| install(window, &shared));
    window.global::<Update>().on_install(callback);
}

/// Cancel: the download's flag is set and its place cleared, so its result is dropped.
fn wire_cancel(window: &AppWindow, updater: &Updater) {
    let shared = updater.clone();
    let callback = with_window(window, move |window| {
        if let Some(running) = &shared.inner.borrow().download {
            log::info!(target: "avalio_lite::update", "cancelling the download of {}", running.version);
            running.cancel.store(true, Ordering::SeqCst);
        }
        shared.inner.borrow_mut().download = None;
        show(window, &shared);
    });
    window.global::<Update>().on_cancel(callback);
}

/// Skip: the offered version is saved as skipped, and the offer goes.
fn wire_skip(window: &AppWindow, updater: &Updater) {
    let shared = updater.clone();
    let callback = with_window(window, move |window| {
        let Some(offer) = shared.offer() else { return };
        let saved = shared.state.borrow_mut().set_skipped_version(Some(offer.version()));
        if let Err(error) = saved {
            report_failure!(window, "skipping the version", &error);
            return;
        }
        let mut inner = shared.inner.borrow_mut();
        let checked = inner.status.as_ref().map_or_else(Local::now, |(_, at)| *at);
        inner.status = Some((Status::Skipped(offer.version().clone()), checked));
        inner.offer = None;
        drop(inner);
        show(window, &shared);
    });
    window.global::<Update>().on_skip(callback);
}

/// Stop skipping: the skipped version is forgotten, and a check is asked for at once.
fn wire_stop_skipping(window: &AppWindow, updater: &Updater) {
    let shared = updater.clone();
    let callback = with_window(window, move |window| {
        let saved = shared.state.borrow_mut().set_skipped_version(None);
        if let Err(error) = saved {
            report_failure!(window, "no longer skipping the version", &error);
            return;
        }
        show(window, &shared);
        check_now(window, &shared, Asked::ByTheUser);
    });
    window.global::<Update>().on_stop_skipping(callback);
}

/// Dismiss: the banner is hidden for this version until the next start.
fn wire_dismiss(window: &AppWindow, updater: &Updater) {
    let shared = updater.clone();
    let callback = with_window(window, move |window| {
        let version = shared.offer().map(|offer| offer.version().clone());
        shared.inner.borrow_mut().dismissed = version;
        show(window, &shared);
    });
    window.global::<Update>().on_dismiss(callback);
}
