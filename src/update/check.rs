// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:59.052384
//+ Revised: 	2026/11/24 17:54:06.017003
// -----------------------------------------------------------------------------

//! The check: every source asked at once, the newest version any of them reports considered, and
//! the decision what to offer. Also when the next automatic check is due. No window, no clock of
//! its own: every rule here is a plain function the tests call.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::Duration;

use chrono::{DateTime, Utc};

use super::release::{Asset, CheckError, Release};
use super::source::SharedSource;
use super::target::{Build, checksum_name, signature_name};

/// The first automatic check waits this long after the window opens, so it doesn't compete with
/// start-up.
pub const FIRST_CHECK_DELAY: Duration = Duration::from_secs(60);
/// After a successful check, the next automatic one.
pub const AFTER_SUCCESS: Duration = Duration::from_secs(24 * 60 * 60);
/// After a failed check, the next automatic one.
pub const AFTER_FAILURE: Duration = Duration::from_secs(60 * 60);

/// A newer release this build can install: its files and their proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Offer {
    pub release: Release,
    pub asset: Asset,
    pub checksum: Asset,
    pub signature: Asset,
}

impl Offer {
    pub fn version(&self) -> &semver::Version {
        &self.release.version
    }
}

/// What a check found; each has its line in Settings › Updates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Status {
    /// built from source: no check, no updates
    SourceBuild,
    UpToDate,
    Available(Offer),
    /// newer, but this target's file, `.b3` or `.minisig` is missing; a release nobody signed is never
    /// offered
    NoBuild(semver::Version),
    /// the user skipped this version or a newer one
    Skipped(semver::Version),
    /// every source failed; each one's reason, in order
    Failed(Vec<(String, CheckError)>),
}

/// Whether `candidate` is offered to a copy running `current`: newer, and not a pre-release
/// unless this copy is one too.
pub fn offered(candidate: &semver::Version, current: &semver::Version) -> bool {
    candidate > current && (candidate.pre.is_empty() || !current.pre.is_empty())
}

/// Asks every source at the same time, each on its own thread, so a slow one costs only its own
/// timeout; a source whose thread panics counts as failed.
pub fn ask_all(sources: &[SharedSource]) -> Vec<(String, Result<Release, CheckError>)> {
    std::thread::scope(|scope| {
        let asked: Vec<_> = sources
            .iter()
            .map(|source| {
                let name = source.name().to_owned();
                let spawned = std::thread::Builder::new()
                    .name(format!("avalio-lite-update-{}", name.to_ascii_lowercase()))
                    .spawn_scoped(scope, || catch_unwind(AssertUnwindSafe(|| source.latest())));
                (name, spawned)
            })
            .collect();
        asked
            .into_iter()
            .map(|(name, spawned)| {
                let answer = match spawned {
                    Ok(handle) => match handle.join() {
                        Ok(Ok(answer)) => answer,
                        Ok(Err(_)) | Err(_) => {
                            log::error!(target: "avalio_lite::update", "asking {name} broke off");
                            Err(CheckError::BrokeOff)
                        }
                    },
                    Err(error) => {
                        log::error!(target: "avalio_lite::update", "couldn't ask {name}: {error}");
                        Err(CheckError::BrokeOff)
                    }
                };
                (name, answer)
            })
            .collect()
    })
}

/// The whole check for `build` running `current`; a build from source asks nobody.
pub fn check(
    sources: &[SharedSource],
    build: Build,
    current: &semver::Version,
    skipped: Option<&semver::Version>,
) -> Status {
    if build == Build::Source {
        return Status::SourceBuild;
    }
    decide(ask_all(sources), build, current, skipped)
}

/// The decision from every source's answer: the newest version any reports (on a tie, the earlier
/// source's files), offered if it is newer, not skipped, and has this build's file with its proof.
/// Only when every source failed is the check failed.
pub fn decide(
    answers: Vec<(String, Result<Release, CheckError>)>,
    build: Build,
    current: &semver::Version,
    skipped: Option<&semver::Version>,
) -> Status {
    let mut failures = Vec::new();
    let mut newest: Option<Release> = None;
    for (name, answer) in answers {
        match answer {
            Ok(release) => {
                if newest.as_ref().is_none_or(|best| release.version > best.version) {
                    newest = Some(release);
                }
            }
            Err(error) => {
                log::warn!(target: "avalio_lite::update", "{name}: {error}");
                failures.push((name, error));
            }
        }
    }
    let Some(release) = newest else { return Status::Failed(failures) };
    if !offered(&release.version, current) {
        return Status::UpToDate;
    }
    if skipped.is_some_and(|skipped| *skipped >= release.version) {
        return Status::Skipped(release.version);
    }
    let Some(name) = build.asset_name(&release.version) else { return Status::SourceBuild };
    let files = (release.asset(&name), release.asset(&checksum_name(&name)), release.asset(&signature_name(&name)));
    match files {
        (Some(asset), Some(checksum), Some(signature)) => {
            let (asset, checksum, signature) = (asset.clone(), checksum.clone(), signature.clone());
            Status::Available(Offer { release, asset, checksum, signature })
        }
        _ => Status::NoBuild(release.version),
    }
}

/// How long until the first automatic check after the window opened: at least
/// `FIRST_CHECK_DELAY`, else when a day has passed since the last successful check. A last check
/// in the future (the clock was set back) waits at most one interval.
pub fn first_check_in(last_check: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Duration {
    let due = last_check.map_or(Duration::ZERO, |last| {
        let since = (now - last).to_std().unwrap_or(Duration::ZERO);
        if last > now { AFTER_SUCCESS } else { AFTER_SUCCESS.saturating_sub(since) }
    });
    due.max(FIRST_CHECK_DELAY)
}

/// How long until the next automatic check after one ended.
#[expect(clippy::fn_params_excessive_bools, reason = "the outcome of the check that ended, mapped to a wait")]
pub fn next_check_in(succeeded: bool) -> Duration {
    if succeeded { AFTER_SUCCESS } else { AFTER_FAILURE }
}
