// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 09:31:17.721797
//+ Revised: 	2026/10/28 10:52:45.590526
// -----------------------------------------------------------------------------

use super::Language;
use crate::gradebook::model::ColumnType;

/// Text shown by Rust code: confirmation questions, status lines and fatal errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Message<'a> {
    ConfirmDeleteClass { class: &'a str },
    ConfirmRemovePupil { pupil: &'a str, class: &'a str },
    ConfirmDeleteColumn { column: &'a str },
    DatabaseMoved { path: &'a str },
    LogsMoved { path: &'a str },
    DatabaseFileType,
    WindowFailed { reason: &'a str },
    LockNeedsPin,
    PinTurnedOn,
    PinChanged,
    PinTurnedOff,
}

pub fn text(language: Language, message: &Message) -> String {
    match language {
        Language::English => english(message),
        Language::Galician => galician(message),
    }
}

fn english(message: &Message) -> String {
    match message {
        Message::ConfirmDeleteClass { class } => {
            format!(
                "Delete the class \"{class}\" with all its columns and marks?\n\nIts pupils stay in their other classes."
            )
        }
        Message::ConfirmRemovePupil { pupil, class } => {
            format!(
                "Remove {pupil} from {class}?\n\nTheir marks in this class are deleted. They stay in their other classes."
            )
        }
        Message::ConfirmDeleteColumn { column } => {
            format!("Delete the column \"{column}\" and everything typed in it?")
        }
        Message::DatabaseMoved { path } => format!("Database moved to {path}"),
        Message::LogsMoved { path } => format!("Logs moved to {path}"),
        Message::DatabaseFileType => "avalio-lite database".to_owned(),
        Message::WindowFailed { reason } => format!("avalio-lite couldn't start: {reason}"),
        Message::LockNeedsPin => "Set a PIN in Settings to lock the app.".to_owned(),
        Message::PinTurnedOn => "The database is now encrypted with your PIN.".to_owned(),
        Message::PinChanged => "PIN changed; the database is re-encrypted with it.".to_owned(),
        Message::PinTurnedOff => "PIN removed; the database is no longer encrypted.".to_owned(),
    }
}

fn galician(message: &Message) -> String {
    match message {
        Message::ConfirmDeleteClass { class } => {
            format!(
                "Eliminar a clase «{class}» con todas as súas columnas e notas?\n\nOs seus alumnos seguen nas outras clases."
            )
        }
        Message::ConfirmRemovePupil { pupil, class } => {
            format!("Quitar a {pupil} de {class}?\n\nBórranse as súas notas nesta clase. Segue nas outras clases.")
        }
        Message::ConfirmDeleteColumn { column } => {
            format!("Eliminar a columna «{column}» e todo o que se escribiu nela?")
        }
        Message::DatabaseMoved { path } => format!("Base de datos movida a {path}"),
        Message::LogsMoved { path } => format!("Rexistros movidos a {path}"),
        Message::DatabaseFileType => "Base de datos de avalio-lite".to_owned(),
        Message::WindowFailed { reason } => format!("Non se puido iniciar avalio-lite: {reason}"),
        Message::LockNeedsPin => "Define un PIN en Axustes para bloquear a aplicación.".to_owned(),
        Message::PinTurnedOn => "A base de datos xa está cifrada co teu PIN.".to_owned(),
        Message::PinChanged => "PIN cambiado; a base de datos cifrouse de novo con el.".to_owned(),
        Message::PinTurnedOff => "PIN eliminado; a base de datos xa non está cifrada.".to_owned(),
    }
}

/// Title given to a new column; `number` counts tests ("Test 3").
pub fn default_column_title(language: Language, column_type: ColumnType, number: usize) -> String {
    match (language, column_type) {
        (Language::English, ColumnType::Info) => "Notes".to_owned(),
        (Language::English, ColumnType::Test) => format!("Test {number}"),
        (Language::English, ColumnType::Weighted) => "Final grade".to_owned(),
        (Language::English, ColumnType::Average) => "Average".to_owned(),
        (Language::English, ColumnType::Round) => "Rounded".to_owned(),
        (Language::Galician, ColumnType::Info) => "Observacións".to_owned(),
        (Language::Galician, ColumnType::Test) => format!("Proba {number}"),
        (Language::Galician, ColumnType::Weighted) => "Nota final".to_owned(),
        (Language::Galician, ColumnType::Average) => "Media".to_owned(),
        (Language::Galician, ColumnType::Round) => "Redondeo".to_owned(),
    }
}
