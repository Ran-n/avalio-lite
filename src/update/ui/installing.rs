// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 03:55:27.464308
//+ Revised: 	2026/11/23 23:19:31.504179
// -----------------------------------------------------------------------------

//! Update and restart: the download on its own thread, the verified file put in place of the
//! program, the window closed as every quit closes it, and the new version started. A failure at
//! any step is shown, and nothing is replaced after a Cancel or quit.

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use slint::ComponentHandle;

use crate::i18n::{self, Message, UpdateMessage};
use crate::ui::{AppWindow, Update, UpdateBanner};
use crate::update::check::Offer;
use crate::update::download::{self, Fetch, Progress, Saving};
use crate::update::release::UpdateError;
use crate::update::target::current_version;
use crate::window::view;
use crate::window::worker::{self, AtQuit, Background, Named, Panicked};

use super::updater::{Running, Updater};
use super::view::show;

/// A download about to start: the offer, the exe it replaces, and the flag a Cancel or quit sets.
struct Placing {
    offer: Offer,
    exe: PathBuf,
    cancel: Arc<AtomicBool>,
}

/// Update and restart: download and verify on its own thread (`avalio-lite-update-download`), then
/// replace the program, close as every quit does, and start the new one.
pub(super) fn install(window: &AppWindow, updater: &Updater) {
    let Some(offer) = updater.offer() else { return };
    if updater.inner.borrow().download.is_some() {
        return;
    }
    let Some(exe) = updater.updates.exe.clone() else {
        log::error!(target: "avalio_lite::update", "this copy can't tell where it runs from, so it can't replace itself");
        let error = UpdateError::Replace { path: "avalio-lite.exe".to_owned(), denied: false };
        show_failure(window, &offer, &error);
        return;
    };
    view::clear_messages(window);
    let placing = Placing { offer, exe, cancel: Arc::new(AtomicBool::new(false)) };
    let finished = Arc::new(AtomicBool::new(false));
    let running = Running {
        version: placing.offer.version().clone(),
        cancel: placing.cancel.clone(),
        finished: finished.clone(),
    };
    updater.inner.borrow_mut().download = Some(running);
    show(window, updater);
    log::info!(target: "avalio_lite::update", "downloading avalio-lite {}", placing.offer.version());
    start_download(window, updater, placing, finished);
}

/// Starts the download on its own thread; its result comes back to `downloaded`. A thread that
/// can't start is a failed download, shown here.
fn start_download(window: &AppWindow, updater: &Updater, placing: Placing, finished: Arc<AtomicBool>) {
    let updates = &updater.updates;
    let (http, keys, folder, limits) =
        (updates.http.clone(), updates.keys.clone(), updates.downloads.clone(), updates.limits);
    let (job_offer, job_cancel) = (placing.offer.clone(), placing.cancel.clone());
    let progress = progress_reporter(window, placing.offer.version().to_string());
    let job = move || {
        let fetch = Fetch { http: &http, offer: &job_offer, keys: &keys };
        let fetched = download::download(&fetch, &Saving { folder: &folder, limits, cancel: &job_cancel }, progress);
        finished.store(true, Ordering::SeqCst);
        fetched
    };
    let (offer, shared) = (placing.offer.clone(), updater.clone());
    let done = move |window: &AppWindow, fetched: Result<Result<PathBuf, UpdateError>, Panicked>| {
        let fetched = fetched.unwrap_or(Err(UpdateError::BrokeOff));
        downloaded(window, &shared, &placing, fetched);
    };
    let on = Background {
        worker: updater.updates.worker.as_ref(),
        thread: Named { name: "avalio-lite-update-download", at_quit: AtQuit::Abandon },
    };
    let started = worker::run_named(on, window, job, done);
    if let Err(error) = started {
        log::error!(target: "avalio_lite::update", "the download couldn't be started: {error}");
        updater.inner.borrow_mut().download = None;
        show(window, updater);
        show_failure(window, &offer, &UpdateError::BrokeOff);
    }
}

/// Sends the download's progress to the banner on the UI thread; a late report after the download
/// ended (or with no event loop, in the headless tests) is dropped.
fn progress_reporter(window: &AppWindow, version: String) -> impl FnMut(Progress) + Send + 'static {
    let weak = window.as_weak();
    move |progress| {
        let version = version.clone();
        let sent = weak.upgrade_in_event_loop(move |window| {
            let update = window.global::<Update>();
            if update.get_banner() == UpdateBanner::Downloading {
                let text = i18n::text(
                    i18n::current(),
                    &Message::Update(UpdateMessage::Downloading { version: &version, progress: Some(progress) }),
                );
                update.set_banner_text(text.into());
            }
        });
        if let Err(error) = sent {
            log::debug!(target: "avalio_lite::update", "progress not shown: {error}");
        }
    }
}

/// A finished download: the download is no longer under way, then the verified file replaces the
/// program and the app restarts, or the failure is shown.
fn downloaded(window: &AppWindow, updater: &Updater, placing: &Placing, fetched: Result<PathBuf, UpdateError>) {
    let offer = &placing.offer;
    // told apart by its own flag, not its version: Cancel then Update again downloads the same one,
    // and the cancelled download's late result must not end the new one
    let still_ours =
        updater.inner.borrow().download.as_ref().is_some_and(|running| Arc::ptr_eq(&running.cancel, &placing.cancel));
    if still_ours {
        updater.inner.borrow_mut().download = None;
    }
    let placed = fetched.and_then(|file| place(updater, placing, file));
    match placed {
        Ok(()) => close_for_restart(window, updater, placing),
        Err(UpdateError::Cancelled) => {
            log::info!(target: "avalio_lite::update", "the download was cancelled; nothing was installed");
            show(window, updater);
        }
        Err(error) => {
            log::error!(target: "avalio_lite::update", "the update to {} failed: {error}", offer.version());
            show(window, updater);
            show_failure(window, offer, &error);
        }
    }
}

/// Replaces the program with the verified download. A Cancel or quit during the hash check must not
/// replace it: then the file is removed and nothing is replaced.
fn place(updater: &Updater, placing: &Placing, file: PathBuf) -> Result<(), UpdateError> {
    if placing.cancel.load(Ordering::SeqCst) {
        if let Err(error) = std::fs::remove_file(&file) {
            log::warn!(target: "avalio_lite::update", "couldn't remove the cancelled download: {error}");
        }
        return Err(UpdateError::Cancelled);
    }
    log::info!(target: "avalio_lite::update", "avalio-lite {} downloaded and verified", placing.offer.version());
    (updater.updates.replace)(&placing.exe, &file)
}

/// The new version is in place: notes this version, sets the program to start after the quit, and
/// closes the window as its close button does, so the normal quit path runs.
fn close_for_restart(window: &AppWindow, updater: &Updater, placing: &Placing) {
    log::info!(target: "avalio_lite::update", "avalio-lite {} is installed; restarting", placing.offer.version());
    // the start that set up the database couldn't note its version (no settings were saved
    // yet), so a copy updated in its first session would take the new version for a first
    // run and never say "Updated to"; noting it here covers that
    if let Err(error) = updater.state.borrow_mut().note_version(&current_version().to_string()) {
        log::warn!("couldn't note the version before the update: {error}");
    }
    updater.inner.borrow_mut().restart = Some(placing.exe.clone());
    // closing the window as its close button does runs the normal quit path (its place is
    // noted, it hides); `launch` then starts the new version
    window.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);
    if let Err(error) = slint::quit_event_loop() {
        log::info!("no event loop to leave for the update: {error}");
    }
}

/// Shows a failed update in the message area, with the offer's release page.
fn show_failure(window: &AppWindow, offer: &Offer, error: &UpdateError) {
    let version = offer.version().to_string();
    let message = Message::Update(UpdateMessage::Failed { version: &version, error, page: &offer.release.page });
    view::show_error_text(window, i18n::text(i18n::current(), &message));
}
