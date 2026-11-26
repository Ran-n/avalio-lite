// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:05.236066
//+ Revised: 	2026/11/24 17:54:10.253122
// -----------------------------------------------------------------------------

//! Page-level moves the tests repeat: scrolling a card of Settings into view, opening the sheet's menus,
//! and clicking a button of the update banner. Each is a scroll or a click a user would make, and
//! asserts nothing about what the page shows: the test that needs that checks it itself.

use super::App;
use super::pixels::Rect;

impl App {
    /// Scrolls the main area from its top, a notch at a time, until the control labelled `label` is
    /// wholly in the window: the same notches every run, so the picture is the same.
    pub fn scroll_into_view(&mut self, label: &str) {
        self.scroll_main(60);
        for _ in 0..60 {
            let height = self.capture().height;
            if self.locate(label).first().is_some_and(|found| found.bottom() + self.px(16.0) <= height) {
                return;
            }
            self.scroll_main(-1);
        }
        panic!("{label:?} never came into view");
    }

    /// Settings scrolled from the top until both `move_label` buttons (the database's and the logs')
    /// are wholly on screen, and with them the Database, Backup and Logs cards between them.
    pub fn show_folder_cards(&mut self, move_label: &str) {
        self.scroll_main(60);
        for _ in 0..80 {
            let height = self.capture().height;
            let found = self.locate(move_label);
            if found.len() == 2 && found[1].bottom() + self.px(16.0) <= height {
                return;
            }
            self.scroll_main(-1);
        }
        panic!("both {move_label:?} buttons never came into view");
    }

    /// The About card scrolled into view, down to Slint's badge at its bottom.
    pub fn show_about(&mut self) {
        self.scroll_main(-80);
    }

    /// The one control labelled `label` on screen, or the test fails: a click or a check needs it.
    pub fn located(&mut self, label: &str) -> Rect {
        let found = self.locate(label);
        assert!(!found.is_empty(), "{label:?} isn't on screen");
        found[0]
    }

    /// The first plain text showing exactly `text`, or the test fails.
    pub fn located_text(&mut self, text: &str) -> Rect {
        let found = self.locate_text(text);
        assert!(!found.is_empty(), "the text {text:?} isn't on screen");
        found[0]
    }

    /// Clicks the sheet's "⋯" button, which opens its menu.
    pub fn open_more_actions(&mut self) {
        self.click("More actions");
    }

    /// Clicks the sheet's "+ Add column" button, which opens the add-column menu.
    pub fn open_add_column_menu(&mut self) {
        self.click("+ Add column");
    }

    /// Opens the pupil menu's "Add from another class…" entry, through the sheet's "⋯" menu.
    pub fn open_add_from_another_class(&mut self) {
        self.open_more_actions();
        self.click_in_popup("Add from another class…");
    }

    /// Closes an open sheet menu without changing anything: its export is cancelled at the save dialog.
    pub fn cancel_export(&mut self, export: &str) {
        self.cancel_next_save();
        self.click_in_popup(export);
    }

    /// Clicks the banner's `label` (Update and restart, Cancel): with Settings open, its Updates card
    /// has the same buttons further down, so the one nearest the top of the screen is the banner's.
    pub fn click_in_banner(&mut self, label: &str) {
        let found = self.locate(label);
        let top =
            (0..found.len()).min_by_key(|&index| found[index].y).unwrap_or_else(|| panic!("no {label:?} on screen"));
        self.click_nth(label, top);
    }
}
