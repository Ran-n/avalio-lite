// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:04.664057
//+ Revised: 	2026/11/24 17:54:09.906916
// -----------------------------------------------------------------------------

//! Real input, as a user gives it: clicks, drags, scrolls and key presses, sent through Windows as
//! the mouse and keyboard would (`window`), and typed text, which the driver posts to the window
//! itself (`common::win32::post_text`), because a character typed fast can be lost otherwise. Each
//! action waits for the app to settle (`settle`) before it returns.

use std::time::{Duration, Instant};

use serde_json::json;

use super::App;
use super::pixels::Rect;

/// How long typed text may take to show up in a field, and how often a field is typed into.
const TYPING_WAIT: Duration = Duration::from_secs(3);
const TYPING_ATTEMPTS: u32 = 2;

impl App {
    pub fn click(&mut self, label: &str) {
        let found = self.locate(label);
        assert_eq!(found.len(), 1, "expected one control labelled {label:?}, found {found:?}");
        self.click_nth(label, 0);
    }

    /// A real click in the middle of the `index`-th control labelled `label`.
    pub fn click_nth(&mut self, label: &str, index: usize) {
        let target = self.the(label, index);
        self.click_at(target);
    }

    /// A real click on the option labelled `label` of the open popup menu.
    pub fn click_in_menu(&mut self, label: &str) {
        let found = self.locate_in_menu(label);
        assert_eq!(found.len(), 1, "expected one menu option {label:?}, found {found:?}");
        self.click_at(found[0]);
    }

    /// A real click on the control labelled `label` of the open popup, hint or not.
    pub fn click_in_popup(&mut self, label: &str) {
        let found = self.locate_in_popup(label);
        assert_eq!(found.len(), 1, "expected one popup control {label:?}, found {found:?}");
        self.click_at(found[0]);
    }

    fn click_at(&mut self, target: Rect) {
        self.window.click((target.x + target.width / 2) as i32, (target.y + target.height / 2) as i32);
        self.settle();
    }

    fn the(&mut self, label: &str, index: usize) -> Rect {
        let found = self.locate(label);
        *found.get(index).unwrap_or_else(|| panic!("no control {index} labelled {label:?} on screen: {found:?}"))
    }

    /// Clicks into the field labelled `label`, clears it and types `text`, then waits until the
    /// field holds exactly that. The keys are posted, so the app may still be working through
    /// them; and a key that lands before the click has focused the field is lost, so a field
    /// that doesn't end up with `text` is typed into once more before the test fails.
    pub fn type_into(&mut self, label: &str, text: &str) {
        for attempt in 1..=TYPING_ATTEMPTS {
            self.clear_and_type(label, text);
            if self.wait_for_text(label, text) {
                return;
            }
            eprintln!("typing {text:?} into {label:?} didn't take (attempt {attempt}); typing again");
        }
        panic!("the field {label:?} never held {text:?}; it holds {:?}", self.text_of(label));
    }

    fn clear_and_type(&mut self, label: &str, text: &str) {
        const VK_END: u16 = 0x23;
        const VK_BACK: u16 = 0x08;
        self.click(label);
        self.window.press(VK_END);
        for _ in 0..80 {
            self.window.press(VK_BACK);
        }
        // the driver posts the text on the window's own thread: see `common::win32::post_text`
        self.send(json!({ "cmd": "type", "text": text }));
        self.settle();
    }

    /// Polls the field until it holds `text`, for up to `TYPING_WAIT`.
    fn wait_for_text(&mut self, label: &str, text: &str) -> bool {
        let deadline = Instant::now() + TYPING_WAIT;
        while Instant::now() < deadline {
            if self.text_of(label) == text {
                return true;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        false
    }

    /// The text of the one field labelled `label`.
    fn text_of(&mut self, label: &str) -> String {
        let reply = self.send(json!({ "cmd": "locate", "label": label }));
        let found = reply["found"].as_array().cloned().unwrap_or_default();
        assert_eq!(found.len(), 1, "expected one field labelled {label:?}, found {found:?}");
        found[0]["value"].as_str().unwrap_or_default().to_owned()
    }

    /// Drags the middle of the `index`-th control labelled `label` sideways by `dx` logical pixels
    /// with the left button held, through real mouse messages.
    pub fn drag_nth(&mut self, label: &str, index: usize, dx: f64) {
        let target = self.the(label, index);
        let from = ((target.x + target.width / 2) as i32, (target.y + target.height / 2) as i32);
        let to = (from.0 + (dx * self.scale).round() as i32, from.1);
        self.window.drag(from, to);
        self.settle();
    }

    /// Presses the `index`th control labelled `label` and drags it `dx` logical pixels sideways,
    /// keeping the button down (for a snapshot mid-drag); `release` lets go. The point it ended at.
    pub fn hold_drag_nth(&mut self, label: &str, index: usize, dx: f64) -> (i32, i32) {
        let target = self.the(label, index);
        let from = ((target.x + target.width / 2) as i32, (target.y + target.height / 2) as i32);
        let to = (from.0 + (dx * self.scale).round() as i32, from.1);
        self.window.press_and_move(from, to);
        self.settle();
        to
    }

    /// Lets go of a drag held by `hold_drag_nth` at `at`.
    pub fn release(&mut self, at: (i32, i32)) {
        self.window.release(at);
        self.settle();
    }

    pub fn press_end(&mut self) {
        const VK_END: u16 = 0x23;
        self.window.press(VK_END);
        self.settle();
    }

    pub fn press_enter(&mut self) {
        const VK_RETURN: u16 = 0x0D;
        self.window.press(VK_RETURN);
        self.settle();
    }

    pub fn press_down(&mut self) {
        const VK_DOWN: u16 = 0x28;
        self.window.press(VK_DOWN);
        self.settle();
    }

    /// Presses `virtual_key` with Ctrl held, as Windows delivers it.
    pub fn press_with_control(&mut self, virtual_key: u16) {
        self.window.press_with_control(virtual_key);
        self.settle();
    }

    pub fn press_escape(&mut self) {
        const VK_ESCAPE: u16 = 0x1B;
        self.window.press(VK_ESCAPE);
        self.settle();
    }

    pub fn press_tab(&mut self) {
        const VK_TAB: u16 = 0x09;
        self.window.press(VK_TAB);
        self.settle();
    }

    pub fn scroll(&mut self, label: &str, notches: i32) {
        let target = self.the(label, 0);
        self.window.scroll((target.x + target.width / 2) as i32, (target.y + target.height / 2) as i32, notches);
        self.settle();
    }

    /// Scrolls the main area (right of the sidebar) by `notches`, whatever is on screen there; with
    /// a big positive count first, it starts from the top every time.
    pub fn scroll_main(&mut self, notches: i32) {
        const MAIN_AREA: (f64, f64) = (600.0, 300.0);
        let (x, y) = ((MAIN_AREA.0 * self.scale) as i32, (MAIN_AREA.1 * self.scale) as i32);
        self.window.scroll(x, y, notches);
        self.settle();
    }

    /// Waits until the event loop has handled everything sent so far and the frame is drawn.
    pub fn settle(&mut self) {
        self.send(json!({ "cmd": "ping" }));
        std::thread::sleep(Duration::from_millis(150));
    }
}
