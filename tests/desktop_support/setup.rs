// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:05.408098
//+ Revised: 	2026/11/24 17:54:10.597346
// -----------------------------------------------------------------------------

//! The window's setup before a look, and the answers for the next dialogs: the size and the minimum
//! size the window is given (what a user does by resizing), the light or dark look (without saving
//! the choice), and what the next pickers and questions answer. None of these is a click.

use std::path::Path;

use serde_json::json;

use super::App;

impl App {
    /// Logical size, as the user would set it by resizing.
    pub fn resize(&mut self, width: u32, height: u32) {
        self.send(json!({ "cmd": "size", "width": width, "height": height }));
        self.settle();
    }

    /// Lowers the window's minimum logical size, as the app does on a screen too small for the
    /// usual one, so `resize` can go below it.
    pub fn minimum(&mut self, width: u32, height: u32) {
        self.send(json!({ "cmd": "minimum", "width": width, "height": height }));
    }

    /// Light or dark, without saving it: only what the screen looks like.
    #[expect(
        clippy::fn_params_excessive_bools,
        reason = "the look under test, light or dark, which every desktop screen is checked in"
    )]
    pub fn theme(&mut self, dark: bool) {
        self.send(json!({ "cmd": "theme", "dark": dark }));
        self.settle();
    }

    /// Answers for the next folder pickers, file pickers and Yes/No questions.
    pub fn answer_dialogs(&mut self, folders: &[&Path], files: &[&Path], answers: &[bool]) {
        self.send(json!({ "cmd": "dialogs", "folders": folders, "files": files, "answers": answers }));
    }

    /// Answers for the next Save dialogs, in order.
    pub fn answer_saves(&mut self, saves: &[&Path]) {
        self.send(json!({ "cmd": "dialogs", "saves": saves }));
    }

    /// The next save dialog is cancelled.
    pub fn cancel_next_save(&mut self) {
        self.send(json!({ "cmd": "dialogs", "saves": [null] }));
    }
}
