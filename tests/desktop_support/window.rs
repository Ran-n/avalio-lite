// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.640367
//+ Revised: 	2026/11/09 10:42:02.390174
// -----------------------------------------------------------------------------

//! The app's real window, through Win32: found by its process, captured with `PrintWindow`, and
//! driven with the same mouse and keyboard messages Windows sends it. The messages are posted to
//! the window, so the user's own cursor and keyboard focus are left alone while the tests run.
//! Typed text is posted by the app's driver on the window's own thread instead (see
//! `common::win32::post_text`).

use std::sync::Once;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, GetDIBits, GetMonitorInfoW, MONITOR_DEFAULTTONEAREST, MONITORINFO,
    MonitorFromWindow, ReleaseDC, SelectObject,
};
use windows_sys::Win32::Storage::Xps::PrintWindow;
use windows_sys::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetClassNameW, GetClientRect, GetWindowRect, PostMessageW, WM_CLOSE, WM_KEYDOWN, WM_KEYUP, WM_KILLFOCUS,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
};

use super::pixels::Image;
use crate::common::win32::{self, KEY_DOWN, KEY_UP, find_window};

/// `PrintWindow` flags: only the client area, and ask DWM for what is really on screen
/// (GPU-rendered content included).
const PW_CLIENTONLY: u32 = 1;
const PW_RENDERFULLCONTENT: u32 = 2;
const MK_LBUTTON: WPARAM = 1;
const WHEEL_DELTA: i32 = 120;
/// Sent when the mouse leaves a window that asked to track it (winuser.h; not in windows-sys' list).
const WM_MOUSELEAVE: u32 = 0x02A3;

pub struct AppWindow {
    hwnd: HWND,
}

// SAFETY: a window handle is just an identifier; every call made with it is thread-safe.
unsafe impl Send for AppWindow {}

impl AppWindow {
    /// Waits for the visible top-level window of process `pid`.
    pub fn of_process(pid: u32, timeout: Duration) -> Self {
        dpi_aware();
        let deadline = Instant::now() + timeout;
        loop {
            if let Some(hwnd) = win32::app_window_of(pid) {
                return Self { hwnd };
            }
            assert!(Instant::now() < deadline, "process {pid} showed no window within {timeout:?}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// The client area's size in physical pixels.
    pub fn client_size(&self) -> (i32, i32) {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: a valid window handle and a RECT to fill.
        unsafe { GetClientRect(self.hwnd, &mut rect) };
        (rect.right - rect.left, rect.bottom - rect.top)
    }

    /// The whole window (frame included) on screen, in physical pixels.
    pub fn outer_rect(&self) -> RECT {
        let mut rect = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        // SAFETY: a valid window handle and a RECT to fill.
        let read = unsafe { GetWindowRect(self.hwnd, &mut rect) };
        assert!(read != 0, "GetWindowRect failed");
        rect
    }

    /// The work area (the screen minus the taskbar) of the monitor the window is on.
    pub fn work_area(&self) -> RECT {
        let empty = RECT { left: 0, top: 0, right: 0, bottom: 0 };
        let mut info =
            MONITORINFO { cbSize: size_of::<MONITORINFO>() as u32, rcMonitor: empty, rcWork: empty, dwFlags: 0 };
        // SAFETY: a valid window handle; the monitor handle comes straight from Windows, and the
        // MONITORINFO has its size set.
        let read = unsafe { GetMonitorInfoW(MonitorFromWindow(self.hwnd, MONITOR_DEFAULTTONEAREST), &mut info) };
        assert!(read != 0, "GetMonitorInfoW failed");
        info.rcWork
    }

    /// The client area as it is on screen.
    pub fn capture(&self) -> Image {
        let (width, height) = self.client_size();
        assert!(width > 0 && height > 0, "the window has no client area");
        // SAFETY: plain GDI calls on objects created and released here.
        unsafe {
            let screen = GetDC(std::ptr::null_mut());
            let memory = CreateCompatibleDC(screen);
            let bitmap = CreateCompatibleBitmap(screen, width, height);
            let previous = SelectObject(memory, bitmap);
            let printed = PrintWindow(self.hwnd, memory, PW_CLIENTONLY | PW_RENDERFULLCONTENT);
            let mut info: BITMAPINFO = std::mem::zeroed();
            info.bmiHeader = BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height, // top-down rows
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB,
                ..std::mem::zeroed()
            };
            let mut bgra = vec![0u8; (width * height * 4) as usize];
            let rows = GetDIBits(memory, bitmap, 0, height as u32, bgra.as_mut_ptr().cast(), &mut info, DIB_RGB_COLORS);
            SelectObject(memory, previous);
            DeleteObject(bitmap);
            DeleteDC(memory);
            ReleaseDC(std::ptr::null_mut(), screen);
            assert!(printed != 0, "PrintWindow failed");
            assert_eq!(rows, height, "GetDIBits read {rows} of {height} rows");
            let rgba = bgra.as_chunks::<4>().0.iter().flat_map(|pixel| [pixel[2], pixel[1], pixel[0], 255]).collect();
            Image::new(width as u32, height as u32, rgba)
        }
    }

    /// A left click at `(x, y)` physical pixels inside the client area, as Windows delivers one.
    pub fn click(&self, x: i32, y: i32) {
        let position = point(x, y);
        self.post(WM_MOUSEMOVE, 0, position);
        self.post(WM_LBUTTONDOWN, MK_LBUTTON, position);
        std::thread::sleep(Duration::from_millis(40));
        self.post(WM_LBUTTONUP, 0, position);
    }

    /// The mouse leaves the window, so no control is left showing its hover colour.
    pub fn mouse_away(&self) {
        self.post(WM_MOUSELEAVE, 0, 0);
    }

    /// Turns the mouse wheel over `(x, y)`; negative `notches` scroll down.
    pub fn scroll(&self, x: i32, y: i32, notches: i32) {
        let mut screen = POINT { x, y };
        // SAFETY: a valid window handle and a POINT to convert.
        unsafe { ClientToScreen(self.hwnd, &mut screen) };
        self.post(WM_MOUSEMOVE, 0, point(x, y));
        let delta = (notches * WHEEL_DELTA) as u16 as usize;
        self.post(WM_MOUSEWHEEL, delta << 16, point(screen.x, screen.y));
    }

    /// The window loses the keyboard focus, as when another window comes to the front.
    pub fn lose_focus(&self) {
        self.post(WM_KILLFOCUS, 0, 0);
    }

    /// Closes the window as its close button would.
    pub fn close(&self) {
        self.post(WM_CLOSE, 0, 0);
    }

    /// Presses and releases one key, by its virtual-key code.
    pub fn press(&self, virtual_key: u16) {
        self.post(WM_KEYDOWN, WPARAM::from(virtual_key), KEY_DOWN);
        self.post(WM_KEYUP, WPARAM::from(virtual_key), KEY_UP);
    }

    /// Presses `virtual_key` with Ctrl held: Ctrl goes down, the key goes down and up, Ctrl goes up.
    pub fn press_with_control(&self, virtual_key: u16) {
        const VK_CONTROL: WPARAM = 0x11;
        self.post(WM_KEYDOWN, VK_CONTROL, KEY_DOWN);
        self.press(virtual_key);
        self.post(WM_KEYUP, VK_CONTROL, KEY_UP);
    }

    fn post(&self, message: u32, wparam: WPARAM, lparam: LPARAM) {
        win32::post(self.hwnd, message, wparam, lparam).unwrap_or_else(|error| panic!("{error}"));
    }
}

/// The test process works in physical pixels, like the captures.
fn dpi_aware() {
    static ONCE: Once = Once::new();
    // SAFETY: called once, before any window is looked at.
    ONCE.call_once(|| unsafe {
        SetProcessDpiAwarenessContext(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
    });
}

fn point(x: i32, y: i32) -> LPARAM {
    ((y as u16 as u32) << 16 | (x as u16 as u32)) as LPARAM
}

/// The window class of every standard Windows dialog, the file and folder pickers included.
const DIALOG_CLASS: &str = "#32770";

fn class_name(hwnd: HWND) -> String {
    let mut buffer = [0u16; 64];
    // SAFETY: a valid window handle and a buffer of the stated length.
    let length = unsafe { GetClassNameW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

/// A native dialog the same process opened (not its main window, nor its helper windows). Waits up
/// to `timeout`.
pub fn dialog_of(pid: u32, main: &AppWindow, timeout: Duration) -> Option<OtherWindow> {
    let deadline = Instant::now() + timeout;
    loop {
        if let Some(hwnd) = find_window(pid, &|hwnd| hwnd != main.hwnd && class_name(hwnd) == DIALOG_CLASS) {
            return Some(OtherWindow { hwnd });
        }
        if Instant::now() >= deadline {
            return None;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// A window of the app other than its main one, such as a native file dialog.
pub struct OtherWindow {
    hwnd: HWND,
}

impl OtherWindow {
    /// Closes it as its close button would; a dialog closed so is cancelled.
    pub fn close(&self) {
        // SAFETY: posting a message to a window handle from EnumWindows.
        unsafe { PostMessageW(self.hwnd, WM_CLOSE, 0, 0) };
    }
}
