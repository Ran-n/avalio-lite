// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.266960
//+ Revised: 	2026/11/23 16:34:34.503636
// -----------------------------------------------------------------------------

//! The pupils on the sheet: adding, renaming, taking out of the class, and deleting one who is in no class,
//! the last two after asking. The rules are in `gradebook::pupils`.

use slint::{ComponentHandle, Model, SharedString};

use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::error::{AppError, Result};
use crate::gradebook::model::Pupil;
use crate::gradebook::pupils;
use crate::i18n::{self, Message, SheetMessage};
use crate::ui::{AppWindow, Gradebook};
use crate::window::{SharedDialogs, alive, with_window1, with_window2};

/// Wires the pupil callbacks of the sheet.
pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_add_new_pupil(window, state);
    wire_add_existing_pupil(window, state);
    wire_delete_pupil_in_no_class(window, state, dialogs);
    wire_set_first_name(window, state);
    wire_set_surname(window, state);
    wire_remove_pupil(window, state, dialogs);
}

/// Adds the pupil typed in as a new one to the open class.
fn wire_add_new_pupil(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    let weak = window.as_weak();
    window.global::<Gradebook>().on_add_new_pupil(move |first_name: SharedString, surname: SharedString| {
        alive(&weak).is_some_and(|window| {
            let class_id = current_class_id(&window);
            run(&window, &shared, "add pupil", |connection| {
                pupils::add_new_pupil(connection, class_id, &first_name, &surname)?;
                Ok(Refresh::All { select: None })
            })
        })
    });
}

/// Adds a pupil from the list of those not in the open class.
fn wire_add_existing_pupil(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_add_existing_pupil(with_window1(window, move |window, index| {
        let class_id = current_class_id(window);
        run(window, &shared, "add existing pupil", |connection| {
            let candidates = pupils::pupils_not_in_class(connection, class_id)?;
            let pupil = pupil_at(&candidates, index)?;
            pupils::add_existing_pupil(connection, class_id, pupil.id)?;
            Ok(Refresh::All { select: None })
        });
    }));
}

/// Deletes a pupil who is in no class, once confirmed.
fn wire_delete_pupil_in_no_class(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, asker) = (state.clone(), dialogs.clone());
    window.global::<Gradebook>().on_delete_pupil_in_no_class(with_window1(window, move |window, index| {
        let name = usize::try_from(index)
            .ok()
            .and_then(|index| window.global::<Gradebook>().get_addable_pupils().row_data(index))
            .map(|pupil| pupil.name);
        let Some(name) = name else {
            run(window, &shared, "delete pupil in no class", |_| Err(AppError::NotEditable));
            return;
        };
        if !asker
            .confirm(&i18n::text(i18n::current(), &Message::Sheet(SheetMessage::ConfirmDeletePupil { pupil: &name })))
        {
            return;
        }
        let class_id = current_class_id(window);
        run(window, &shared, "delete pupil in no class", |connection| {
            let candidates = pupils::pupils_not_in_class(connection, class_id)?;
            let pupil = pupil_at(&candidates, index)?;
            pupils::delete_pupil_in_no_class(connection, pupil.id)?;
            Ok(Refresh::Sheet)
        });
    }));
}

/// Sets a pupil's first name as typed.
fn wire_set_first_name(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_set_first_name(with_window2(
        window,
        move |window, pupil_id, text: SharedString| {
            run(window, &shared, "set first name", |connection| {
                pupils::set_first_name(connection, i64::from(pupil_id), &text)?;
                Ok(Refresh::PupilName(i64::from(pupil_id)))
            });
        },
    ));
}

/// Sets a pupil's surname as typed.
fn wire_set_surname(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_set_surname(with_window2(window, move |window, pupil_id, text: SharedString| {
        run(window, &shared, "set surname", |connection| {
            pupils::set_surname(connection, i64::from(pupil_id), &text)?;
            Ok(Refresh::PupilName(i64::from(pupil_id)))
        });
    }));
}

/// Removes a pupil from the open class, once confirmed.
fn wire_remove_pupil(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.global::<Gradebook>().on_remove_pupil(with_window1(window, move |window, pupil_id| {
        let Some(question) = removal_question(window, pupil_id) else {
            run(window, &shared, "remove pupil", |_| Err(AppError::NotEditable));
            return;
        };
        if !dialogs.confirm(&question) {
            return;
        }
        let class_id = current_class_id(window);
        run(window, &shared, "remove pupil", |connection| {
            pupils::remove_from_class(connection, class_id, i64::from(pupil_id))?;
            Ok(Refresh::All { select: None })
        });
    }));
}

/// The pupil listed at `index` in the list of those not in the open class; an index the list
/// doesn't have is not editable.
fn pupil_at(candidates: &[Pupil], index: i32) -> Result<&Pupil> {
    usize::try_from(index).ok().and_then(|index| candidates.get(index)).ok_or(AppError::NotEditable)
}

/// The question before removing the pupil, or `None` when the sheet no longer shows them.
fn removal_question(window: &AppWindow, pupil_id: i32) -> Option<String> {
    let gradebook = window.global::<Gradebook>();
    let name = gradebook.get_rows().iter().find(|row| row.pupil_id == pupil_id).map(|row| row.full_name)?;
    let class = gradebook.get_class_name();
    Some(i18n::text(i18n::current(), &Message::Sheet(SheetMessage::ConfirmRemovePupil { pupil: &name, class: &class })))
}
