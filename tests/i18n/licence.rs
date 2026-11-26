// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.567783
//+ Revised: 	2026/11/23 00:26:09.567783
// -----------------------------------------------------------------------------

//! The licence texts: one of each, checked in both languages.

use avalio_lite::i18n::{LicenceMessage, Message};

use super::assert_messages_translated;

/// One of each licence message. The match below has no wildcard, so a new variant won't compile
/// until it is added here too.
pub fn every_licence_message() -> Vec<Message<'static>> {
    let all = [
        LicenceMessage::AskForKey,
        LicenceMessage::InvalidKey,
        LicenceMessage::InUseElsewhere { email: "a@b.c" },
        LicenceMessage::Revoked { email: "a@b.c" },
        LicenceMessage::Expired { date: Some("30/06/2027"), email: "a@b.c" },
        LicenceMessage::Expired { date: None, email: "a@b.c" },
        LicenceMessage::Withdrawn { email: "a@b.c" },
        LicenceMessage::NeedsInternet,
        LicenceMessage::Wait { minutes: 2 },
        LicenceMessage::Unreachable,
        LicenceMessage::NotSaved,
        LicenceMessage::ConnectBy { date: "30/06/2027" },
        LicenceMessage::EndsOn { date: "30/06/2027", email: "a@b.c" },
        LicenceMessage::Permanent,
        LicenceMessage::Until { date: "30/06/2027" },
        LicenceMessage::OfflineUntil { date: "30/06/2027" },
        LicenceMessage::Confirmed,
    ];
    for message in &all {
        match message {
            LicenceMessage::AskForKey
            | LicenceMessage::InvalidKey
            | LicenceMessage::InUseElsewhere { .. }
            | LicenceMessage::Revoked { .. }
            | LicenceMessage::Expired { .. }
            | LicenceMessage::Withdrawn { .. }
            | LicenceMessage::NeedsInternet
            | LicenceMessage::Wait { .. }
            | LicenceMessage::Unreachable
            | LicenceMessage::NotSaved
            | LicenceMessage::ConnectBy { .. }
            | LicenceMessage::EndsOn { .. }
            | LicenceMessage::Permanent
            | LicenceMessage::Until { .. }
            | LicenceMessage::OfflineUntil { .. }
            | LicenceMessage::Confirmed => {}
        }
    }
    all.into_iter().map(Message::Licence).collect()
}

#[test]
fn every_licence_message_is_translated_and_keeps_its_details() {
    assert_messages_translated(&every_licence_message());
}
