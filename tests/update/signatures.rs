// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:33.998580
//+ Revised: 	2026/11/24 16:26:02.985340
// -----------------------------------------------------------------------------

//! Signatures in minisign form: the trusted comment, the built-in keys and the signature check.

use crate::common::release_key;
use crate::common::releases::version;
use crate::support::{hash_of, hashes_of, keys, signed, stranger};
use avalio_lite::update::signature::hashes::{self};
use avalio_lite::update::signature::{
    self, BUILT_IN_KEYS, SigError, bridge, checksum_line, minisign, parse_checksum, public_key, readable_keys,
    secret_key,
};
use ed25519_dalek::VerifyingKey;
use proptest::prelude::*;
use std::fs;

/// Both checks, as the download makes them: the statement for this release file, then the bytes.
fn checked(
    keys: &[VerifyingKey],
    text: &str,
    (version_text, name): (&str, &str),
    content: &[u8],
) -> Result<(), SigError> {
    minisign::verify(keys, text, (&version(version_text), name), &hashes_of(content))
}

#[test]
fn the_trusted_comment_names_the_version_the_asset_and_the_hash() {
    assert_eq!(
        minisign::trusted_comment(&version("1.2.3"), "a.exe", "ABCDEF"),
        "avalio-lite-release-v2 version:1.2.3 file:a.exe blake3:abcdef",
        "the hash in lowercase"
    );
}

#[test]
fn a_minisig_is_four_lines_in_minisigns_layout() {
    let text = signed(&release_key(), "1.0.0", "a.exe", b"x");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(lines.len(), 4, "{text}");
    assert_eq!(lines[0], minisign::UNTRUSTED_COMMENT);
    assert!(lines[1].starts_with("RUQ"), "prehashed mode, \"ED\" in base64: {}", lines[1]);
    let comment = minisign::trusted_comment(&version("1.0.0"), "a.exe", &hash_of(b"x"));
    assert_eq!(lines[2], format!("trusted comment: {comment}"));
    assert!(text.ends_with('\n'));
}

#[test]
fn a_signature_is_accepted_only_for_its_own_comment_file_and_key() {
    let good = signed(&release_key(), "1.0.0", "a.exe", b"x");
    assert_eq!(checked(&keys(), &good, ("1.0.0", "a.exe"), b"x"), Ok(()));
    assert_eq!(checked(&keys(), &good, ("1.0.1", "a.exe"), b"x"), Err(SigError::NotSigned), "another version");
    assert_eq!(checked(&keys(), &good, ("1.0.0", "b.exe"), b"x"), Err(SigError::NotSigned), "another asset");
    assert_eq!(checked(&keys(), &good, ("1.0.0", "a.exe"), b"y"), Err(SigError::NotSigned), "other bytes");
    let strangers = signed(&stranger(), "1.0.0", "a.exe", b"x");
    assert_eq!(checked(&keys(), &strangers, ("1.0.0", "a.exe"), b"x"), Err(SigError::NotSigned), "a stranger's key");
}

#[test]
fn the_comment_check_and_the_file_check_are_separate() {
    let good = signed(&release_key(), "1.0.0", "a.exe", b"x");
    let parsed = minisign::parse(&good).unwrap();
    // before the download: the right statement; another hash is refused
    let key =
        minisign::accept(&keys(), &parsed, &minisign::trusted_comment(&version("1.0.0"), "a.exe", &hash_of(b"x")))
            .unwrap();
    assert_eq!(key, release_key().verifying_key());
    assert_eq!(
        minisign::accept(&keys(), &parsed, &minisign::trusted_comment(&version("1.0.0"), "a.exe", &hash_of(b"y"))),
        Err(SigError::NotSigned)
    );
    // after the download: the first signature over other bytes is refused
    assert_eq!(minisign::check_file(&key, &parsed, &hashes_of(b"x").blake2b), Ok(()));
    assert_eq!(minisign::check_file(&key, &parsed, &hashes_of(b"y").blake2b), Err(SigError::NotSigned));
}

#[test]
fn the_hash_case_the_untrusted_comment_and_crlf_change_nothing() {
    let good = signed(&release_key(), "1.0.0", "a.exe", b"x");
    let parsed = minisign::parse(&good).unwrap();
    let upper = hash_of(b"x").to_uppercase();
    assert!(minisign::accept(&keys(), &parsed, &minisign::trusted_comment(&version("1.0.0"), "a.exe", &upper)).is_ok());
    let other_comment = good.replacen(minisign::UNTRUSTED_COMMENT, "untrusted comment: anything at all", 1);
    assert_eq!(checked(&keys(), &other_comment, ("1.0.0", "a.exe"), b"x"), Ok(()));
    assert_eq!(checked(&keys(), &good.replace('\n', "\r\n"), ("1.0.0", "a.exe"), b"x"), Ok(()));
    assert_eq!(checked(&keys(), good.trim_end(), ("1.0.0", "a.exe"), b"x"), Ok(()), "without its last line break");
    let bare_return = format!("{}\r", good.trim_end());
    assert_eq!(checked(&keys(), &bare_return, ("1.0.0", "a.exe"), b"x"), Ok(()), "half of its last CRLF");
}

#[test]
fn text_that_is_not_a_prehashed_minisig_is_named_unreadable() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let good = signed(&release_key(), "1.0.0", "a.exe", b"x");
    let lines: Vec<&str> = good.lines().collect();
    let with = |index: usize, line: &str| {
        let mut lines = lines.clone();
        lines[index] = line;
        lines.join("\n")
    };
    let mut legacy = b64.decode(lines[1]).unwrap();
    legacy[1] = b'd';
    let legacy = with(1, &b64.encode(&legacy));
    let short = with(1, &b64.encode(&b64.decode(lines[1]).unwrap()[..73]));
    let old = bridge::signature_text(&bridge::sign(&release_key(), "a.exe", &hash_of(b"x")), "a.exe");
    let cases = [
        ("empty", String::new()),
        ("missing lines", lines[..3].join("\n")),
        ("an extra line", format!("{good}more\n")),
        ("no untrusted prefix", with(0, "signature from avalio-lite-sign")),
        ("no trusted prefix", with(2, &lines[2]["trusted comment: ".len()..])),
        ("bad base64", with(1, "not base64!")),
        ("a short signature line", short),
        ("a short global signature", with(3, &b64.encode([0u8; 63]))),
        ("legacy mode", legacy),
        ("a 0.10.0 .sig", old),
    ];
    for (what, text) in cases {
        assert_eq!(minisign::parse(&text), Err(SigError::UnreadableSignature), "{what}: {text:?}");
    }
}

#[test]
fn parts_of_two_genuine_files_cannot_be_mixed() {
    let a = signed(&release_key(), "1.0.0", "a.exe", b"x");
    let b = signed(&release_key(), "1.0.0", "b.exe", b"y");
    let (a, b): (Vec<&str>, Vec<&str>) = (a.lines().collect(), b.lines().collect());
    // b's comment and global signature over a's file signature
    let mixed = [a[0], a[1], b[2], b[3]].join("\n");
    assert_eq!(checked(&keys(), &mixed, ("1.0.0", "b.exe"), b"y"), Err(SigError::NotSigned));
    assert_eq!(checked(&keys(), &mixed, ("1.0.0", "b.exe"), b"x"), Err(SigError::NotSigned));
    // a's comment with b's global signature
    let mixed = [a[0], a[1], a[2], b[3]].join("\n");
    assert_eq!(checked(&keys(), &mixed, ("1.0.0", "a.exe"), b"x"), Err(SigError::NotSigned));
}

#[test]
fn a_changed_key_id_is_refused() {
    use base64::Engine;
    let b64 = base64::engine::general_purpose::STANDARD;
    let good = signed(&release_key(), "1.0.0", "a.exe", b"x");
    let lines: Vec<&str> = good.lines().collect();
    let mut bytes = b64.decode(lines[1]).unwrap();
    bytes[2] ^= 1;
    let changed = [lines[0], &b64.encode(&bytes), lines[2], lines[3]].join("\n");
    assert_eq!(checked(&keys(), &changed, ("1.0.0", "a.exe"), b"x"), Err(SigError::NotSigned));
}

#[test]
fn a_file_name_with_a_line_break_is_never_signed() {
    let refused = minisign::sign(&release_key(), &version("1.0.0"), "a.exe\ntrusted comment: x", &hashes_of(b"x"));
    assert_eq!(refused, Err(SigError::LineBreak));
}

#[test]
fn keys_have_minisigns_public_key_form() {
    use base64::Engine;
    let key = release_key().verifying_key();
    let text = minisign::public_key_text(&key);
    assert_eq!(text.len(), 56);
    assert!(text.starts_with("RW"), "{text}");
    let bytes = base64::engine::general_purpose::STANDARD.decode(&text).unwrap();
    assert_eq!(&bytes[..2], b"Ed");
    assert_eq!(bytes[2..10], minisign::key_id(&key));
    assert_eq!(&bytes[10..], key.as_bytes());
    assert_eq!(minisign::key_id(&key).as_slice(), &blake3::hash(key.as_bytes()).as_bytes()[..8]);
}

/// Reference vectors: the two hashes of a fixed input, so neither library can change under us.
#[test]
fn both_hashes_match_their_reference_vectors() {
    let hashes = hashes_of(b"abc");
    assert_eq!(hashes.blake3, "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85");
    assert_eq!(
        signature::hex(&hashes.blake2b),
        "ba80a53f981c4d0d6a2797b69f12f6e94c212f14685ac4b74b12bb6fdbffa2d1\
         7d87c5392aab792dc252d5de4533cc9518d38aa8dbf1925ab92386edd4009923"
    );
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("abc");
    let content = vec![b'a'; 200_000];
    fs::write(&file, &content).unwrap();
    assert_eq!(hashes::of_file(&file).unwrap(), hashes_of(&content), "in pieces as in one go");
}

#[test]
fn checksum_files_are_read_as_b3sum_writes_them() {
    let hash = hash_of(b"x");
    assert_eq!(parse_checksum(&checksum_line(&hash, "a.exe")), Ok(hash.clone()));
    assert_eq!(parse_checksum(&format!("{}  a.exe\r\nanother line", hash.to_uppercase())), Ok(hash.clone()));
    assert_eq!(parse_checksum(&hash), Ok(hash.clone()), "just the hash");
    for bad in ["", "abc  a.exe", &format!("{}  a.exe", hash.replace('a', "x")), &format!("\n{hash}")] {
        assert_eq!(parse_checksum(bad), Err(SigError::UnreadableChecksum), "{bad:?}");
    }
}

#[test]
fn either_trusted_key_is_enough_and_a_rotated_out_key_stops_counting() {
    let (main, backup) = (release_key(), stranger());
    let both = vec![main.verifying_key(), backup.verifying_key()];
    for key in [&main, &backup] {
        assert_eq!(checked(&both, &signed(key, "1.0.0", "a.exe", b"x"), ("1.0.0", "a.exe"), b"x"), Ok(()));
    }
    let rotated = vec![backup.verifying_key()];
    let by_main = signed(&main, "1.0.0", "a.exe", b"x");
    assert_eq!(checked(&rotated, &by_main, ("1.0.0", "a.exe"), b"x"), Err(SigError::NotSigned));
}

#[test]
fn an_unreadable_key_among_good_ones_hides_neither_result() {
    let good = signature::hex(&release_key().verifying_key().to_bytes());
    let keys = readable_keys(&["not a key", &good]).unwrap();
    assert_eq!(keys, vec![release_key().verifying_key()]);
    assert_eq!(readable_keys(&["", "nope"]), Err(SigError::NoKey));
    assert!(matches!(public_key("12"), Err(SigError::BadKey(_))));
    // 64 bytes, but not 64 characters: cutting it in pairs would cut through a letter
    assert!(matches!(public_key(&"é".repeat(32)), Err(SigError::BadKey(_))));
}

#[test]
fn the_built_in_keys_are_valid_and_distinct() {
    let keys = readable_keys(&BUILT_IN_KEYS).unwrap();
    assert_eq!(keys.len(), 2, "both read");
    assert_ne!(keys[0], keys[1]);
    assert!(keys.iter().all(|key| *key != release_key().verifying_key()), "the tests' key is no release key");
}

/// Pins a regression: a placeholder once stood in for the backup key, and the build trusted only one.
#[test]
fn the_built_in_keys_are_hex_public_keys() {
    for key in BUILT_IN_KEYS {
        assert!(key.len() == 64 && key.bytes().all(|byte| byte.is_ascii_hexdigit()), "{key:?} isn't 64 hex digits");
        assert!(public_key(key).is_ok(), "{key} isn't an Ed25519 public key");
    }
}

#[test]
fn secret_keys_read_as_hex_or_as_the_first_keys_base64url() {
    use base64::Engine;
    let key = release_key();
    let hex = signature::hex(&key.to_bytes());
    assert_eq!(secret_key(&hex).unwrap().to_bytes(), key.to_bytes());
    assert_eq!(secret_key(&format!("{hex}\r\n")).unwrap().to_bytes(), key.to_bytes(), "as a pipe gives it");
    let base64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(key.to_bytes());
    assert_eq!(secret_key(&format!("{base64}\n")).unwrap().to_bytes(), key.to_bytes());
    assert!(secret_key("nope").is_err());
}

/// The contract: what this code writes, an independent minisign implementation accepts.
#[test]
fn an_independent_minisign_verifier_accepts_the_signatures() {
    let content = b"the new version";
    let text = signed(&release_key(), "1.0.0", "a.exe", content);
    let public_text = minisign::public_key_text(&release_key().verifying_key());
    let public = minisign_verify::PublicKey::from_base64(&public_text).unwrap();
    let signature = minisign_verify::Signature::decode(&text).unwrap();
    let comment = minisign::trusted_comment(&version("1.0.0"), "a.exe", &hash_of(content));
    assert_eq!(signature.trusted_comment(), comment);
    assert!(public.verify(content, &signature, false).is_ok());
    assert!(public.verify(b"other bytes", &signature, false).is_err());
}

proptest! {
    /// Changing any part of what is signed makes the check fail.
    #[test]
    fn any_change_to_what_is_signed_fails(
        major in 0u64..50, other_major in 0u64..50, name in "[a-z0-9.-]{1,30}", other_name in "[a-z0-9.-]{1,30}",
        content in proptest::collection::vec(any::<u8>(), 0..64), other_content in proptest::collection::vec(any::<u8>(), 0..64),
    ) {
        let (version_a, version_b) = (semver::Version::new(major, 0, 0), semver::Version::new(other_major, 0, 0));
        let text = minisign::sign(&release_key(), &version_a, &name, &hashes_of(&content)).unwrap();
        let same = version_a == version_b && name == other_name && content == other_content;
        let accepted = minisign::verify(&keys(), &text, (&version_b, &other_name), &hashes_of(&other_content)).is_ok();
        prop_assert_eq!(accepted, same);
    }

    /// Changing any signed byte of a .minisig makes it unreadable or refused, never accepted.
    #[test]
    fn any_changed_byte_of_a_minisig_fails(index in 0usize..400, flip in 1u8..=255) {
        let text = signed(&release_key(), "1.0.0", "a.exe", b"x").into_bytes();
        let index = index % text.len();
        // the untrusted comment's text after its prefix isn't signed
        let first_line_end = text.iter().position(|byte| *byte == b'\n').unwrap();
        prop_assume!(index < "untrusted comment: ".len() || index >= first_line_end);
        let mut changed = text.clone();
        changed[index] ^= flip;
        // the last line break isn't signed either: it may be missing, or be a CRLF cut in half
        prop_assume!(index != text.len() - 1 || changed[index] != b'\r');
        let changed = String::from_utf8_lossy(&changed).into_owned();
        prop_assert!(checked(&keys(), &changed, ("1.0.0", "a.exe"), b"x").is_err());
    }
}
