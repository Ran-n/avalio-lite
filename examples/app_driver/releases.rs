// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:01.692633
//+ Revised: 	2026/11/24 17:54:03.628201
// -----------------------------------------------------------------------------

//! The driver's forge: the release the `release` command sets, which the app's update checks find in
//! place of GitHub, and the fixture servers that a `serve` release is published on (they stay up
//! until the driver ends). `DriverSource` is what the app's updater asks.

use std::cell::RefCell;
use std::sync::LazyLock;
use std::sync::atomic::{AtomicBool, Ordering};

use avalio_lite::update::release::{Asset, CheckError, Release};
use avalio_lite::update::source::{FixedRelease, ReleaseSource};
use avalio_lite::update::target::{Form, asset_name, checksum_name, signature_name};

use crate::common;

thread_local! {
    /// the fixture servers of the releases published so far, kept running
    static PUBLISHED: RefCell<Vec<common::releases::Published>> = const { RefCell::new(Vec::new()) };
}

/// What the driver's forge answers; set with the `release` command. Read on the check's thread.
static RELEASES: LazyLock<FixedRelease> = LazyLock::new(FixedRelease::default);
/// The next checks hang (`release` with "hang": true), as a forge that never answers.
static HANGS: AtomicBool = AtomicBool::new(false);

/// The driver's forge: answers what `release` set, or never.
pub struct DriverSource;

impl ReleaseSource for DriverSource {
    fn name(&self) -> &str {
        "GitHub"
    }

    fn latest(&self) -> Result<Release, CheckError> {
        if HANGS.load(Ordering::SeqCst) {
            std::thread::sleep(std::time::Duration::from_secs(600));
        }
        RELEASES.latest()
    }
}

/// What the driver's forge does on the next checks: answer what `release` set, or never answer.
#[derive(Clone, Copy)]
pub enum ForgeMode {
    Answers,
    Hangs,
}

/// Sets what the next checks do.
pub fn set_mode(mode: ForgeMode) {
    HANGS.store(matches!(mode, ForgeMode::Hangs), Ordering::SeqCst);
}

/// Sets what the next check finds: `text` as the version, listed at addresses nothing serves, or,
/// with `serve` ("good", "bad-signature", "bad-checksum" or "slow"), published on a fixture server.
pub fn offer(text: &str, serve: Option<&str>) -> Result<(), String> {
    let version = semver::Version::parse(text).map_err(|error| error.to_string())?;
    let release = match serve {
        None => listed(&version),
        Some(how) => publish(text, how)?,
    };
    RELEASES.set(Ok(release));
    Ok(())
}

/// A release naming this build's file and its proof, at addresses nothing serves: enough to be
/// offered.
fn listed(version: &semver::Version) -> Release {
    let name = asset_name(version, common::releases::TARGET, Form::Portable);
    let assets = [name.clone(), checksum_name(&name), signature_name(&name)]
        .into_iter()
        .map(|file| Asset { url: format!("https://example.org/{file}"), name: file })
        .collect();
    Release {
        version: version.clone(),
        page: format!("https://github.com/Ran-n/avalio-lite/releases/tag/v{version}"),
        assets,
    }
}

/// A release published on a fixture server, spoiled as `how` says.
fn publish(version: &str, how: &str) -> Result<Release, String> {
    use common::http::{self, Served};
    let content = vec![b'n'; 512 * 1024];
    let published = common::releases::Published::new(version, &content, &common::release_key());
    match how {
        "good" => {}
        "bad-signature" => {
            let stranger = ed25519_dalek::SigningKey::from_bytes(&[9; 32]);
            let text = avalio_lite::update::signature::minisign::sign(
                &stranger,
                &published.release.version,
                &published.name,
                &published.hashes,
            )
            .map_err(|error| error.to_string())?;
            published.serve(&published.signature_path(), http::file(text.as_bytes()));
        }
        "bad-checksum" => published.serve(&published.asset_path(), http::file(b"other bytes")),
        "slow" => published.serve(
            &published.asset_path(),
            Served { paced: Some((8 * 1024, std::time::Duration::from_millis(100))), ..http::file(&content) },
        ),
        other => return Err(format!("unknown serve {other:?}")),
    }
    let release = published.release.clone();
    PUBLISHED.with(|kept| kept.borrow_mut().push(published));
    Ok(release)
}
