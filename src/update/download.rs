// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.534581
//+ Revised: 	2026/11/23 11:49:20.805997
// -----------------------------------------------------------------------------

//! The verified download. Order matters: the `.b3` and the `.minisig` are fetched and the
//! signature's statement checked for this version and asset name before one byte of the asset is
//! requested; then the asset is streamed to a temporary file, hashed as it arrives (BLAKE3 and
//! BLAKE2b-512 together), compared with the signed hash, and the signature checked over the bytes
//! received. On any failure the temporary file is deleted.

use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

mod proof;

pub use proof::{Proof, proof};

use ed25519_dalek::VerifyingKey;

use super::check::Offer;
use super::release::{NetworkError, UpdateError};
use super::signature::hashes::{Hasher, Hashes};
use super::source::Http;

/// Temporary download files start with this, so leftover cleanup knows them.
pub const PREFIX: &str = "avalio-lite-update-";

const CHUNK: usize = 64 * 1024;
const MIB: u64 = 1024 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// the largest download, declared or delivered
    pub size: u64,
    /// no data for this long ends it
    pub stall: Duration,
    /// the whole download may take this long
    pub total: Duration,
}

impl Default for Limits {
    fn default() -> Self {
        Self { size: 200 * MIB, stall: Duration::from_secs(60), total: Duration::from_secs(30 * 60) }
    }
}

/// How far a download has come: sent once per whole percent, or per MiB when the size is unknown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    Percent(u8),
    Bytes(u64),
}

/// What to download: the offer, the client that asks for it, and the keys its proof must hold for.
pub struct Fetch<'a> {
    pub http: &'a Http,
    pub offer: &'a Offer,
    pub keys: &'a [VerifyingKey],
}

/// Where a download goes and when it stops: the folder, the limits, and the flag a quit or Cancel
/// sets.
pub struct Saving<'a> {
    pub folder: &'a Path,
    pub limits: Limits,
    pub cancel: &'a AtomicBool,
}

/// Downloads the offered asset into a new temporary file in `saving.folder` once its proof holds,
/// and returns the file, verified. `saving.cancel` is checked between chunks and once more after
/// the hash check, so a quit or Cancel never lets a download through.
pub fn download(fetch: &Fetch, saving: &Saving, mut progress: impl FnMut(Progress)) -> Result<PathBuf, UpdateError> {
    let (Fetch { http, offer, keys }, Saving { folder, limits, cancel }) = (fetch, saving);
    let (offer, folder, limits, cancel): (&Offer, &Path, Limits, &AtomicBool) = (offer, folder, *limits, cancel);
    let proof = proof(http, offer, keys)?;
    if cancel.load(Ordering::SeqCst) {
        return Err(UpdateError::Cancelled);
    }
    fs::create_dir_all(folder).map_err(|error| UpdateError::Disk(format!("{}: {error}", folder.display())))?;
    let file = tempfile::Builder::new()
        .prefix(PREFIX)
        .tempfile_in(folder)
        .map_err(|error| UpdateError::Disk(format!("{}: {error}", folder.display())))?;
    // a failure from here on drops `file`, which deletes it
    let mut file = file;
    let client = http.for_download(limits.stall, limits.total);
    let streaming = Saving { folder, limits, cancel };
    stream((&client, offer), &mut file, &streaming, &mut progress).and_then(|got| {
        proof.check(&got, &offer.asset.name)?;
        if cancel.load(Ordering::SeqCst) {
            return Err(UpdateError::Cancelled);
        }
        Ok(())
    })?;
    let (_, path) = file.keep().map_err(|error| UpdateError::Disk(error.to_string()))?;
    Ok(path)
}

/// Streams the asset into `file` in 64 KiB chunks and returns its hashes.
fn stream(
    (client, offer): (&Http, &Offer),
    file: &mut tempfile::NamedTempFile,
    saving: &Saving,
    progress: &mut impl FnMut(Progress),
) -> Result<Hashes, UpdateError> {
    let limits = saving.limits;
    let started = Instant::now();
    let mut response = client.get(&offer.asset.url, "application/octet-stream").map_err(slow_or_network)?;
    let declared = declared_length(&response, offer, limits)?;
    let mut body = response.body_mut().with_config().limit(limits.size.saturating_add(1)).reader();
    let mut hasher = Hasher::new();
    let mut buffer = vec![0u8; CHUNK];
    let (mut received, mut last_sent) = (0u64, None);
    loop {
        check_running(saving, started, &offer.asset.name)?;
        let read = body.read(&mut buffer).map_err(|error| slow_or_network_io(&error))?;
        if read == 0 {
            break;
        }
        received += read as u64;
        if received > limits.size {
            return Err(UpdateError::TooLarge(NetworkError::TooLarge));
        }
        hasher.update(&buffer[..read]);
        file.write_all(&buffer[..read]).map_err(|error| UpdateError::Disk(error.to_string()))?;
        let now = progress_of(received, declared);
        if last_sent != Some(now) {
            progress(now);
            last_sent = Some(now);
        }
    }
    if declared.is_some_and(|length| length != received) {
        return Err(UpdateError::Network(NetworkError::ConnectionFailed));
    }
    file.as_file().sync_all().map_err(|error| UpdateError::Disk(error.to_string()))?;
    Ok(hasher.finalize())
}

/// The length the server declares for the asset, refused when it is over the size limit before a
/// byte of the body is read.
fn declared_length(
    response: &ureq::http::Response<ureq::Body>,
    offer: &Offer,
    limits: Limits,
) -> Result<Option<u64>, UpdateError> {
    let declared = response.body().content_length();
    if declared.is_some_and(|length| length > limits.size) {
        log::warn!(target: "avalio_lite::update", "{} declares {declared:?} bytes, over the limit", offer.asset.name);
        return Err(UpdateError::TooLarge(NetworkError::TooLarge));
    }
    Ok(declared)
}

/// Stops a download that was cancelled (a quit or Cancel) or has run past the whole-download limit.
fn check_running(saving: &Saving, started: Instant, name: &str) -> Result<(), UpdateError> {
    if saving.cancel.load(Ordering::SeqCst) {
        log::info!(target: "avalio_lite::update", "the download of {name} was cancelled");
        return Err(UpdateError::Cancelled);
    }
    if started.elapsed() > saving.limits.total {
        return Err(UpdateError::TooSlow(NetworkError::TimedOut));
    }
    Ok(())
}

/// How far the download has come: a whole percent of the declared length, or per MiB when the
/// length is unknown or empty.
fn progress_of(received: u64, declared: Option<u64>) -> Progress {
    match declared {
        Some(length) if length > 0 => Progress::Percent(((received * 100) / length).min(100) as u8),
        _ => Progress::Bytes(received / MIB * MIB),
    }
}

fn slow_or_network(error: NetworkError) -> UpdateError {
    match error {
        NetworkError::TimedOut => UpdateError::TooSlow(error),
        NetworkError::TooLarge => UpdateError::TooLarge(error),
        other => UpdateError::Network(other),
    }
}

fn slow_or_network_io(error: &std::io::Error) -> UpdateError {
    slow_or_network(NetworkError::from_io(error))
}
