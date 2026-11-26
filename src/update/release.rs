// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:59.186961
//+ Revised: 	2026/11/22 21:06:12.691672
// -----------------------------------------------------------------------------

//! A release as a forge describes it, the forges' answers read into it, and what can go wrong on
//! the way.

use serde_json::Value;

use super::signature::SigError;

/// The latest release a source reports.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Release {
    pub version: semver::Version,
    /// the release's page, for downloading by hand when an update can't be installed
    pub page: String,
    pub assets: Vec<Asset>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Asset {
    pub name: String,
    pub url: String,
}

impl Release {
    pub fn asset(&self, name: &str) -> Option<&Asset> {
        self.assets.iter().find(|asset| asset.name == name)
    }
}

/// Why a request failed, in the short words the user is shown: no URL, no header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetworkError {
    TimedOut,
    NoServer,
    ConnectionFailed,
    TooLarge,
    NotHttps,
    Tls,
    /// any status other than 200
    Status(u16),
}

impl NetworkError {
    /// The reason for a failed ureq call.
    pub fn from_ureq(error: &ureq::Error) -> Self {
        match error {
            ureq::Error::Timeout(_) | ureq::Error::BodyStalled => Self::TimedOut,
            ureq::Error::HostNotFound => Self::NoServer,
            ureq::Error::BodyExceedsLimit(_) | ureq::Error::LargeResponseHeader(..) => Self::TooLarge,
            ureq::Error::RequireHttpsOnly(_) | ureq::Error::TlsRequired => Self::NotHttps,
            ureq::Error::StatusCode(status) => Self::Status(*status),
            ureq::Error::Tls(_) => Self::Tls,
            ureq::Error::Io(io) => Self::from_io(io),
            other => {
                // the TLS library's own errors depend on the features ureq was built with
                let text = other.to_string().to_ascii_lowercase();
                if text.contains("tls") || text.contains("certificate") { Self::Tls } else { Self::ConnectionFailed }
            }
        }
    }

    /// The reason for a failed read of an answer's body.
    pub fn from_io(error: &std::io::Error) -> Self {
        match error.kind() {
            std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock => Self::TimedOut,
            _ => {
                // ureq wraps its own errors (a body over the limit) in io::Error
                match error.get_ref().and_then(|inner| inner.downcast_ref::<ureq::Error>()) {
                    Some(inner) => Self::from_ureq(inner),
                    None => Self::ConnectionFailed,
                }
            }
        }
    }
}

impl std::fmt::Display for NetworkError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::TimedOut => formatter.write_str("the request timed out"),
            Self::NoServer => formatter.write_str("the server couldn't be found"),
            Self::ConnectionFailed => formatter.write_str("the connection failed"),
            Self::TooLarge => formatter.write_str("the answer was too large"),
            Self::NotHttps => formatter.write_str("the server didn't use HTTPS"),
            Self::Tls => formatter.write_str("the secure connection failed"),
            Self::Status(status) => write!(formatter, "the server answered {status}"),
        }
    }
}

/// Why one source's check failed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CheckError {
    #[error("{0}")]
    Network(NetworkError),
    /// the answer wasn't a release: unreadable JSON, a missing field, a tag that isn't a version
    #[error("unexpected answer: {0}")]
    Unreadable(String),
    /// the check's thread panicked
    #[error("the check broke off")]
    BrokeOff,
}

/// Why a download or install failed. Each has its own sentence for the user (`i18n`).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum UpdateError {
    #[error("{0}")]
    Network(NetworkError),
    /// the `.b3` or `.minisig` wasn't readable, or the signature isn't by a trusted key (before or
    /// after the download)
    #[error("not signed: {0}")]
    NotSigned(SigError),
    /// the downloaded bytes don't match the signed checksum
    #[error("the download didn't match its checksum")]
    ChecksumMismatch,
    /// declared, or delivered, over the size limit
    #[error("{0}")]
    TooLarge(NetworkError),
    /// no data for too long, or the whole download took too long
    #[error("{0}")]
    TooSlow(NetworkError),
    #[error("cancelled")]
    Cancelled,
    #[error("{0}")]
    Disk(String),
    /// the running program couldn't be replaced; it is still the old one. `denied`: its folder
    /// can't be written to (an all-users install, a locked USB stick); else the file is in use or
    /// can't be changed. The system's own words go to the log.
    #[error("couldn't replace {path} ({})", if *denied { "no permission" } else { "in use or can't be changed" })]
    Replace { path: String, denied: bool },
    /// the download thread panicked
    #[error("the download broke off")]
    BrokeOff,
}

/// A version from a release tag, with or without its leading `v`.
pub fn parse_tag(tag: &str) -> Result<semver::Version, CheckError> {
    let version = tag.strip_prefix('v').unwrap_or(tag);
    semver::Version::parse(version)
        .map_err(|error| CheckError::Unreadable(format!("the tag {tag:?} isn't a version: {error}")))
}

/// GitHub's `releases/latest`: version from `tag_name`, page `html_url`, files
/// `assets[].name` + `browser_download_url`.
pub fn parse_github(json: &str) -> Result<Release, CheckError> {
    let value = parse_json(json)?;
    let version = parse_tag(text(&value, &["tag_name"])?)?;
    let page = web_page(text(&value, &["html_url"])?)?;
    let assets = list(&value, &["assets"])?
        .iter()
        .map(|asset| {
            Ok(Asset {
                name: text(asset, &["name"])?.to_owned(),
                url: text(asset, &["browser_download_url"])?.to_owned(),
            })
        })
        .collect::<Result<_, CheckError>>()?;
    Ok(Release { version, page, assets })
}

/// GitLab's `releases/permalink/latest`: version from `tag_name`, page `_links.self`, files
/// `assets.links[].name` + `direct_asset_url` (else `url`).
pub fn parse_gitlab(json: &str) -> Result<Release, CheckError> {
    let value = parse_json(json)?;
    let version = parse_tag(text(&value, &["tag_name"])?)?;
    let page = web_page(text(&value, &["_links", "self"])?)?;
    let assets = list(&value, &["assets", "links"])?
        .iter()
        .map(|link| {
            let url = text(link, &["direct_asset_url"]).or_else(|_| text(link, &["url"]))?;
            Ok(Asset { name: text(link, &["name"])?.to_owned(), url: url.to_owned() })
        })
        .collect::<Result<_, CheckError>>()?;
    Ok(Release { version, page, assets })
}

fn parse_json(json: &str) -> Result<Value, CheckError> {
    serde_json::from_str(json).map_err(|error| CheckError::Unreadable(format!("not JSON: {error}")))
}

fn field<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Value, CheckError> {
    path.iter()
        .try_fold(value, |value, key| value.get(key))
        .ok_or_else(|| CheckError::Unreadable(format!("the answer has no {}", path.join("."))))
}

fn text<'a>(value: &'a Value, path: &[&str]) -> Result<&'a str, CheckError> {
    field(value, path)?.as_str().ok_or_else(|| CheckError::Unreadable(format!("{} isn't text", path.join("."))))
}

fn list<'a>(value: &'a Value, path: &[&str]) -> Result<&'a Vec<Value>, CheckError> {
    field(value, path)?.as_array().ok_or_else(|| CheckError::Unreadable(format!("{} isn't a list", path.join("."))))
}

/// The page, refused unless it is an `https://` address and nothing else: it is handed to Windows
/// to open, which would as readily start a program named there.
fn web_page(address: &str) -> Result<String, CheckError> {
    if is_web_address(address) {
        Ok(address.to_owned())
    } else {
        Err(CheckError::Unreadable(format!("the release page {address:?} isn't a web address")))
    }
}

pub fn is_web_address(address: &str) -> bool {
    const SCHEME: &str = "https://";
    address.len() > SCHEME.len()
        && address.starts_with(SCHEME)
        && !address.chars().any(|character| character.is_whitespace() || character.is_control())
}
