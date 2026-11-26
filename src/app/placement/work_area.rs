// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 01:29:26.592095
//+ Revised: 	2026/11/24 17:54:04.395445
// -----------------------------------------------------------------------------

//! The screen query of the window's place: the work area of the monitor under the mouse, of the
//! monitor a saved window lies on, or of the one at a point, read from Windows. The only part of
//! the placement that calls Win32; elsewhere it reports that the screen can't be read.

use super::fit::{Frame, WorkArea};
use crate::platform::config::WindowPlace;

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
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTOPRIMARY, MonitorFromPoint};

    // SAFETY: plain calls with a point and flags; with MONITOR_DEFAULTTOPRIMARY it always answers.
    let monitor = unsafe { MonitorFromPoint(POINT { x, y }, MONITOR_DEFAULTTOPRIMARY) };
    check(!monitor.is_null(), "MonitorFromPoint found no monitor")?;
    monitor_area(monitor)
}

/// The work area and frame of the monitor that holds most of the saved window (its size taken at
/// 100 %, close enough to pick the monitor); `None` when it lies on none of them.
#[cfg(windows)]
pub fn work_area_of(saved: &WindowPlace) -> Result<Option<(WorkArea, Frame)>, String> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{MONITOR_DEFAULTTONULL, MonitorFromRect};

    let reach = |start: i32, size: u32| start.saturating_add(i32::try_from(size).unwrap_or(i32::MAX));
    let rect =
        RECT { left: saved.x, top: saved.y, right: reach(saved.x, saved.width), bottom: reach(saved.y, saved.height) };
    // SAFETY: a RECT and a flag; with MONITOR_DEFAULTTONULL it answers null when no monitor overlaps.
    let monitor = unsafe { MonitorFromRect(&rect, MONITOR_DEFAULTTONULL) };
    if monitor.is_null() {
        return Ok(None);
    }
    monitor_area(monitor).map(Some)
}

/// The work area and scale of `monitor`, and the frame a normal window gets at that scale.
#[cfg(windows)]
fn monitor_area(monitor: windows_sys::Win32::Graphics::Gdi::HMONITOR) -> Result<(WorkArea, Frame), String> {
    use windows_sys::Win32::Foundation::RECT;
    use windows_sys::Win32::Graphics::Gdi::{GetMonitorInfoW, MONITORINFO};
    use windows_sys::Win32::UI::HiDpi::{AdjustWindowRectExForDpi, GetDpiForMonitor, MDT_EFFECTIVE_DPI};
    use windows_sys::Win32::UI::WindowsAndMessaging::WS_OVERLAPPEDWINDOW;

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
#[expect(clippy::fn_params_excessive_bools, reason = "a Win32 call's BOOL result, turned into a Result here")]
fn check(succeeded: bool, failure: &str) -> Result<(), String> {
    if succeeded { Ok(()) } else { Err(failure.to_owned()) }
}

/// Off Windows the screen can't be read, so the window keeps Slint's defaults.
#[cfg(not(windows))]
pub fn work_area() -> Result<(WorkArea, Frame), String> {
    Err("reading the work area is only implemented on Windows".to_owned())
}

/// Off Windows every point reads as the one screen [`work_area`] reports.
#[cfg(not(windows))]
pub fn work_area_at(_point: (i32, i32)) -> Result<(WorkArea, Frame), String> {
    work_area()
}

/// Off Windows the saved window is taken to be on the one screen [`work_area`] reports.
#[cfg(not(windows))]
pub fn work_area_of(_saved: &WindowPlace) -> Result<Option<(WorkArea, Frame)>, String> {
    work_area().map(Some)
}
