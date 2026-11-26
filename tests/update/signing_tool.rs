// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.081540
//+ Revised: 	2026/11/24 16:26:02.908578
// -----------------------------------------------------------------------------

//! The signing tool: the formats it writes, its keys and the refusals it makes.

use crate::common::release_key;
use crate::common::releases::TARGET;
use crate::support::{as_0_10_0, hash_of, hashes_of, keys};
use avalio_lite::update::signature::{self, BUILT_IN_KEYS, minisign, public_key};
use avalio_lite::update::target::{
    Form, asset_name, bridge_asset_name, bridge_signature_name, current_version, signature_name,
};
use std::fs;
use std::process::{Command, Stdio};

/// The real tool, signing bridge files from this checkout with a throwaway key: the old `.sig`
/// format for the old names, the new one for the new names.
#[test]
fn the_signing_tool_writes_each_format_for_its_names() {
    let dir = tempfile::tempdir().unwrap();
    let key_file = dir.path().join("test.key");
    let public = sign_tool(&["keygen", key_file.to_str().unwrap()], None);
    assert!(public.0, "{}", public.2);
    let public = public_key(first_line(&public.1)).unwrap();
    let version = current_version();
    let bridge_name = bridge_asset_name(&version, Form::Setup);
    let new_name = asset_name(&version, TARGET, Form::Portable);
    for name in [&bridge_name, &new_name] {
        fs::write(dir.path().join(name), b"payload").unwrap();
    }
    let paths: Vec<String> =
        [&bridge_name, &new_name].iter().map(|name| dir.path().join(name).display().to_string()).collect();
    let mut args = vec!["sign", key_file.to_str().unwrap()];
    args.extend(paths.iter().map(String::as_str));
    let (ok, out, error) = sign_tool(&args, None);
    assert!(ok, "{error}");
    assert!(out.contains("(bridge format)"), "{out}");
    let old = fs::read_to_string(dir.path().join(bridge_signature_name(&bridge_name))).unwrap();
    assert!(as_0_10_0::verify(&public, &bridge_name, &hash_of(b"payload"), &old), "{old}");
    let new = fs::read_to_string(dir.path().join(signature_name(&new_name))).unwrap();
    assert_eq!(minisign::verify(&[public], &new, (&version, &new_name), &hashes_of(b"payload")), Ok(()));
    assert!(!dir.path().join(signature_name(&bridge_name)).exists(), "a bridge name gets no .minisig");
    assert!(!dir.path().join(bridge_signature_name(&new_name)).exists(), "a new name gets no .sig");
}

fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or_default().trim()
}

// ---------------------------------------------------------------------------------------------
// The signing tool

fn sign_tool(args: &[&str], stdin: Option<&str>) -> (bool, String, String) {
    let mut command = Command::new(env!("CARGO_BIN_EXE_avalio-lite-sign"));
    command
        .args(args)
        .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() })
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = command.spawn().unwrap();
    if let Some(text) = stdin {
        use std::io::Write;
        child.stdin.take().unwrap().write_all(text.as_bytes()).unwrap();
    }
    let output = child.wait_with_output().unwrap();
    (
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).into_owned(),
        String::from_utf8_lossy(&output.stderr).into_owned(),
    )
}

#[test]
fn a_generated_key_signs_what_the_updater_accepts() {
    let dir = tempfile::tempdir().unwrap();
    let key_file = dir.path().join("k");
    let (ok, public, _) = sign_tool(&["keygen", key_file.to_str().unwrap()], None);
    assert!(ok);
    let public = public_key(first_line(&public)).unwrap();
    let shown = sign_tool(&["public", key_file.to_str().unwrap()], None).1;
    let shown: Vec<&str> = shown.lines().collect();
    assert_eq!(
        shown,
        [signature::hex(&public.to_bytes()), minisign::public_key_text(&public)],
        "hex, then minisign's form"
    );
    let name = asset_name(&current_version(), TARGET, Form::Portable);
    let asset = dir.path().join(&name);
    fs::write(&asset, b"payload").unwrap();
    assert!(sign_tool(&["sign", key_file.to_str().unwrap(), asset.to_str().unwrap()], None).0);
    let text = fs::read_to_string(dir.path().join(signature_name(&name))).unwrap();
    assert_eq!(minisign::verify(&[public], &text, (&current_version(), &name), &hashes_of(b"payload")), Ok(()));
    // and an independent minisign verifier, with the key as the tool prints it
    let independent = minisign_verify::PublicKey::from_base64(shown[1]).unwrap();
    let signature = minisign_verify::Signature::decode(&text).unwrap();
    assert!(independent.verify(b"payload", &signature, false).is_ok());
}

#[test]
fn a_piped_key_signs_and_never_rests_in_a_file() {
    let dir = tempfile::tempdir().unwrap();
    let secret = signature::hex(&release_key().to_bytes());
    let name = asset_name(&current_version(), TARGET, Form::Setup);
    let asset = dir.path().join(&name);
    fs::write(&asset, b"payload").unwrap();
    let (ok, _, error) = sign_tool(&["sign", "-", asset.to_str().unwrap()], Some(&format!("{secret}\n")));
    assert!(ok, "{error}");
    let text = fs::read_to_string(dir.path().join(signature_name(&name))).unwrap();
    assert_eq!(minisign::verify(&keys(), &text, (&current_version(), &name), &hashes_of(b"payload")), Ok(()));
    let independent = minisign_verify::PublicKey::from_base64(&minisign::public_key_text(&keys()[0])).unwrap();
    assert!(independent.verify(b"payload", &minisign_verify::Signature::decode(&text).unwrap(), false).is_ok());
    let files: Vec<_> = fs::read_dir(dir.path()).unwrap().map(|entry| entry.unwrap().file_name()).collect();
    assert_eq!(files.len(), 2, "the asset and its .minisig only: {files:?}");
    assert_eq!(
        first_line(
            &sign_tool(
                &["public", "-"],
                Some(&format!(
                    "{secret}

"
                ))
            )
            .1
        ),
        signature::hex(&release_key().verifying_key().to_bytes())
    );
}

#[test]
fn keys_prints_the_built_in_keys_in_minisigns_form_signing_key_first() {
    let (ok, out, error) = sign_tool(&["keys"], None);
    assert!(ok, "{error}");
    let expected: Vec<String> =
        BUILT_IN_KEYS.iter().map(|key| minisign::public_key_text(&public_key(key).unwrap())).collect();
    assert_eq!(out.lines().collect::<Vec<_>>(), expected);
}

/// The README publishes the keys a download is checked with; they must be the built-in ones.
#[test]
fn the_readme_publishes_the_built_in_keys() {
    let readme = fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/README.md")).unwrap();
    let block = readme
        .split_once("Check that a download is genuine")
        .and_then(|(_, rest)| rest.split_once("</details>"))
        .map(|(block, _)| block)
        .expect("the README has a \"Check that a download is genuine\" block");
    let mut published: Vec<&str> = Vec::new();
    // each key once, in the order they first appear (the minisign line names the signing key too)
    for word in block.split(|c: char| !(c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')) {
        if word.len() == 56 && word.starts_with("RW") && !published.contains(&word) {
            published.push(word);
        }
    }
    let built_in: Vec<String> =
        BUILT_IN_KEYS.iter().map(|key| minisign::public_key_text(&public_key(key).unwrap())).collect();
    assert_eq!(published, built_in, "README keys, in order: signing key, then backup");
}

#[test]
fn keygen_never_overwrites_a_key() {
    let dir = tempfile::tempdir().unwrap();
    let key_file = dir.path().join("k");
    fs::write(&key_file, "precious").unwrap();
    let (ok, _, error) = sign_tool(&["keygen", key_file.to_str().unwrap()], None);
    assert!(!ok && !error.is_empty());
    assert_eq!(fs::read_to_string(&key_file).unwrap(), "precious");
}

#[test]
fn the_signing_tool_says_why_it_refuses() {
    let dir = tempfile::tempdir().unwrap();
    let secret = signature::hex(&release_key().to_bytes());
    let other = dir.path().join("avalio-lite-0.0.1-windows-x86_64-portable.exe");
    fs::write(&other, b"x").unwrap();
    let (ok, _, error) = sign_tool(&["sign", "-", other.to_str().unwrap()], Some(&secret));
    assert!(!ok && error.contains("sign from the tag's checkout"), "{error}");
    assert!(!dir.path().join("avalio-lite-0.0.1-windows-x86_64-portable.exe.minisig").exists());
    let (ok, _, error) = sign_tool(&["sign", "-", other.to_str().unwrap()], Some("not a key"));
    assert!(!ok && error.contains("key"), "{error}");
    for args in [&["frobnicate"][..], &["sign", "-"], &["verify"], &[]] {
        let (ok, _, error) = sign_tool(args, None);
        assert!(!ok && error.contains("usage"), "{args:?}: {error}");
    }
    // signed by the tests' key, which isn't built in
    let name = asset_name(&current_version(), TARGET, Form::Portable);
    let asset = dir.path().join(&name);
    fs::write(&asset, b"payload").unwrap();
    assert!(sign_tool(&["sign", "-", asset.to_str().unwrap()], Some(&secret)).0);
    let (ok, _, error) = sign_tool(&["verify", asset.to_str().unwrap()], None);
    assert!(!ok && error.contains(&name) && error.contains("isn't by a trusted release key"), "{error}");
}
