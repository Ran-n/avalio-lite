// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/29 02:54:46.632108
//+ Revised: 	2026/11/03 11:13:31.216831
// -----------------------------------------------------------------------------

//! Keyboard shortcuts: which keys run which action, their text form in `config.toml`, and the
//! rules a shortcut must follow. No file access, so the rules can be tested on their own; `ui` wires them to the window.

mod action;
mod combo;
mod key;
mod set;
pub mod ui;

pub use action::Action;
pub use combo::{KeyCombo, ShortcutError};
pub use key::Key;
pub use set::{ProblemKind, ShortcutProblem, Shortcuts};
