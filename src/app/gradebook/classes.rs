// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:52.882316
//+ Revised: 	2026/10/28 15:53:08.718869
// -----------------------------------------------------------------------------

use slint::ComponentHandle;

use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::app::SharedDialogs;
use crate::gradebook::classes;
use crate::i18n::{self, Message};
use crate::ui::{AppWindow, Gradebook};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let gradebook = window.global::<Gradebook>();

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_select_class(move |class_id| {
        let Some(window) = weak.upgrade() else { return };
        run(&window, &shared, "open class", |_| Ok(Refresh::All { select: Some(i64::from(class_id)) }));
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_create_class(move |name| {
        let Some(window) = weak.upgrade() else { return };
        run(&window, &shared, "create class", |connection| {
            let class_id = classes::create_class(connection, &name)?;
            Ok(Refresh::All { select: Some(class_id) })
        });
    });

    let (weak, shared) = (window.as_weak(), state.clone());
    gradebook.on_rename_class(move |name| {
        let Some(window) = weak.upgrade() else { return };
        let class_id = current_class_id(&window);
        run(&window, &shared, "rename class", |connection| {
            classes::rename_class(connection, class_id, &name)?;
            Ok(Refresh::ClassList)
        });
    });

    let (weak, shared, dialogs) = (window.as_weak(), state.clone(), dialogs.clone());
    gradebook.on_delete_class(move || {
        let Some(window) = weak.upgrade() else { return };
        let name = window.global::<Gradebook>().get_class_name();
        let question = i18n::text(i18n::current(), &Message::ConfirmDeleteClass { class: &name });
        if !dialogs.confirm(&question) {
            return;
        }
        let class_id = current_class_id(&window);
        run(&window, &shared, "delete class", |connection| {
            classes::delete_class(connection, class_id)?;
            Ok(Refresh::All { select: None })
        });
    });
}
