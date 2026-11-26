// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.912568
//+ Revised: 	2026/11/25 11:43:07.170411
// -----------------------------------------------------------------------------

//! The sheet's texts in Galician: the confirmations and the note on Ctrl+S.

use crate::i18n::messages::SheetMessage;

/// A sheet text in Galician.
pub(super) fn message(message: &SheetMessage) -> String {
    match message {
        SheetMessage::ConfirmDeleteClass { class } => format!(
            "Eliminar a clase «{class}» con todas as súas columnas e notas?\n\nOs seus alumnos seguen nas outras clases."
        ),
        SheetMessage::ConfirmRemovePupil { pupil, class } => {
            format!("Quitar a {pupil} de {class}?\n\nBórranse as súas notas nesta clase. Segue nas outras clases.")
        }
        SheetMessage::ConfirmDeleteColumn { column } => {
            format!("Eliminar a columna «{column}» e todo o que se escribiu nela?")
        }
        SheetMessage::ConfirmDeletePupil { pupil } => {
            format!("Eliminar a {pupil}? Non está en ningunha clase; non se pode desfacer.")
        }
        SheetMessage::UntitledColumn { letter } => format!("columna {letter} (sen título)"),
        SheetMessage::EverythingSaved => "Está todo gardado. avalio lite garda cada cambio ao escribilo.".to_owned(),
        SheetMessage::ConfirmRemovePicture { pupil } => format!("Eliminar a foto de {pupil}?"),
        SheetMessage::PictureFileType => "Imaxe".to_owned(),
        SheetMessage::PupilViewWhileUpdating => "Estase a descargar unha actualización que reiniciará avalio lite. Agarda por ela, ou cancélaa, antes de amosarlle as notas a un alumno.".to_owned(),
        SheetMessage::PupilNotInClass => "Ese alumno xa non está nesta clase.".to_owned(),
    }
}
