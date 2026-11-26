// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.258250
//+ Revised: 	2026/11/26 13:24:01.423390
// -----------------------------------------------------------------------------

//! Asset and file names per version and target, and which builds can update themselves.

use crate::common::releases::{TARGET, version};
use crate::support::BUILD;
use avalio_lite::update::target::{
    Build, Form, RELEASE_TARGET_VARIABLE, TARGETS, VERSION_VARIABLE, asset_name, bridge_asset_name,
    bridge_signature_name, build_for, checksum_name, current_version, signature_name,
};
use std::process::Command;

// ---------------------------------------------------------------------------------------------
// Targets and names

#[test]
fn assets_are_named_by_version_target_and_form() {
    let version = version("1.2.3-beta.1");
    assert_eq!(asset_name(&version, TARGET, Form::Portable), "avalio-lite-1.2.3-beta.1-windows-x86_64-portable.exe");
    assert_eq!(asset_name(&version, TARGET, Form::Setup), "avalio-lite-1.2.3-beta.1-windows-x86_64-setup.exe");
    assert_eq!(
        asset_name(&version, "linux-x86_64", Form::Portable),
        "avalio-lite-1.2.3-beta.1-linux-x86_64-portable",
        "only Windows programs end in .exe"
    );
    assert_eq!(checksum_name("a.exe"), "a.exe.b3");
    assert_eq!(signature_name("a.exe"), "a.exe.minisig");
    assert_eq!(bridge_signature_name("a.exe"), "a.exe.sig");
    assert_eq!(TARGETS, &["windows-x86_64"]);
}

#[test]
fn the_bridge_names_are_the_ones_0_10_0_looks_for() {
    let version = version("0.11.0");
    // 0.10.0's own code: format!("avalio-lite-{}-{kind}.exe", release.version)
    assert_eq!(bridge_asset_name(&version, Form::Portable), "avalio-lite-0.11.0-portable.exe");
    assert_eq!(bridge_asset_name(&version, Form::Setup), "avalio-lite-0.11.0-setup.exe");
}

#[test]
fn only_a_known_target_makes_a_release_build() {
    assert_eq!(build_for(Some(TARGET), None), BUILD);
    assert_eq!(build_for(None, None), Build::Source);
    assert_eq!(build_for(Some(""), None), Build::Source);
    assert_eq!(build_for(Some("linux-riscv"), None), Build::Source, "an unknown target is a build from source");
    assert_eq!(build_for(None, Some(TARGET.into())), BUILD, "a debug build's run-time value");
    assert_eq!(build_for(Some("linux-riscv"), Some(TARGET.into())), BUILD, "which wins over the compiled one");
}

#[test]
fn a_release_build_updates_from_its_portable_file_and_a_source_build_from_nothing() {
    let version = version("2.0.0");
    assert_eq!(BUILD.asset_name(&version), Some(asset_name(&version, TARGET, Form::Portable)));
    assert_eq!(Build::Source.asset_name(&version), None);
}

#[test]
fn this_build_knows_its_version() {
    assert_eq!(current_version().to_string(), env!("CARGO_PKG_VERSION"));
}

/// The desktop tests pin the version a debug build shows; a value that isn't a version is ignored.
#[test]
fn a_debug_build_takes_its_version_from_the_variable() {
    let asset_with = |value: &str| {
        let output = Command::new(env!("CARGO_BIN_EXE_avalio-lite-asset-name"))
            .args([TARGET])
            .env(VERSION_VARIABLE, value)
            .output()
            .unwrap();
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        String::from_utf8_lossy(&output.stdout).trim().to_owned()
    };
    assert_eq!(asset_with("1.2.3"), asset_name(&version("1.2.3"), TARGET, Form::Portable));
    assert_eq!(asset_with("not a version"), asset_name(&current_version(), TARGET, Form::Portable));
}

/// The release workflow asks this program for the names, so it can't spell them differently.
#[test]
fn the_asset_name_tool_prints_the_names_the_updater_uses() {
    let run = |args: &[&str], target: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_avalio-lite-asset-name"));
        command.args(args).env_remove(RELEASE_TARGET_VARIABLE);
        if let Some(target) = target {
            command.env(RELEASE_TARGET_VARIABLE, target);
        }
        let output = command.output().unwrap();
        (
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).trim().to_owned(),
            String::from_utf8_lossy(&output.stderr).into_owned(),
        )
    };
    let version = current_version();
    let portable = asset_name(&version, TARGET, Form::Portable);
    assert_eq!(run(&[], Some(TARGET)).1, portable, "a release build prints its own asset name");
    let (ok, _, error) = run(&[], None);
    assert!(!ok && error.contains("from source"), "{error}");
    assert_eq!(run(&[TARGET, "setup"], None).1, asset_name(&version, TARGET, Form::Setup));
    assert_eq!(run(&[TARGET], None).1, portable, "portable unless another form is named");
    assert_eq!(run(&[TARGET, "--bridge"], None).1, bridge_asset_name(&version, Form::Portable));
    assert_eq!(run(&[TARGET, "portable", "--bridge"], None).1, bridge_asset_name(&version, Form::Portable));
    assert_eq!(run(&[TARGET, "setup", "--bridge"], None).1, bridge_asset_name(&version, Form::Setup));
    let (ok, _, error) = run(&["macos-m68k"], None);
    assert!(!ok && error.contains("isn't a release target"), "{error}");
    for args in [&[TARGET, "portable", "--bridge", "more"][..], &[TARGET, "zip"]] {
        let (ok, _, error) = run(args, None);
        assert!(!ok && error.contains("usage"), "{args:?}: {error}");
    }
}

#[test]
fn the_app_answers_version() {
    let output = Command::new(env!("CARGO_BIN_EXE_avalio-lite")).arg("--version").output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), format!("avalio lite {}", env!("CARGO_PKG_VERSION")));
}
