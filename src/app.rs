// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.224094
//+ Revised: 	2026/11/25 11:43:01.935625
// -----------------------------------------------------------------------------

//! Starts the app and wires the features to the window: the composition root. The setup and
//! Settings screens live here; each feature's own wiring is in its `ui` module. The process around
//! the window (logging, `--version`, the start and the clean shutdown) is in `lifecycle`.

mod lifecycle;
pub mod placement;
mod settings;
mod setup;

pub use lifecycle::{print_version, run, run_with};

use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;

use clavren_client::Client;
use slint::ComponentHandle;

use crate::i18n::Language;
use crate::lock::AttemptLimiter;
use crate::platform::config::{ConfigStore, Preferences};
use crate::platform::logging::FileLogger;
use crate::state::{AppState, Startup};
use crate::ui::AppWindow;
use crate::update::ui::{Updater, Updates};
use crate::window::worker::SharedWorker;
use crate::window::{SharedDialogs, SharedState, language, theme, view};
use crate::{about, backup, gradebook, licence, lock, pupil_view, shortcuts, spreadsheet, update};

/// A window wired to its state and showing the screen startup chose, not yet running.
pub struct Session {
    pub window: AppWindow,
    pub state: SharedState,
    /// runs the licence checks; waited for at shutdown
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
    let updater = Updater::new(&state, updates);
    wire_features(&window, &state, &dialogs, settings_reword(&updater, &licence));
    licence::ui::wire(&window, &licence, &worker);
    update::ui::wire(&window, &updater);
    show_screens(&window, &state, &updater, &licence);
    // before run() shows the window, so it appears in its place; every screen shares the window
    placement::apply(&window, preferences.window.as_ref());
    remember_placement_on_close(&window, &state);
    Ok(Session { window, state, worker, updater })
}

/// Wires the Settings screen and the features' screens. Settings gets `reword`, which redraws its
/// update and licence lines.
fn wire_features(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs, reword: settings::Reword) {
    settings::wire(window, state, dialogs, reword);
    backup::ui::wire(window, state, dialogs);
    about::ui::wire(window, dialogs);
    gradebook::ui::wire(window, state, dialogs);
    spreadsheet::ui::wire(window, state, dialogs);
    // the lock screen and the pupil view ask for the same PIN, so they share the count of wrong ones
    let wrong_pins = Rc::new(RefCell::new(AttemptLimiter::default()));
    lock::ui::wire(window, state, &wrong_pins);
    pupil_view::ui::wire(window, state, &wrong_pins);
    shortcuts::ui::wire(window, state);
}

/// What the Settings card redraws its update and licence lines with, from the saved state (offline).
fn settings_reword(updater: &Updater, licence: &Arc<Client>) -> settings::Reword {
    let (updater, licence) = (updater.clone(), licence.clone());
    Rc::new(move |window: &AppWindow| {
        update::ui::show(window, &updater);
        // the card's lines and any warning on the sheet, from the saved state (offline)
        licence::ui::show(window, &licence, &Ok(licence.status()));
    })
}

/// Shows the startup screen, then the update banner and the licence line on the sheet.
fn show_screens(window: &AppWindow, state: &SharedState, updater: &Updater, licence: &Arc<Client>) {
    show_startup(window, state);
    update::ui::show(window, updater);
    // offline: only the saved state and the cached token are read. After the startup screen, so a
    // warning it puts on the sheet isn't cleared by it.
    licence::ui::show(window, licence, &Ok(licence.status()));
}

/// Notes the window's place as it closes, by its close button, Alt+F4 or an update's restart (which
/// asks the window to close the same way).
fn remember_placement_on_close(window: &AppWindow, state: &SharedState) {
    let (weak, state) = (window.as_weak(), state.clone());
    window.window().on_close_requested(move || {
        if let Some(window) = weak.upgrade() {
            placement::remember(&window, &state);
        }
        slint::CloseRequestResponse::HideWindow
    });
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
    if let (Startup::Ready | Startup::Locked, Some(config)) = (&startup, state.borrow().config()) {
        view::show_main(window, config);
    }
    match startup {
        Startup::Ready => {
            lock::ui::show_unprotected(window);
            gradebook::ui::show_database(window, state);
        }
        // the classes load once the PIN has opened the database
        Startup::Locked => lock::ui::show_locked(window),
        Startup::NeedsSetup(reason) => view::show_setup(window, &reason),
    }
}
