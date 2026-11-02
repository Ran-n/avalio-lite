// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.266960
//+ Revised: 	2026/11/02 08:27:49.146744
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, Model, SharedString};

use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::app::{SharedDialogs, alive, with_window1, with_window2};
use crate::error::AppError;
use crate::gradebook::pupils;
use crate::i18n::{self, Message};
use crate::ui::{AppWindow, Gradebook};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let gradebook = window.global::<Gradebook>();

    let shared = state.clone();
    let weak = window.as_weak();
    gradebook.on_add_new_pupil(move |name: SharedString| {
        alive(&weak).is_some_and(|window| {
            let class_id = current_class_id(&window);
            run(&window, &shared, "add pupil", |connection| {
                pupils::add_new_pupil(connection, class_id, &name)?;
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
            .map(|pupil| pupil.name)
            .unwrap_or_default();
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
    gradebook.on_rename_pupil(with_window2(window, move |window, pupil_id, name: SharedString| {
        run(window, &shared, "rename pupil", |connection| {
            pupils::rename_pupil(connection, i64::from(pupil_id), &name)?;
            Ok(Refresh::PupilName(i64::from(pupil_id)))
        });
    }));

    let (shared, dialogs) = (state.clone(), dialogs.clone());
    gradebook.on_remove_pupil(with_window1(window, move |window, pupil_id| {
        if !dialogs.confirm(&removal_question(window, pupil_id)) {
            return;
        }
        let class_id = current_class_id(window);
        run(window, &shared, "remove pupil", |connection| {
            pupils::remove_from_class(connection, class_id, i64::from(pupil_id))?;
            Ok(Refresh::All { select: None })
        });
    }));
}

fn removal_question(window: &AppWindow, pupil_id: i32) -> String {
    let gradebook = window.global::<Gradebook>();
    let name = gradebook.get_rows().iter().find(|row| row.pupil_id == pupil_id).map(|row| row.name).unwrap_or_default();
    let class = gradebook.get_class_name();
    i18n::text(i18n::current(), &Message::ConfirmRemovePupil { pupil: &name, class: &class })
}
