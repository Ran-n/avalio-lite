// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:31.491228
//+ Revised: 	2026/11/23 13:00:31.491228
// -----------------------------------------------------------------------------

//! Driving the real window: finding, clicking, typing and reading the controls.

use crate::common::Ui;
use crate::common::controls;
use avalio_lite::error::AppError;
use avalio_lite::i18n::{self, Language, Message};
use avalio_lite::ui::Gradebook;
use i_slint_backend_testing::ElementHandle;
use slint::platform::{Key, PointerEventButton, WindowEvent};
use slint::{ComponentHandle, SharedString};

pub(crate) const PIN: &str = "2468";

// ---------------------------------------------------------------------------------------------
// Driving the window

pub(crate) fn elements(ui: &Ui, label: &str) -> Vec<ElementHandle> {
    controls::labelled(&ui.session.window, label)
}

pub(crate) fn find(ui: &Ui, label: &str) -> ElementHandle {
    let found = elements(ui, label);
    assert_eq!(found.len(), 1, "expected one control labelled {label:?}, found {}", found.len());
    found.into_iter().next().unwrap()
}

pub(crate) fn shows(ui: &Ui, label: &str) -> bool {
    !elements(ui, label).is_empty()
}

/// Any visible text (plain `Text` elements included) equal to `text`.
pub(crate) fn shows_text(ui: &Ui, text: &str) -> bool {
    ElementHandle::find_by_accessible_label(&ui.session.window, text).next().is_some()
}

pub(crate) fn click_element(element: &ElementHandle) {
    element.mock_single_click(PointerEventButton::Left);
}

pub(crate) fn click(ui: &Ui, label: &str) {
    click_element(&find(ui, label));
}

/// A menu option, told apart from a column header with the same title by its hint.
pub(crate) fn click_menu_option(ui: &Ui, title: &str) {
    let options: Vec<ElementHandle> = elements(ui, title)
        .into_iter()
        .filter(|element| element.accessible_description().is_some_and(|hint| !hint.is_empty()))
        .collect();
    assert_eq!(options.len(), 1, "expected one menu option {title:?}");
    click_element(&options[0]);
}

pub(crate) fn click_nth(ui: &Ui, label: &str, index: usize) {
    click_element(&elements(ui, label)[index]);
}

pub(crate) fn key(ui: &Ui, text: impl Into<SharedString>) {
    let text = text.into();
    let window = ui.session.window.window();
    window.dispatch_event(WindowEvent::KeyPressed { text: text.clone() });
    window.dispatch_event(WindowEvent::KeyReleased { text });
}

pub(crate) fn with_control(ui: &Ui, text: &str) {
    let window = ui.session.window.window();
    let control: SharedString = Key::Control.into();
    window.dispatch_event(WindowEvent::KeyPressed { text: control.clone() });
    key(ui, text);
    window.dispatch_event(WindowEvent::KeyReleased { text: control });
}

pub(crate) fn type_text(ui: &Ui, text: &str) {
    for character in text.chars() {
        key(ui, character.to_string());
    }
}

/// Selects everything in the focused field and types `text` over it.
pub(crate) fn replace_text(ui: &Ui, text: &str) {
    with_control(ui, "a");
    if text.is_empty() {
        key(ui, Key::Backspace);
    }
    type_text(ui, text);
}

/// Clicks the field labelled `label` and types `text` over what is in it.
pub(crate) fn type_into(ui: &Ui, label: &str, text: &str) {
    click(ui, label);
    replace_text(ui, text);
}

pub(crate) fn press_enter(ui: &Ui) {
    key(ui, Key::Return);
}

pub(crate) fn gradebook(ui: &Ui) -> Gradebook<'_> {
    ui.session.window.global::<Gradebook>()
}

pub(crate) fn english(message: &Message) -> String {
    i18n::text(Language::English, message)
}

pub(crate) fn error_text(error: &AppError) -> String {
    i18n::error_message(Language::English, error)
}

// ---------------------------------------------------------------------------------------------
// Data

pub(crate) fn count(ui: &Ui, sql: &str) -> i64 {
    ui.session.state.borrow().connection().unwrap().query_row(sql, [], |row| row.get(0)).unwrap()
}
