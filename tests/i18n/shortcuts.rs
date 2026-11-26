// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.834383
//+ Revised: 	2026/11/23 00:26:09.834383
// -----------------------------------------------------------------------------

//! The keyboard shortcut texts: one of each, checked in both languages.

use avalio_lite::i18n::{Message, ShortcutsMessage};

use super::assert_messages_translated;

/// One of each shortcut message. The match below has no wildcard, so a new variant won't compile
/// until it is added here too.
pub fn every_shortcuts_message() -> Vec<Message<'static>> {
    let all = [ShortcutsMessage::ShortcutChanged { action: "Exam", keys: "boom" }, ShortcutsMessage::ShortcutsReset];
    for message in &all {
        match message {
            ShortcutsMessage::ShortcutChanged { .. } | ShortcutsMessage::ShortcutsReset => {}
        }
    }
    all.into_iter().map(Message::Shortcuts).collect()
}

#[test]
fn every_shortcuts_message_is_translated_and_keeps_its_details() {
    assert_messages_translated(&every_shortcuts_message());
}
