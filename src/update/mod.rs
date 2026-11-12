// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.678835
//+ Revised: 	2026/11/12 14:39:55.467722
// -----------------------------------------------------------------------------

//! Updating the app from its releases on GitHub: whether a newer version is out (`source`), and
//! downloading it (`download`), checking its signature (`signature`) and putting it in place
//! (`install`). The UI side is in `ui`.

pub mod download;
pub mod install;
pub mod signature;
pub mod source;
pub mod ui;
#[cfg(windows)]
pub mod windows;

use chrono::{DateTime, TimeDelta, Utc};
use serde::Deserialize;

/// The release key's public half (`release_sign new-key`); every downloaded update must carry a
/// signature it verifies.
pub const PUBLIC_KEY: &str = "8b8n9Uqg_5-ReHLUbp5xyfL6HwdVgYNXiJQdP0Uex5o";

/// The key every update must be signed with.
pub fn release_key() -> ed25519_dalek::VerifyingKey {
    // a constant of this build, and a test proves it parses
    signature::public_key(PUBLIC_KEY).expect("the embedded release key is valid")
}

/// The latest published release (drafts and pre-releases left out by GitHub).
pub const RELEASES_API: &str = "https://api.github.com/repos/Ran-n/avalio-lite/releases/latest";

/// How long after a check the next automatic one is due.
pub const CHECK_INTERVAL: TimeDelta = TimeDelta::hours(24);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: semver::Version,
    /// the release's page, for "What's new" and for downloading by hand
    pub page: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Asset {
    pub name: String,
    #[serde(rename = "browser_download_url")]
    pub url: String,
    pub size: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpdateError {
    #[error("{0}")]
    Network(String),
    #[error("GitHub asks to wait before checking again")]
    RateLimited,
    #[error("unexpected answer: {0}")]
    BadAnswer(String),
    /// the release lacks the file this copy updates from (or its signature)
    #[error("the release has no {0}")]
    MissingAsset(String),
    #[error("{0}")]
    Disk(String),
    /// the download isn't signed with the release key: it may have been tampered with
    #[error("the update couldn't be verified: {0}")]
    NotVerified(signature::SigError),
    /// the new portable exe couldn't be put in place of the running one, which stays where it was
    #[error("{0}")]
    Swap(String),
    /// the new portable exe couldn't be put in place, and the running one couldn't be put back
    /// either: it is left at `old` (`<exe>.old`)
    #[error("{detail}; the app's exe is left at {}", old.display())]
    SwapLeftAside { old: std::path::PathBuf, detail: String },
}

impl UpdateError {
    /// What the user is told about a failed check or download; `None` for the failures that
    /// have a message of their own (a missing file, a bad signature, a failed swap).
    pub fn problem(&self) -> Option<crate::i18n::UpdateProblem> {
        use crate::i18n::UpdateProblem;
        match self {
            Self::Network(_) => Some(UpdateProblem::Offline),
            Self::RateLimited => Some(UpdateProblem::RateLimited),
            Self::BadAnswer(_) => Some(UpdateProblem::BadAnswer),
            Self::Disk(_) => Some(UpdateProblem::Disk),
            Self::MissingAsset(_) | Self::NotVerified(_) | Self::Swap(_) | Self::SwapLeftAside { .. } => None,
        }
    }
}

#[derive(Deserialize)]
struct ReleaseJson {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<Asset>,
}

/// A release from the GitHub API's JSON. The tag is `vX.Y.Z`.
pub fn parse_release(json: &str) -> Result<Release, UpdateError> {
    let release: ReleaseJson = serde_json::from_str(json).map_err(|error| UpdateError::BadAnswer(error.to_string()))?;
    let version = release.tag_name.strip_prefix('v').unwrap_or(&release.tag_name);
    let version = semver::Version::parse(version)
        .map_err(|error| UpdateError::BadAnswer(format!("the tag {:?} isn't a version: {error}", release.tag_name)))?;
    Ok(Release { version, page: release.html_url, assets: release.assets })
}

/// This build's version.
pub fn current_version() -> semver::Version {
    // Cargo only accepts semver versions, so the package's own always parses
    semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| semver::Version::new(0, 0, 0))
}

pub fn newer(release: &Release, current: &semver::Version) -> bool {
    release.version > *current
}

/// Whether an automatic check is due: never checked, the last one is a day old, or it is noted
/// in the future (the clock was once set ahead), which would otherwise stop the checks until the
/// clock caught up with it.
pub fn due(last: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    last.is_none_or(|last| last > now || now - last >= CHECK_INTERVAL)
}
