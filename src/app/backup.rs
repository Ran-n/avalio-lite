// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 09:42:12.835428
//+ Revised: 	2026/11/02 08:27:48.906387
// -----------------------------------------------------------------------------

//! The Backup card in Settings: exporting everything to an `.avalio` file, restoring one, and
//! adding the classes of one, with the inline prompt for a backup's own PIN.

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;

use chrono::Local;
use slint::{ComponentHandle, SharedString};

use crate::app::dialogs::FileKind;
use crate::app::{SharedDialogs, SharedState, gradebook, lock_screen, view, with_window};
use crate::backup::{self, OpenedBackup};
use crate::error::AppError;
use crate::i18n::{self, Message};
use crate::ui::{self, AppWindow};

/// What to do with a backup once it is open.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Use {
    Restore,
    AddClasses,
}

/// A backup waiting for its PIN in the card's prompt.
#[derive(Debug, Clone)]
struct Pending {
    purpose: Use,
    file: PathBuf,
}

type SharedPending = Rc<RefCell<Option<Pending>>>;

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_export(window, state, dialogs);
    let pending: SharedPending = Rc::default();
    wire_opening(window, state, dialogs, &pending);
    wire_prompt(window, state, dialogs, &pending);
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
                log::warn!("export backup failed: {error}");
                view::show_error(window, &error);
            }
        }
    }));
}

fn wire_opening(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs, pending: &SharedPending) {
    let global = window.global::<ui::Backup>();
    global.on_restore(pick_and_open(window, state, dialogs, pending, Use::Restore));
    global.on_add_classes(pick_and_open(window, state, dialogs, pending, Use::AddClasses));
}

/// A button's callback: the Open dialog, then the backup opened for `purpose`.
fn pick_and_open(
    window: &AppWindow,
    state: &SharedState,
    dialogs: &SharedDialogs,
    pending: &SharedPending,
    purpose: Use,
) -> impl FnMut() + 'static {
    let (shared, dialogs, pending) = (state.clone(), dialogs.clone(), pending.clone());
    with_window(window, move |window| {
        let Some(file) = dialogs.pick_file(FileKind::Backup) else { return };
        open(window, &shared, &dialogs, &pending, Pending { purpose, file }, None);
    })
}

fn wire_prompt(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs, pending: &SharedPending) {
    let (shared, dialogs, waiting) = (state.clone(), dialogs.clone(), pending.clone());
    window.global::<ui::Backup>().on_pin_entered(with_window(window, move |window| {
        let Some(backup) = waiting.borrow().clone() else {
            log::error!("a backup PIN was entered, but no backup is waiting for one");
            return;
        };
        let pin = window.global::<ui::Backup>().get_pin();
        open(window, &shared, &dialogs, &waiting, backup, Some(&pin));
    }));

    let waiting = pending.clone();
    window.global::<ui::Backup>().on_pin_cancelled(with_window(window, move |window| {
        if waiting.borrow_mut().take().is_some() {
            log::info!("the backup PIN prompt was closed");
        }
        close_prompt(window);
    }));
}

/// Opens the backup, then uses it; when it needs a PIN other than the app's, shows the prompt and
/// keeps it waiting (a wrong PIN keeps the prompt open, with the error under the field).
fn open(
    window: &AppWindow,
    state: &SharedState,
    dialogs: &SharedDialogs,
    pending: &SharedPending,
    backup: Pending,
    pin: Option<&str>,
) {
    let opened = state.borrow().open_backup(&backup.file, pin);
    match opened {
        Err(AppError::WrongPin) => {
            let global = window.global::<ui::Backup>();
            global.set_pin(SharedString::new());
            let error =
                if pin.is_some() { i18n::error_message(i18n::current(), &AppError::WrongPin) } else { String::new() };
            global.set_pin_error(error.into());
            global.set_pin_needed(true);
            view::clear_messages(window);
            *pending.borrow_mut() = Some(backup);
        }
        Err(error) => {
            pending.borrow_mut().take();
            close_prompt(window);
            log::warn!("opening the backup {} failed: {error}", backup.file.display());
            view::show_error(window, &error);
        }
        Ok(opened) => {
            pending.borrow_mut().take();
            close_prompt(window);
            match backup.purpose {
                Use::Restore => restore(window, state, dialogs, opened),
                Use::AddClasses => add_classes(window, state, opened),
            }
        }
    }
}

/// Asks once, with the backup's counts so the user sees it is the right file, then replaces
/// everything. Dropping `backup` on any path deletes its copy.
fn restore(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs, backup: OpenedBackup) {
    let contents = backup.contents();
    let file = backup.file_name();
    let question = Message::ConfirmRestore { file: &file, classes: contents.classes, pupils: contents.pupils };
    if !dialogs.confirm(&i18n::text(i18n::current(), &question)) {
        return;
    }
    let result = state.borrow_mut().restore_backup(backup);
    match result {
        Ok(()) => {
            // a reload that failed has its error on screen, which a success message would hide
            if gradebook::show_database(window, state) {
                view::show_status(window, i18n::text(i18n::current(), &Message::BackupRestored { file: &file }));
            }
        }
        Err(error) => {
            log::warn!("restore backup failed: {error}");
            view::show_error(window, &error);
            // a failed swap that couldn't reopen the database leaves it closed: ask for the PIN again
            if state.borrow().is_locked() {
                lock_screen::show_locked(window);
            }
        }
    }
}

/// Adds the backup's classes without asking, since nothing is deleted or replaced, then opens the
/// first one added and says what came in.
fn add_classes(window: &AppWindow, state: &SharedState, backup: OpenedBackup) {
    let file = backup.file_name();
    let result = state.borrow().add_backup_classes(backup);
    let summary = match result {
        Ok(summary) => summary,
        Err(error) => {
            log::warn!("add classes from a backup failed: {error}");
            view::show_error(window, &error);
            return;
        }
    };
    let language = i18n::current();
    let Some(&first) = summary.class_ids.first() else {
        view::show_status(window, i18n::text(language, &Message::BackupHasNoClasses));
        return;
    };
    if !gradebook::redraw_selecting(window, state, Some(first)) {
        return;
    }
    let mut status = i18n::text(
        language,
        &Message::BackupClassesAdded {
            classes: summary.class_ids.len(),
            pupils: summary.pupils,
            existing: summary.existing_pupils,
            file: &file,
        },
    );
    if !summary.renamed.is_empty() {
        status.push(' ');
        status.push_str(&i18n::text(language, &Message::BackupClassesRenamed { pairs: &summary.renamed }));
    }
    view::show_status(window, status);
}

/// Hides the backup PIN prompt and forgets what was typed in it.
fn close_prompt(window: &AppWindow) {
    let global = window.global::<ui::Backup>();
    global.set_pin_needed(false);
    global.set_pin(SharedString::new());
    global.set_pin_error(SharedString::new());
}

fn documents() -> PathBuf {
    dirs::document_dir().or_else(dirs::home_dir).unwrap_or_default()
}
