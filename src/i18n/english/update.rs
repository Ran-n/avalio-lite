// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:08.356943
//+ Revised: 	2026/11/23 00:26:08.356943
// -----------------------------------------------------------------------------

//! The update texts in English: the Settings status lines, the banner and the note after an update.

use crate::i18n::mebibytes;
use crate::i18n::messages::UpdateMessage;
use crate::update::download::Progress;
use crate::update::release::{CheckError, NetworkError, UpdateError};

/// An update text in English.
pub(super) fn message(message: &UpdateMessage) -> String {
    match message {
        UpdateMessage::Version { version, target: Some(target) } => {
            format!("Version {version} · release build for {target}")
        }
        UpdateMessage::Version { version, target: None } => format!("Version {version} · built from source"),
        UpdateMessage::SourceBuild => {
            "This copy was built from source. Update it with git pull and a new build.".to_owned()
        }
        UpdateMessage::NotChecked => "Not checked for updates yet.".to_owned(),
        UpdateMessage::UpToDate { time } => format!("avalio lite is up to date (checked {time})."),
        UpdateMessage::Available { version, time } => format!("avalio lite {version} is available (checked {time})."),
        UpdateMessage::NoBuild { version } => {
            format!("avalio lite {version} is out, but has no build for this system yet.")
        }
        UpdateMessage::Skipped { version, time } => {
            format!("avalio lite {version} is available; you chose to skip it (checked {time}).")
        }
        UpdateMessage::CheckFailed { reasons } => check_failed(reasons),
        UpdateMessage::Offer { version } => format!("avalio lite {version} is available."),
        UpdateMessage::Downloading { version, progress } => downloading(version, progress),
        UpdateMessage::Failed { version, error, page } => update_failed(version, error, page),
        UpdateMessage::Updated { version } => format!("Updated to avalio lite {version}."),
        UpdateMessage::NotRestarted => {
            "avalio lite was updated, but couldn't open again by itself. Open it again from the Start menu or its folder.".to_owned()
        }
    }
}

/// Every source's reason for failing, in order, as the Settings line shows them.
fn check_failed(reasons: &[(String, CheckError)]) -> String {
    let reasons: Vec<String> =
        reasons.iter().map(|(source, error)| format!("{source}: {}", check_reason(error))).collect();
    format!("Couldn't check for updates: {}.", reasons.join("; "))
}

/// The banner while the update downloads: a percentage, a size, or nothing when neither is known.
fn downloading(version: &str, progress: &Option<Progress>) -> String {
    match progress {
        Some(Progress::Percent(percent)) => format!("Downloading avalio lite {version}… {percent} %"),
        Some(Progress::Bytes(bytes)) => {
            format!("Downloading avalio lite {version}… {} MiB", mebibytes(*bytes, '.'))
        }
        None => format!("Downloading avalio lite {version}…"),
    }
}

/// A failed source's reason, as the Settings line shows it.
fn check_reason(error: &CheckError) -> String {
    match error {
        CheckError::Network(reason) => network_reason(*reason),
        CheckError::Unreadable(_) => "the answer wasn't understood".to_owned(),
        CheckError::BrokeOff => "the check broke off".to_owned(),
    }
}

/// Why a request failed, in short words: no URL, no header.
fn network_reason(reason: NetworkError) -> String {
    match reason {
        NetworkError::TimedOut => "the request timed out".to_owned(),
        NetworkError::NoServer => "the server couldn't be found".to_owned(),
        NetworkError::ConnectionFailed => "the connection failed".to_owned(),
        NetworkError::TooLarge => "the answer was too large".to_owned(),
        NetworkError::NotHttps => "the server didn't use HTTPS".to_owned(),
        NetworkError::Tls => "the secure connection failed".to_owned(),
        NetworkError::Status(status) => format!("the server answered {status}"),
    }
}

/// A failed download or install: its own sentence for a refused proof or a failed replace, else
/// the reason and where to download it by hand.
fn update_failed(version: &str, error: &UpdateError, page: &str) -> String {
    let reason = match error {
        UpdateError::ChecksumMismatch => {
            return format!("The download of avalio lite {version} didn't match its checksum, so it wasn't installed.");
        }
        UpdateError::NotSigned(_) => {
            return format!(
                "The download of avalio lite {version} isn't signed with avalio lite's release key, so it wasn't installed."
            );
        }
        UpdateError::Replace { path, denied } => {
            let reason =
                if *denied { "its folder can't be written to" } else { "the file is in use or can't be changed" };
            return format!(
                "avalio lite couldn't replace itself at {path}: {reason}. Download it from the release page instead: {page}"
            );
        }
        UpdateError::Network(reason) | UpdateError::TooLarge(reason) | UpdateError::TooSlow(reason) => {
            network_reason(*reason)
        }
        UpdateError::Disk(_) => "it couldn't be saved on this computer".to_owned(),
        UpdateError::BrokeOff | UpdateError::Cancelled => "the download broke off".to_owned(),
    };
    format!("avalio lite {version} couldn't be downloaded: {reason}. Try again, or download it from {page}")
}
