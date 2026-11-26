// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 01:29:26.658046
//+ Revised: 	2026/11/24 17:54:04.295333
// -----------------------------------------------------------------------------

//! What the window's place keeps for the next start: how the window is shown as the app closes,
//! and the place it is noted as. A maximised or minimised window keeps the place it last had in
//! its normal size, so a restart returns to it.

use super::fit::{PREFERRED_HEIGHT, PREFERRED_WIDTH};
use crate::platform::config::WindowPlace;
use crate::ui::AppWindow;
use crate::window::SharedState;

/// How the window was shown as the app closed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Shown {
    /// The outer top-left corner in physical pixels and the content size in logical pixels.
    Normal {
        x: i32,
        y: i32,
        width: f32,
        height: f32,
    },
    /// The outer top-left corner of the maximised window, which tells the screen it was on.
    Maximized {
        x: i32,
        y: i32,
    },
    Minimized,
}

/// What to keep for the next start, given what was kept before; `None` keeps that unchanged. A
/// maximised window keeps the place it returns to (unknown to the app while maximised: the one
/// kept before, else the preferred size on the same screen); a minimised one keeps the last.
pub fn remembered(shown: Shown, before: Option<WindowPlace>) -> Option<WindowPlace> {
    match shown {
        Shown::Normal { x, y, width, height } => Some(WindowPlace {
            x,
            y,
            width: width.round().max(0.0) as u32,
            height: height.round().max(0.0) as u32,
            maximized: false,
        }),
        Shown::Maximized { x, y } => Some(match before {
            Some(before) => WindowPlace { maximized: true, ..before },
            None => {
                WindowPlace { x, y, width: PREFERRED_WIDTH as u32, height: PREFERRED_HEIGHT as u32, maximized: true }
            }
        }),
        Shown::Minimized => None,
    }
}

/// How `window` is shown now.
pub fn shown(window: &AppWindow) -> Shown {
    use slint::ComponentHandle;

    let window = window.window();
    let position = window.position();
    if window.is_minimized() {
        Shown::Minimized
    } else if window.is_maximized() {
        Shown::Maximized { x: position.x, y: position.y }
    } else {
        let size = window.size().to_logical(window.scale_factor());
        Shown::Normal { x: position.x, y: position.y, width: size.width, height: size.height }
    }
}

/// Notes where `window` is in the settings, for the next start; done as the window closes, by
/// every way it closes. Without settings (before the first-run screen is done) nothing is noted.
pub fn remember(window: &AppWindow, state: &SharedState) {
    let mut state = state.borrow_mut();
    if state.config().is_none() {
        log::info!("no settings yet, so the window's place isn't kept");
        return;
    }
    let Some(place) = remembered(shown(window), state.preferences().window) else {
        log::info!("the window closed minimised; its last place is kept");
        return;
    };
    match state.set_window_place(place) {
        Ok(()) => log::info!("window place kept: {place:?}"),
        Err(error) => log::error!("couldn't keep the window's place for the next start: {error}"),
    }
}
