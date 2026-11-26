// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:09.166152
//+ Revised: 	2026/11/23 00:26:09.166152
// -----------------------------------------------------------------------------

//! The update texts in Galician: the Settings status lines, the banner and the note after an update.

use crate::i18n::mebibytes;
use crate::i18n::messages::UpdateMessage;
use crate::update::download::Progress;
use crate::update::release::{CheckError, NetworkError, UpdateError};

/// An update text in Galician.
pub(super) fn message(message: &UpdateMessage) -> String {
    match message {
        UpdateMessage::Version { version, target: Some(target) } => {
            format!("Versión {version} · versión publicada para {target}")
        }
        UpdateMessage::Version { version, target: None } => {
            format!("Versión {version} · compilada desde o código fonte")
        }
        UpdateMessage::SourceBuild => {
            "Esta copia compilouse desde o código fonte. Actualízaa con git pull e unha nova compilación.".to_owned()
        }
        UpdateMessage::NotChecked => "Aínda non se buscaron actualizacións.".to_owned(),
        UpdateMessage::UpToDate { time } => format!("avalio lite está ao día (comprobado ás {time})."),
        UpdateMessage::Available { version, time } => {
            format!("Xa está dispoñible avalio lite {version} (comprobado ás {time}).")
        }
        UpdateMessage::NoBuild { version } => {
            format!("Saíu avalio lite {version}, pero aínda non hai versión para este sistema.")
        }
        UpdateMessage::Skipped { version, time } => {
            format!("Xa está dispoñible avalio lite {version}; escolliches saltala (comprobado ás {time}).")
        }
        UpdateMessage::CheckFailed { reasons } => check_failed(reasons),
        UpdateMessage::Offer { version } => format!("Xa está dispoñible avalio lite {version}."),
        UpdateMessage::Downloading { version, progress } => downloading(version, progress),
        UpdateMessage::Failed { version, error, page } => update_failed(version, error, page),
        UpdateMessage::Updated { version } => format!("Actualizado a avalio lite {version}."),
        UpdateMessage::NotRestarted => {
            "avalio lite actualizouse, pero non se puido abrir de novo só. Ábreo outra vez desde o menú Inicio ou o seu cartafol.".to_owned()
        }
    }
}

/// Every source's reason for failing, in order, as the Settings line shows them.
fn check_failed(reasons: &[(String, CheckError)]) -> String {
    let reasons: Vec<String> =
        reasons.iter().map(|(source, error)| format!("{source}: {}", check_reason(error))).collect();
    format!("Non se puideron buscar actualizacións: {}.", reasons.join("; "))
}

/// The banner while the update downloads: a percentage, a size, or nothing when neither is known.
fn downloading(version: &str, progress: &Option<Progress>) -> String {
    match progress {
        Some(Progress::Percent(percent)) => format!("Descargando avalio lite {version}… {percent} %"),
        Some(Progress::Bytes(bytes)) => {
            format!("Descargando avalio lite {version}… {} MiB", mebibytes(*bytes, ','))
        }
        None => format!("Descargando avalio lite {version}…"),
    }
}

/// A failed source's reason in Galician, as the English `check_reason` says.
fn check_reason(error: &CheckError) -> String {
    match error {
        CheckError::Network(reason) => network_reason(*reason),
        CheckError::Unreadable(_) => "non se entendeu a resposta".to_owned(),
        CheckError::BrokeOff => "a comprobación interrompeuse".to_owned(),
    }
}

/// Why a request failed, in short words in Galician.
fn network_reason(reason: NetworkError) -> String {
    match reason {
        NetworkError::TimedOut => "esgotouse o tempo de espera".to_owned(),
        NetworkError::NoServer => "non se atopou o servidor".to_owned(),
        NetworkError::ConnectionFailed => "fallou a conexión".to_owned(),
        NetworkError::TooLarge => "a resposta era demasiado grande".to_owned(),
        NetworkError::NotHttps => "o servidor non usou HTTPS".to_owned(),
        NetworkError::Tls => "fallou a conexión segura".to_owned(),
        NetworkError::Status(status) => format!("o servidor respondeu {status}"),
    }
}

/// A failed download or install in Galician: its own sentence for a refused proof or a failed
/// replace, else the reason and where to download it by hand.
fn update_failed(version: &str, error: &UpdateError, page: &str) -> String {
    let reason = match error {
        UpdateError::ChecksumMismatch => {
            return format!(
                "A descarga de avalio lite {version} non coincidiu coa súa suma de verificación, así que non se instalou."
            );
        }
        UpdateError::NotSigned(_) => {
            return format!(
                "A descarga de avalio lite {version} non está asinada coa chave de publicación de avalio lite, así que non se instalou."
            );
        }
        UpdateError::Replace { path, denied } => {
            let reason = if *denied {
                "non se pode escribir no seu cartafol"
            } else {
                "o ficheiro está en uso ou non se pode cambiar"
            };
            return format!(
                "avalio lite non se puido substituír en {path}: {reason}. Descárgao desde a páxina da versión: {page}"
            );
        }
        UpdateError::Network(reason) | UpdateError::TooLarge(reason) | UpdateError::TooSlow(reason) => {
            network_reason(*reason)
        }
        UpdateError::Disk(_) => "non se puido gardar neste ordenador".to_owned(),
        UpdateError::BrokeOff | UpdateError::Cancelled => "a descarga interrompeuse".to_owned(),
    };
    format!("Non se puido descargar avalio lite {version}: {reason}. Téntao de novo ou descárgao desde {page}")
}
