// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.228225
//+ Revised: 	2026/10/28 15:53:07.228225
// -----------------------------------------------------------------------------

//! The version lives in one place, `Cargo.toml`; everything that shows or ships it must follow.

use std::fs;
use std::path::PathBuf;

fn read(relative: &str) -> String {
    fs::read_to_string(PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(relative)).unwrap()
}

fn manifest_version() -> String {
    let manifest: toml::Table = toml::from_str(&read("Cargo.toml")).unwrap();
    manifest["package"]["version"].as_str().unwrap().to_owned()
}

#[test]
fn the_app_reports_the_manifest_version() {
    // what the app logs at startup
    assert_eq!(env!("CARGO_PKG_VERSION"), manifest_version());
}

#[test]
fn the_lock_file_agrees_with_the_manifest() {
    let lock: toml::Table = toml::from_str(&read("Cargo.lock")).unwrap();
    let ours =
        lock["package"].as_array().unwrap().iter().find(|package| package["name"].as_str() == Some("avalio-lite"));
    assert_eq!(ours.unwrap()["version"].as_str(), Some(manifest_version().as_str()));
}

#[test]
fn releases_take_the_version_from_the_manifest_only() {
    let script = read("scripts/release.ps1");
    assert!(script.contains("cargo metadata"), "the release script reads the version from Cargo.toml");
    assert!(script.contains("avalio-lite-$version-portable.exe"), "the portable exe is named with it");
    assert!(script.contains("/DAppVersion=$version"), "the installer gets it");
    let installer = read("installer/avalio-lite.iss");
    assert!(installer.contains("#error AppVersion must be defined"), "the installer has no version of its own");
    assert!(installer.contains("OutputBaseFilename=avalio-lite-{#AppVersion}-setup"));
}

#[test]
fn the_changelog_covers_the_current_version() {
    let changelog = read("CHANGELOG.md");
    let version = manifest_version();
    assert!(
        changelog.contains(&format!("## [{version}]")) || changelog.contains("## [Unreleased]"),
        "CHANGELOG.md has no section for {version} (or [Unreleased] before its first release)"
    );
}
