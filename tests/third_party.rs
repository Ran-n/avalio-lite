// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 18:39:49.574584
//+ Revised: 	2026/11/23 16:34:46.272712
// -----------------------------------------------------------------------------

//! `THIRD-PARTY-LICENSES.md`, which Settings › About links to, names every crate built into the
//! app with its version and licence, as `cargo metadata` resolves Cargo.lock for Windows. A changed
//! dependency fails this test until the file is written again: `AVALIO_UPDATE_GOLDEN=1 cargo test
//! --test third_party`.

use std::collections::{BTreeSet, HashMap};
use std::path::PathBuf;
use std::process::Command;

use serde_json::Value;

const FILE: &str = "THIRD-PARTY-LICENSES.md";
const TARGET: &str = "x86_64-pc-windows-msvc";

/// A crate in the app: name, version, licence, and where to read its source.
#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Crate {
    name: String,
    version: String,
    licence: String,
    link: String,
}

fn metadata() -> Value {
    let cargo = std::env::var("CARGO").unwrap_or_else(|_| "cargo".to_owned());
    let output = Command::new(cargo)
        .args(["metadata", "--format-version", "1", "--locked", "--filter-platform", TARGET])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .expect("cargo metadata runs");
    assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
    serde_json::from_slice(&output.stdout).expect("cargo metadata answers JSON")
}

/// The crates the app is built from: those the app reaches through normal dependencies (not the
/// ones only tests or build scripts use), without the app itself.
fn crates_in_the_app(metadata: &Value) -> BTreeSet<Crate> {
    let resolve = &metadata["resolve"];
    let root = resolve["root"].as_str().expect("a root package");
    let nodes: HashMap<&str, &Value> =
        resolve["nodes"].as_array().unwrap().iter().map(|node| (node["id"].as_str().unwrap(), node)).collect();
    let packages: HashMap<&str, &Value> = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .map(|package| (package["id"].as_str().unwrap(), package))
        .collect();
    let mut reached = BTreeSet::new();
    let mut waiting = vec![root];
    while let Some(id) = waiting.pop() {
        for dependency in nodes[id]["deps"].as_array().unwrap() {
            let normal = dependency["dep_kinds"].as_array().unwrap().iter().any(|kind| kind["kind"].is_null());
            let next = dependency["pkg"].as_str().unwrap();
            if normal && reached.insert(next) {
                waiting.push(next);
            }
        }
    }
    reached
        .into_iter()
        .map(|id| {
            let package = packages[id];
            let text = |key: &str| package[key].as_str().unwrap_or_default().to_owned();
            let (name, version) = (text("name"), text("version"));
            let licence = match package["license"].as_str() {
                Some(licence) => licence.to_owned(),
                None if package["license_file"].is_string() => "see its licence file".to_owned(),
                None => "not stated".to_owned(),
            };
            let from_registry = package["source"].as_str().is_some_and(|source| source.starts_with("registry+"));
            let link =
                if from_registry { format!("https://crates.io/crates/{name}/{version}") } else { text("repository") };
            Crate { name, version, licence, link }
        })
        .collect()
}

/// The file's `metadata` header with the blank line after it, which rewriting the list keeps.
fn header(file: &str) -> &str {
    let end = file.find("# Third-party licences").unwrap_or(0);
    &file[..end]
}

fn written(crates: &BTreeSet<Crate>) -> String {
    let mut text = String::from(
        "# Third-party licences\n\n\
         avalio lite is built with the open-source crates below, each under the licence named next to it; \
         every crate's page links to its source and the full licence text. Where a crate offers a choice \
         (`MIT OR Apache-2.0`), avalio lite uses it under either.\n\n\
         Slint, the toolkit the window is drawn with, is used under the Slint Royalty-free Desktop, Mobile, \
         and Web Applications License 2.0; Settings › About shows the attribution it asks for.\n\n\
         This file is written by `tests/third_party.rs` from Cargo.lock; don't edit it by hand. It may name a crate \
         only the tests use, but never misses one the app is built with.\n\n\
         | Crate | Version | Licence |\n|---|---|---|\n",
    );
    for one in crates {
        let name = if one.link.is_empty() { one.name.clone() } else { format!("[{}]({})", one.name, one.link) };
        text += &format!("| {name} | {} | {} |\n", one.version, one.licence);
    }
    text
}

#[test]
fn the_third_party_licences_list_every_crate_in_the_app() {
    let crates = crates_in_the_app(&metadata());
    let names: BTreeSet<&str> = crates.iter().map(|one| one.name.as_str()).collect();
    for expected in ["slint", "rusqlite", "ureq", "ed25519-dalek", "blake3"] {
        assert!(names.contains(expected), "{expected} is built into the app");
    }
    // cargo metadata merges the features the tests turn on, so a crate the tests switch on inside
    // Slint (its testing backend) can be listed too: one too many is harmless in a licence list
    for only_tests in ["proptest", "slint-build", "winresource"] {
        assert!(!names.contains(only_tests), "{only_tests} isn't in the app, only in its tests or build");
    }
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(FILE);
    let current = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
    let text = format!("{}{}", header(&current), written(&crates));
    if std::env::var_os("AVALIO_UPDATE_GOLDEN").is_some_and(|value| value == "1") {
        std::fs::write(&path, &text).unwrap();
    }
    let on_disk = std::fs::read_to_string(&path).unwrap_or_default().replace("\r\n", "\n");
    assert!(on_disk == text, "{FILE} is out of date; write it again with AVALIO_UPDATE_GOLDEN=1");
}
