// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.826099
//+ Revised: 	2026/11/24 17:54:06.419485
// -----------------------------------------------------------------------------

//! Where releases are read from: GitHub and GitLab, both asked on every check (`check`), and one
//! HTTP client for everything the updater fetches. Sources are a strategy, so tests use fakes.

use std::io::Read;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use super::release::{CheckError, NetworkError, Release, parse_github, parse_gitlab};
use super::target::debug_override;

pub const GITHUB_LATEST: &str = "https://api.github.com/repos/Ran-n/avalio-lite/releases/latest";
pub const GITLAB_LATEST: &str = "https://gitlab.com/api/v4/projects/Ran-n%2Favalio-lite/releases/permalink/latest";

/// Debug builds only: `<github-url>|<gitlab-url>`, both sources pointed elsewhere (a local test
/// server), plain HTTP allowed.
pub const SOURCES_VARIABLE: &str = "AVALIO_LITE_UPDATE_SOURCES";

pub const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
/// A whole check, from asking to the last byte of the answer.
pub const CHECK_TIMEOUT: Duration = Duration::from_secs(30);
/// The largest release answer read.
pub const ANSWER_LIMIT: u64 = 1024 * 1024;
/// The largest `.b3` or `.minisig` read.
pub const PROOF_LIMIT: u64 = 1024;

pub const USER_AGENT: &str = concat!("avalio-lite/", env!("CARGO_PKG_VERSION"));

pub trait ReleaseSource: Send + Sync {
    /// What the user is told the source is called: "GitHub", "GitLab".
    fn name(&self) -> &str;
    /// The latest published release.
    fn latest(&self) -> Result<Release, CheckError>;
}

pub type SharedSource = Arc<dyn ReleaseSource>;

/// The one HTTP client: HTTPS only (unless the debug override allows plain HTTP), 10 s to connect,
/// and only the URL, `Accept` and `User-Agent` sent: no identifier, no personal data.
#[derive(Clone)]
pub struct Http {
    agent: ureq::Agent,
    transport: Transport,
}

/// Which addresses a client may ask.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Transport {
    /// HTTPS only: the real forges
    HttpsOnly,
    /// plain HTTP too: only the tests' local servers, in a debug build
    AllowHttp,
}

impl Http {
    /// `whole`: how long one request may take, from asking to the last byte.
    pub fn new(transport: Transport, whole: Duration) -> Self {
        Self::build(transport, None, whole)
    }

    /// The same client for a download: no data for `stall`, or `whole` in all, ends it.
    pub fn for_download(&self, stall: Duration, whole: Duration) -> Self {
        Self::build(self.transport, Some(stall), whole)
    }

    fn build(transport: Transport, per_read: Option<Duration>, whole: Duration) -> Self {
        let config = ureq::Agent::config_builder()
            .https_only(transport == Transport::HttpsOnly)
            .timeout_connect(Some(CONNECT_TIMEOUT))
            .timeout_per_read(per_read)
            .timeout_global(Some(whole))
            .user_agent(USER_AGENT)
            .build();
        Self { agent: config.into(), transport }
    }

    /// GET `url` with `accept`; a status other than 200 is an error.
    pub fn get(&self, url: &str, accept: &str) -> Result<ureq::http::Response<ureq::Body>, NetworkError> {
        self.agent.get(url).header("Accept", accept).call().map_err(|error| NetworkError::from_ureq(&error))
    }

    /// GET `url` as text of at most `limit` bytes.
    pub fn text(&self, url: &str, accept: &str, limit: u64) -> Result<String, NetworkError> {
        let mut response = self.get(url, accept)?;
        let mut body = Vec::new();
        response
            .body_mut()
            .with_config()
            .limit(limit)
            .reader()
            .read_to_end(&mut body)
            .map_err(|error| NetworkError::from_io(&error))?;
        // an answer that isn't UTF-8 is read as far as it goes; the parser then refuses it
        Ok(String::from_utf8_lossy(&body).into_owned())
    }
}

/// A forge's latest-release endpoint.
pub struct Forge {
    name: &'static str,
    url: String,
    accept: &'static str,
    parse: fn(&str) -> Result<Release, CheckError>,
    http: Http,
}

impl Forge {
    pub fn github(url: &str, http: Http) -> Self {
        Self { name: "GitHub", url: url.to_owned(), accept: "application/vnd.github+json", parse: parse_github, http }
    }

    pub fn gitlab(url: &str, http: Http) -> Self {
        Self { name: "GitLab", url: url.to_owned(), accept: "application/json", parse: parse_gitlab, http }
    }
}

impl ReleaseSource for Forge {
    fn name(&self) -> &str {
        self.name
    }

    fn latest(&self) -> Result<Release, CheckError> {
        let answer = self.http.text(&self.url, self.accept, ANSWER_LIMIT).map_err(CheckError::Network)?;
        (self.parse)(&answer)
    }
}

/// The sources this build asks, GitHub first, and the client the proof and downloads use: the real forges, or
/// in a debug build the addresses in `AVALIO_LITE_UPDATE_SOURCES` (plain HTTP allowed).
pub fn configured() -> (Vec<SharedSource>, Http) {
    let overridden = debug_override(SOURCES_VARIABLE).and_then(|value| {
        let parts = value.split_once('|').map(|(github, gitlab)| (github.to_owned(), gitlab.to_owned()));
        if parts.is_none() {
            log::warn!("{SOURCES_VARIABLE} isn't <github-url>|<gitlab-url>; asking the real forges");
        }
        parts
    });
    let (github, gitlab, transport) = match &overridden {
        Some((github, gitlab)) => {
            log::warn!("asking {github} and {gitlab} for releases ({SOURCES_VARIABLE} is set)");
            (github.as_str(), gitlab.as_str(), Transport::AllowHttp)
        }
        None => (GITHUB_LATEST, GITLAB_LATEST, Transport::HttpsOnly),
    };
    let http = Http::new(transport, CHECK_TIMEOUT);
    let sources: Vec<SharedSource> =
        vec![Arc::new(Forge::github(github, http.clone())), Arc::new(Forge::gitlab(gitlab, http.clone()))];
    (sources, http)
}

/// Answers with whatever was set last: for the tests and the desktop test driver.
pub struct FixedRelease {
    name: String,
    answer: Mutex<Option<Result<Release, CheckError>>>,
    calls: Mutex<usize>,
    panics: std::sync::atomic::AtomicBool,
}

impl Default for FixedRelease {
    fn default() -> Self {
        Self::named("GitHub")
    }
}

impl FixedRelease {
    pub fn named(name: &str) -> Self {
        Self {
            name: name.to_owned(),
            answer: Mutex::default(),
            calls: Mutex::default(),
            panics: std::sync::atomic::AtomicBool::default(),
        }
    }

    /// Makes the next asks panic, as a bug in the lookup would.
    pub fn panic_on_ask(&self) {
        self.panics.store(true, std::sync::atomic::Ordering::Relaxed);
    }

    pub fn set(&self, answer: Result<Release, CheckError>) {
        *self.answer.lock().unwrap_or_else(std::sync::PoisonError::into_inner) = Some(answer);
    }

    /// How many times the latest release was asked for.
    pub fn calls(&self) -> usize {
        *self.calls.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
    }
}

impl ReleaseSource for FixedRelease {
    fn name(&self) -> &str {
        &self.name
    }

    fn latest(&self) -> Result<Release, CheckError> {
        *self.calls.lock().unwrap_or_else(std::sync::PoisonError::into_inner) += 1;
        assert!(!self.panics.load(std::sync::atomic::Ordering::Relaxed), "the lookup broke");
        self.answer
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
            .unwrap_or(Err(CheckError::Network(NetworkError::ConnectionFailed)))
    }
}
