// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:31.516106
//+ Revised: 	2026/10/29 00:32:02.088866
// -----------------------------------------------------------------------------

use std::path::Path;

use slint::SharedString;

use crate::app::state::SetupReason;
use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::i18n;
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

pub fn show_error(window: &AppWindow, error: &AppError) {
    window.set_status_message(SharedString::new());
    window.set_error_message(i18n::error_message(i18n::current(), error).into());
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
