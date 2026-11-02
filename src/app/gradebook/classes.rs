// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:52.882316
//+ Revised: 	2026/11/02 08:27:48.962932
// -----------------------------------------------------------------------------

use slint::{ComponentHandle, SharedString};

use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::app::{SharedDialogs, alive, with_window, with_window1};
use crate::gradebook::classes;
use crate::i18n::{self, Message};
use crate::ui::{AppWindow, Gradebook};

pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let gradebook = window.global::<Gradebook>();

    let shared = state.clone();
    gradebook.on_select_class(with_window1(window, move |window, class_id| {
        run(window, &shared, "open class", |_| Ok(Refresh::All { select: Some(i64::from(class_id)) }));
    }));

    let shared = state.clone();
    gradebook.on_select_adjacent_class(with_window1(window, move |window, delta| {
        let current = current_class_id(window);
        run(window, &shared, "switch class", |connection| {
            let list = classes::list_classes(connection)?;
            let here = list.iter().position(|class| class.id == current);
            match classes::adjacent_index(list.len(), here, delta) {
                Some(index) => Ok(Refresh::All { select: Some(list[index].id) }),
                None => Ok(Refresh::Nothing),
            }
        });
    }));

    let shared = state.clone();
    let weak = window.as_weak();
    gradebook.on_create_class(move |name: SharedString| {
        alive(&weak).is_some_and(|window| {
            run(&window, &shared, "create class", |connection| {
                let class_id = classes::create_class(connection, &name)?;
                Ok(Refresh::All { select: Some(class_id) })
            })
        })
    });

    let shared = state.clone();
    gradebook.on_rename_class(with_window1(window, move |window, name: SharedString| {
        let class_id = current_class_id(window);
        run(window, &shared, "rename class", |connection| {
            classes::rename_class(connection, class_id, &name)?;
            Ok(Refresh::ClassName)
        });
    }));

    let (shared, dialogs) = (state.clone(), dialogs.clone());
    gradebook.on_delete_class(with_window(window, move |window| {
        let name = window.global::<Gradebook>().get_class_name();
        let question = i18n::text(i18n::current(), &Message::ConfirmDeleteClass { class: &name });
        if !dialogs.confirm(&question) {
            return;
        }
        let class_id = current_class_id(window);
        run(window, &shared, "delete class", |connection| {
            classes::delete_class(connection, class_id)?;
            Ok(Refresh::All { select: None })
        });
    }));
}
