// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:07.075111
//+ Revised: 	2026/11/25 11:43:07.542487
// -----------------------------------------------------------------------------

//! The sheet topic of `Message`. Its texts are in `english/sheet.rs` and `galician/sheet.rs`.

/// The sheet's own texts: confirmations of deletions and removals, untitled columns, and the note
/// on Ctrl+S.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SheetMessage<'a> {
    ConfirmDeleteClass {
        class: &'a str,
    },
    ConfirmRemovePupil {
        pupil: &'a str,
        class: &'a str,
    },
    ConfirmDeleteColumn {
        column: &'a str,
    },
    /// a pupil who is in no class; deleting them is for good
    ConfirmDeletePupil {
        pupil: &'a str,
    },
    /// how a left-out column with no header is named, by its letter
    UntitledColumn {
        letter: &'a str,
    },
    /// the answer to Ctrl+S: there is nothing to save, since every change is saved as it is typed
    EverythingSaved,
    ConfirmRemovePicture {
        pupil: &'a str,
    },
    /// the filter's name in the dialog a pupil's picture is chosen in
    PictureFileType,
    /// the pupil view can't start: the update under way ends in a restart, which would leave it
    PupilViewWhileUpdating,
    /// the pupil picked in the pupil view was removed from the class meanwhile
    PupilNotInClass,
}
