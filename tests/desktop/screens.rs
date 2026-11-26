// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:03.283262
//+ Revised: 	2026/11/26 05:25:10.839382
// -----------------------------------------------------------------------------

//! The looks a screen is checked in: light and dark, at the normal and the minimum window size, and in
//! Galician at the minimum size. Each capture is named `<screen>-<look>-<width>x<height>` (Galician:
//! `<screen>-gl-…`, or `<screen>-gl-dark-…` in dark), which is the baseline's name, so a test names
//! the screens it captures. The controls a look must show are checked by the test, on each capture it
//! is given.

use crate::desktop_support::App;
use crate::desktop_support::pixels::{Image, Rect};

pub const NORMAL: (u32, u32) = (1280, 800);
/// The window's minimum size (`ui/app-window.slint`).
pub const MINIMUM: (u32, u32) = (1000, 600);

/// Light and dark, at both sizes; `check` runs on each capture.
pub fn every_look(app: &mut App, screen: &str, check: impl FnMut(&mut App, &Image, bool, (u32, u32))) {
    every_look_after(app, screen, |_| {}, check);
}

/// Like `every_look`, with `prepare` run after each resize (a scroll position, say, which a new
/// window size changes).
pub fn every_look_after(
    app: &mut App,
    screen: &str,
    mut prepare: impl FnMut(&mut App),
    mut check: impl FnMut(&mut App, &Image, bool, (u32, u32)),
) {
    for dark in [false, true] {
        for size in [NORMAL, MINIMUM] {
            app.resize(size.0, size.1);
            prepare(app);
            app.theme(dark);
            let theme = if dark { "dark" } else { "light" };
            let image = app.snapshot(&format!("{screen}-{theme}-{}x{}", size.0, size.1));
            check(app, &image, dark, size);
        }
    }
}

/// The window as the Galician look has it: the minimum size, light. A test that replays a look's
/// steps without its capture sets the window this way too.
pub fn galician_window(app: &mut App) {
    app.resize(MINIMUM.0, MINIMUM.1);
    app.theme(false);
}

/// The Galician screen at the minimum size: the longest labels in the least room.
pub fn galician_look(app: &mut App, screen: &str) -> Image {
    galician_window(app);
    app.snapshot(&format!("{screen}-gl-{}x{}", MINIMUM.0, MINIMUM.1))
}

/// The Galician screen in dark, at the same size: the dark half of a Galician README screenshot.
pub fn galician_dark_look(app: &mut App, screen: &str) -> Image {
    app.resize(MINIMUM.0, MINIMUM.1);
    app.theme(true);
    app.snapshot(&format!("{screen}-gl-dark-{}x{}", MINIMUM.0, MINIMUM.1))
}

/// `control` lies wholly inside the window: nothing pushed off-screen or cut at the edge.
pub fn assert_on_screen(image: &Image, control: Rect, what: &str) {
    assert!(control.width > 0 && control.height > 0, "{what} has no size");
    assert!(
        control.right() <= image.width && control.bottom() <= image.height,
        "{what} at {control:?} runs past the {}x{} window",
        image.width,
        image.height
    );
}
