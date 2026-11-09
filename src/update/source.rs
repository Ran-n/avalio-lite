// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.826099
//+ Revised: 	2026/11/08 04:36:59.384454
// -----------------------------------------------------------------------------

//! Where releases are read from: GitHub's API in the app, a fixed answer in the tests. Behind a
//! trait so no test touches the network.

use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::{RELEASES_API, Release, UpdateError, parse_release};

pub trait ReleaseSource: Send + Sync {
    /// The latest published release.
    fn latest(&self) -> Result<Release, UpdateError>;
}

pub type SharedSource = Arc<dyn ReleaseSource>;

/// GitHub's releases API, without an account (60 requests an hour, far above one a day).
pub struct GitHub {
    agent: ureq::Agent,
    url: String,
}

pub const TIMEOUT: Duration = Duration::from_secs(15);

/// Points the updater at another repository's latest release, for trying an update end to end
/// with a scratch release. Safe in any build: a file from there is still used only if it is
/// signed with the release key.
pub const RELEASES_API_ENV: &str = "AVALIO_RELEASES_API";

/// Where to read releases from: `overridden` (the environment variable's value) when it is set
/// and not empty, logged, else the real API.
pub fn releases_api(overridden: Option<String>) -> String {
    match overridden.filter(|url| !url.trim().is_empty()) {
        Some(url) => {
            log::warn!("reading releases from {url} ({RELEASES_API_ENV} is set), not from GitHub's avalio-lite page");
            url
        }
        None => RELEASES_API.to_owned(),
    }
}

impl GitHub {
    pub fn new() -> Self {
        Self::at(RELEASES_API)
    }

    /// The API, or the address in `AVALIO_RELEASES_API` when it is set (see `releases_api`).
    pub fn configured() -> Self {
        Self::at(&releases_api(std::env::var(RELEASES_API_ENV).ok()))
    }

    /// Another address answering like the API, for trying a release out.
    pub fn at(url: &str) -> Self {
        let config = ureq::Agent::config_builder().timeout_global(Some(TIMEOUT)).http_status_as_error(false).build();
        Self { agent: config.into(), url: url.to_owned() }
    }
}

impl Default for GitHub {
    fn default() -> Self {
        Self::new()
    }
}

impl ReleaseSource for GitHub {
    fn latest(&self) -> Result<Release, UpdateError> {
        let mut response = self
            .agent
            .get(&self.url)
            .header("Accept", "application/vnd.github+json")
            // GitHub refuses requests without one
            .header("User-Agent", concat!("avalio-lite/", env!("CARGO_PKG_VERSION")))
            .call()
            .map_err(|error| UpdateError::Network(error.to_string()))?;
        let status = response.status().as_u16();
        if matches!(status, 403 | 429) {
            return Err(UpdateError::RateLimited);
        }
        let body = response.body_mut().read_to_string().map_err(|error| UpdateError::Network(error.to_string()))?;
        if status != 200 {
            return Err(UpdateError::BadAnswer(format!("HTTP {status}")));
        }
        parse_release(&body)
    }
}

/// Answers with whatever was set last: for the tests and the desktop test driver.
#[derive(Default)]
pub struct FixedRelease {
    answer: Mutex<Option<Result<Release, UpdateError>>>,
    calls: Mutex<usize>,
}

impl FixedRelease {
    pub fn set(&self, answer: Result<Release, UpdateError>) {
        *self.answer.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(answer);
    }

    /// How many times the latest release was asked for.
    pub fn calls(&self) -> usize {
        *self.calls.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl ReleaseSource for FixedRelease {
    fn latest(&self) -> Result<Release, UpdateError> {
        *self.calls.lock().unwrap_or_else(std::sync::PoisonError::into_inner) += 1;
        self.answer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .unwrap_or_else(|| Err(UpdateError::Network("no answer set".into())))
    }
}
