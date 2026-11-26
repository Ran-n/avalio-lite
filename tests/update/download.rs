// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.654639
//+ Revised: 	2026/11/24 16:26:03.183317
// -----------------------------------------------------------------------------

//! The download and its proof: size, progress, checksum and the signed hash.

use crate::common::http::{self, Served, UNKNOWN_LENGTH};
use crate::common::release_key;
use crate::common::releases::{self, Published, version};
use crate::support::{files_in, hash_of, hashes_of, keys, signed, stranger};
use avalio_lite::update::download::{self, Fetch, Limits, PREFIX, Progress, Saving, proof};
use avalio_lite::update::release::{NetworkError, UpdateError};
use avalio_lite::update::signature::hashes::Hashes;
use avalio_lite::update::signature::{SigError, bridge, minisign};
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

// ---------------------------------------------------------------------------------------------
// The proof and the download

fn fetch(
    published: &Published,
    limits: Limits,
    cancel: &AtomicBool,
) -> (Result<PathBuf, UpdateError>, tempfile::TempDir) {
    let folder = tempfile::tempdir().unwrap();
    let got = download::download(
        &Fetch { http: &releases::http(), offer: &published.offer(), keys: &keys() },
        &Saving { folder: folder.path(), limits, cancel },
        |_| {},
    );
    (got, folder)
}

#[test]
fn a_good_download_verifies() {
    let published = Published::new("1.1.0", b"the new version", &release_key());
    let (got, folder) = fetch(&published, Limits::default(), &AtomicBool::new(false));
    let file = got.unwrap();
    assert_eq!(fs::read(&file).unwrap(), b"the new version");
    assert!(file.file_name().unwrap().to_string_lossy().starts_with(PREFIX));
    assert_eq!(file.parent().unwrap(), folder.path());
    assert_eq!(
        (published.server.count(&published.checksum_path()), published.server.count(&published.signature_path())),
        (1, 1)
    );
}

#[test]
fn progress_is_sent_once_per_whole_percent() {
    let content = vec![7u8; 640 * 1024];
    let published = Published::new("1.1.0", &content, &release_key());
    let folder = tempfile::tempdir().unwrap();
    let mut seen = Vec::new();
    download::download(
        &Fetch { http: &releases::http(), offer: &published.offer(), keys: &keys() },
        &Saving { folder: folder.path(), limits: Limits::default(), cancel: &AtomicBool::new(false) },
        |progress| seen.push(progress),
    )
    .unwrap();
    assert_eq!(seen.last(), Some(&Progress::Percent(100)));
    assert!(seen.windows(2).all(|pair| pair[0] != pair[1]), "{seen:?}");
}

#[test]
fn without_a_size_progress_is_sent_per_mebibyte() {
    let content = vec![7u8; 2 * 1024 * 1024 + 10];
    let published = Published::new("1.1.0", &content, &release_key());
    published.serve(&published.asset_path(), Served { announced: Some(UNKNOWN_LENGTH), ..http::file(&content) });
    let folder = tempfile::tempdir().unwrap();
    let mut seen = Vec::new();
    download::download(
        &Fetch { http: &releases::http(), offer: &published.offer(), keys: &keys() },
        &Saving { folder: folder.path(), limits: Limits::default(), cancel: &AtomicBool::new(false) },
        |progress| seen.push(progress),
    )
    .unwrap();
    assert_eq!(seen, [Progress::Bytes(0), Progress::Bytes(1024 * 1024), Progress::Bytes(2 * 1024 * 1024)]);
}

#[test]
fn a_checksum_mismatch_deletes_the_file() {
    let published = Published::new("1.1.0", b"the new version", &release_key());
    published.serve(&published.asset_path(), http::file(b"something else"));
    let (got, folder) = fetch(&published, Limits::default(), &AtomicBool::new(false));
    assert_eq!(got, Err(UpdateError::ChecksumMismatch));
    assert!(files_in(folder.path()).is_empty());
}

#[test]
fn an_oversized_download_is_refused_declared_or_delivered() {
    let content = vec![1u8; 4096];
    let published = Published::new("1.1.0", &content, &release_key());
    let limits = Limits { size: 1024, ..Limits::default() };
    let (got, folder) = fetch(&published, limits, &AtomicBool::new(false));
    assert_eq!(got, Err(UpdateError::TooLarge(NetworkError::TooLarge)), "declared");
    assert!(files_in(folder.path()).is_empty());
    published.serve(&published.asset_path(), Served { announced: Some(UNKNOWN_LENGTH), ..http::file(&content) });
    let (got, folder) = fetch(&published, limits, &AtomicBool::new(false));
    assert_eq!(got, Err(UpdateError::TooLarge(NetworkError::TooLarge)), "delivered");
    assert!(files_in(folder.path()).is_empty());
    assert_eq!(Limits::default().size, 200 * 1024 * 1024);
}

#[test]
fn a_cancelled_download_deletes_the_file() {
    let content = vec![3u8; 256 * 1024];
    let published = Published::new("1.1.0", &content, &release_key());
    published.serve(
        &published.asset_path(),
        Served { paced: Some((16 * 1024, Duration::from_millis(20))), ..http::file(&content) },
    );
    let folder = tempfile::tempdir().unwrap();
    let cancel = AtomicBool::new(false);
    let got = download::download(
        &Fetch { http: &releases::http(), offer: &published.offer(), keys: &keys() },
        &Saving { folder: folder.path(), limits: Limits::default(), cancel: &cancel },
        |progress| {
            if progress != Progress::Percent(0) {
                cancel.store(true, Ordering::SeqCst);
            }
        },
    );
    assert_eq!(got, Err(UpdateError::Cancelled));
    assert!(files_in(folder.path()).is_empty());
    let (got, folder) = fetch(&published, Limits::default(), &AtomicBool::new(true));
    assert_eq!(got, Err(UpdateError::Cancelled), "cancelled before it began");
    assert!(files_in(folder.path()).is_empty());
}

#[test]
fn a_download_that_stalls_or_takes_too_long_is_refused() {
    let content = vec![3u8; 64 * 1024];
    let published = Published::new("1.1.0", &content, &release_key());
    published.serve(
        &published.asset_path(),
        // a pause far past the limit, so a held-up client can't find the whole body waiting
        Served { paced: Some((32 * 1024, Duration::from_secs(600))), ..http::file(&content) },
    );
    let stalls = Limits { stall: Duration::from_millis(300), ..Limits::default() };
    let (got, folder) = fetch(&published, stalls, &AtomicBool::new(false));
    assert!(matches!(got, Err(UpdateError::TooSlow(_))), "{got:?}");
    assert!(files_in(folder.path()).is_empty());
    published.serve(
        &published.asset_path(),
        Served { paced: Some((8 * 1024, Duration::from_millis(100))), ..http::file(&content) },
    );
    let slow = Limits { total: Duration::from_millis(400), ..Limits::default() };
    let (got, folder) = fetch(&published, slow, &AtomicBool::new(false));
    assert!(matches!(got, Err(UpdateError::TooSlow(_))), "{got:?}");
    assert!(files_in(folder.path()).is_empty());
    // a server fast enough never to stall can still run past the limit for the whole download
    published.serve(&published.asset_path(), http::file(&content));
    let at_once = Limits { total: Duration::ZERO, ..Limits::default() };
    let (got, folder) = fetch(&published, at_once, &AtomicBool::new(false));
    assert!(matches!(got, Err(UpdateError::TooSlow(_))), "{got:?}");
    assert!(files_in(folder.path()).is_empty());
    assert_eq!(
        (Limits::default().stall, Limits::default().total),
        (Duration::from_secs(60), Duration::from_secs(1800))
    );
}

#[test]
fn a_truncated_download_is_refused() {
    let published = Published::new("1.1.0", b"the new version", &release_key());
    published.serve(&published.asset_path(), Served { announced: Some(1000), ..http::file(b"the new") });
    let (got, folder) = fetch(&published, Limits::default(), &AtomicBool::new(false));
    assert!(matches!(got, Err(UpdateError::Network(_))), "{got:?}");
    assert!(files_in(folder.path()).is_empty());
}

/// A way to spoil what a published release serves.
type Spoil = Box<dyn Fn(&Published)>;

/// Proofs that are wrong in what they hash or sign, each with the label a failure names.
fn wrong_proofs(content: &'static [u8]) -> Vec<(&'static str, Spoil)> {
    vec![
        ("a malformed .b3", Box::new(move |p: &Published| p.serve(&p.checksum_path(), http::file(b"not a checksum")))),
        (
            "a stranger's signature",
            Box::new(move |p: &Published| {
                p.serve(&p.signature_path(), http::file(signed(&stranger(), "1.1.0", &p.name, content).as_bytes()));
            }),
        ),
        (
            "a genuine signature for other bytes",
            Box::new(move |p: &Published| {
                p.serve(&p.signature_path(), http::file(signed(&release_key(), "1.1.0", &p.name, b"other").as_bytes()));
            }),
        ),
        (
            "a genuine signature for another version",
            Box::new(move |p: &Published| {
                p.serve(&p.signature_path(), http::file(signed(&release_key(), "1.0.9", &p.name, content).as_bytes()));
            }),
        ),
        (
            "a 0.10.0-format signature",
            Box::new(move |p: &Published| {
                let old = bridge::signature_text(&bridge::sign(&release_key(), &p.name, &p.hash), &p.name);
                p.serve(&p.signature_path(), http::file(old.as_bytes()));
            }),
        ),
    ]
}

/// Signature files that are missing or larger than the limit, each with its label.
fn unusable_signature_files() -> Vec<(&'static str, Spoil)> {
    vec![
        (
            "a missing .minisig",
            Box::new(move |p: &Published| p.serve(&p.signature_path(), http::answer("404 Not Found", b""))),
        ),
        (
            "a .minisig over the size limit",
            Box::new(move |p: &Published| p.serve(&p.signature_path(), http::file(&[b'a'; 4096]))),
        ),
    ]
}

/// Each bad proof is refused before one byte of the asset is requested.
#[test]
fn a_bad_proof_is_refused_with_no_request_for_the_binary() {
    let content = b"the new version";
    for (what, spoil) in wrong_proofs(content) {
        let published = Published::new("1.1.0", content, &release_key());
        spoil(&published);
        let (got, folder) = fetch(&published, Limits::default(), &AtomicBool::new(false));
        assert!(matches!(got, Err(UpdateError::NotSigned(_) | UpdateError::Network(_))), "{what}: {got:?}");
        assert_eq!(published.server.count(&published.asset_path()), 0, "{what}: the binary was requested");
        assert!(files_in(folder.path()).is_empty(), "{what}");
    }
}

/// A signature file that is missing or over the size limit is refused before one byte of the asset
/// is requested.
#[test]
fn a_missing_or_oversized_signature_is_refused_with_no_request_for_the_binary() {
    let content = b"the new version";
    for (what, spoil) in unusable_signature_files() {
        let published = Published::new("1.1.0", content, &release_key());
        spoil(&published);
        let (got, folder) = fetch(&published, Limits::default(), &AtomicBool::new(false));
        assert!(matches!(got, Err(UpdateError::NotSigned(_) | UpdateError::Network(_))), "{what}: {got:?}");
        assert_eq!(published.server.count(&published.asset_path()), 0, "{what}: the binary was requested");
        assert!(files_in(folder.path()).is_empty(), "{what}");
    }
}

/// The right statement over a first signature for other bytes passes the check before the
/// download, and is caught by the one after it.
#[test]
fn a_signature_over_other_bytes_is_refused_after_the_download_and_the_file_deleted() {
    let content = b"the new version";
    let published = Published::new("1.1.0", content, &release_key());
    let spoiled = Hashes { blake3: hash_of(content), blake2b: hashes_of(b"other bytes").blake2b };
    let text = minisign::sign(&release_key(), &version("1.1.0"), &published.name, &spoiled).unwrap();
    published.serve(&published.signature_path(), http::file(text.as_bytes()));
    let (got, folder) = fetch(&published, Limits::default(), &AtomicBool::new(false));
    assert_eq!(got, Err(UpdateError::NotSigned(SigError::NotSigned)));
    assert_eq!(published.server.count(&published.asset_path()), 1, "refused only once it had arrived");
    assert!(files_in(folder.path()).is_empty());
}

#[test]
fn the_proof_is_the_signed_hash_and_the_key_that_signed_it() {
    let published = Published::new("1.1.0", b"x", &release_key());
    let proof = proof(&releases::http(), &published.offer(), &keys()).unwrap();
    assert_eq!(proof.blake3, hash_of(b"x"));
    assert_eq!(proof.key, release_key().verifying_key());
}
