// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.224094
//+ Revised: 	2026/11/12 14:39:51.499943
// -----------------------------------------------------------------------------

//! Starts the app and wires the features to the window: the composition root. The setup and
//! Settings screens live here; each feature's own wiring is in its `ui` module.

pub mod placement;
mod settings;
mod setup;

use std::cell::RefCell;
use std::process::ExitCode;
use std::rc::Rc;
use std::sync::Arc;

use clavren_client::Client;
use slint::ComponentHandle;

use crate::i18n::{self, Language, Message};
use crate::platform::config::{ConfigStore, Preferences};
use crate::platform::logging::{self, FileLogger};
use crate::state::{AppState, Startup};
use crate::ui::AppWindow;
use crate::update::source::GitHub;
use crate::update::ui::{Updater, Updates};
use crate::window::dialogs::NativeDialogs;
use crate::window::worker::{SharedWorker, ThreadWorker};
use crate::window::{SharedDialogs, SharedState, language, theme, view};
use crate::{backup, gradebook, licence, lock, shortcuts, spreadsheet, update};

/// A window wired to its state and showing the screen startup chose, not yet running.
pub struct Session {
    pub window: AppWindow,
    pub state: SharedState,
    /// runs the licence checks and update downloads; waited for at shutdown
    pub worker: SharedWorker,
    pub updater: Updater,
}

/// What the app uses of the world outside it, each behind a trait so the tests can stand in.
pub struct Ports {
    pub dialogs: SharedDialogs,
    /// the licence client (Clavren)
    pub licence: Arc<Client>,
    /// runs work off the UI thread
    pub worker: SharedWorker,
    /// where new versions are looked for, checked against and downloaded to
    pub updates: Updates,
}

pub fn run() -> ExitCode {
    run_with(Rc::new(NativeDialogs))
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
            dialogs.show_fatal(&i18n::text(i18n::current(), &Message::WindowFailed));
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

fn launch(logger: &'static FileLogger, dialogs: SharedDialogs, system_language: Language) -> Result<(), String> {
    let store = ConfigStore::default_location().map_err(|error| i18n::error_message(i18n::current(), &error))?;
    let client = licence::windows_client();
    let ports = Ports {
        dialogs,
        licence: client.clone(),
        worker: Rc::new(ThreadWorker::default()),
        updates: Updates {
            source: Arc::new(GitHub::configured()),
            key: update::release_key(),
            installation: update::windows::this_installation(),
            downloads: update::install::download_folder(),
        },
    };
    let session = open(store, logger, ports, system_language)?;
    let checks = licence::ui::start_checks(&session.window, &client);
    update::ui::after_start(&session.window, &session.updater);
    // looks for a new version now if one is due, then hourly; stops with the window
    let _looking = update::ui::start_looking(&session.window, &session.updater);
    if let Err(error) = &checks {
        log::error!("the daily licence check couldn't be started: {error}");
        view::show_error_text(&session.window, i18n::text(i18n::current(), &Message::BackgroundFailed));
    }
    let result = session.window.run();
    // a licence request still under way ends within its 15 s limit
    if let Ok(checks) = checks {
        checks.stop();
    }
    session.worker.join();
    let pending = session.updater.pending();
    if let Some(pending) = &pending {
        log::info!("the update starts once the app has shut down: {pending:?}");
    }
    session.state.borrow_mut().shutdown();
    // last, with the database closed: the installer replaces the exe, the new exe opens the database
    if let Some(pending) = pending
        && let Err(error) = update::windows::start(&pending)
    {
        session.updater.not_started(&error.to_string());
    }
    result.map_err(|error| error.to_string())
}

/// Builds the window with the saved language (or `system_language` until one is chosen) and theme,
/// wires it to the app state, and shows the startup screen: the sheet, the lock screen or setup. The
/// licence screen covers the sheet until `licence` says this computer is licensed.
pub fn open(
    store: ConfigStore,
    logger: &'static FileLogger,
    ports: Ports,
    system_language: Language,
) -> Result<Session, String> {
    let Ports { dialogs, licence, worker, updates } = ports;
    let preferences = saved_preferences(&store);
    let state = Rc::new(RefCell::new(AppState::new(store, logger)));
    let window = AppWindow::new().map_err(|error| error.to_string())?;
    language::apply(&window, preferences.language.unwrap_or(system_language));
    theme::apply_preference(&window, &preferences);
    setup::wire(&window, &state, &dialogs);
    let updater = Updater::new(&state, &dialogs, &worker, updates);
    let reword: settings::Reword = {
        let (updater, licence) = (updater.clone(), licence.clone());
        Rc::new(move |window: &AppWindow| {
            update::ui::reword(window, &updater);
            // the card's lines and any warning on the sheet, from the saved state (offline)
            licence::ui::show(window, &licence, &Ok(licence.status()));
        })
    };
    settings::wire(&window, &state, &dialogs, reword);
    backup::ui::wire(&window, &state, &dialogs);
    gradebook::ui::wire(&window, &state, &dialogs);
    spreadsheet::ui::wire(&window, &state, &dialogs);
    lock::ui::wire(&window, &state);
    shortcuts::ui::wire(&window, &state);
    licence::ui::wire(&window, &licence, &worker);
    update::ui::wire(&window, &updater);
    show_startup(&window, &state);
    update::ui::show_setting(&window, &updater);
    // offline: only the saved state and the cached token are read. After the startup screen, so a
    // warning it puts on the sheet isn't cleared by it.
    licence::ui::show(&window, &licence, &Ok(licence.status()));
    // before run() shows the window, so it appears in its place; every screen shares the window
    placement::apply(&window);
    Ok(Session { window, state, worker, updater })
}

/// Language and theme from the settings file, or the defaults (follow Windows) when there is none.
/// An unreadable file is reported later by `AppState::start`, so it is only logged here.
fn saved_preferences(store: &ConfigStore) -> Preferences {
    match store.load() {
        Ok(Some(config)) => config.preferences,
        Ok(None) => Preferences::default(),
        Err(error) => {
            log::warn!("settings file unreadable, following the Windows settings: {error}");
            Preferences::default()
        }
    }
}

fn show_startup(window: &AppWindow, state: &Rc<RefCell<AppState>>) {
    let startup = state.borrow_mut().start();
    match startup {
        Startup::Ready => {
            if let Some(config) = state.borrow().config() {
                view::show_main(window, config);
            }
            lock::ui::show_unprotected(window);
            gradebook::ui::show_database(window, state);
        }
        // the classes load once the PIN has opened the database
        Startup::Locked => {
            if let Some(config) = state.borrow().config() {
                view::show_main(window, config);
            }
            lock::ui::show_locked(window);
        }
        Startup::NeedsSetup(reason) => view::show_setup(window, &reason),
    }
}
