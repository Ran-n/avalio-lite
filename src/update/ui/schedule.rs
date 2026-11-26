// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 03:55:27.395480
//+ Revised: 	2026/11/23 03:55:27.395480
// -----------------------------------------------------------------------------

//! The timers and the checks: the first check after the window opens, the next one planned after
//! each check (60 s, then 24 h after success, 1 h after failure), and the check's result as the
//! updater records and reports it. The startup housekeeping of an earlier run is here too.

use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local, Utc};
use slint::ComponentHandle;

use crate::i18n::{self, Message, UpdateMessage};
use crate::ui::AppWindow;
use crate::update::check::{self, Status, first_check_in, next_check_in};
use crate::update::install::clean_leftovers;
use crate::update::release::CheckError;
use crate::update::target::{Build, current_version};
use crate::window::view;
use crate::window::worker::{self, AtQuit, Background, Named, Panicked};

use super::updater::Updater;
use super::view::show;

/// Whether a check was asked for by the user or by the timer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Asked {
    /// by itself: a failure only changes the Settings line
    Automatically,
    /// Check now, or Stop skipping: a failure is shown too
    ByTheUser,
}

/// What an earlier run left: download files and the old updater's leftovers are cleaned up, an
/// installed copy's uninstall entry is brought to this version, and "Updated to …" (or that the
/// restart failed) is said once. The real app only.
pub fn after_start(window: &AppWindow, updater: &Updater) {
    let updates = &updater.updates;
    clean_leftovers(&updates.downloads, updates.exe.as_deref(), SystemTime::now());
    #[cfg(windows)]
    if let (Build::Release(_), Some(folder)) = (updates.build, updates.exe.as_deref().and_then(|exe| exe.parent())) {
        let mut entry = crate::update::windows::UninstallEntry::default();
        match crate::update::registration::bring_up_to_date(&mut entry, folder, &current_version()) {
            Ok(outcome) => log::info!(target: "avalio_lite::update", "uninstall entry: {outcome:?}"),
            Err(error) => log::warn!(target: "avalio_lite::update", "couldn't update the uninstall entry: {error}"),
        }
    }
    let failed = updater.state.borrow().preferences().update_failed;
    if let Some(reason) = failed {
        log::warn!(target: "avalio_lite::update", "the last update didn't restart: {reason}");
        view::show_error_text(window, i18n::text(i18n::current(), &Message::Update(UpdateMessage::NotRestarted)));
        if let Err(error) = updater.state.borrow_mut().set_update_failed(None) {
            log::warn!("couldn't clear the note of the failed restart: {error}");
        }
    }
    let version = current_version().to_string();
    let noted = updater.state.borrow_mut().note_version(&version);
    match noted {
        // a first run (nothing noted yet) is no update
        Ok(Some(_)) => {
            log::info!(target: "avalio_lite::update", "updated to avalio-lite {version}");
            view::show_status(
                window,
                i18n::text(i18n::current(), &Message::Update(UpdateMessage::Updated { version: &version })),
            );
        }
        Ok(None) => {}
        Err(error) => log::warn!("couldn't note the version that ran: {error}"),
    }
}

/// Starts the automatic checks: the first one `FIRST_CHECK_DELAY` after the window opens (or when
/// a day has passed since the last successful one), once the settings are read.
pub fn start(window: &AppWindow, updater: &Updater) {
    show(window, updater);
    schedule(window, updater, None);
}

/// (Re)plans the next automatic check: after `after`, or from the saved last check when `None`.
/// Any timer planned before is dropped. No timer when automatic checks are off.
pub(super) fn schedule(window: &AppWindow, updater: &Updater, after: Option<Duration>) {
    let mut inner = updater.inner.borrow_mut();
    inner.generation += 1;
    inner.timer = None;
    let preferences = updater.state.borrow().preferences();
    if !updater.updates.automatic || !preferences.check_automatically || updater.updates.build == Build::Source {
        return;
    }
    let last = preferences.last_check.and_then(|seconds| DateTime::<Utc>::from_timestamp(seconds, 0));
    let wait = after.unwrap_or_else(|| first_check_in(last, Utc::now()));
    let generation = inner.generation;
    let (weak, shared) = (window.as_weak(), updater.clone());
    let timer = slint::Timer::default();
    timer.start(slint::TimerMode::SingleShot, wait, move || {
        if shared.inner.borrow().generation != generation {
            return;
        }
        if let Some(window) = crate::window::alive(&weak) {
            automatic_check(&window, &shared);
        }
    });
    inner.timer = Some(timer);
    log::info!(target: "avalio_lite::update", "next automatic check in {wait:?}");
}

/// What the timer runs: a check whose failure only changes the Settings line. Public for the
/// headless tests, which have no event loop for the timer to fire on.
pub fn automatic_check(window: &AppWindow, updater: &Updater) {
    check_now(window, updater, Asked::Automatically);
}

/// Checks now, on its own thread (`avalio-lite-update`), abandoned at quit.
pub(super) fn check_now(window: &AppWindow, updater: &Updater, asked: Asked) {
    if updater.inner.borrow().checking {
        return;
    }
    if asked == Asked::ByTheUser {
        view::clear_messages(window);
    }
    updater.inner.borrow_mut().checking = true;
    show(window, updater);
    let sources = updater.updates.sources.clone();
    let build = updater.updates.build;
    let skipped =
        updater.state.borrow().preferences().skipped_version.and_then(|text| semver::Version::parse(&text).ok());
    let shared = updater.clone();
    let on = Background {
        worker: updater.updates.worker.as_ref(),
        thread: Named { name: "avalio-lite-update", at_quit: AtQuit::Abandon },
    };
    let started = worker::run_named(
        on,
        window,
        move || check::check(&sources, build, &current_version(), skipped.as_ref()),
        move |window, status| checked(window, &shared, asked, status),
    );
    if let Err(error) = started {
        log::error!(target: "avalio_lite::update", "the check couldn't be started: {error}");
        checked(window, updater, asked, Err(Panicked));
    }
}

/// A check's result: recorded, reported, the next check planned and the view redrawn.
fn checked(window: &AppWindow, updater: &Updater, asked: Asked, status: Result<Status, Panicked>) {
    // a check whose thread broke off counts as failed everywhere
    let status = status.unwrap_or_else(|Panicked| broke_off(updater));
    let succeeded = !matches!(status, Status::Failed(_));
    record(updater, &status);
    report(window, updater, asked, &status);
    schedule(window, updater, Some(next_check_in(succeeded)));
    show(window, updater);
}

/// A check whose thread broke off: every source failed with `BrokeOff`.
fn broke_off(updater: &Updater) -> Status {
    let reasons =
        updater.updates.sources.iter().map(|source| (source.name().to_owned(), CheckError::BrokeOff)).collect();
    Status::Failed(reasons)
}

/// Notes the result in the updater: the check is no longer running, the offer is set or cleared,
/// and the status line gets the result and its time.
fn record(updater: &Updater, status: &Status) {
    let mut inner = updater.inner.borrow_mut();
    inner.checking = false;
    match status {
        Status::Available(offer) => inner.offer = Some(offer.clone()),
        Status::UpToDate | Status::NoBuild(_) | Status::Skipped(_) | Status::SourceBuild => inner.offer = None,
        // a failed check never removes an offer already known
        Status::Failed(_) => {}
    }
    inner.status = Some((status.clone(), Local::now()));
}

/// Logs the result; a failure is said when the user asked for the check, and a successful check's
/// time is saved as the last check.
fn report(window: &AppWindow, updater: &Updater, asked: Asked, status: &Status) {
    match status {
        Status::Failed(reasons) => {
            log::warn!(target: "avalio_lite::update", "the check failed: {reasons:?}");
            if asked == Asked::ByTheUser {
                view::show_error_text(
                    window,
                    i18n::text(i18n::current(), &Message::Update(UpdateMessage::CheckFailed { reasons })),
                );
            }
        }
        other => {
            log::info!(target: "avalio_lite::update", "check finished: {other:?}");
            // only a successful check is noted
            if let Err(error) = updater.state.borrow_mut().set_last_check(Utc::now()) {
                log::warn!("couldn't note when updates were last checked: {error}");
            }
        }
    }
}
