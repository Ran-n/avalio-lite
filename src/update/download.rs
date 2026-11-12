// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.534581
//+ Revised: 	2026/11/12 14:39:55.295731
// -----------------------------------------------------------------------------

//! Downloading a release file and checking it against its signature before anything runs it.

use std::fs::{self, File};
use std::io;
use std::path::Path;
use std::time::Duration;

use ed25519_dalek::VerifyingKey;

use super::signature::{self, file_hash};
use super::{Asset, UpdateError};

/// Connecting may take this long; the whole download (the portable exe is about 24 MB) much more.
pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(15);
pub const DOWNLOAD_TIMEOUT: Duration = Duration::from_secs(600);

pub fn agent() -> ureq::Agent {
    let config = ureq::Agent::config_builder()
        .timeout_connect(Some(CONNECT_TIMEOUT))
        .timeout_global(Some(DOWNLOAD_TIMEOUT))
        .http_status_as_error(false)
        .build();
    config.into()
}

/// Downloads `asset` to `to`, streaming it to disk. Anything short of the whole file (a failed
/// request, another size than announced) deletes what was written.
pub fn download(agent: &ureq::Agent, asset: &Asset, to: &Path) -> Result<(), UpdateError> {
    let result = fetch(agent, asset, to);
    if result.is_err()
        && to.exists()
        && let Err(error) = fs::remove_file(to)
    {
        log::warn!("couldn't remove the partial download {}: {error}", to.display());
    }
    result
}

fn fetch(agent: &ureq::Agent, asset: &Asset, to: &Path) -> Result<(), UpdateError> {
    let mut response = agent
        .get(&asset.url)
        .header("User-Agent", concat!("avalio-lite/", env!("CARGO_PKG_VERSION")))
        .call()
        .map_err(|error| UpdateError::Network(error.to_string()))?;
    let status = response.status().as_u16();
    if status != 200 {
        // the server answered, so this is no connection problem
        return Err(UpdateError::BadAnswer(format!("{}: HTTP {status}", asset.name)));
    }
    let mut file = File::create(to).map_err(|error| UpdateError::Disk(format!("{}: {error}", to.display())))?;
    let mut body = response.body_mut().with_config().limit(asset.size.saturating_add(1)).reader();
    let written = io::copy(&mut body, &mut file).map_err(|error| UpdateError::Network(error.to_string()))?;
    file.sync_all().map_err(|error| UpdateError::Disk(format!("{}: {error}", to.display())))?;
    if written != asset.size {
        return Err(UpdateError::Network(format!("{}: got {written} bytes of {}", asset.name, asset.size)));
    }
    Ok(())
}

/// Checks the downloaded `file` against the signature line in `sig_file`, for `file_name` (the
/// name it was published under). A file that fails is deleted, so it can't be run by mistake.
pub fn verify_file(key: &VerifyingKey, file: &Path, sig_file: &Path, file_name: &str) -> Result<(), UpdateError> {
    let checked = (|| {
        let line = fs::read_to_string(sig_file).map_err(|error| UpdateError::Disk(error.to_string()))?;
        let hash = file_hash(file).map_err(|error| UpdateError::Disk(error.to_string()))?;
        signature::verify(key, file_name, &hash, &line).map_err(UpdateError::NotVerified)
    })();
    if checked.is_err()
        && let Err(error) = fs::remove_file(file)
    {
        log::warn!("couldn't remove the unverified download {}: {error}", file.display());
    }
    checked
}
