// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/22 21:06:23.753690
//+ Revised: 	2026/11/24 17:54:11.158124
// -----------------------------------------------------------------------------

//! The guard against personal data on screen (src/platform/personal.rs) and the scratch folders
//! that rely on it (tests/common/scratch.rs).

mod common;

use avalio_lite::platform::personal::{first_naming, names_any, names_the_user, user_names};
use common::scratch::{scratch_dir, scratch_dir_in, scratch_root};
use serde_json::json;

fn names(list: &[&str]) -> Vec<String> {
    list.iter().map(|name| (*name).to_owned()).collect()
}

#[test]
fn a_name_is_found_as_a_whole_word_or_path_component_in_any_case() {
    let ana = names(&["ana"]);
    assert!(names_any(r"C:\Users\ana\AppData\Local\Temp", &ana));
    assert!(names_any(r"C:\Users\ANA\x", &ana));
    assert!(names_any("Ana Pérez", &ana));
    assert!(names_any("/home/ana", &ana));
    assert!(names_any("ana", &ana));
}

#[test]
fn a_name_inside_a_longer_word_is_not_found() {
    let ana = names(&["ana"]);
    assert!(!names_any("banana", &ana));
    assert!(!names_any(r"C:\Users\anabel\x", &ana));
    assert!(!names_any("analysis", &ana));
    assert!(!names_any("", &ana));
}

#[test]
fn no_names_or_an_empty_name_match_nothing() {
    assert!(!names_any(r"C:\Users\ana", &[]));
    assert!(!names_any(r"C:\Users\ana", &names(&[""])));
}

#[test]
fn a_name_is_found_in_nested_keys_and_values_with_its_path() {
    let ana = names(&["ana"]);
    let state = json!({
        "rows": [{ "cells": [{ "text": "8" }, { "text": r"C:\Users\ana\marks.db" }] }],
        "columns": ["First name"],
    });
    assert_eq!(first_naming(&state, &ana), Some("rows[0].cells[1].text".to_owned()));
    let in_a_key = json!({ "folders": { "ana": "data" } });
    assert_eq!(first_naming(&in_a_key, &ana), Some("folders.ana".to_owned()));
    let clean = json!({ "rows": [{ "text": "banana" }], "count": 3, "on": true, "none": null });
    assert_eq!(first_naming(&clean, &ana), None);
}

#[test]
fn the_user_names_have_no_empties_or_repeats() {
    let found = user_names();
    assert!(found.iter().all(|name| !name.trim().is_empty()));
    for (index, name) in found.iter().enumerate() {
        assert!(!found[index + 1..].iter().any(|other| other.eq_ignore_ascii_case(name)), "{name} twice");
    }
}

#[test]
fn scratch_folders_are_under_the_shared_profile_and_never_name_the_user() {
    let root = scratch_root();
    assert!(root.ends_with("avalio-lite-tests"));
    let dir = scratch_dir();
    assert!(dir.path().starts_with(&root));
    assert!(dir.path().file_name().unwrap().to_string_lossy().starts_with("run-"));
    assert!(!names_any(&dir.path().to_string_lossy(), &user_names()));
}

/// The self-check: a scratch root inside the user's profile (as the system temp folder is) fails.
#[test]
fn a_scratch_folder_inside_the_user_profile_is_refused() {
    let profile = std::env::var_os("USERPROFILE").expect("USERPROFILE is set on Windows");
    let name = std::path::Path::new(&profile).file_name().unwrap().to_string_lossy().into_owned();
    let inside = tempfile::Builder::new().prefix("avalio-lite-personal-").tempdir_in(&profile).unwrap();
    let refused = std::panic::catch_unwind(|| scratch_dir_in(inside.path(), &[name]));
    assert!(refused.is_err(), "a folder under the profile names the user");
}

#[test]
fn the_user_running_the_tests_is_found_in_a_text() {
    let names = user_names();
    let name = names.first().expect("the environment names the user");
    // the messages name nobody: the name is the one of whoever runs the tests
    assert!(names_the_user(&format!("a/{name}/b")), "the user's own name is found");
    assert!(!names_the_user(""), "and not in an empty text");
}
