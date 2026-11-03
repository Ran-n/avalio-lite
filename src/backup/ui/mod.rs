// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 09:42:12.835428
//+ Revised: 	2026/11/03 11:39:47.335866
// -----------------------------------------------------------------------------

//! The Backup card in Settings: exporting everything to an `.avalio` file, restoring one, and
//! adding the classes of one, with the inline prompt for a backup's own PIN.

mod restore;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use chrono::Local;
use slint::ComponentHandle;

use crate::backup::{self};
use crate::i18n::{self, Message};
use crate::ui::{self, AppWindow};
use crate::window::dialogs::FileKind;
use crate::window::report_failure;
use crate::window::{SharedDialogs, SharedState, view, with_window};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_export(window, state, dialogs);
    restore::wire(window, state, dialogs);
}

fn wire_export(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    // the folder of the last backup this session, where the next Save dialog opens
    let last_folder: Rc<RefCell<Option<PathBuf>>> = Rc::default();
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.global::<ui::Backup>().on_export_everything(with_window(window, move |window| {
        let start = last_folder.borrow().clone().unwrap_or_else(documents);
        let name = backup::default_file_name(Local::now().date_naive());
        let Some(destination) = dialogs.save_file(FileKind::Backup, &start, &name) else { return };
        *last_folder.borrow_mut() = destination.parent().map(Path::to_path_buf);
        match shared.borrow().export_backup(&destination) {
            Ok(()) => {
                let path = destination.display().to_string();
                view::show_status(window, i18n::text(i18n::current(), &Message::BackupExported { path: &path }));
            }
            Err(error) => {
                report_failure!(window, "export backup", &error);
            }
        }
    }));
}

fn documents() -> PathBuf {
    dirs::document_dir().or_else(dirs::home_dir).unwrap_or_default()
}
