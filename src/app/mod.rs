// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.224094
//+ Revised: 	2026/11/03 08:27:15.657429
// -----------------------------------------------------------------------------

//! Starts the app and wires the features to the window: the composition root. The setup and
//! Settings screens live here; each feature's own wiring is in its `ui` module.

pub mod placement;
mod settings;
mod setup;

use std::cell::RefCell;
use std::process::ExitCode;
use std::rc::Rc;

use slint::ComponentHandle;

use crate::i18n::{self, Language, Message};
use crate::platform::config::{ConfigStore, Preferences};
use crate::platform::logging::{self, FileLogger};
use crate::state::{AppState, Startup};
use crate::ui::AppWindow;
use crate::window::dialogs::NativeDialogs;
use crate::window::{SharedDialogs, SharedState, language, theme, view};
use crate::{backup, gradebook, lock, shortcuts, spreadsheet};

/// A window wired to its state and showing the screen startup chose, not yet running.
pub struct Session {
    pub window: AppWindow,
    pub state: SharedState,
}

pub fn run() -> ExitCode {
    run_with(Rc::new(NativeDialogs))
}

/// The whole app: logging, the window, the event loop, then a clean shutdown.
pub fn run_with(dialogs: SharedDialogs) -> ExitCode {
    let logger = logging::install();
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

fn launch(logger: &'static FileLogger, dialogs: SharedDialogs, system_language: Language) -> Result<(), String> {
    let store = ConfigStore::default_location().map_err(|error| i18n::error_message(i18n::current(), &error))?;
    let session = open(store, logger, dialogs, system_language)?;
    let result = session.window.run();
    session.state.borrow_mut().shutdown();
    result.map_err(|error| error.to_string())
}

/// Builds the window with the saved language (or `system_language` until one is chosen) and theme,
/// wires it to the app state, and shows the startup screen: the sheet, the lock screen or setup.
pub fn open(
    store: ConfigStore,
    logger: &'static FileLogger,
    dialogs: SharedDialogs,
    system_language: Language,
) -> Result<Session, String> {
    let preferences = saved_preferences(&store);
    let state = Rc::new(RefCell::new(AppState::new(store, logger)));
    let window = AppWindow::new().map_err(|error| error.to_string())?;
    language::apply(&window, preferences.language.unwrap_or(system_language));
    theme::apply_preference(&window, &preferences);
    setup::wire(&window, &state, &dialogs);
    settings::wire(&window, &state, &dialogs);
    backup::ui::wire(&window, &state, &dialogs);
    gradebook::ui::wire(&window, &state, &dialogs);
    spreadsheet::ui::wire(&window, &state, &dialogs);
    lock::ui::wire(&window, &state);
    shortcuts::ui::wire(&window, &state);
    show_startup(&window, &state);
    // before run() shows the window, so it appears in its place; every screen shares the window
    placement::apply(&window);
    Ok(Session { window, state })
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
