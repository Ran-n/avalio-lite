// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:59.346486
//+ Revised: 	2026/11/22 21:06:11.974247
// -----------------------------------------------------------------------------

//! Which release build this is, and what its files are called. The one place asset names are made:
//! the updater asks for them, and the release workflow gets them from `avalio-lite-asset-name`, so
//! the two can't drift apart.

use std::sync::OnceLock;

/// The targets a release is built for: `<os>-<arch>`. avalio-lite is a Windows app.
pub const TARGETS: &[&str] = &["windows-x86_64"];

/// The build-time variable a release build is compiled with; any other build is built from source.
pub const RELEASE_TARGET_VARIABLE: &str = "AVALIO_LITE_RELEASE_TARGET";

/// The program's lowercase name, which starts every asset name.
pub const APP: &str = "avalio-lite";

/// The two forms every target ships in.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Form {
    /// one file that runs from wherever it is put
    Portable,
    /// the installer (Inno Setup), per user, no elevation
    Setup,
}

impl Form {
    pub const ALL: [Self; 2] = [Self::Portable, Self::Setup];

    pub fn word(self) -> &'static str {
        match self {
            Self::Portable => "portable",
            Self::Setup => "setup",
        }
    }

    pub fn from_word(word: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|form| form.word() == word)
    }
}

/// `avalio-lite-<version>-<target>-<portable|setup>`, plus `.exe` on Windows.
pub fn asset_name(version: &semver::Version, target: &str, form: Form) -> String {
    format!("{APP}-{version}-{target}-{}{}", form.word(), extension(target))
}

/// The name copies from 0.6.0 to 0.10.0 look for: `avalio-lite-<version>-<portable|setup>.exe`.
/// Every release carries these too until the bridge ends (docs/development.md, Releases), so
/// those copies can still update themselves to a version that knows the new names.
pub fn bridge_asset_name(version: &semver::Version, form: Form) -> String {
    format!("{APP}-{version}-{}.exe", form.word())
}

/// `<asset>.b3`: one `b3sum` line.
pub fn checksum_name(asset: &str) -> String {
    format!("{asset}.b3")
}

/// `<asset>.minisig`: the release key's signature, in minisign's format.
pub fn signature_name(asset: &str) -> String {
    format!("{asset}.minisig")
}

/// `<asset>.sig`: the signature 0.6.0 to 0.10.0 read, written only for the bridge names.
pub fn bridge_signature_name(asset: &str) -> String {
    format!("{asset}.sig")
}

fn extension(target: &str) -> &'static str {
    if target.starts_with("windows-") { ".exe" } else { "" }
}

/// This build's version, from `Cargo.toml`.
pub fn current_version() -> semver::Version {
    // Cargo only accepts semver versions, so the package's own always parses
    semver::Version::parse(env!("CARGO_PKG_VERSION")).unwrap_or_else(|_| semver::Version::new(0, 0, 0))
}

/// What kind of build this is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Build {
    /// a release build for this target: it checks for updates and replaces itself
    Release(&'static str),
    /// anything else: it never checks and never replaces itself
    Source,
}

impl Build {
    /// The asset this build updates from: the portable file for its target (an installed copy is
    /// the same binary, so it updates from it too).
    pub fn asset_name(self, version: &semver::Version) -> Option<String> {
        match self {
            Self::Release(target) => Some(asset_name(version, target, Form::Portable)),
            Self::Source => None,
        }
    }
}

/// This build: the target compiled in through `AVALIO_LITE_RELEASE_TARGET`, or, in a debug build
/// only, the same variable read at run time (the tests). An unknown value is logged once and counts
/// as a build from source.
pub fn this_build() -> Build {
    static BUILD: OnceLock<Build> = OnceLock::new();
    *BUILD.get_or_init(|| build_for(option_env!("AVALIO_LITE_RELEASE_TARGET"), debug_override(RELEASE_TARGET_VARIABLE)))
}

/// The build `compiled` (the build-time value) and `overridden` (a debug build's run-time value)
/// make; split out of `this_build` for the tests.
pub fn build_for(compiled: Option<&str>, overridden: Option<String>) -> Build {
    let value = overridden.or_else(|| compiled.map(str::to_owned)).filter(|value| !value.is_empty());
    let Some(value) = value else { return Build::Source };
    match TARGETS.iter().find(|target| **target == value) {
        Some(target) => Build::Release(target),
        None => {
            log::warn!(
                "{RELEASE_TARGET_VARIABLE}={value:?} is no target avalio-lite is released for; treated as built from source"
            );
            Build::Source
        }
    }
}

/// A run-time override, read in debug builds only: a release build ignores every one of them.
pub fn debug_override(variable: &str) -> Option<String> {
    if cfg!(debug_assertions) { std::env::var(variable).ok().filter(|value| !value.is_empty()) } else { None }
}
