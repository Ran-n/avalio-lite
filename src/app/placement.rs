// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/02 21:48:11.396112
//+ Revised: 	2026/11/03 12:59:56.556852
// -----------------------------------------------------------------------------

//! Where the window opens: centred on the screen the user started the app from, as large as is
//! comfortable up to the preferred size, and maximised when even the minimum size doesn't fit.

use crate::ui::AppWindow;

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
    /// The outer top-left corner in physical pixels and the content size in logical pixels.
    Window { x: i32, y: i32, width: f32, height: f32 },
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
    let outer_width = (width * area.scale).round() as i64 + i64::from(frame.left + frame.right);
    let outer_height = (height * area.scale).round() as i64 + i64::from(frame.top + frame.bottom);
    // never left of or above the work area, so the title bar can always be grabbed
    let x = i64::from(area.left) + ((i64::from(area.width) - outer_width) / 2).max(0);
    let y = i64::from(area.top) + ((i64::from(area.height) - outer_height) / 2).max(0);
    Placement::Window { x: x as i32, y: y as i32, width, height }
}

/// Places the window before it is shown; when the screen can't be read, Slint's defaults stay.
pub fn apply(window: &AppWindow) {
    apply_with(window, work_area());
}

/// [`apply`] with the screen already read, so a failed reading can be tested.
pub fn apply_with(window: &AppWindow, work_area: Result<(WorkArea, Frame), String>) {
    use slint::ComponentHandle;

    let (area, frame) = match work_area {
        Ok(found) => found,
        Err(error) => {
            log::warn!("couldn't read the screen's work area, keeping the default window size: {error}");
            return;
        }
    };
    let screen = format!("a {}×{} work area at {}", area.width, area.height, area.scale);
    match place(&area, frame) {
        Placement::Window { x, y, width, height } => {
            window.window().set_maximized(false);
            window.window().set_size(slint::LogicalSize::new(width, height));
            window.window().set_position(slint::PhysicalPosition::new(x, y));
            log::info!("window placed at {x},{y}, {width}×{height} on {screen}");
        }
        Placement::Maximized { min_width, min_height } => {
            window.set_fit_min_width(min_width);
            window.set_fit_min_height(min_height);
            window.window().set_maximized(true);
            log::info!("window maximised, minimum {min_width}×{min_height}, on {screen}");
        }
    }
}

/// The primary monitor is the one at the origin.
pub const PRIMARY_SCREEN: (i32, i32) = (0, 0);

/// [`work_area_at`] the monitor under the mouse pointer (where the user just started the app), or
/// the primary monitor when the pointer can't be read.
#[cfg(windows)]
pub fn work_area() -> Result<(WorkArea, Frame), String> {
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::GetCursorPos;

    let mut point = POINT { x: 0, y: 0 };
    // SAFETY: a POINT to fill.
    if unsafe { GetCursorPos(&mut point) } == 0 {
        log::warn!("couldn't read the mouse position, using the primary screen: GetCursorPos failed");
        return work_area_at(PRIMARY_SCREEN);
    }
    work_area_at((point.x, point.y))
}

/// The work area and scale of the monitor holding the screen point `(x, y)` (the primary one when
/// no monitor does), and the frame a normal window gets at that scale.
#[cfg(windows)]
pub fn work_area_at((x, y): (i32, i32)) -> Result<(WorkArea, Frame), String> {
    use windows_sys::Win32::Foundation::{POINT, RECT};
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITOR_DEFAULTTOPRIMARY, MONITORINFO, MonitorFromPoint};
    use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows_sys::Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW;

    // SAFETY: plain calls with a point and flags; with MONITOR_DEFAULTTOPRIMARY it always answers.
    let monitor = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTOPRIMARY) };
    check(!monitor.is_null(), "MonitorFromPoint found no monitor")?;
    let empty = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    let mut info = MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, rcMonitor: empty, rcWork: empty, dwFlags: 0 };
    // SAFETY: a monitor handle Windows just gave, and a MONITORINFO with its size set.
    check(unsafe { GetMonitorInfoW(monitor, &mut info) } != 0, "GetMonitorInfoW failed")?;
    let (mut dpi, mut dpi_y) = (0, 0);
    // SAFETY: the same monitor handle and two numbers to fill.
    let result = unsafe { GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dpi, &mut dpi_y) };
    check(result == 0 && dpi != 0, &format!("GetDpiForMonitor failed ({result:#x})"))?;
    let mut outer = empty;
    // SAFETY: a RECT to grow by the frame of an ordinary top-level window.
    check(
        unsafe { AdjustWindowRectExForDpi(&mut outer, WS_OVERLAPPEDWINDOW, 0, 0, dpi) } != 0,
        "AdjustWindowRectExForDpi failed",
    )?;
    let work = info.rcWork;
    let area = WorkArea {
        left: work.left,
        top: work.top,
        width: (work.right - work.left).max(0) as u32,
        height: (work.bottom - work.top).max(0) as u32,
        scale: dpi as f32 / 96.0,
    };
    let frame = Frame {
        left: outer.left.unsigned_abs(),
        top: outer.top.unsigned_abs(),
        right: outer.right.unsigned_abs(),
        bottom: outer.bottom.unsigned_abs(),
    };
    Ok((area, frame))
}

/// `Err(failure)` unless the Win32 call `succeeded`.
#[cfg(windows)]
fn check(succeeded: bool, failure: &str) -> Result<(), String> {
    if succeeded { Ok(()) } else { Err(failure.to_owned()) }
}

#[cfg(not(windows))]
pub fn work_area() -> Result<(WorkArea, Frame), String> {
    Err("reading the work area is only implemented on Windows".to_owned())
}

#[cfg(not(windows))]
pub fn work_area_at(_point: (i32, i32)) -> Result<(WorkArea, Frame), String> {
    work_area()
}
