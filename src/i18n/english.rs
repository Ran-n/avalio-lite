// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/03 11:13:30.944950
//+ Revised: 	2026/11/23 00:26:14.456934
// -----------------------------------------------------------------------------

//! The Rust-side texts and error messages in English, one file per topic so a translator sees one
//! topic at a time.

mod app;
mod data;
mod errors;
mod licence;
mod sheet;
mod shortcuts;
mod spreadsheet;
mod update;

pub(in crate::i18n) use errors::error;

use crate::i18n::messages::Message;

/// A message in English, from the file of its topic.
pub(super) fn message(message: &Message) -> String {
    match message {
        Message::Sheet(message) => sheet::message(message),
        Message::Data(message) => data::message(message),
        Message::Spreadsheet(message) => spreadsheet::message(message),
        Message::Shortcuts(message) => shortcuts::message(message),
        Message::Licence(message) => licence::message(message),
        Message::Update(message) => update::message(message),
        Message::App(message) => app::message(message),
    }
}
