// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.516106
//+ Revised: 	2026/11/03 08:27:17.029395
// -----------------------------------------------------------------------------

use std::path::Path;
use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use crate::error::AppError;
use crate::i18n;
use crate::platform::config::{self, AppConfig};
use crate::state::SetupReason;
use crate::ui::{AppWindow, SetupNotice};

pub fn show_setup(window: &AppWindow, reason: &SetupReason) {
    let (notice, detail) = match reason {
        SetupReason::FirstRun => (SetupNotice::FirstRun, String::new()),
        SetupReason::DatabaseMissing(path) => (SetupNotice::DatabaseMissing, path.display().to_string()),
        SetupReason::ConfigUnreadable(message) => (SetupNotice::ConfigUnreadable, message.clone()),
        SetupReason::DatabaseUnusable(message) => (SetupNotice::DatabaseUnusable, message.clone()),
        SetupReason::LeftLocked(path) => (SetupNotice::LeftLocked, path.display().to_string()),
    };
    let default_folder = display(&config::default_data_dir());
    window.set_setup_notice(notice);
    window.set_setup_notice_detail(detail.into());
    window.set_setup_database_folder(default_folder.clone());
    window.set_setup_log_folder(default_folder);
    window.set_error_message(SharedString::new());
    window.set_setup_required(true);
}

pub fn show_main(window: &AppWindow, config: &AppConfig) {
    show_locations(window, config);
    window.set_error_message(SharedString::new());
    window.set_setup_required(false);
}

/// The database and log locations shown in Settings.
pub fn show_locations(window: &AppWindow, config: &AppConfig) {
    window.set_database_path(display(&config.database_path));
    window.set_log_folder(display(&config.log_dir));
}

/// The message of the last action is shown by whichever page is open (the sheet or Settings), so
/// it is always where the user is looking. The callers log the error with its details.
pub fn show_error(window: &AppWindow, error: &AppError) {
    show_error_text(window, i18n::error_message(i18n::current(), error));
}

/// A notice worded by the app itself rather than by an error, shown where errors are.
pub fn show_error_text(window: &AppWindow, text: String) {
    window.set_status_message(SharedString::new());
    window.set_error_message(text.into());
}

pub fn clear_error(window: &AppWindow) {
    window.set_error_message(SharedString::new());
}

pub fn clear_status(window: &AppWindow) {
    window.set_status_message(SharedString::new());
}

pub fn clear_messages(window: &AppWindow) {
    window.set_error_message(SharedString::new());
    window.set_status_message(SharedString::new());
}

pub fn show_status(window: &AppWindow, message: String) {
    window.set_error_message(SharedString::new());
    window.set_status_message(message.into());
}

pub fn display(path: &Path) -> SharedString {
    path.display().to_string().into()
}

/// A Slint list model holding `items`, for the properties that take one.
pub fn model<T: Clone + 'static>(items: Vec<T>) -> ModelRc<T> {
    ModelRc::from(Rc::new(VecModel::from(items)))
}
