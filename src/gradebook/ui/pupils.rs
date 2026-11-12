// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.266960
//+ Revised: 	2026/11/12 14:39:52.730390
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, Model, SharedString};

use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::error::AppError;
use crate::gradebook::pupils;
use crate::i18n::{self, Message};
use crate::ui::{AppWindow, Gradebook};
use crate::window::{SharedDialogs, alive, with_window1, with_window2};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let gradebook = window.global::<Gradebook>();

    let shared = state.clone();
    let weak = window.as_weak();
    gradebook.on_add_new_pupil(move |first_name: SharedString, surname: SharedString| {
        alive(&weak).is_some_and(|window| {
            let class_id = current_class_id(&window);
            run(&window, &shared, "add pupil", |connection| {
                pupils::add_new_pupil(connection, class_id, &first_name, &surname)?;
                Ok(Refresh::All { select: None })
            })
        })
    });

    let shared = state.clone();
    gradebook.on_add_existing_pupil(with_window1(window, move |window, index| {
        let class_id = current_class_id(window);
        run(window, &shared, "add existing pupil", |connection| {
            let candidates = pupils::pupils_not_in_class(connection, class_id)?;
            let pupil =
                usize::try_from(index).ok().and_then(|index| candidates.get(index)).ok_or(AppError::NotEditable)?;
            pupils::add_existing_pupil(connection, class_id, pupil.id)?;
            Ok(Refresh::All { select: None })
        });
    }));

    let (shared, asker) = (state.clone(), dialogs.clone());
    gradebook.on_delete_pupil_in_no_class(with_window1(window, move |window, index| {
        let name = usize::try_from(index)
            .ok()
            .and_then(|index| window.global::<Gradebook>().get_addable_pupils().row_data(index))
            .map(|pupil| pupil.name);
        let Some(name) = name else {
            run(window, &shared, "delete pupil in no class", |_| Err(AppError::NotEditable));
            return;
        };
        if !asker.confirm(&i18n::text(i18n::current(), &Message::ConfirmDeletePupil { pupil: &name })) {
            return;
        }
        let class_id = current_class_id(window);
        run(window, &shared, "delete pupil in no class", |connection| {
            let candidates = pupils::pupils_not_in_class(connection, class_id)?;
            let pupil =
                usize::try_from(index).ok().and_then(|index| candidates.get(index)).ok_or(AppError::NotEditable)?;
            pupils::delete_pupil_in_no_class(connection, pupil.id)?;
            Ok(Refresh::Sheet)
        });
    }));

    let shared = state.clone();
    gradebook.on_set_first_name(with_window2(window, move |window, pupil_id, text: SharedString| {
        run(window, &shared, "set first name", |connection| {
            pupils::set_first_name(connection, i64::from(pupil_id), &text)?;
            Ok(Refresh::PupilName(i64::from(pupil_id)))
        });
    }));

    let shared = state.clone();
    gradebook.on_set_surname(with_window2(window, move |window, pupil_id, text: SharedString| {
        run(window, &shared, "set surname", |connection| {
            pupils::set_surname(connection, i64::from(pupil_id), &text)?;
            Ok(Refresh::PupilName(i64::from(pupil_id)))
        });
    }));

    let (shared, dialogs) = (state.clone(), dialogs.clone());
    gradebook.on_remove_pupil(with_window1(window, move |window, pupil_id| {
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

/// The question before removing the pupil, or `None` when the sheet no longer shows them.
fn removal_question(window: &AppWindow, pupil_id: i32) -> Option<String> {
    let gradebook = window.global::<Gradebook>();
    let name = gradebook.get_rows().iter().find(|row| row.pupil_id == pupil_id).map(|row| row.full_name)?;
    let class = gradebook.get_class_name();
    Some(i18n::text(i18n::current(), &Message::ConfirmRemovePupil { pupil: &name, class: &class }))
}
