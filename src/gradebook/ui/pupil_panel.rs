// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/25 09:47:16.851540
//+ Revised: 	2026/11/25 11:43:04.599745
// -----------------------------------------------------------------------------

//! The pupil panel: one pupil's picture, name and number in the open class. Opened from the start of
//! their row, it takes the place of the column panel and the other way round.

use rusqlite::Connection;
use slint::{ComponentHandle, Model, SharedString};

use super::avatars::{Size, initials, pupil_avatar};
use super::view::{Refresh, current_class_id, ui_id};
use super::{SharedState, panel, run};
use crate::error::Result;
use crate::gradebook::{pictures, pupils};
use crate::i18n::{self, Message, SheetMessage};
use crate::ui::{AppWindow, Gradebook, PupilPanelItem};
use crate::window::dialogs::FileKind;
use crate::window::view::model;
use crate::window::{SharedDialogs, with_window, with_window1};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_open(window, state);
    wire_choose_picture(window, state, dialogs);
    wire_remove_picture(window, state, dialogs);
    window.global::<Gradebook>().on_close_pupil_panel(with_window(window, close));
}

fn wire_open(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_open_pupil_panel(with_window1(window, move |window, pupil_id| {
        run(window, &shared, "open pupil panel", |connection| {
            panel::close(window);
            show(window, connection, pupil_id)?;
            Ok(Refresh::Nothing)
        });
    }));
}

/// Asks for a photo and makes it the pupil's picture.
fn wire_choose_picture(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.global::<Gradebook>().on_choose_picture(with_window1(window, move |window, pupil_id| {
        let Some(path) = dialogs.pick_file(FileKind::Picture) else { return };
        run(window, &shared, "set picture", |connection| {
            pictures::set_picture(connection, i64::from(pupil_id), &pictures::from_file(&path)?)?;
            Ok(Refresh::Sheet)
        });
    }));
}

/// Deletes the pupil's picture, once confirmed.
fn wire_remove_picture(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.global::<Gradebook>().on_remove_picture(with_window1(window, move |window, pupil_id| {
        let name: SharedString =
            shown(window).filter(|pupil| pupil.id == pupil_id).map(|pupil| pupil.full_name).unwrap_or_default();
        let question = Message::Sheet(SheetMessage::ConfirmRemovePicture { pupil: &name });
        if !dialogs.confirm(&i18n::text(i18n::current(), &question)) {
            return;
        }
        run(window, &shared, "remove picture", |connection| {
            pictures::remove_picture(connection, i64::from(pupil_id))?;
            Ok(Refresh::Sheet)
        });
    }));
}

/// The pupil the panel is open on.
fn shown(window: &AppWindow) -> Option<PupilPanelItem> {
    window.global::<Gradebook>().get_panel_pupil().row_data(0)
}

/// Opens the panel on the pupil, or closes it when they aren't in the open class. The one-item model
/// is replaced each time, so the panel is made anew for another pupil.
fn show(window: &AppWindow, connection: &Connection, pupil_id: i32) -> Result<()> {
    let listed = pupils::class_list(connection, current_class_id(window))?;
    let found = listed.iter().zip(1..).find(|(pupil, _)| ui_id(pupil.id) == pupil_id);
    let items = match found {
        Some((pupil, number)) => vec![PupilPanelItem {
            id: pupil_id,
            full_name: pupil.full_name().into(),
            initials: initials(pupil).into(),
            number,
            avatar: pupil_avatar(connection, pupil.id, Size::Panel)?,
        }],
        None => Vec::new(),
    };
    window.global::<Gradebook>().set_panel_pupil(model(items));
    Ok(())
}

pub fn close(window: &AppWindow) {
    window.global::<Gradebook>().set_panel_pupil(model(Vec::new()));
}

/// After the sheet or a name changed: the same pupil, with their name, number and picture as they
/// are now.
pub fn keep_in_sync(window: &AppWindow, connection: &Connection) -> Result<()> {
    shown(window).map_or(Ok(()), |pupil| show(window, connection, pupil.id))
}
