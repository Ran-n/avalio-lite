// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.893101
//+ Revised: 	2026/11/12 14:39:55.679372
// -----------------------------------------------------------------------------

//! The update bar and the Updates card: looking for a newer release on the worker, by itself (at
//! start, then whenever a day has passed, looked at hourly) or on request, and what to show; and
//! Update now: download, verify, put in place, and leave the app so the new version can start.

use std::cell::RefCell;
use std::fs;
use std::path::PathBuf;
use std::rc::Rc;
use std::time::Duration;

use chrono::Utc;
use ed25519_dalek::VerifyingKey;
use slint::{ComponentHandle, SharedString};

use crate::i18n::{self, Message, UpdateProblem};
use crate::ui::{AppWindow, Update};
use crate::update::download::{self, verify_file};
use crate::update::install::{
    Copy, Installation, Pending, asset_for, clean_leftovers, new_exe, swap_portable, writable,
};
use crate::update::signature::file_hash;
use crate::update::source::SharedSource;
use crate::update::{Asset, Release, UpdateError, current_version, due, newer};
use crate::window::worker::{self, Panicked, SharedWorker};
use crate::window::{SharedDialogs, SharedState, report_failure, view, with_window, with_window1};

/// How often the app looks whether a day has passed since the last check.
const LOOK_EVERY: Duration = Duration::from_secs(3600);

/// Where updates come from and go to.
pub struct Updates {
    pub source: SharedSource,
    /// the release key's public half: a download must be signed with it
    pub key: VerifyingKey,
    /// this copy of the app; `None` when it can't tell, and then Update now opens the release page
    pub installation: Option<Installation>,
    /// where an installed copy downloads the installer (and every copy the signature)
    pub downloads: PathBuf,
}

/// What the update screens need: the app state, the dialogs (for the browser), the worker, where
/// updates come from and go to, the newer release found, and what to start once the app closes.
#[derive(Clone)]
pub struct Updater {
    state: SharedState,
    dialogs: SharedDialogs,
    worker: SharedWorker,
    updates: Rc<Updates>,
    found: Rc<RefCell<Option<Release>>>,
    pending: Rc<RefCell<Option<Pending>>>,
}

impl Updater {
    pub fn new(state: &SharedState, dialogs: &SharedDialogs, worker: &SharedWorker, updates: Updates) -> Self {
        Self {
            state: state.clone(),
            dialogs: dialogs.clone(),
            worker: worker.clone(),
            updates: Rc::new(updates),
            found: Rc::default(),
            pending: Rc::default(),
        }
    }

    /// The newer release the bar is showing.
    pub fn found(&self) -> Option<Release> {
        self.found.borrow().clone()
    }

    /// What to start once the app has shut down: a downloaded and verified update.
    pub fn pending(&self) -> Option<Pending> {
        self.pending.borrow().clone()
    }

    /// Keeps why the update couldn't start, for the next start to say; after shutdown, when the
    /// window is gone.
    pub fn not_started(&self, reason: &str) {
        if let Err(error) = self.state.borrow_mut().set_update_failed(Some(reason.to_owned())) {
            log::error!("couldn't keep why the update didn't start ({reason}): {error}");
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Asked {
    /// by itself: failures are only logged, and a hidden version stays hidden
    Automatically,
    /// Check now: every result is said
    ByTheUser,
}

pub fn wire(window: &AppWindow, updater: &Updater) {
    let update = window.global::<Update>();

    let shared = updater.clone();
    update.on_check_now(with_window(window, move |window| look(window, &shared, Asked::ByTheUser)));

    let shared = updater.clone();
    update.on_set_automatic(with_window1(window, move |window, on: bool| {
        let saved = shared.state.borrow_mut().set_check_updates(on);
        match saved {
            Ok(()) => window.global::<Update>().set_automatic(on),
            Err(error) => {
                report_failure!(window, "saving the update setting", &error);
            }
        }
    }));

    let shared = updater.clone();
    update.on_whats_new(with_window(window, move |window| open_page(window, &shared)));

    let shared = updater.clone();
    update.on_update_now(with_window(window, move |window| update_now(window, &shared)));

    let shared = updater.clone();
    update.on_hide(with_window(window, move |window| {
        let Some(release) = shared.found() else { return };
        let skipped = shared.state.borrow_mut().skip_update(&release.version.to_string());
        if let Err(error) = skipped {
            report_failure!(window, "hiding the update", &error);
            return;
        }
        window.global::<Update>().set_notice(SharedString::new());
    }));
}

/// What an earlier run left to say or clean up, once the settings file is read: the files an update
/// left, "Updated to …" after an update, or why the last one couldn't start. The real app only.
pub fn after_start(window: &AppWindow, updater: &Updater) {
    if let Some(installation) = &updater.updates.installation {
        clean_leftovers(&installation.exe, &updater.updates.downloads);
    }
    let failed = updater.state.borrow().preferences().update_failed;
    if let Some(reason) = failed {
        log::warn!("the last update couldn't start: {reason}");
        view::show_error_text(window, i18n::text(i18n::current(), &Message::UpdateNotStarted));
        if let Err(error) = updater.state.borrow_mut().set_update_failed(None) {
            log::warn!("couldn't clear the note of the failed update: {error}");
        }
    }
    let version = current_version().to_string();
    let noted = updater.state.borrow_mut().note_version(&version);
    match noted {
        // a first run (nothing noted yet) is no update
        Ok(Some(_)) => {
            log::info!("updated to avalio-lite {version}");
            view::show_status(window, i18n::text(i18n::current(), &Message::UpdatedTo { version: &version }));
        }
        Ok(None) => {}
        Err(error) => log::warn!("couldn't note the version that ran: {error}"),
    }
}

/// The update bar in the current language, after the language changed: it was worded in the old
/// one when the new version was found. A hidden (empty) bar stays hidden.
pub fn reword(window: &AppWindow, updater: &Updater) {
    let update = window.global::<Update>();
    if update.get_notice().is_empty() {
        return;
    }
    if let Some(release) = updater.found() {
        let version = release.version.to_string();
        update.set_notice(i18n::text(i18n::current(), &Message::UpdateAvailable { version: &version }).into());
    }
}

/// Shows the saved "Check for updates automatically" setting; once the settings file is read.
pub fn show_setting(window: &AppWindow, updater: &Updater) {
    window.global::<Update>().set_automatic(updater.state.borrow().preferences().check_updates);
}

/// Looks for a new version now if one is due, then hourly whether a day has passed: the real app
/// only (the headless tests call `look_if_due` themselves). The timer stops with the window.
pub fn start_looking(window: &AppWindow, updater: &Updater) -> slint::Timer {
    look_if_due(window, updater);
    let timer = slint::Timer::default();
    let (weak, shared) = (window.as_weak(), updater.clone());
    timer.start(slint::TimerMode::Repeated, LOOK_EVERY, move || {
        if let Some(window) = crate::window::alive(&weak) {
            look_if_due(&window, &shared);
        }
    });
    timer
}

/// Looks for a new version if automatic checks are on and the last was a day ago or more.
pub fn look_if_due(window: &AppWindow, updater: &Updater) {
    let preferences = updater.state.borrow().preferences();
    if preferences.check_updates && due(preferences.last_update_check, Utc::now()) {
        look(window, updater, Asked::Automatically);
    }
}

fn look(window: &AppWindow, updater: &Updater, asked: Asked) {
    let update = window.global::<Update>();
    if asked == Asked::ByTheUser {
        update.set_busy(true);
        view::clear_messages(window);
    }
    let source = updater.updates.source.clone();
    let shared = updater.clone();
    let started = worker::run(
        updater.worker.as_ref(),
        window,
        move || source.latest(),
        move |window, latest| match latest {
            Ok(latest) => looked(window, &shared, asked, latest),
            Err(Panicked) => not_run(window, asked),
        },
    );
    if let Err(error) = started {
        log::error!("looking for a new version couldn't be started: {error}");
        not_run(window, asked);
    }
}

/// Work that couldn't start, or broke off: the card stops waiting; like a failed automatic check,
/// an automatic one is only logged.
fn not_run(window: &AppWindow, asked: Asked) {
    window.global::<Update>().set_busy(false);
    if asked == Asked::ByTheUser {
        view::show_error_text(window, i18n::text(i18n::current(), &Message::BackgroundFailed));
    }
}

fn looked(window: &AppWindow, updater: &Updater, asked: Asked, latest: Result<Release, UpdateError>) {
    let update = window.global::<Update>();
    update.set_busy(false);
    // a failed check counts too, so a broken network brings one try a day, not one an hour
    if let Err(error) = updater.state.borrow_mut().set_last_update_check(Utc::now()) {
        log::warn!("couldn't note when updates were last looked for: {error}");
    }
    let release = match latest {
        Ok(release) => release,
        Err(error) => {
            log::warn!("looking for a new version failed: {error}");
            if asked == Asked::ByTheUser {
                // looking only goes online, so every failure is one of the problems
                let problem = error.problem().unwrap_or(UpdateProblem::BadAnswer);
                view::show_error_text(window, i18n::text(i18n::current(), &Message::UpdateCheckFailed { problem }));
            }
            return;
        }
    };
    let current = current_version();
    let version = release.version.to_string();
    if !newer(&release, &current) {
        log::info!("avalio-lite {current} is up to date (latest {version})");
        *updater.found.borrow_mut() = None;
        update.set_notice(SharedString::new());
        if asked == Asked::ByTheUser {
            let current = current.to_string();
            view::show_status(window, i18n::text(i18n::current(), &Message::UpToDate { version: &current }));
        }
        return;
    }
    log::info!("avalio-lite {version} is available");
    let hidden = updater.state.borrow().preferences().skipped_update.as_deref() == Some(version.as_str());
    *updater.found.borrow_mut() = Some(release);
    if hidden && asked == Asked::Automatically {
        return;
    }
    update.set_notice(i18n::text(i18n::current(), &Message::UpdateAvailable { version: &version }).into());
}

fn open_page(window: &AppWindow, updater: &Updater) {
    let Some(release) = updater.found() else { return };
    if let Err(error) = updater.dialogs.open_link(&release.page) {
        log::warn!("couldn't open {}: {error}", release.page);
        view::show_error_text(window, i18n::text(i18n::current(), &Message::LinkFailed { url: &release.page }));
    }
}

/// Update now: the file this copy needs is downloaded and verified on the worker (and, for a
/// portable copy, swapped in); then the app closes, and `launch` starts the new version.
fn update_now(window: &AppWindow, updater: &Updater) {
    let Some(release) = updater.found() else { return };
    let Some(installation) = updater.updates.installation.clone() else {
        open_page(window, updater);
        return;
    };
    let language = i18n::current();
    let (asset, sig) = match asset_for(&release, &installation.copy) {
        Ok((asset, sig)) => (asset.clone(), sig.clone()),
        Err(error) => {
            log::warn!("can't update from {}: {error}", release.version);
            let text = match &error {
                UpdateError::MissingAsset(file) => Message::UpdateFileMissing { file },
                // asset_for only ever misses a file; any other error reads as a failed download
                other => Message::UpdateDownloadFailed { problem: other.problem().unwrap_or(UpdateProblem::BadAnswer) },
            };
            view::show_error_text(window, i18n::text(language, &text));
            return;
        }
    };
    if let Copy::Portable(exe) = &installation.copy
        && let Some(folder) = exe.parent()
        && let Err(error) = writable(folder)
    {
        log::warn!("the portable copy's folder {} can't be written to: {error}", folder.display());
        view::show_error_text(window, i18n::text(language, &Message::UpdateFolderReadOnly));
        return;
    }
    let update = window.global::<Update>();
    update.set_busy(true);
    view::clear_messages(window);
    log::info!("downloading avalio-lite {}", release.version);
    let (key, downloads) = (updater.updates.key, updater.updates.downloads.clone());
    let job = move || fetch_and_place(&installation.copy, &asset, &sig, &key, &downloads);
    let shared = updater.clone();
    let started = worker::run(updater.worker.as_ref(), window, job, move |window, placed| match placed {
        Ok(placed) => ready(window, &shared, placed),
        Err(Panicked) => not_run(window, Asked::ByTheUser),
    });
    if let Err(error) = started {
        log::error!("the update couldn't be started: {error}");
        not_run(window, Asked::ByTheUser);
    }
}

/// Off the UI thread: download the file and its signature and verify them. Whatever went wrong,
/// nothing downloaded and unverified is left behind. A portable exe is swapped in later, on the UI
/// thread (`ready`): done here, a window closed meanwhile would replace the exe with no restart, or
/// the process could end between the swap's two renames and leave no exe at all.
fn fetch_and_place(
    copy: &Copy,
    asset: &Asset,
    sig: &Asset,
    key: &VerifyingKey,
    downloads: &std::path::Path,
) -> Result<Pending, UpdateError> {
    fs::create_dir_all(downloads).map_err(|error| UpdateError::Disk(format!("{}: {error}", downloads.display())))?;
    let file = match copy {
        Copy::Installed => downloads.join(&asset.name),
        Copy::Portable(exe) => new_exe(exe),
    };
    let sig_file = downloads.join(&sig.name);
    let agent = download::agent();
    let fetched = download::download(&agent, asset, &file)
        .and_then(|()| download::download(&agent, sig, &sig_file))
        .and_then(|()| verify_file(key, &file, &sig_file, &asset.name));
    if let Err(error) = fetched {
        discard(&file);
        discard(&sig_file);
        return Err(error);
    }
    match copy {
        Copy::Installed => {
            // noted so the installer is checked again just before it runs (it waits in a temp folder)
            let hash = file_hash(&file).map_err(|error| UpdateError::Disk(format!("{}: {error}", file.display())))?;
            Ok(Pending::RunInstaller { installer: file, hash })
        }
        Copy::Portable(exe) => Ok(Pending::StartNew(exe.clone())),
    }
}

/// Removes a download that failed or didn't verify, if it is there; a failure is logged, and the
/// next start's clean-up tries again.
fn discard(file: &std::path::Path) {
    if let Err(error) = fs::remove_file(file)
        && error.kind() != std::io::ErrorKind::NotFound
    {
        log::warn!("couldn't remove the failed download {}: {error}", file.display());
    }
}

/// Back on the UI thread: close the app for the new version to start, or say what went wrong.
fn ready(window: &AppWindow, updater: &Updater, fetched: Result<Pending, UpdateError>) {
    let language = i18n::current();
    // the verified portable exe takes the running one's place now, just before the app closes
    let placed = fetched.and_then(|pending| match &pending {
        Pending::StartNew(exe) => swap_portable(exe).map(|()| pending),
        Pending::RunInstaller { .. } => Ok(pending),
    });
    match placed {
        Ok(pending) => {
            log::info!("the update is ready; the app closes for it: {pending:?}");
            // the start that set up the database couldn't note its version (no settings were saved
            // yet), so a copy updated in its first session would take the new version for a first
            // run and never say "Updated to"; noting it here covers that
            let version = current_version().to_string();
            if let Err(error) = updater.state.borrow_mut().note_version(&version) {
                log::warn!("couldn't note the version before the update: {error}");
            }
            *updater.pending.borrow_mut() = Some(pending);
            // closing the window runs the normal quit path; `launch` then starts the update
            if let Err(error) = window.hide() {
                log::warn!("couldn't hide the window for the update: {error}");
            }
            if let Err(error) = slint::quit_event_loop() {
                log::info!("no event loop to leave for the update: {error}");
            }
        }
        Err(error) => {
            window.global::<Update>().set_busy(false);
            let text = match &error {
                UpdateError::NotVerified(problem) => {
                    log::error!("the downloaded update failed its signature check: {problem}");
                    i18n::text(language, &Message::UpdateNotVerified)
                }
                UpdateError::Swap(reason) => {
                    log::error!("the new version couldn't be put in place: {reason}");
                    i18n::text(language, &Message::UpdateNotPlaced)
                }
                UpdateError::SwapLeftAside { old, detail } => {
                    log::error!("the new version couldn't be put in place, nor the old one back: {detail}");
                    i18n::text(language, &Message::UpdateLeftAside { path: &old.display().to_string() })
                }
                UpdateError::MissingAsset(file) => {
                    log::warn!("the release has no {file}");
                    i18n::text(language, &Message::UpdateFileMissing { file })
                }
                other => {
                    log::warn!("the update couldn't be downloaded: {other}");
                    let problem = other.problem().unwrap_or(UpdateProblem::BadAnswer);
                    i18n::text(language, &Message::UpdateDownloadFailed { problem })
                }
            };
            view::show_error_text(window, text);
        }
    }
}
