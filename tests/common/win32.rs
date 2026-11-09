// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/09 10:42:02.250808
//+ Revised: 	2026/11/09 10:42:02.250808
// -----------------------------------------------------------------------------

//! The app's real window through Win32, for both sides of the desktop tests: the tests find it by
//! its process (`tests/desktop_support/window.rs`), and the test driver (`examples/app_driver.rs`)
//! finds its own window to type into.

use windows_sys::Win32::Foundation::{HWND, LPARAM, WPARAM};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, PostMessageW, WM_CHAR, WM_KEYDOWN, WM_KEYUP,
};

/// The main window's title (`ui/app-window.slint`); the process has helper windows too.
const APP_TITLE: &str = "avalio-lite";
/// The `lParam` of a key going down, and of one going up (previous state down, transition up).
pub const KEY_DOWN: LPARAM = 1;
pub const KEY_UP: LPARAM = 1 | (0b11 << 30);

fn title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 64];
    // SAFETY: a valid window handle and a buffer of the stated length.
    let length = unsafe { GetWindowTextW(hwnd, buffer.as_mut_ptr(), buffer.len() as i32) };
    String::from_utf16_lossy(&buffer[..length.max(0) as usize])
}

/// The main window of process `pid`, once it is visible.
pub fn app_window_of(pid: u32) -> Option<HWND> {
    find_window(pid, &|hwnd| title(hwnd) == APP_TITLE)
}

/// The first visible top-level window of process `pid` that `accept` takes.
pub fn find_window(pid: u32, accept: &dyn Fn(HWND) -> bool) -> Option<HWND> {
    struct Search<'a> {
        pid: u32,
        accept: &'a dyn Fn(HWND) -> bool,
        found: HWND,
    }
    unsafe extern "system" fn visit(hwnd: HWND, data: LPARAM) -> i32 {
        // SAFETY: `data` is the `Search` passed to EnumWindows below, alive for the whole call.
        let search = unsafe { &mut *(data as *mut Search) };
        let mut owner = 0;
        // SAFETY: a window handle from EnumWindows and a u32 to fill.
        unsafe { GetWindowThreadProcessId(hwnd, &mut owner) };
        // SAFETY: as above.
        if owner == search.pid && unsafe { IsWindowVisible(hwnd) } != 0 && (search.accept)(hwnd) {
            search.found = hwnd;
            return 0;
        }
        1
    }
    let mut search = Search { pid, accept, found: std::ptr::null_mut() };
    // SAFETY: the callback only touches `search`, which outlives the call.
    unsafe { EnumWindows(Some(visit), &mut search as *mut Search as LPARAM) };
    (!search.found.is_null()).then_some(search.found)
}

/// Posts `message` to `hwnd`; a message Windows refuses is an error.
pub fn post(hwnd: HWND, message: u32, wparam: WPARAM, lparam: LPARAM) -> Result<(), String> {
    // SAFETY: posting a message to a window handle; Windows checks it is valid.
    let posted = unsafe { PostMessageW(hwnd, message, wparam, lparam) };
    if posted == 0 { Err(format!("PostMessage {message:#x} failed")) } else { Ok(()) }
}

/// Types `text` one character at a time, the way Windows delivers text that isn't tied to a key
/// (an input method, a paste tool): a `VK_PACKET` key press carrying the character.
///
/// Call it on the window's own thread. winit pairs a key-down with its character only if the
/// `WM_CHAR` is already queued when it handles the `WM_KEYDOWN`; otherwise it finishes the key
/// without text and then drops the character as input-method text. Posted from another process,
/// the app could handle the key-down first and lose characters at random; posted here, all three
/// messages are queued before the event loop reads the first.
pub fn post_text(hwnd: HWND, text: &str) -> Result<(), String> {
    const VK_PACKET: WPARAM = 0xE7;
    for unit in text.encode_utf16() {
        post(hwnd, WM_KEYDOWN, VK_PACKET, KEY_DOWN)?;
        post(hwnd, WM_CHAR, WPARAM::from(unit), KEY_DOWN)?;
        post(hwnd, WM_KEYUP, VK_PACKET, KEY_UP)?;
    }
    Ok(())
}
