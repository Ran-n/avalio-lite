// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:53.266960
//+ Revised: 	2026/10/28 15:53:08.850877
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, Model};

use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::app::SharedDialogs;
use crate::error::AppError;
use crate::gradebook::pupils;
use crate::i18n::{self, Message};
use crate::ui::{AppWindow, Gradebook};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let gradebook = window.global::<Gradebook>();

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_add_new_pupil(move |name| {
        let Some(window) = weak.upgrade() else { return };
        let class_id = current_class_id(&window);
        run(&window, &shared, "add pupil", |connection| {
            pupils::add_new_pupil(connection, class_id, &name)?;
            Ok(Refresh::All { select: None })
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_add_existing_pupil(move |index| {
        let Some(window) = weak.upgrade() else { return };
        let class_id = current_class_id(&window);
        run(&window, &shared, "add existing pupil", |connection| {
            let candidates = pupils::pupils_not_in_class(connection, class_id)?;
            let pupil =
                usize::try_from(index).ok().and_then(|index| candidates.get(index)).ok_or(AppError::NotEditable)?;
            pupils::add_existing_pupil(connection, class_id, pupil.id)?;
            Ok(Refresh::All { select: None })
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_rename_pupil(move |pupil_id, name| {
        let Some(window) = weak.upgrade() else { return };
        run(&window, &shared, "rename pupil", |connection| {
            pupils::rename_pupil(connection, i64::from(pupil_id), &name)?;
            Ok(Refresh::Nothing)
        });
    });

    let (weak, shared, dialogs) = (window.as_weak(), state.clone(), dialogs.clone());
    gradebook.on_remove_pupil(move |pupil_id| {
        let Some(window) = weak.upgrade() else { return };
        if !dialogs.confirm(&removal_question(&window, pupil_id)) {
            return;
        }
        let class_id = current_class_id(&window);
        run(&window, &shared, "remove pupil", |connection| {
            pupils::remove_from_class(connection, class_id, i64::from(pupil_id))?;
            Ok(Refresh::All { select: None })
        });
    });
}

fn removal_question(window: &AppWindow, pupil_id: i32) -> String {
    let gradebook = window.global::<Gradebook>();
    let name = gradebook.get_rows().iter().find(|row| row.pupil_id == pupil_id).map(|row| row.name).unwrap_or_default();
    let class = gradebook.get_class_name();
    i18n::text(i18n::current(), &Message::ConfirmRemovePupil { pupil: &name, class: &class })
}
