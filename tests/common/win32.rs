// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/09 10:42:02.250808
//+ Revised: 	2026/11/23 14:42:38.749619
// -----------------------------------------------------------------------------

//! The app's real window through Win32, for both sides of the desktop tests: the tests find it by
//! its process (`tests/desktop_support/window.rs`), and the test driver (`examples/app_driver/`)
//! finds its own window to type into.

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Dwm::{DWMWA_CLOAK, DWMWA_CLOAKED, DwmGetWindowAttribute, DwmSetWindowAttribute};
use windows_sys::Win32::System::Threading::GetCurrentThreadId;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CWPSTRUCT, CallNextHookEx, EnumWindows, GWL_STYLE, GetWindowLongPtrW, GetWindowTextW, GetWindowThreadProcessId,
    IsWindowVisible, PostMessageW, SetWindowsHookExW, WH_CALLWNDPROC, WM_CHAR, WM_CREATE, WM_KEYDOWN, WM_KEYUP,
    WM_SHOWWINDOW, WS_CHILD,
};

/// Set to `1` to see the app's window while the desktop tests run; without it the window is hidden.
pub const SHOW_WINDOW_ENV: &str = "AVALIO_LITE_SHOW_TEST_WINDOW";

/// Whether this run shows the app's window (see [`SHOW_WINDOW_ENV`]).
pub fn window_shown() -> bool {
    shows_window(std::env::var_os(SHOW_WINDOW_ENV).as_deref())
}

/// Whether a value of [`SHOW_WINDOW_ENV`] shows the window: only `1`.
pub fn shows_window(value: Option<&std::ffi::OsStr>) -> bool {
    value.is_some_and(|value| value == "1")
}

/// The main window's title (`ui/app-window.slint`); the process has helper windows too.
const APP_TITLE: &str = "avalio lite";
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

/// Cloaks a top-level window as it is created or shown; child windows are left alone.
fn cloak_on_create_or_show(message: &CWPSTRUCT) {
    let showing = message.message == WM_SHOWWINDOW && message.wParam != 0;
    if message.message != WM_CREATE && !showing {
        return;
    }
    // SAFETY: a window handle Windows is delivering a message to.
    let style = unsafe { GetWindowLongPtrW(message.hwnd, GWL_STYLE) } as u32;
    if style & WS_CHILD == 0
        && let Err(error) = cloak(message.hwnd)
    {
        // inside a window hook there is no caller to hand the error to
        eprintln!("{error}");
    }
}

/// Keeps every top-level window this thread creates off the screen: each is cloaked (DWM still
/// composes and paints it, so `PrintWindow` captures and posted messages work as before) as it is
/// created and again as it is shown, before its first frame reaches the screen. A cloaked window is
/// left out of the taskbar and Alt+Tab, and it never takes the foreground from the user.
pub fn hide_windows_of_this_thread() -> Result<(), String> {
    unsafe extern "system" fn watch(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
        // a non-negative code carries a message about to reach a window procedure
        if code >= 0 {
            // SAFETY: for WH_CALLWNDPROC, `lparam` points to the CWPSTRUCT of the message.
            let message = unsafe { &*(lparam as *const CWPSTRUCT) };
            cloak_on_create_or_show(message);
        }
        // SAFETY: passes the message on, as every hook must.
        unsafe { CallNextHookEx(std::ptr::null_mut(), code, wparam, lparam) }
    }
    // SAFETY: a hook procedure in this module, on this thread only; it stays for the process' life.
    let hook = unsafe { SetWindowsHookExW(WH_CALLWNDPROC, Some(watch), std::ptr::null_mut(), GetCurrentThreadId()) };
    if hook.is_null() { Err(format!("SetWindowsHookEx failed: {}", std::io::Error::last_os_error())) } else { Ok(()) }
}

/// Cloaks `hwnd`, a window of this process.
fn cloak(hwnd: HWND) -> Result<(), String> {
    let on: i32 = 1;
    // SAFETY: a window handle and a BOOL of the size given.
    let result =
        unsafe { DwmSetWindowAttribute(hwnd, DWMWA_CLOAK as u32, (&raw const on).cast(), size_of::<i32>() as u32) };
    if result < 0 { Err(format!("DwmSetWindowAttribute(DWMWA_CLOAK) failed: {result:#x}")) } else { Ok(()) }
}

/// Whether `hwnd` (any process' window) is cloaked, so it isn't on the screen.
pub fn is_cloaked(hwnd: HWND) -> Result<bool, String> {
    let mut cloaked: u32 = 0;
    // SAFETY: a window handle and a u32 of the size given to fill.
    let result = unsafe {
        DwmGetWindowAttribute(hwnd, DWMWA_CLOAKED as u32, (&raw mut cloaked).cast(), size_of::<u32>() as u32)
    };
    if result < 0 { Err(format!("DwmGetWindowAttribute(DWMWA_CLOAKED) failed: {result:#x}")) } else { Ok(cloaked != 0) }
}
