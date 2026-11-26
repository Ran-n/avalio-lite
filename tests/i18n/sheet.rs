// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.757616
//+ Revised: 	2026/11/25 11:43:15.393700
// -----------------------------------------------------------------------------

//! The sheet's texts: one of each, checked in both languages.

use avalio_lite::i18n::{Message, SheetMessage};

use super::assert_messages_translated;

/// One of each sheet message. The match below has no wildcard, so a new variant won't compile until
/// it is added here too, and then this topic's test checks its translation.
pub fn every_sheet_message() -> Vec<Message<'static>> {
    let all = [
        SheetMessage::ConfirmDeleteClass { class: "1A" },
        SheetMessage::ConfirmRemovePupil { pupil: "Ana", class: "1A" },
        SheetMessage::ConfirmDeleteColumn { column: "Exam" },
        SheetMessage::ConfirmDeletePupil { pupil: "Ana" },
        SheetMessage::UntitledColumn { letter: "D" },
        SheetMessage::EverythingSaved,
        SheetMessage::ConfirmRemovePicture { pupil: "Ana" },
        SheetMessage::PictureFileType,
        SheetMessage::PupilViewWhileUpdating,
        SheetMessage::PupilNotInClass,
    ];
    for message in &all {
        match message {
            SheetMessage::ConfirmDeleteClass { .. }
            | SheetMessage::ConfirmRemovePupil { .. }
            | SheetMessage::ConfirmDeleteColumn { .. }
            | SheetMessage::ConfirmDeletePupil { .. }
            | SheetMessage::UntitledColumn { .. }
            | SheetMessage::EverythingSaved
            | SheetMessage::ConfirmRemovePicture { .. }
            | SheetMessage::PictureFileType
            | SheetMessage::PupilViewWhileUpdating
            | SheetMessage::PupilNotInClass => {}
        }
    }
    all.into_iter().map(Message::Sheet).collect()
}

#[test]
fn every_sheet_message_is_translated_and_keeps_its_details() {
    assert_messages_translated(&every_sheet_message());
}
