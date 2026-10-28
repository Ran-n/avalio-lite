// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.640367
//+ Revised: 	2026/10/28 17:08:59.283582
// -----------------------------------------------------------------------------

//! The app's real window, through Win32: found by its process, captured with `PrintWindow`, and
//! driven with the same mouse and keyboard messages Windows sends it. The messages are posted to
//! the window, so the user's own cursor and keyboard focus are left alone while the tests run.

use std::sync::Once;
use std::time::{Duration, Instant};

use windows_sys::Win32::Foundation::{HWND, LPARAM, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, DIB_RGB_COLORS,
    DeleteDC, DeleteObject, GetDC, GetDIBits, ReleaseDC, SelectObject,
};
use windows_sys::Win32::Storage::Xps::PrintWindow;
use windows_sys::Win32::UI::HiDpi::{DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2, SetProcessDpiAwarenessContext};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClientRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CHAR,
    WM_KEYDOWN, WM_KEYUP, WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MOUSEMOVE, WM_MOUSEWHEEL,
};

use super::pixels::Image;

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
            if let Some(hwnd) = visible_window_of(pid) {
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

    /// Types `text` one character at a time, the way Windows delivers text that isn't tied to a
    /// key (an input method, a paste tool): a `VK_PACKET` key press carrying the character.
    pub fn type_text(&self, text: &str) {
        const VK_PACKET: WPARAM = 0xE7;
        for unit in text.encode_utf16() {
            self.post(WM_KEYDOWN, VK_PACKET, 1);
            self.post(WM_CHAR, WPARAM::from(unit), 1);
            self.post(WM_KEYUP, VK_PACKET, 1 | (0b11 << 30));
        }
    }

    /// Presses and releases one key, by its virtual-key code.
    pub fn press(&self, virtual_key: u16) {
        self.post(WM_KEYDOWN, WPARAM::from(virtual_key), 1);
        self.post(WM_KEYUP, WPARAM::from(virtual_key), 1 | (0b11 << 30));
    }

    fn post(&self, message: u32, wparam: WPARAM, lparam: LPARAM) {
        // SAFETY: posting a message to a valid window handle.
        let posted = unsafe { PostMessageW(self.hwnd, message, wparam, lparam) };
        assert!(posted != 0, "PostMessage {message:#x} failed");
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

/// The main window's title (`ui/app-window.slint`); the process has helper windows too.
const APP_TITLE: &str = "avalio-lite";

fn title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 64];
    // SAFETY: a valid window handle and a buffer of the stated length.
    let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

fn visible_window_of(pid: u32) -> Option<HWND> {
    struct Search {
        pid: u32,
        found: HWND,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        // SAFETY: `data` is the `Search` passed to EnumWindows below, alive for the whole call.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut owner = 0;
        // SAFETY: a window handle from EnumWindows and a u32 to fill.
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        // SAFETY: as above.
        if owner == search.pid && unsafe { IsWindowVisible(hwnd) } != 0 && title(hwnd) == APP_TITLE {
            search.found = hwnd;
            return 0;
        }
        1
    }
    let mut search = Search { pid, found: std::ptr::null_mut() };
    // SAFETY: the callback only touches `search`, which outlives the call.
    unsafe { EnumWindows(Some(visit), &mut search as *mut Search as LPARAM) };
    (!search.found.is_null()).then_some(search.found)
}
