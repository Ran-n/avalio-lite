// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 01:29:26.446699
//+ Revised: 	2026/11/24 17:54:04.092485
// -----------------------------------------------------------------------------

//! The process around the window: logging and panics, `--version`, the real ports the updater
//! uses, and the start to the event loop and the clean shutdown after it. What the window shows
//! and wires is in `app` itself.

use std::process::ExitCode;
use std::rc::Rc;

use slint::ComponentHandle;

use super::{Ports, open};
use crate::i18n::{self, AppMessage, Language, Message};
use crate::platform::config::ConfigStore;
use crate::platform::logging::{self, FileLogger};
use crate::update::ui::Updates;
use crate::window::SharedDialogs;
use crate::window::dialogs::NativeDialogs;
use crate::window::view;
use crate::window::worker::ThreadWorker;
use crate::{licence, update};

/// The whole app with the native dialogs: see [`run_with`].
pub fn run() -> ExitCode {
    run_with(Rc::new(NativeDialogs))
}

/// `--version`: prints `avalio-lite X.Y.Z`. A release build is a windowed program with no console
/// of its own, so with no output to write to (started from a terminal, not piped) it writes to the
/// terminal it was started from.
pub fn print_version() -> ExitCode {
    #[cfg(windows)]
    {
        use windows_sys::Win32::System::Console::{
            ATTACH_PARENT_PROCESS, AttachConsole, GetStdHandle, STD_OUTPUT_HANDLE,
        };
        // SAFETY: plain calls with constants; a failed attach only means there is no terminal.
        unsafe {
            if GetStdHandle(STD_OUTPUT_HANDLE).is_null() {
                AttachConsole(ATTACH_PARENT_PROCESS);
            }
        }
    }
    use std::io::Write;
    let mut out = std::io::stdout();
    match writeln!(out, "{} {}", crate::about::DISPLAY_NAME, env!("CARGO_PKG_VERSION")).and_then(|()| out.flush()) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => ExitCode::FAILURE,
    }
}

/// The whole app: logging, the window, the event loop, then a clean shutdown.
pub fn run_with(dialogs: SharedDialogs) -> ExitCode {
    let logger = logging::install();
    log_panics();
    log::info!("starting avalio-lite {}", env!("CARGO_PKG_VERSION"));
    // Until the settings file is read, anything shown follows the Windows language.
    let system_language = Language::from_system();
    i18n::set_current(system_language);
    match launch(logger, dialogs.clone(), system_language) {
        Ok(()) => ExitCode::SUCCESS,
        Err(reason) => {
            log::error!("avalio-lite could not start: {reason}");
            dialogs.show_fatal(&i18n::text(i18n::current(), &Message::App(AppMessage::WindowFailed)));
            ExitCode::FAILURE
        }
    }
}

/// Puts every panic in the log, with where it happened: a release build has no console, so the
/// default report (to stderr) would go nowhere. The default report still runs after it.
fn log_panics() {
    let default = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let location = info.location().map_or_else(|| "an unknown place".to_owned(), ToString::to_string);
        let payload = info.payload();
        let text = payload
            .downcast_ref::<&str>()
            .map(|text| (*text).to_owned())
            .or_else(|| payload.downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "no message".to_owned());
        log::error!("panicked at {location}: {text}");
        log::logger().flush();
        default(info);
    }));
}

/// Opens the window, runs the event loop, then shuts down in order: the checks, the updater and the
/// worker stop, the database closes, and only then a pending update is started.
fn launch(logger: &'static FileLogger, dialogs: SharedDialogs, system_language: Language) -> Result<(), String> {
    let store = ConfigStore::default_location().map_err(|error| i18n::error_message(i18n::current(), &error))?;
    let client = licence::windows_client();
    let ports =
        Ports { dialogs, licence: client.clone(), worker: Rc::new(ThreadWorker::default()), updates: real_updates() };
    let session = open(store, logger, ports, system_language)?;
    let checks = licence::ui::start_checks(&session.window, &client);
    update::ui::after_start(&session.window, &session.updater);
    // the first automatic check a minute after the window opens, then daily; stops with the window
    update::ui::start(&session.window, &session.updater);
    if let Err(error) = &checks {
        log::error!("the daily licence check couldn't be started: {error}");
        view::show_error_text(
            &session.window,
            i18n::text(i18n::current(), &Message::App(AppMessage::BackgroundFailed)),
        );
    }
    let result = session.window.run();
    // a licence request still under way ends within its 15 s limit
    if let Ok(checks) = checks {
        checks.stop();
    }
    // a download is cancelled and waited for (at most 5 s); a check is abandoned
    session.updater.quit();
    session.worker.join();
    let restart = session.updater.restart();
    if let Some(program) = &restart {
        log::info!("the new version starts once the app has shut down: {}", program.display());
    }
    session.state.borrow_mut().shutdown();
    // last, with the database closed: the new version opens it
    if let Some(program) = restart
        && let Err(error) = update::install::relaunch(&program, &[])
    {
        session.updater.not_restarted(&error.to_string());
    }
    result.map_err(|error| error.to_string())
}

/// The updater's ports in the real app: the two forges, the built-in keys, this build's target and
/// where it runs from.
fn real_updates() -> Updates {
    let (sources, http) = update::source::configured();
    let keys = update::signature::trusted_keys().unwrap_or_else(|error| {
        // the built-in keys are constants a test proves valid; with none, every download is refused
        log::error!("no release key to check updates with: {error}");
        Vec::new()
    });
    let exe = std::env::current_exe()
        .inspect_err(|error| log::warn!("couldn't tell where the app runs from, so it can't update itself: {error}"))
        .ok();
    Updates {
        sources,
        http,
        keys,
        build: update::target::this_build(),
        exe,
        downloads: update::install::download_folder(),
        limits: update::download::Limits::default(),
        automatic: true,
        worker: Rc::new(ThreadWorker::default()),
        replace: update::install::replace,
    }
}
