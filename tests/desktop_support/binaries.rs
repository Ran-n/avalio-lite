// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 14:38:04.501087
//+ Revised: 	2026/11/24 17:54:09.705406
// -----------------------------------------------------------------------------

//! Builds the driver and the shipped app once per test run, with the tests' toolchain and profile, and
//! the turn lock that lets one test at a time have its window on screen. Does not own the processes:
//! `app` starts the driver and `real_app` the shipped exe.

use std::path::PathBuf;
use std::process::Command;
use std::sync::{Mutex, MutexGuard, OnceLock};

use super::manifest_dir;

/// One window on screen at a time: the tests take turns.
pub fn turn() -> MutexGuard<'static, ()> {
    static TURN: Mutex<()> = Mutex::new(());
    TURN.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Builds the driver once per test run, with the same toolchain and profile as the tests.
pub fn driver_exe() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| build(&["--example", "app_driver"], &["examples", "app_driver.exe"])).clone()
}

/// The app itself, as users run it (with the real Windows dialogs), built once per test run.
pub fn app_exe() -> PathBuf {
    static BUILT: OnceLock<PathBuf> = OnceLock::new();
    BUILT.get_or_init(|| build(&["--bin", "avalio-lite"], &["avalio-lite.exe"])).clone()
}

/// Builds `target` with the tests' toolchain and profile; returns `output` under target/<profile>.
fn build(target: &[&str], output: &[&str]) -> PathBuf {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let status = Command::new(cargo)
        .args(["build", "--quiet"])
        .args(target)
        .arg("--manifest-path")
        .arg(manifest_dir().join("Cargo.toml"))
        .status()
        .expect("cargo runs");
    assert!(status.success(), "building {target:?} failed");
    // target/<profile>/deps/desktop-<hash>.exe -> target/<profile>/<output>
    let profile = std::env::current_exe().unwrap().parent().unwrap().parent().unwrap().to_path_buf();
    output.iter().fold(profile, |path, part| path.join(part))
}
