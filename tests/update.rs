// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:35.336843
//+ Revised: 	2026/11/08 04:36:59.159781
// -----------------------------------------------------------------------------

//! Updating: release signatures (what is signed, the `.sig` line, and every way a check fails),
//! looking for a new version (GitHub's answer, which version is newer, when a check is due), and
//! installing it (which file, downloading, verifying, swapping the portable exe, cleaning up).

mod common;

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use avalio_lite::update::download::{agent, download, verify_file};
use avalio_lite::update::install::{
    Copy, asset_for, clean_leftovers, copy_kind, new_exe, old_exe, swap_portable, writable,
};
use avalio_lite::update::signature::{
    self, CONTEXT, SigError, encode_key, file_hash, message, parse_sig_line, private_key, public_key, sig_line, verify,
};
use avalio_lite::update::source::{FixedRelease, GitHub, ReleaseSource, releases_api};
use avalio_lite::update::{
    Asset, PUBLIC_KEY, RELEASES_API, Release, UpdateError, current_version, due, newer, parse_release, release_key,
};
use chrono::{TimeDelta, TimeZone, Utc};
use common::http::{self, Served};
use ed25519_dalek::SigningKey;
use semver::Version;

const SETUP: &str = "avalio-lite-0.6.0-setup.exe";
const PORTABLE: &str = "avalio-lite-0.6.0-portable.exe";
const HASH: &str = "8a8c21344949ba60600fc20a77dbfb624c891318b2124db0ea27baf9ec0ebec2";

fn key() -> SigningKey {
    SigningKey::from_bytes(&[7; 32])
}

fn signed(name: &str, hash: &str) -> String {
    sig_line(&signature::sign(&key(), name, hash), name)
}

#[test]
fn the_signed_message_names_the_context_the_file_and_its_hash() {
    assert_eq!(message(SETUP, HASH), format!("avalio-lite-release-v1\n{SETUP}\n{HASH}"));
    assert_eq!(CONTEXT, "avalio-lite-release-v1");
}

#[test]
fn a_good_signature_verifies() {
    assert_eq!(verify(&key().verifying_key(), SETUP, HASH, &signed(SETUP, HASH)), Ok(()));
}

#[test]
fn a_trailing_newline_in_the_sig_file_is_fine() {
    let line = format!("{}\r\n", signed(SETUP, HASH));
    assert_eq!(verify(&key().verifying_key(), SETUP, HASH, &line), Ok(()));
}

#[test]
fn the_setup_signature_does_not_pass_for_the_portable() {
    let result = verify(&key().verifying_key(), PORTABLE, HASH, &signed(SETUP, HASH));
    assert_eq!(result, Err(SigError::WrongFile { expected: PORTABLE.into(), found: SETUP.into() }));
}

#[test]
fn a_renamed_signature_line_fails() {
    // the name in the line edited to match: the signature still covers the original name
    let forged = signed(SETUP, HASH).replace(SETUP, PORTABLE);
    assert_eq!(verify(&key().verifying_key(), PORTABLE, HASH, &forged), Err(SigError::BadSignature));
}

#[test]
fn a_changed_file_fails() {
    let other = HASH.replace('8', "9");
    assert_eq!(verify(&key().verifying_key(), SETUP, &other, &signed(SETUP, HASH)), Err(SigError::BadSignature));
}

#[test]
fn a_signature_from_another_key_fails() {
    let someone = SigningKey::from_bytes(&[9; 32]);
    let line = sig_line(&signature::sign(&someone, SETUP, HASH), SETUP);
    assert_eq!(verify(&key().verifying_key(), SETUP, HASH, &line), Err(SigError::BadSignature));
}

#[test]
fn malformed_sig_lines_are_rejected() {
    let good = signed(SETUP, HASH);
    let (encoded, _) = good.split_once("  ").unwrap();
    for line in [
        String::new(),
        encoded.to_owned(),
        format!("{encoded} {SETUP}"),
        format!("!!!not base64!!!  {SETUP}"),
        format!("{}  {SETUP}", &encoded[..20]),
        format!("{encoded}  "),
    ] {
        assert_eq!(parse_sig_line(&line).map(|_| ()), Err(SigError::Malformed), "{line:?}");
    }
}

#[test]
fn keys_round_trip_through_their_text() {
    let text = encode_key(&[7; 32]);
    assert_eq!(private_key(&text).unwrap().to_bytes(), key().to_bytes());
    let public = encode_key(&key().verifying_key().to_bytes());
    assert_eq!(public_key(&format!("{public}\n")).unwrap(), key().verifying_key());
    assert_eq!(public_key("short").unwrap_err(), SigError::BadKey);
    assert_eq!(private_key("!!").unwrap_err(), SigError::BadKey);
}

#[test]
fn the_embedded_public_key_is_valid() {
    assert!(public_key(avalio_lite::update::PUBLIC_KEY).is_ok());
}

#[test]
fn a_file_is_hashed_with_blake3() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("x.exe");
    fs::write(&path, b"avalio").unwrap();
    assert_eq!(file_hash(&path).unwrap(), blake3::hash(b"avalio").to_hex().to_string());
    assert!(file_hash(&dir.path().join("missing.exe")).is_err());
}

// ---------------------------------------------------------------------------------------------
// Looking for a new version

fn golden_release() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden/github-release-latest.json");
    fs::read_to_string(path).unwrap()
}

#[test]
fn a_real_github_answer_is_read() {
    let release = parse_release(&golden_release()).unwrap();
    assert_eq!(release.version, Version::new(0, 5, 0));
    assert_eq!(release.page, "https://github.com/Ran-n/avalio-lite/releases/tag/v0.5.0");
    let setup = release.assets.iter().find(|asset| asset.name == "avalio-lite-0.5.0-setup.exe").unwrap();
    assert_eq!(setup.size, 9_667_340);
    assert!(setup.url.ends_with("/v0.5.0/avalio-lite-0.5.0-setup.exe"), "{}", setup.url);
}

#[test]
fn answers_that_are_not_a_release_are_refused() {
    for json in [
        r#"{"html_url": "https://x"}"#,
        r#"{"tag_name": "latest", "html_url": "https://x"}"#,
        "<html>Bad gateway</html>",
        "",
    ] {
        assert!(matches!(parse_release(json), Err(UpdateError::BadAnswer(_))), "{json:?}");
    }
}

#[test]
fn a_tag_without_its_v_and_a_release_without_files_still_read() {
    let release = parse_release(r#"{"tag_name": "1.2.3", "html_url": "https://x"}"#).unwrap();
    assert_eq!(release.version, Version::new(1, 2, 3));
    assert!(release.assets.is_empty());
}

#[test]
fn only_a_higher_version_is_newer() {
    let release =
        |version: &str| Release { version: Version::parse(version).unwrap(), page: String::new(), assets: Vec::new() };
    let current = Version::new(0, 5, 0);
    assert!(!newer(&release("0.5.0"), &current));
    assert!(!newer(&release("0.4.9"), &current));
    assert!(newer(&release("0.5.1"), &current));
    assert!(newer(&release("1.0.0"), &current));
}

#[test]
fn this_build_knows_its_version() {
    assert_eq!(current_version().to_string(), env!("CARGO_PKG_VERSION"));
}

#[test]
fn a_check_is_due_once_a_day() {
    let now = Utc.with_ymd_and_hms(2027, 3, 1, 9, 0, 0).unwrap();
    assert!(due(None, now), "never checked");
    assert!(!due(Some(now - TimeDelta::minutes(23 * 60 + 59)), now));
    assert!(due(Some(now - TimeDelta::hours(24)), now));
}

#[test]
fn the_fixed_source_answers_what_was_set_and_counts_the_calls() {
    let source = FixedRelease::default();
    assert!(matches!(source.latest(), Err(UpdateError::Network(_))), "nothing set yet");
    source.set(Err(UpdateError::RateLimited));
    assert_eq!(source.latest(), Err(UpdateError::RateLimited));
    assert_eq!(source.calls(), 2);
}

// ---------------------------------------------------------------------------------------------
// Installing an update

fn asset(name: &str, base: &str, size: usize) -> Asset {
    Asset { name: name.to_owned(), url: format!("{base}/{name}"), size: size as u64 }
}

fn release_with(names: &[&str]) -> Release {
    Release {
        version: Version::new(0, 6, 0),
        page: String::new(),
        assets: names.iter().map(|name| asset(name, "http://x", 1)).collect(),
    }
}

#[test]
fn a_copy_in_the_installers_folder_is_installed() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("avalio-lite.exe");
    fs::write(&exe, b"").unwrap();
    let upper = PathBuf::from(dir.path().to_string_lossy().to_uppercase());
    let with_slash = PathBuf::from(format!("{}\\", dir.path().display()));
    assert_eq!(copy_kind(&exe, &[upper]), Copy::Installed, "any letter case");
    assert_eq!(copy_kind(&exe, &[with_slash]), Copy::Installed, "with a trailing separator");
}

#[test]
fn a_copy_anywhere_else_is_portable() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("avalio-lite-0.5.0-portable.exe");
    assert_eq!(copy_kind(&exe, &[]), Copy::Portable(exe.clone()));
    assert_eq!(copy_kind(&exe, &[PathBuf::from(r"C:\Program Files\avalio-lite")]), Copy::Portable(exe.clone()));
}

#[test]
fn each_copy_updates_from_its_own_file_and_signature() {
    let names = [
        "avalio-lite-0.6.0-setup.exe",
        "avalio-lite-0.6.0-setup.exe.sig",
        "avalio-lite-0.6.0-portable.exe",
        "avalio-lite-0.6.0-portable.exe.sig",
    ];
    let release = release_with(&names);
    let (exe, sig) = asset_for(&release, &Copy::Installed).unwrap();
    assert_eq!((exe.name.as_str(), sig.name.as_str()), (names[0], names[1]));
    let (exe, sig) = asset_for(&release, &Copy::Portable(PathBuf::from("x.exe"))).unwrap();
    assert_eq!((exe.name.as_str(), sig.name.as_str()), (names[2], names[3]));
}

#[test]
fn a_release_without_the_file_or_its_signature_is_refused() {
    let unsigned = release_with(&["avalio-lite-0.6.0-setup.exe"]);
    assert_eq!(
        asset_for(&unsigned, &Copy::Installed).unwrap_err(),
        UpdateError::MissingAsset("avalio-lite-0.6.0-setup.exe.sig".into())
    );
    let other = release_with(&["avalio-lite-0.6.0-setup.exe", "avalio-lite-0.6.0-setup.exe.sig"]);
    assert_eq!(
        asset_for(&other, &Copy::Portable(PathBuf::from("x.exe"))).unwrap_err(),
        UpdateError::MissingAsset("avalio-lite-0.6.0-portable.exe".into())
    );
}

#[test]
fn a_download_is_streamed_to_disk() {
    let body = vec![42u8; 300_000];
    let base = http::serve(HashMap::from([("/a.exe".to_owned(), http::file(&body))]));
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join("a.exe");
    download(&agent(), &asset("a.exe", &base, body.len()), &to).unwrap();
    assert_eq!(fs::read(&to).unwrap(), body);
}

#[test]
fn a_download_cut_short_leaves_no_file() {
    let served = Served { announced: Some(1000), ..http::file(&[1; 100]) };
    let base = http::serve(HashMap::from([("/a.exe".to_owned(), served)]));
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join("a.exe");
    assert!(matches!(download(&agent(), &asset("a.exe", &base, 1000), &to), Err(UpdateError::Network(_))));
    assert!(!to.exists(), "the partial file is gone");
}

#[test]
fn a_download_of_another_size_than_announced_is_refused() {
    let base = http::serve(HashMap::from([("/a.exe".to_owned(), http::file(b"short"))]));
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join("a.exe");
    assert!(download(&agent(), &asset("a.exe", &base, 999), &to).is_err());
    assert!(!to.exists());
}

#[test]
fn a_missing_download_is_refused() {
    let base = http::serve(HashMap::new());
    let dir = tempfile::tempdir().unwrap();
    let to = dir.path().join("a.exe");
    let error = download(&agent(), &asset("a.exe", &base, 1), &to).unwrap_err();
    assert!(error.to_string().contains("404"), "{error}");
    assert!(!to.exists());
}

/// `name` with `content` and its `.sig` (by `key`) in `dir`.
fn signed_file(dir: &Path, name: &str, content: &[u8], key: &SigningKey) -> (PathBuf, PathBuf) {
    let file = dir.join(name);
    fs::write(&file, content).unwrap();
    let hash = blake3::hash(content).to_hex().to_string();
    let sig = dir.join(format!("{name}.sig"));
    fs::write(&sig, sig_line(&signature::sign(key, name, &hash), name)).unwrap();
    (file, sig)
}

#[test]
fn a_signed_download_verifies() {
    let dir = tempfile::tempdir().unwrap();
    let (file, sig) = signed_file(dir.path(), SETUP, b"new version", &key());
    assert_eq!(verify_file(&key().verifying_key(), &file, &sig, SETUP), Ok(()));
    assert!(file.exists());
}

#[test]
fn a_tampered_download_is_refused_and_deleted() {
    let dir = tempfile::tempdir().unwrap();
    let (file, sig) = signed_file(dir.path(), SETUP, b"new version", &key());
    fs::write(&file, b"new versioN").unwrap();
    assert_eq!(
        verify_file(&key().verifying_key(), &file, &sig, SETUP),
        Err(UpdateError::NotVerified(SigError::BadSignature))
    );
    assert!(!file.exists(), "it can't be run by mistake");
}

#[test]
fn a_download_under_another_name_is_refused() {
    let dir = tempfile::tempdir().unwrap();
    let (file, sig) = signed_file(dir.path(), SETUP, b"new version", &key());
    assert!(matches!(
        verify_file(&key().verifying_key(), &file, &sig, PORTABLE),
        Err(UpdateError::NotVerified(SigError::WrongFile { .. }))
    ));
}

#[test]
fn the_portable_exe_is_swapped_and_the_old_one_kept_aside() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("avalio-lite.exe");
    fs::write(&exe, b"old").unwrap();
    fs::write(new_exe(&exe), b"new").unwrap();
    swap_portable(&exe).unwrap();
    assert_eq!(fs::read(&exe).unwrap(), b"new");
    assert_eq!(fs::read(old_exe(&exe)).unwrap(), b"old");
    assert!(!new_exe(&exe).exists());
}

#[test]
fn a_failed_swap_puts_the_old_exe_back() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("avalio-lite.exe");
    fs::write(&exe, b"old").unwrap();
    // no .new to move in
    assert!(matches!(swap_portable(&exe), Err(UpdateError::Swap(_))));
    assert_eq!(fs::read(&exe).unwrap(), b"old", "the running exe is where it was");
}

#[test]
fn an_unwritable_folder_is_noticed_before_downloading() {
    let dir = tempfile::tempdir().unwrap();
    assert!(writable(dir.path()).is_ok());
    assert!(!dir.path().join("avalio-lite.write-test").exists(), "the probe is removed");
    assert!(writable(&dir.path().join("gone")).is_err());
}

#[test]
fn what_an_update_left_is_cleaned_up() {
    let dir = tempfile::tempdir().unwrap();
    let exe = dir.path().join("avalio-lite.exe");
    let downloads = dir.path().join("downloads");
    fs::create_dir_all(&downloads).unwrap();
    for leftover in [old_exe(&exe), new_exe(&exe), downloads.join("setup.exe")] {
        fs::write(leftover, b"x").unwrap();
    }
    clean_leftovers(&exe, &downloads);
    assert!(!old_exe(&exe).exists() && !new_exe(&exe).exists() && !downloads.exists());
    clean_leftovers(&exe, &downloads);
}

#[test]
fn the_release_key_this_build_trusts_is_the_published_one() {
    assert_eq!(release_key(), public_key(PUBLIC_KEY).unwrap());
}

// ---------------------------------------------------------------------------------------------
// The real API (online; run with `cargo test --test update -- --ignored`)

#[test]
#[ignore = "goes online to api.github.com"]
fn the_real_api_names_the_latest_release() {
    let release = avalio_lite::update::source::GitHub::new().latest().unwrap();
    assert!(release.version >= Version::new(0, 5, 0), "{}", release.version);
    assert!(release.page.starts_with("https://github.com/Ran-n/avalio-lite/releases/"), "{}", release.page);
    assert!(release.assets.iter().any(|asset| asset.name.ends_with("-setup.exe")));
}

#[test]
fn releases_come_from_avalio_lites_github_page_unless_told_otherwise() {
    assert_eq!(releases_api(None), RELEASES_API);
    assert_eq!(releases_api(Some("  ".into())), RELEASES_API, "an empty variable counts as unset");
    let scratch = "https://api.github.com/repos/Ran-n/avalio-lite-update-test/releases/latest";
    assert_eq!(releases_api(Some(scratch.into())), scratch);
}

/// GitHub's client against a local server answering like the API.
fn github_answering(served: Served) -> Result<Release, UpdateError> {
    let base = http::serve(HashMap::from([("/latest".to_owned(), served)]));
    GitHub::at(&format!("{base}/latest")).latest()
}

#[test]
fn github_reads_the_latest_release() {
    let body = fs::read("tests/golden/github-release-latest.json").unwrap();
    let release = github_answering(http::file(&body)).unwrap();
    assert_eq!(release, parse_release(std::str::from_utf8(&body).unwrap()).unwrap());
}

#[test]
fn github_asking_to_wait_is_rate_limited() {
    assert_eq!(github_answering(http::answer("403 Forbidden", b"{}")), Err(UpdateError::RateLimited));
    assert_eq!(github_answering(http::answer("429 Too Many Requests", b"{}")), Err(UpdateError::RateLimited));
}

#[test]
fn github_answering_anything_else_is_a_bad_answer() {
    assert_eq!(
        github_answering(http::answer("500 Internal Server Error", b"oops")),
        Err(UpdateError::BadAnswer("HTTP 500".into()))
    );
    assert!(matches!(github_answering(http::file(b"not json")), Err(UpdateError::BadAnswer(_))));
}

#[test]
fn github_out_of_reach_is_a_network_error() {
    // a port nothing listens on: bound, then closed again
    let port = std::net::TcpListener::bind("127.0.0.1:0").unwrap().local_addr().unwrap().port();
    let error = GitHub::at(&format!("http://127.0.0.1:{port}/latest")).latest().unwrap_err();
    assert!(matches!(error, UpdateError::Network(_)), "{error:?}");
}

#[test]
fn github_cut_off_mid_answer_is_a_network_error() {
    let served = Served { announced: Some(5000), ..http::file(b"{\"tag_name\":") };
    assert!(matches!(github_answering(served), Err(UpdateError::Network(_))));
}
