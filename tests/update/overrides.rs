// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 21:04:22.485219
//+ Revised: 	2026/11/23 21:04:22.485219
// -----------------------------------------------------------------------------

//! The debug-build overrides (`AVALIO_LITE_UPDATE_*`), read from the environment. Each case runs
//! this test binary again with the variable set, so no test changes the environment of another.

use crate::common::release_key;
use crate::common::releases::Published;
use avalio_lite::update::install::{DIR_VARIABLE, download_folder};
use avalio_lite::update::release::NetworkError;
use avalio_lite::update::signature::{self, KEY_VARIABLE, trusted_keys};
use avalio_lite::update::source::{SOURCES_VARIABLE, configured};
use std::path::Path;
use std::process::Command;

/// Set for the second run only: the child tests do nothing in an ordinary run.
const CHILD: &str = "AVALIO_LITE_TEST_CHILD";

/// The folder the download-folder case names; nothing is made there.
const FOLDER: &str = "some-folder";

/// Runs the ignored test `name` alone, in a process of its own with `variable` set to `value`.
fn runs_with(name: &str, variable: &str, value: &str) {
    let output = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", &format!("overrides::{name}"), "--ignored", "--nocapture"])
        .env(variable, value)
        .env(CHILD, "1")
        .output()
        .unwrap();
    let out = String::from_utf8_lossy(&output.stdout);
    let error = String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success() && out.contains("1 passed"), "{out}{error}");
}

fn in_child() -> bool {
    std::env::var_os(CHILD).is_some()
}

#[test]
fn a_debug_build_asks_the_addresses_it_is_told() {
    let published = Published::new("9.0.0", b"new", &release_key());
    let value = format!("{}|{}", published.server.url("/github"), published.server.url("/gitlab"));
    runs_with("child_asks_the_given_addresses", SOURCES_VARIABLE, &value);
    assert_eq!((published.server.count("/github"), published.server.count("/gitlab")), (1, 1));
}

#[test]
#[ignore = "run by a_debug_build_asks_the_addresses_it_is_told, in a process of its own"]
fn child_asks_the_given_addresses() {
    if !in_child() {
        return;
    }
    for source in configured().0 {
        assert_eq!(source.latest().unwrap().version.to_string(), "9.0.0", "{}", source.name());
    }
}

#[test]
fn addresses_that_are_not_a_pair_leave_the_real_forges() {
    runs_with("child_keeps_the_real_forges", SOURCES_VARIABLE, "http://127.0.0.1:1/only-one");
}

#[test]
#[ignore = "run by addresses_that_are_not_a_pair_leave_the_real_forges, in a process of its own"]
fn child_keeps_the_real_forges() {
    if !in_child() {
        return;
    }
    let (sources, http) = configured();
    let names: Vec<&str> = sources.iter().map(|source| source.name()).collect();
    assert_eq!(names, ["GitHub", "GitLab"]);
    // the real forges are asked over HTTPS only, so plain HTTP is refused without a request
    assert_eq!(http.text("http://127.0.0.1:1/", "*/*", 1), Err(NetworkError::NotHttps));
}

#[test]
fn a_debug_build_trusts_the_key_it_is_told() {
    let key = signature::hex(&release_key().verifying_key().to_bytes());
    runs_with("child_trusts_the_given_key", KEY_VARIABLE, &key);
}

#[test]
#[ignore = "run by a_debug_build_trusts_the_key_it_is_told, in a process of its own"]
fn child_trusts_the_given_key() {
    if in_child() {
        assert_eq!(trusted_keys().unwrap(), [release_key().verifying_key()]);
    }
}

#[test]
fn a_debug_build_downloads_where_it_is_told() {
    runs_with("child_downloads_to_the_given_folder", DIR_VARIABLE, FOLDER);
}

#[test]
#[ignore = "run by a_debug_build_downloads_where_it_is_told, in a process of its own"]
fn child_downloads_to_the_given_folder() {
    if in_child() {
        assert_eq!(download_folder(), Path::new(FOLDER));
    }
}
