// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:59.418192
//+ Revised: 	2026/11/24 17:54:07.076955
// -----------------------------------------------------------------------------

//! Releases published on the fixture server for the updater tests: the asset this build updates
//! from, its `.b3` and `.minisig` made with the tests' key, and both forges' answers naming them.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use avalio_lite::update::check::Offer;
use avalio_lite::update::download::Limits;
use avalio_lite::update::release::{Asset, Release, UpdateError};
use avalio_lite::update::signature::checksum_line;
use avalio_lite::update::signature::hashes::{self, Hashes};
use avalio_lite::update::signature::minisign;
use avalio_lite::update::source::{FixedRelease, Http, SharedSource};
use avalio_lite::update::target::{Build, Form, asset_name, checksum_name, signature_name};
use avalio_lite::update::ui::Updates;
use ed25519_dalek::SigningKey;

use super::http::{self, Served, Server};
use super::release_key;

pub const TARGET: &str = "windows-x86_64";

pub fn version(text: &str) -> semver::Version {
    semver::Version::parse(text).unwrap()
}

/// The asset a release build updates from.
pub fn asset_for(version: &semver::Version) -> String {
    asset_name(version, TARGET, Form::Portable)
}

/// A release of `content` as `version`, published on a new fixture server: the asset, its `.b3`,
/// its `.minisig` (by `key`), and at `/github` and `/gitlab` each forge's answer naming them.
pub struct Published {
    pub server: Server,
    pub release: Release,
    pub name: String,
    /// the BLAKE3 hash, as the `.b3` has it
    pub hash: String,
    pub hashes: Hashes,
}

impl Published {
    pub fn new(version_text: &str, content: &[u8], key: &SigningKey) -> Self {
        let version = version(version_text);
        let name = asset_for(&version);
        let hashes = hashes::of_bytes(content);
        let hash = hashes.blake3.clone();
        let signature = minisign::sign(key, &version, &name, &hashes).expect("an asset name has no line break");
        let server = http::serve(HashMap::from([
            (format!("/{name}"), http::file(content)),
            (format!("/{}", checksum_name(&name)), http::file(checksum_line(&hash, &name).as_bytes())),
            (format!("/{}", signature_name(&name)), http::file(signature.as_bytes())),
        ]));
        let release = release_at(&server, &version, &name);
        server.set("/github", http::file(github_answer(&release).as_bytes()));
        server.set("/gitlab", http::file(gitlab_answer(&release).as_bytes()));
        Self { server, release, name, hash, hashes }
    }

    /// Replaces what `path` (`/<asset>`, `/<asset>.minisig`…) serves.
    pub fn serve(&self, path: &str, served: Served) {
        self.server.set(path, served);
    }

    pub fn asset_path(&self) -> String {
        format!("/{}", self.name)
    }

    pub fn checksum_path(&self) -> String {
        format!("/{}", checksum_name(&self.name))
    }

    pub fn signature_path(&self) -> String {
        format!("/{}", signature_name(&self.name))
    }

    pub fn offer(&self) -> Offer {
        let find = |name: &str| self.release.asset(name).unwrap().clone();
        Offer {
            release: self.release.clone(),
            asset: find(&self.name),
            checksum: find(&checksum_name(&self.name)),
            signature: find(&signature_name(&self.name)),
        }
    }

    /// A source answering this release.
    pub fn source(&self, name: &str) -> SharedSource {
        let source = FixedRelease::named(name);
        source.set(Ok(self.release.clone()));
        std::sync::Arc::new(source)
    }
}

fn release_at(server: &Server, version: &semver::Version, name: &str) -> Release {
    let assets = [name.to_owned(), checksum_name(name), signature_name(name)]
        .into_iter()
        .map(|file| Asset { url: server.url(&format!("/{file}")), name: file })
        .collect();
    Release {
        version: version.clone(),
        page: format!("https://github.com/Ran-n/avalio-lite/releases/tag/v{version}"),
        assets,
    }
}

pub fn github_answer(release: &Release) -> String {
    let assets: Vec<_> = release
        .assets
        .iter()
        .map(|asset| serde_json::json!({ "name": asset.name, "browser_download_url": asset.url, "size": 1 }))
        .collect();
    serde_json::json!({ "tag_name": format!("v{}", release.version), "html_url": release.page, "prerelease": false, "assets": assets })
        .to_string()
}

pub fn gitlab_answer(release: &Release) -> String {
    let links: Vec<_> = release
        .assets
        .iter()
        .map(|asset| serde_json::json!({ "name": asset.name, "direct_asset_url": asset.url, "url": asset.url }))
        .collect();
    let page = release.page.replace("github.com", "gitlab.com");
    serde_json::json!({ "tag_name": format!("v{}", release.version), "_links": { "self": page }, "assets": { "links": links } })
        .to_string()
}

/// The tests' HTTP client: plain HTTP to the fixture server, short limits.
pub fn http() -> Http {
    Http::new(avalio_lite::update::source::Transport::AllowHttp, std::time::Duration::from_secs(10))
}

/// Tests replace a scratch exe, never their own: a plain move of the verified file onto it. Like
/// `install::replace`, the download is gone afterwards either way.
pub fn replace_scratch(exe: &Path, new: &Path) -> Result<(), UpdateError> {
    let moved = std::fs::rename(new, exe);
    if moved.is_err() {
        std::fs::remove_file(new).unwrap();
    }
    moved.map_err(|error| UpdateError::Replace {
        path: exe.display().to_string(),
        denied: error.kind() == std::io::ErrorKind::PermissionDenied,
    })
}

/// The updater's ports for a test: `sources`, the tests' key, a release build for `TARGET` unless
/// `running` says otherwise, its scratch exe, downloads into `downloads`, the work done at once.
pub fn updates(sources: Vec<SharedSource>, running: Running, downloads: PathBuf) -> Updates {
    updates_on(sources, running, downloads, std::rc::Rc::new(avalio_lite::window::worker::InlineWorker))
}

/// Which build the app under test is, the exe it would replace, and whether it checks by itself.
pub struct Running {
    pub build: Build,
    pub exe: Option<PathBuf>,
    pub automatic: bool,
}

/// `updates` with the work on `worker` (the desktop driver's real threads).
pub fn updates_on(
    sources: Vec<SharedSource>,
    running: Running,
    downloads: PathBuf,
    worker: avalio_lite::window::worker::SharedWorker,
) -> Updates {
    let Running { build, exe, automatic } = running;
    Updates {
        sources,
        http: http(),
        keys: vec![release_key().verifying_key()],
        build,
        exe,
        downloads,
        limits: Limits::default(),
        automatic,
        worker,
        replace: replace_scratch,
    }
}
