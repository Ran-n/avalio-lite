// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:01.540332
//+ Revised: 	2026/11/24 17:54:03.282733
// -----------------------------------------------------------------------------

//! The driver's commands, one function each (`main`'s `dispatch` calls them by name). Each reads its
//! JSON fields, does what the command says to the window, the dialogs, the fake licence or the fake
//! forge, and answers with its reply fields. Nothing here reads the app's data: that is `state`.

use std::path::Path;

use crate::common::FakeDialogs;
use crate::common::controls;
use crate::common::licence::Mode;
use crate::common::win32;
use crate::{LICENCE, releases};
use avalio_lite::ui::AppWindow;
use i_slint_backend_testing::ElementHandle;
use i_slint_core::window::{PopupWindowLocation, WindowInner};
use serde_json::{Value, json};
use slint::ComponentHandle;

/// `ping`: the window's scale factor, which the tests convert their pixels by.
pub fn ping(window: &AppWindow) -> Result<Value, String> {
    Ok(json!({ "scale": window.window().scale_factor() }))
}

/// `size`: the logical size of the window.
pub fn size(window: &AppWindow, command: &Value) -> Result<Value, String> {
    let width = command["width"].as_f64().ok_or("missing width")? as f32;
    let height = command["height"].as_f64().ok_or("missing height")? as f32;
    // a window the app opened maximised (on a small screen) takes a size only once restored
    window.window().set_maximized(false);
    window.window().set_size(slint::LogicalSize::new(width, height));
    Ok(json!({}))
}

/// `minimum`: lowers the window's minimum logical size, as the app does on a screen too small.
pub fn minimum(window: &AppWindow, command: &Value) -> Result<Value, String> {
    window.set_fit_min_width(command["width"].as_f64().ok_or("missing width")? as f32);
    window.set_fit_min_height(command["height"].as_f64().ok_or("missing height")? as f32);
    Ok(json!({}))
}

/// `theme`: the light or dark look, without saving the choice.
pub fn theme(window: &AppWindow, command: &Value) -> Result<Value, String> {
    window.invoke_apply_theme(command["dark"].as_bool().ok_or("missing dark")?);
    Ok(json!({}))
}

/// `locate`: the controls labelled `label` (or the plain texts showing it), and the open popup's place.
pub fn locate(window: &AppWindow, command: &Value) -> Result<Value, String> {
    let label = command["label"].as_str().ok_or("missing label")?;
    let elements = if command["text"].as_bool() == Some(true) {
        controls::texts(window, label)
    } else {
        controls::labelled(window, label)
    };
    let found: Vec<Value> = elements.iter().map(element_json).collect();
    Ok(json!({ "found": found, "popup": popup_position(window) }))
}

fn element_json(element: &ElementHandle) -> Value {
    let (position, size) = (element.absolute_position(), element.size());
    let description = element.accessible_description().unwrap_or_default();
    let value = element.accessible_value().unwrap_or_default();
    json!({
        "x": position.x, "y": position.y, "width": size.width, "height": size.height,
        "description": description.as_str(), "value": value.as_str(),
    })
}

/// Where the open popup menu sits, if one is open (its controls are placed relative to it).
fn popup_position(window: &AppWindow) -> Option<Value> {
    WindowInner::from_pub(window.window()).active_popups().iter().find_map(|popup| match &popup.location {
        PopupWindowLocation::ChildWindow(at) => Some(json!({ "x": at.x, "y": at.y })),
        PopupWindowLocation::TopLevel(_) => None,
    })
}

/// `dialogs`: the answers for the next folder pickers, file pickers, save dialogs and questions.
pub fn dialogs(dialogs: &FakeDialogs, command: &Value) -> Result<Value, String> {
    for folder in command["folders"].as_array().into_iter().flatten() {
        dialogs.pick_folder_next(folder.as_str().map(Path::new));
    }
    for file in command["files"].as_array().into_iter().flatten() {
        dialogs.pick_file_next(file.as_str().map(Path::new));
    }
    for save in command["saves"].as_array().into_iter().flatten() {
        dialogs.save_file_next(save.as_str().map(Path::new));
    }
    for answer in command["answers"].as_array().into_iter().flatten() {
        dialogs.answer_next(answer.as_bool().ok_or("answers are true or false")?);
    }
    Ok(json!({}))
}

/// `licence`: how the fake Clavren answers from now on.
pub fn licence(command: &Value) -> Result<Value, String> {
    let mode = if command["offline"].as_bool() == Some(true) {
        Mode::Offline
    } else if let Some(code) = command["refuse"].as_str() {
        let status = command["status"].as_u64().ok_or("missing status")?;
        // the modes keep `'static` codes; a driver run makes only a handful
        Mode::Refuse(Box::leak(code.to_owned().into_boxed_str()), status as u16)
    } else {
        Mode::Normal
    };
    LICENCE.with(|cell| cell.borrow().as_ref().map(|licence| licence.server.set_mode(mode)));
    Ok(json!({}))
}

/// `release`: what the next update checks find, and whether they hang.
pub fn release(command: &Value) -> Result<Value, String> {
    let hangs = command["hang"].as_bool().unwrap_or(false);
    releases::set_mode(if hangs { releases::ForgeMode::Hangs } else { releases::ForgeMode::Answers });
    let text = command["version"].as_str().ok_or("missing version")?;
    releases::offer(text, command["serve"].as_str())?;
    Ok(json!({}))
}

/// `activate`: whether the window counts as the active one; answers what it was.
pub fn activate(window: &AppWindow, command: &Value) -> Result<Value, String> {
    let active = command["active"].as_bool().ok_or("missing active")?;
    let was = WindowInner::from_pub(window.window()).active();
    // straight to Slint: a posted WM_SETFOCUS would make winit replay the keys the user
    // holds down in another window (modifiers reach the app, so a held Ctrl could stick).
    // Only on a change: Slint sends the focused control a focus event on every one.
    if active != was {
        window.window().dispatch_event(slint::platform::WindowEvent::WindowActiveChanged(active));
    }
    Ok(json!({ "was": was }))
}

/// `type`: types the text into the focused control, as Windows messages to the window.
pub fn type_text(command: &Value) -> Result<Value, String> {
    let text = command["text"].as_str().ok_or("missing text")?;
    let hwnd = win32::app_window_of(std::process::id()).ok_or("the window isn't showing")?;
    win32::post_text(hwnd, text)?;
    Ok(json!({}))
}

/// `quit`: closes the window as the close button does (it notes its place and hides), then ends the
/// event loop, so the app shuts down.
pub fn quit(window: &AppWindow) -> Result<Value, String> {
    window.window().dispatch_event(slint::platform::WindowEvent::CloseRequested);
    slint::quit_event_loop().map_err(|error| error.to_string())?;
    Ok(json!({}))
}
