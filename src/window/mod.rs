// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 22:53:31.432066
//+ Revised: 	2026/11/03 10:00:17.340810
// -----------------------------------------------------------------------------

//! The UI kit every feature's `ui` module uses: the shared state and dialogs handles, the callback
//! wrappers that only run while the window is alive, dialogs, the status and error line, theme and
//! language.

pub mod dialogs;
pub mod language;
pub mod theme;
pub mod view;

use std::cell::RefCell;
use std::rc::Rc;

use slint::ComponentHandle;

use crate::state::AppState;
use crate::ui::AppWindow;
use dialogs::Dialogs;

pub type SharedState = Rc<RefCell<AppState>>;
pub type SharedDialogs = Rc<dyn Dialogs>;

/// Wraps a UI callback so it runs with the window, and only while the window is alive: the window
/// is held weakly (a strong one inside its own callback would keep it alive forever). A callback
/// that fires after the window is gone does nothing, and says so in the debug log.
macro_rules! window_callbacks {
    ($($name:ident($($argument:ident: $type:ident),*);)*) => {$(
        pub fn $name<$($type),*>(
            window: &AppWindow,
            mut action: impl FnMut(&AppWindow $(, $type)*) + 'static,
        ) -> impl FnMut($($type),*) + 'static {
            let weak = window.as_weak();
            move |$($argument),*| {
                if let Some(window) = alive(&weak) {
                    action(&window $(, $argument)*);
                }
            }
        }
    )*};
}

window_callbacks! {
    with_window();
    with_window1(a: A);
    with_window2(a: A, b: B);
    with_window3(a: A, b: B, c: C);
    with_window5(a: A, b: B, c: C, d: D, e: E);
}

/// Logs that `action` failed with `error` as a warning and shows the error under the screen's
/// title. A macro, so the log line keeps the target of the feature that failed.
macro_rules! report_failure {
    ($window:expr, $action:expr, $error:expr) => {{
        log::warn!("{} failed: {}", $action, $error);
        $crate::window::view::show_error($window, $error);
    }};
}
pub(crate) use report_failure;

/// The window behind `weak`, or `None` (logged) once it has closed.
pub fn alive(weak: &slint::Weak<AppWindow>) -> Option<AppWindow> {
    let window = weak.upgrade();
    if window.is_none() {
        log::debug!("callback after the window closed");
    }
    window
}
