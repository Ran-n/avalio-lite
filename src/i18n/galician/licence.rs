// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.792203
//+ Revised: 	2026/11/23 00:26:08.792203
// -----------------------------------------------------------------------------

//! The licence texts in Galician: the licence screen's notices, the sheet's warnings and the Settings
//! card.

use crate::i18n::messages::LicenceMessage;

/// A licence text in Galician.
pub(super) fn message(message: &LicenceMessage) -> String {
    match message {
        LicenceMessage::AskForKey => ask_for_key(),
        LicenceMessage::InvalidKey => "Esa clave non é válida para avalio lite. Revísaa e téntao de novo.".to_owned(),
        LicenceMessage::InUseElsewhere { email } => {
            format!("Esta clave xa se está a usar noutro ordenador. Para pasala a este, escribe a {email}.")
        }
        LicenceMessage::Revoked { email } => {
            format!("Esta licenza foi cancelada. Escribe a {email} se cres que é un erro.")
        }
        LicenceMessage::Expired { date, email } => expired(*date, email),
        LicenceMessage::Withdrawn { email } => {
            format!("Agora mesmo non se pode licenciar avalio lite. Escribe a {email}.")
        }
        LicenceMessage::NeedsInternet => needs_internet(),
        LicenceMessage::Wait { minutes } => format!("Demasiados intentos. Téntao de novo en {minutes} min."),
        LicenceMessage::Unreachable => {
            "Non se pode contactar co servidor de licenzas. Comproba a conexión a internet e téntao de novo.".to_owned()
        }
        LicenceMessage::NotSaved => not_saved(),
        LicenceMessage::ConnectBy { date } => {
            format!("Conéctate a internet antes do {date} para seguir usando avalio lite.")
        }
        LicenceMessage::EndsOn { date, email } => {
            format!("A túa licenza remata o {date}. Escribe a {email} para renovala.")
        }
        LicenceMessage::Permanent => "Permanente".to_owned(),
        LicenceMessage::Until { date } => format!("Ata o {date}"),
        LicenceMessage::OfflineUntil { date } => format!("Funciona sen conexión ata o {date}"),
        LicenceMessage::Confirmed => "Licenza confirmada.".to_owned(),
    }
}

/// The licence screen's first notice, with the format of a key.
fn ask_for_key() -> String {
    "avalio lite necesita unha clave de licenza para funcionar. Recibíchela por correo; ten esta forma:
BRN-XXXX-XXXX-XXXX-XXXX"
        .to_owned()
}

/// The notice when a licence has ended, with its date when the server gave one.
fn expired(date: Option<&str>, email: &str) -> String {
    match date {
        Some(date) => format!("A túa licenza rematou o {date}. Escribe a {email} para renovala."),
        None => format!("A túa licenza rematou. Escribe a {email} para renovala."),
    }
}

/// The notice for a computer that has to connect to the internet to check the licence.
fn needs_internet() -> String {
    "Conéctate a internet para que avalio lite poida confirmar a túa licenza. Funciona sen conexión ata 30 días seguidos."
        .to_owned()
}

/// The notice when the licence couldn't be written to this computer.
fn not_saved() -> String {
    "Non se puido gardar a licenza neste ordenador. Comproba que hai espazo libre e téntao de novo; o rexistro ten os detalles."
        .to_owned()
}
