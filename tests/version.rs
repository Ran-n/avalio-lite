// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/28 15:53:07.228225
//+ Revised: 	2026/11/22 21:06:21.409097
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

/// The AppId of the installer, which the app reads its uninstall record under.
const APP_ID: &str = "{6F2B8A1E-3C4D-4E5F-9A7B-8C1D2E3F4A5B}";

#[test]
fn releases_take_the_version_from_the_manifest_only() {
    // the shared helper reads the version with cargo metadata and every asset name from the helper program
    let common = read("scripts/release-common.ps1");
    assert!(common.contains("cargo metadata --no-deps --format-version 1"), "the version comes from Cargo.toml");
    assert!(common.contains("avalio-lite-asset-name"), "the asset names come from the helper program");

    let build = read("packaging/windows/build-installer.ps1");
    assert!(build.contains("release-common.ps1"), "the installer build uses the shared helper");
    assert!(build.contains("Get-AppVersion") && build.contains("Get-AssetName -Form setup"));
    assert!(
        build.contains("/DAppVersion=$version") && build.contains("/DOutputName=$outputName"),
        "the installer gets both"
    );

    let release = read("scripts/release.ps1");
    assert!(
        release.contains("Get-AppVersion") && release.contains("Get-AssetName"),
        "the release reads both from the manifest and the helper"
    );

    let installer = read("packaging/windows/avalio-lite.iss");
    assert!(installer.contains("#error AppVersion must be defined"), "the installer has no version of its own");
    assert!(installer.contains("#error OutputName must be defined"), "the installer has no output name of its own");
    assert!(installer.contains("OutputBaseFilename={#OutputName}"), "the output name is the one passed in");
}

#[test]
fn the_installer_is_per_user_with_the_settings_it_needs() {
    let installer = read("packaging/windows/avalio-lite.iss");
    let has_line = |line: &str| installer.lines().any(|candidate| candidate == line);
    assert!(has_line("PrivilegesRequired=lowest"), "per user, without administrator rights");
    assert!(has_line("UninstallDisplayName=avalio lite"), "the uninstall entry keeps the plain name after an update");
    let desktop_task =
        installer.lines().find(|line| line.starts_with("Name: \"desktopicon\"")).expect("a desktop task");
    assert!(desktop_task.contains("Flags: unchecked"), "the desktop icon is opt-in");
    assert!(installer.contains("MinVersion=10.0.17763"), "Windows 10 1809 or later");
    assert!(has_line("AppPublisher=Breren"), "Breren publishes every app");
    assert!(has_line("AppPublisherURL=https://breren.com"));
}

#[test]
fn the_workflow_refuses_a_tag_that_is_not_the_manifest_version() {
    let workflow = read(".github/workflows/release.yml");
    assert!(workflow.contains("cargo metadata --no-deps --format-version 1"), "the version comes from Cargo.toml");
    assert!(workflow.contains("tag=\"${GITHUB_REF_NAME#v}\""), "the tag is compared without its v");
    assert!(workflow.contains("if [ \"$tag\" != \"$version\" ]"), "a mismatch stops the workflow");
    assert!(workflow.contains("verify:") && workflow.contains("needs: verify"), "every build waits for the check");
}

#[test]
fn the_installer_app_id_is_the_one_the_app_reads() {
    let installer = read("packaging/windows/avalio-lite.iss");
    let app_id = installer
        .lines()
        .find_map(|line| line.strip_prefix("AppId="))
        .expect("the installer sets an AppId")
        .replace("{{", "{");
    assert_eq!(app_id, APP_ID);
    // the uninstall record the app looks for is this AppId under HKCU, with Inno's _is1 suffix
    let record = format!("HKCU\\Software\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{app_id}_is1");
    assert!(record.contains(APP_ID));
    assert!(
        read("packaging/windows/test-installer.ps1").contains(APP_ID),
        "the installer test looks under the same AppId"
    );
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
