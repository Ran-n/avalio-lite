// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 00:26:10.003580
//+ Revised: 	2026/11/23 00:26:10.003580
// -----------------------------------------------------------------------------

//! The update texts: one of each, every way a check or a download can fail included, checked in both
//! languages.

use avalio_lite::i18n::{Message, UpdateMessage};
use avalio_lite::update::download::Progress;
use avalio_lite::update::release::{CheckError, NetworkError, UpdateError};
use avalio_lite::update::signature::SigError;

use super::assert_messages_translated;

/// One of each update message. The match below has no wildcard, so a new variant won't compile until
/// it is added here too.
pub fn every_update_message() -> Vec<Message<'static>> {
    let mut all = vec![
        UpdateMessage::Version { version: "0.11.0", target: Some("windows-x86_64") },
        UpdateMessage::Version { version: "0.11.0", target: None },
        UpdateMessage::SourceBuild,
        UpdateMessage::NotChecked,
        UpdateMessage::UpToDate { time: "14:02" },
        UpdateMessage::Available { version: "0.12.0", time: "14:02" },
        UpdateMessage::NoBuild { version: "0.12.0" },
        UpdateMessage::Skipped { version: "0.12.0", time: "14:02" },
        UpdateMessage::CheckFailed { reasons: every_check_failure() },
        UpdateMessage::Offer { version: "0.12.0" },
        UpdateMessage::Downloading { version: "0.12.0", progress: None },
        UpdateMessage::Downloading { version: "0.12.0", progress: Some(Progress::Percent(42)) },
        UpdateMessage::Downloading { version: "0.12.0", progress: Some(Progress::Bytes(3_250_000)) },
    ];
    for error in every_update_error() {
        all.push(UpdateMessage::Failed { version: "0.12.0", error, page: "https://example.org/release" });
    }
    all.extend([UpdateMessage::Updated { version: "0.6.0" }, UpdateMessage::NotRestarted]);
    for message in &all {
        match message {
            UpdateMessage::Version { .. }
            | UpdateMessage::SourceBuild
            | UpdateMessage::NotChecked
            | UpdateMessage::UpToDate { .. }
            | UpdateMessage::Available { .. }
            | UpdateMessage::NoBuild { .. }
            | UpdateMessage::Skipped { .. }
            | UpdateMessage::CheckFailed { .. }
            | UpdateMessage::Offer { .. }
            | UpdateMessage::Downloading { .. }
            | UpdateMessage::Failed { .. }
            | UpdateMessage::Updated { .. }
            | UpdateMessage::NotRestarted => {}
        }
    }
    all.into_iter().map(Message::Update).collect()
}

#[test]
fn every_update_message_is_translated_and_keeps_its_details() {
    assert_messages_translated(&every_update_message());
}

/// Every reason a check fails, from each source; kept complete by the match.
fn every_check_failure() -> &'static [(String, CheckError)] {
    let all: Vec<CheckError> = every_network_error()
        .into_iter()
        .map(CheckError::Network)
        .chain([CheckError::Unreadable("x".into()), CheckError::BrokeOff])
        .collect();
    for error in &all {
        match error {
            CheckError::Network(_) | CheckError::Unreadable(_) | CheckError::BrokeOff => {}
        }
    }
    Box::leak(all.into_iter().map(|error| ("GitHub".to_owned(), error)).collect::<Vec<_>>().into_boxed_slice())
}

/// Every reason a request fails, each one listed; kept complete by the match.
fn every_network_error() -> Vec<NetworkError> {
    let all = vec![
        NetworkError::TimedOut,
        NetworkError::NoServer,
        NetworkError::ConnectionFailed,
        NetworkError::TooLarge,
        NetworkError::NotHttps,
        NetworkError::Tls,
        NetworkError::Status(503),
    ];
    for error in &all {
        match error {
            NetworkError::TimedOut
            | NetworkError::NoServer
            | NetworkError::ConnectionFailed
            | NetworkError::TooLarge
            | NetworkError::NotHttps
            | NetworkError::Tls
            | NetworkError::Status(_) => {}
        }
    }
    all
}

/// Every way a download or install fails, each network reason included; kept complete by the match.
fn every_update_error() -> Vec<&'static UpdateError> {
    let mut all: Vec<UpdateError> = every_network_error().into_iter().map(UpdateError::Network).collect();
    all.extend([
        UpdateError::NotSigned(SigError::NotSigned),
        UpdateError::ChecksumMismatch,
        UpdateError::TooLarge(NetworkError::TooLarge),
        UpdateError::TooSlow(NetworkError::TimedOut),
        UpdateError::Cancelled,
        UpdateError::Disk("full".into()),
        UpdateError::Replace { path: r"C:\Programs\avalio-lite\avalio-lite.exe".into(), denied: true },
        UpdateError::Replace { path: r"D:\avalio-lite.exe".into(), denied: false },
        UpdateError::BrokeOff,
    ]);
    for error in &all {
        match error {
            UpdateError::Network(_)
            | UpdateError::NotSigned(_)
            | UpdateError::ChecksumMismatch
            | UpdateError::TooLarge(_)
            | UpdateError::TooSlow(_)
            | UpdateError::Cancelled
            | UpdateError::Disk(_)
            | UpdateError::Replace { .. }
            | UpdateError::BrokeOff => {}
        }
    }
    all.into_iter().map(|error| &*Box::leak(Box::new(error))).collect()
}
