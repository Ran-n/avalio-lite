// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 01:29:26.521818
//+ Revised: 	2026/11/24 17:54:04.195672
// -----------------------------------------------------------------------------

//! The fitting maths of the window's place: the preferred and minimum sizes, how a window of the
//! preferred size shrinks or is maximised to fit a work area, and where a saved place lands on the
//! screen it is on. Pure, with no screen and no window, so the tests can state every case.

use crate::platform::config::WindowPlace;

/// The window's preferred and minimum sizes, in logical pixels. They must match
/// `preferred-width`/`preferred-height` and `fit-min-width`/`fit-min-height` in
/// `ui/app-window.slint`.
pub const PREFERRED_WIDTH: f32 = 1280.0;
pub const PREFERRED_HEIGHT: f32 = 800.0;
pub const MIN_WIDTH: f32 = 1000.0;
pub const MIN_HEIGHT: f32 = 600.0;
/// Logical pixels left free on each side, so the window's edges and shadow show it was placed on
/// purpose.
pub const MARGIN: f32 = 24.0;

/// A monitor's work area (the screen minus the taskbar and docked bars), in physical pixels, and
/// its scale (physical pixels per logical pixel).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WorkArea {
    pub left: i32,
    pub top: i32,
    pub width: u32,
    pub height: u32,
    pub scale: f32,
}

/// How far the window's outer rectangle reaches past its content on each side (title bar and
/// borders), in physical pixels; the position Windows takes is the outer rectangle's.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Frame {
    pub left: u32,
    pub top: u32,
    pub right: u32,
    pub bottom: u32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Placement {
    /// The outer top-left corner in physical pixels and the content size in logical pixels;
    /// `maximized`: shown maximised, returning to this place when restored.
    Window { x: i32, y: i32, width: f32, height: f32, maximized: bool },
    /// Too small for the minimum size: maximised, with the minimum lowered to what the screen holds.
    Maximized { min_width: f32, min_height: f32 },
}

/// The placement rules, apart from any screen so they can be tested.
pub fn place(area: &WorkArea, frame: Frame) -> Placement {
    let (work_width, work_height) = (area.width as f32 / area.scale, area.height as f32 / area.scale);
    let width = PREFERRED_WIDTH.min(work_width - 2.0 * MARGIN);
    let height = PREFERRED_HEIGHT.min(work_height - 2.0 * MARGIN);
    if width < MIN_WIDTH || height < MIN_HEIGHT {
        // a maximised window's content is the work area minus the title bar, which the minimum
        // must not exceed, or the window would reach past the screen
        let room = |work: u32, frame: u32| (work.saturating_sub(frame) as f32 / area.scale).max(0.0);
        return Placement::Maximized {
            min_width: MIN_WIDTH.min(room(area.width, frame.left + frame.right)),
            min_height: MIN_HEIGHT.min(room(area.height, frame.top + frame.bottom)),
        };
    }
    let (outer_width, outer_height) = outer(area, frame, width, height);
    // never left of or above the work area, so the title bar can always be grabbed
    let x = i64::from(area.left) + ((i64::from(area.width) - outer_width) / 2).max(0);
    let y = i64::from(area.top) + ((i64::from(area.height) - outer_height) / 2).max(0);
    Placement::Window { x: x as i32, y: y as i32, width, height, maximized: false }
}

/// The saved place on the screen `area` it is (mostly) on: its size kept between the minimum and
/// what the work area holds, and the window moved just enough to lie wholly on the work area, so
/// a screen that shrank or a taskbar that moved can't leave it out of reach. `None` when the
/// screen can't hold the minimum size; the window is then placed as on a first start.
pub fn restore(saved: &WindowPlace, area: &WorkArea, frame: Frame) -> Option<Placement> {
    let room = |work: u32, frame: u32| work.saturating_sub(frame) as f32 / area.scale;
    let room_width = room(area.width, frame.left + frame.right);
    let room_height = room(area.height, frame.top + frame.bottom);
    if room_width < MIN_WIDTH || room_height < MIN_HEIGHT {
        return None;
    }
    let width = (saved.width as f32).clamp(MIN_WIDTH, room_width);
    let height = (saved.height as f32).clamp(MIN_HEIGHT, room_height);
    let (outer_width, outer_height) = outer(area, frame, width, height);
    let within = |at: i32, start: i32, work: u32, size: i64| {
        let last = i64::from(start) + (i64::from(work) - size).max(0);
        i64::from(at).clamp(i64::from(start), last) as i32
    };
    Some(Placement::Window {
        x: within(saved.x, area.left, area.width, outer_width),
        y: within(saved.y, area.top, area.height, outer_height),
        width,
        height,
        maximized: saved.maximized,
    })
}

/// The outer size in physical pixels of a window with `width`×`height` logical content.
fn outer(area: &WorkArea, frame: Frame, width: f32, height: f32) -> (i64, i64) {
    (
        (width * area.scale).round() as i64 + i64::from(frame.left + frame.right),
        (height * area.scale).round() as i64 + i64::from(frame.top + frame.bottom),
    )
}
