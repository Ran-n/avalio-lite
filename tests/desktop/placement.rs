// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.026157
//+ Revised: 	2026/11/24 17:54:08.344948
// -----------------------------------------------------------------------------

//! Where the window opens: fully inside the work area of its screen, and centred on it.

use crate::desktop_support::{self, App};

use super::fixtures::{config, with_sample_class};

/// The real exe opens fully inside the work area of its screen, centred (src/app/placement.rs).
#[test]
#[ignore = "desktop: opens the real window; run with --ignored"]
fn the_window_opens_inside_the_work_area() {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CMONITORS};
    let _turn = desktop_support::turn();
    let fixture = with_sample_class();
    let app = App::start(&config(&fixture), "en");
    let (window, work) = (app.window.outer_rect(), app.window.work_area());
    eprintln!(
        "window ({}, {})–({}, {}) in the work area ({}, {})–({}, {})",
        window.left, window.top, window.right, window.bottom, work.left, work.top, work.right, work.bottom
    );
    assert!(
        window.left >= work.left
            && window.top >= work.top
            && window.right <= work.right
            && window.bottom <= work.bottom,
        "the window reaches past the work area"
    );
    let centre = |low: i32, high: i32| (low + high) as f64 / 2.0;
    assert!((centre(window.left, window.right) - centre(work.left, work.right)).abs() <= 2.0, "not centred across");
    assert!((centre(window.top, window.bottom) - centre(work.top, work.bottom)).abs() <= 2.0, "not centred down");
    // SAFETY: a plain query.
    if unsafe { GetSystemMetrics(SM_CMONITORS) } > 1 {
        // starting on another screen means moving the user's own mouse there, which these tests never do
        eprintln!("note: more than one monitor; the start on a second one is covered by tests/placement.rs only");
    } else {
        eprintln!("note: one monitor here, so the start on a second one isn't checked");
    }
    app.quit();
}
