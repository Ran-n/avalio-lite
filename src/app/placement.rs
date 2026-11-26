// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 21:48:11.396112
//+ Revised: 	2026/11/23 01:29:27.252694
// -----------------------------------------------------------------------------

//! Where the window opens: where it was when the app last closed, when that is still on a screen;
//! else centred on the screen the user started the app from, as large as is comfortable up to the
//! preferred size, and maximised when even the minimum size doesn't fit. Where it is as it closes
//! is noted for the next start.
//!
//! The parts: `fit` (the sizes and the fitting maths, pure), `work_area` (the Win32 screen query)
//! and `remember` (what is kept for the next start). This module places the window on the screen.

mod fit;
mod remember;
mod work_area;

pub use crate::platform::config::WindowPlace;
pub use fit::{
    Frame, MARGIN, MIN_HEIGHT, MIN_WIDTH, PREFERRED_HEIGHT, PREFERRED_WIDTH, Placement, WorkArea, place, restore,
};
pub use remember::{Shown, remember, remembered, shown};
pub use work_area::{PRIMARY_SCREEN, work_area, work_area_at, work_area_of};

use crate::ui::AppWindow;

/// Places the window before it is shown: where it was (`saved`) when that is on a screen, else as
/// on a first start. When the screen can't be read, Slint's defaults stay.
pub fn apply(window: &AppWindow, saved: Option<&WindowPlace>) {
    if let Some(saved) = saved {
        match work_area_of(saved) {
            Ok(Some((area, frame))) => match restore(saved, &area, frame) {
                Some(placement) => return set(window, placement, &area),
                None => log::info!("the saved window place doesn't fit its screen any more; placing it afresh"),
            },
            Ok(None) => log::info!("the saved window place {saved:?} is on no screen now; placing it afresh"),
            Err(error) => log::warn!("couldn't read the screen of the saved window place, placing it afresh: {error}"),
        }
    }
    apply_with(window, work_area());
}

/// [`apply`] with the screen already read, so a failed reading can be tested.
pub fn apply_with(window: &AppWindow, work_area: Result<(WorkArea, Frame), String>) {
    let (area, frame) = match work_area {
        Ok(found) => found,
        Err(error) => {
            log::warn!("couldn't read the screen's work area, keeping the default window size: {error}");
            return;
        }
    };
    set(window, place(&area, frame), &area);
}

/// Shows `window` as `placement` says, on the screen `area`.
fn set(window: &AppWindow, placement: Placement, area: &WorkArea) {
    use slint::ComponentHandle;

    let screen = format!("a {}×{} work area at {}", area.width, area.height, area.scale);
    match placement {
        Placement::Window { x, y, width, height, maximized } => {
            window.window().set_maximized(false);
            window.window().set_size(slint::LogicalSize::new(width, height));
            window.window().set_position(slint::PhysicalPosition::new(x, y));
            // maximised last, so restoring returns to the place just set
            window.window().set_maximized(maximized);
            let how = if maximized { "maximised, restoring to" } else { "placed at" };
            log::info!("window {how} {x},{y}, {width}×{height} on {screen}");
        }
        Placement::Maximized { min_width, min_height } => {
            window.set_fit_min_width(min_width);
            window.set_fit_min_height(min_height);
            window.window().set_maximized(true);
            log::info!("window maximised, minimum {min_width}×{min_height}, on {screen}");
        }
    }
}
