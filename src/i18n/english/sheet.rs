// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.033853
//+ Revised: 	2026/11/25 11:43:06.472992
// -----------------------------------------------------------------------------

//! The sheet's texts in English: the confirmations and the note on Ctrl+S.

use crate::i18n::messages::SheetMessage;

/// A sheet text in English.
pub(super) fn message(message: &SheetMessage) -> String {
    match message {
        SheetMessage::ConfirmDeleteClass { class } => format!(
            "Delete the class \"{class}\" with all its columns and marks?\n\nIts pupils stay in their other classes."
        ),
        SheetMessage::ConfirmRemovePupil { pupil, class } => format!(
            "Remove {pupil} from {class}?\n\nTheir marks in this class are deleted. They stay in their other classes."
        ),
        SheetMessage::ConfirmDeleteColumn { column } => {
            format!("Delete the column \"{column}\" and everything typed in it?")
        }
        SheetMessage::ConfirmDeletePupil { pupil } => {
            format!("Delete {pupil}? They're in no class; this can't be undone.")
        }
        SheetMessage::UntitledColumn { letter } => format!("column {letter} (no title)"),
        SheetMessage::EverythingSaved => "Everything is saved. avalio lite saves each change as you type.".to_owned(),
        SheetMessage::ConfirmRemovePicture { pupil } => format!("Delete the picture of {pupil}?"),
        SheetMessage::PictureFileType => "Picture".to_owned(),
        SheetMessage::PupilViewWhileUpdating => "An update is downloading and will restart avalio lite. Wait for it, or cancel it, before showing a pupil their grades.".to_owned(),
        SheetMessage::PupilNotInClass => "That pupil is no longer in this class.".to_owned(),
    }
}
