// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.328065
//+ Revised: 	2026/11/23 00:26:09.328065
// -----------------------------------------------------------------------------

//! The data texts: the database, the logs, backups, the PIN and the lock; one of each, checked in
//! both languages.

use avalio_lite::i18n::{DataMessage, Message};

use super::assert_messages_translated;

/// One of each data message. The match below has no wildcard, so a new variant won't compile until
/// it is added here too.
pub fn every_data_message() -> Vec<Message<'static>> {
    let renamed: &'static [(String, String)] = Box::leak(Box::new([("1A".to_owned(), "1A (2)".to_owned())]));
    let all = [
        DataMessage::DatabaseMoved { path: "D:\\x" },
        DataMessage::DatabaseMovedOldKept { path: "D:\\x", old: "C:\\old" },
        DataMessage::LogsMoved { path: "D:\\y" },
        DataMessage::DatabaseFileType,
        DataMessage::BackupFileType,
        DataMessage::BackupExported { path: "D:\\x" },
        DataMessage::ConfirmRestore { file: "Exam", classes: 4, pupils: 1 },
        DataMessage::BackupRestored { file: "Exam" },
        DataMessage::BackupClassesAdded { classes: 3, pupils: 1, existing: 2, file: "Exam" },
        DataMessage::BackupClassesAdded { classes: 1, pupils: 2, existing: 1, file: "Exam" },
        DataMessage::BackupClassesAdded { classes: 2, pupils: 2, existing: 0, file: "Exam" },
        DataMessage::BackupClassesRenamed { pairs: renamed },
        DataMessage::BackupHasNoClasses,
        DataMessage::LockNeedsPin,
        DataMessage::PinTurnedOn,
        DataMessage::PinChanged,
        DataMessage::PinTurnedOff,
    ];
    for message in &all {
        match message {
            DataMessage::DatabaseMoved { .. }
            | DataMessage::DatabaseMovedOldKept { .. }
            | DataMessage::LogsMoved { .. }
            | DataMessage::DatabaseFileType
            | DataMessage::BackupFileType
            | DataMessage::BackupExported { .. }
            | DataMessage::ConfirmRestore { .. }
            | DataMessage::BackupRestored { .. }
            | DataMessage::BackupClassesAdded { .. }
            | DataMessage::BackupClassesRenamed { .. }
            | DataMessage::BackupHasNoClasses
            | DataMessage::LockNeedsPin
            | DataMessage::PinTurnedOn
            | DataMessage::PinChanged
            | DataMessage::PinTurnedOff => {}
        }
    }
    all.into_iter().map(Message::Data).collect()
}

#[test]
fn every_data_message_is_translated_and_keeps_its_details() {
    assert_messages_translated(&every_data_message());
}
