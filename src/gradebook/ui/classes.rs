// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 23:00:52.882316
//+ Revised: 	2026/11/23 16:34:34.299419
// -----------------------------------------------------------------------------

//! The class list: opening, adding, renaming, duplicating and deleting a class, the last after asking. The
//! rules are in `gradebook::classes`.

use slint::{ComponentHandle, SharedString};

use super::items::from_ui_contents;
use super::view::{Refresh, current_class_id};
use super::{SharedState, run};
use crate::gradebook::classes;
use crate::i18n::{self, Message, SheetMessage};
use crate::ui::{AppWindow, Gradebook};
use crate::window::{SharedDialogs, alive, with_window, with_window1};

/// Wires the class callbacks of the sheet's sidebar.
pub fn wire(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    wire_select_class(window, state);
    wire_select_adjacent_class(window, state);
    wire_create_class(window, state);
    wire_rename_class(window, state);
    wire_duplicate_class(window, state);
    wire_delete_class(window, state, dialogs);
}

/// Opens the class the sidebar was clicked on.
fn wire_select_class(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_select_class(with_window1(window, move |window, class_id| {
        run(window, &shared, "open class", |_| Ok(Refresh::All { select: Some(i64::from(class_id)) }));
    }));
}

/// Opens the class before or after the open one, by `delta`.
fn wire_select_adjacent_class(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_select_adjacent_class(with_window1(window, move |window, delta| {
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
}

/// Creates a class with the name typed and opens it.
fn wire_create_class(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    let weak = window.as_weak();
    window.global::<Gradebook>().on_create_class(move |name: SharedString| {
        alive(&weak).is_some_and(|window| {
            run(&window, &shared, "create class", |connection| {
                let class_id = classes::create_class(connection, &name)?;
                Ok(Refresh::All { select: Some(class_id) })
            })
        })
    });
}

/// Renames the open class.
fn wire_rename_class(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_rename_class(with_window1(window, move |window, name: SharedString| {
        let class_id = current_class_id(window);
        run(window, &shared, "rename class", |connection| {
            classes::rename_class(connection, class_id, &name)?;
            Ok(Refresh::ClassName)
        });
    }));
}

/// Makes a copy of the open class, with the contents the sheet asked for, and opens the copy.
fn wire_duplicate_class(window: &AppWindow, state: &SharedState) {
    let shared = state.clone();
    window.global::<Gradebook>().on_duplicate_class(with_window1(window, move |window, contents| {
        let class_id = current_class_id(window);
        run(window, &shared, "duplicate class", |connection| {
            let copy = classes::duplicate_class(connection, class_id, from_ui_contents(contents))?;
            Ok(Refresh::All { select: Some(copy) })
        });
    }));
}

/// Deletes the open class, once confirmed.
fn wire_delete_class(window: &AppWindow, state: &SharedState, dialogs: &SharedDialogs) {
    let (shared, dialogs) = (state.clone(), dialogs.clone());
    window.global::<Gradebook>().on_delete_class(with_window(window, move |window| {
        let name = window.global::<Gradebook>().get_class_name();
        let question = i18n::text(i18n::current(), &Message::Sheet(SheetMessage::ConfirmDeleteClass { class: &name }));
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
