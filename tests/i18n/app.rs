// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.242270
//+ Revised: 	2026/11/23 00:26:09.242270
// -----------------------------------------------------------------------------

//! The app-level texts: one of each, checked in both languages.

use avalio_lite::i18n::{AppMessage, Message};

use super::assert_messages_translated;

/// One of each app message. The match below has no wildcard, so a new variant won't compile until it
/// is added here too.
pub fn every_app_message() -> Vec<Message<'static>> {
    let all = [
        AppMessage::WindowFailed,
        AppMessage::BackgroundFailed,
        AppMessage::LinkFailed { address: "https://breren.com" },
    ];
    for message in &all {
        match message {
            AppMessage::WindowFailed | AppMessage::BackgroundFailed | AppMessage::LinkFailed { .. } => {}
        }
    }
    all.into_iter().map(Message::App).collect()
}

#[test]
fn every_app_message_is_translated_and_keeps_its_details() {
    assert_messages_translated(&every_app_message());
}
