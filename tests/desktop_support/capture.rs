// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:04.574422
//+ Revised: 	2026/11/24 17:54:09.806151
// -----------------------------------------------------------------------------

//! Captures of the window, and their checks against the reviewed baselines in
//! `tests/desktop/baselines/`. A capture waits until the picture has settled and is taken as the
//! active window (so another window's focus never changes it). `snapshot` compares it with the
//! baseline for the display scale; `AVALIO_UPDATE_BASELINES=1` rewrites the baseline instead. The
//! pixel checks in `pixels` do the comparing; the failure images go to `target/desktop-failures`.

use std::time::Duration;

use serde_json::json;

use super::App;
use super::manifest_dir;
use super::pixels::Image;

const UPDATE_ENV: &str = "AVALIO_UPDATE_BASELINES";
/// The most, in logical pixels, a blinking text caret covers: a line about 1 wide and 31 high (2
/// physical pixels wide from 125 %), with some to spare.
const CARET_SIZE: (f64, f64) = (3.0, 40.0);
/// Per channel: anti-aliasing may wobble by a step or two between runs.
const PIXEL_TOLERANCE: u8 = 12;
/// Share of the screen allowed to differ before a snapshot fails (a blinking caret, say).
const CHANGED_SHARE: f64 = 0.001;
/// How many times a capture tells the window it is active before taking a lost activation for
/// another window keeping the foreground.
const ACTIVATION_ATTEMPTS: u32 = 3;

impl App {
    /// `logical` pixels (as the .slint files measure) in physical pixels at the display's scale.
    pub fn px(&self, logical: f64) -> u32 {
        (logical * self.scale).round() as u32
    }

    /// An area of `logical` square pixels in physical ones, for the minimum size of a blob.
    pub fn area(&self, logical: f64) -> u32 {
        (logical * self.scale * self.scale).round() as u32
    }

    /// The window as it is now, once two captures in a row agree apart from a blinking caret
    /// (animations have finished). It is captured as the active window, as the user sees it while
    /// working in it, whichever window really has the foreground.
    pub fn capture(&mut self) -> Image {
        self.capture_with(true)
    }

    /// `capture`, with the mouse left where it is when `leave` is false: a mouse that leaves the
    /// window lets go of a held drag.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "test harness: whether the capture lets go of a held drag first"
    )]
    fn capture_with(&mut self, leave: bool) -> Image {
        // Slint draws a field focused only while its window is active, and another window brought
        // to the front during the run takes that away; the user's foreground stays where it is.
        // Windows can also deliver a late loss of focus (a window just opened, a dialog just
        // closed) while the frames settle, so the window must still be active once they have.
        // Afterwards the window goes back to how it was, so the clicks and keys that follow reach
        // it as they did before the capture.
        for _ in 0..ACTIVATION_ATTEMPTS {
            if leave {
                self.window.mouse_away();
            }
            let was = self.send(json!({ "cmd": "activate", "active": true }))["was"] == json!(true);
            self.settle();
            let image = self.settled_capture();
            let active = self.state()["active"] == json!(true);
            self.send(json!({ "cmd": "activate", "active": was }));
            self.settle();
            if active {
                return image;
            }
        }
        panic!("the app window isn't active; another window took the foreground during the run");
    }

    /// The window as it is this instant, animations under way included.
    pub fn glimpse(&mut self) -> Image {
        self.window.capture()
    }

    /// The window once two captures in a row agree apart from a blinking caret.
    fn settled_capture(&mut self) -> Image {
        let mut previous = self.window.capture();
        for _ in 0..20 {
            std::thread::sleep(Duration::from_millis(120));
            let current = self.window.capture();
            // a focused field's caret blinks on and off for as long as it has focus, so a change
            // no bigger than one caret still counts as settled; a page still scrolling or fading
            // changes a wider area, however few pixels it moves
            let settled = current
                .changed_area(&previous)
                .is_none_or(|area| area.width <= self.px(CARET_SIZE.0) && area.height <= self.px(CARET_SIZE.1));
            if settled {
                return current;
            }
            previous = current;
        }
        let failures = manifest_dir().join("target").join("desktop-failures");
        let current = self.window.capture();
        let (changed, marked) = current.difference(&previous, 0);
        previous.save(&failures.join("unsettled-previous.png"));
        current.save(&failures.join("unsettled-current.png"));
        marked.save(&failures.join("unsettled-diff.png"));
        panic!("the window never stopped changing ({changed} pixels still changing); see {}", failures.display());
    }

    /// Compares the window with `baselines/<name>@<scale>.png`. On a difference, the capture and a
    /// marked diff are written next to the target folder for review. With `AVALIO_UPDATE_BASELINES=1`
    /// the baseline is rewritten instead (open and review each one before committing it).
    pub fn snapshot(&mut self, name: &str) -> Image {
        self.snapshot_with(name, true)
    }

    /// `snapshot` in the middle of a drag held by `hold_drag_nth`: the mouse stays in the window.
    pub fn snapshot_holding(&mut self, name: &str) -> Image {
        self.snapshot_with(name, false)
    }

    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "test harness: whether the capture lets go of a held drag first"
    )]
    fn snapshot_with(&mut self, name: &str, leave: bool) -> Image {
        let image = self.capture_with(leave);
        let file = format!("{name}@{}.png", (self.scale * 100.0).round() as u32);
        let baseline = manifest_dir().join("tests").join("desktop").join("baselines").join(&file);
        if std::env::var_os(UPDATE_ENV).is_some_and(|value| value == "1") {
            image.save(&baseline);
            return image;
        }
        // baselines are per display scale; a scale without them is a failure, never a skip
        let expected = Image::load(&baseline).unwrap_or_else(|| {
            panic!(
                "no baseline {file} for {} % display scaling; record this scale's baselines with \
                 `$env:{UPDATE_ENV} = \"1\"; cargo test --test desktop -- --ignored --test-threads=1` and \
                 open and review every new image before committing it",
                (self.scale * 100.0).round()
            )
        });
        let failures = manifest_dir().join("target").join("desktop-failures");
        if (expected.width, expected.height) != (image.width, image.height) {
            image.save(&failures.join(file.replace(".png", "-actual.png")));
            panic!(
                "{file}: the screen is {}x{}, the baseline {}x{}",
                image.width, image.height, expected.width, expected.height
            );
        }
        // positions are whole physical pixels only at a whole display scale
        let shift = if self.scale.fract() == 0.0 { 0 } else { 1 };
        let (changed, marked) = image.difference_within(&expected, PIXEL_TOLERANCE, shift);
        let allowed = (f64::from(image.width * image.height) * CHANGED_SHARE) as usize;
        if changed > allowed {
            image.save(&failures.join(file.replace(".png", "-actual.png")));
            marked.save(&failures.join(file.replace(".png", "-diff.png")));
            panic!("{file}: {changed} pixels changed (allowed {allowed}); see {}", failures.display());
        }
        image
    }
}
