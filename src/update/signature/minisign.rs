// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/22 21:06:23.144782
//+ Revised: 	2026/11/24 17:54:06.281610
// -----------------------------------------------------------------------------

//! `<asset>.minisig`: a signature in minisign's format (https://jedisct1.github.io/minisign/), so
//! anyone can check a download with `minisign -Vm <asset> -P <key>`. Four lines:
//!
//! ```text
//! untrusted comment: signature from avalio-lite-sign
//! <base64 of "ED" + 8-byte key ID + Ed25519 signature over the file's BLAKE2b-512 hash>
//! trusted comment: avalio-lite-release-v2 version:<version> file:<asset> blake3:<hash>
//! <base64 of an Ed25519 signature over (the first signature's 64 bytes + the trusted comment)>
//! ```
//!
//! The trusted comment is the release statement: it names the version and the file as well as the
//! hash, so a genuine older build, or another target's, can't pass for the offered one. Only the
//! prehashed mode (`ED`) is written or read; minisign's legacy mode (`Ed`) is refused.

use base64::Engine;
use base64::engine::general_purpose::STANDARD;
use ed25519_dalek::{SIGNATURE_LENGTH, Signature, Signer, SigningKey, VerifyingKey};

use super::hashes::{BLAKE2B_LENGTH, Hashes};
use super::{CONTEXT, SigError};

/// The first line every `.minisig` this tool writes starts with; it isn't signed.
pub const UNTRUSTED_COMMENT: &str = "untrusted comment: signature from avalio-lite-sign";

pub const KEY_ID_LENGTH: usize = 8;

const UNTRUSTED_PREFIX: &str = "untrusted comment: ";
const TRUSTED_PREFIX: &str = "trusted comment: ";
/// minisign's prehashed signature algorithm; the legacy one is `Ed`
const PREHASHED: &[u8; 2] = b"ED";
/// the algorithm a minisign public key names
const KEY_ALGORITHM: &[u8; 2] = b"Ed";
const SIGNATURE_LINE_LENGTH: usize = 2 + KEY_ID_LENGTH + SIGNATURE_LENGTH;

/// A `.minisig` file, read but not yet checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Parsed {
    pub key_id: [u8; KEY_ID_LENGTH],
    /// over the file's BLAKE2b-512 hash
    pub signature: Signature,
    pub trusted_comment: String,
    /// over `signature` + `trusted_comment`
    pub global: Signature,
}

/// The statement a release file's signature carries.
pub fn trusted_comment(version: &semver::Version, asset: &str, blake3_hex: &str) -> String {
    format!("{CONTEXT} version:{version} file:{asset} blake3:{}", blake3_hex.to_ascii_lowercase())
}

/// The first 8 bytes of the BLAKE3 hash of the public key: the same key always has the same ID.
pub fn key_id(key: &VerifyingKey) -> [u8; KEY_ID_LENGTH] {
    let hash = blake3::hash(key.as_bytes());
    let mut id = [0u8; KEY_ID_LENGTH];
    id.copy_from_slice(&hash.as_bytes()[..KEY_ID_LENGTH]);
    id
}

/// The key in minisign's form: base64 of `Ed` + key ID + the key's 32 bytes; 56 characters,
/// starting `RW`.
pub fn public_key_text(key: &VerifyingKey) -> String {
    STANDARD.encode([KEY_ALGORITHM.as_slice(), &key_id(key), key.as_bytes()].concat())
}

/// The `.minisig` text for a file with these `hashes`, as `version`'s `asset`. Refuses an asset name
/// with a line break, which would end the trusted comment early.
pub fn sign(key: &SigningKey, version: &semver::Version, asset: &str, hashes: &Hashes) -> Result<String, SigError> {
    if asset.contains(['\n', '\r']) {
        return Err(SigError::LineBreak);
    }
    let comment = trusted_comment(version, asset, &hashes.blake3);
    let signature = key.sign(&hashes.blake2b);
    let global = key.sign(&global_message(&signature, &comment));
    let line = [PREHASHED.as_slice(), &key_id(&key.verifying_key()), &signature.to_bytes()].concat();
    Ok(format!(
        "{UNTRUSTED_COMMENT}\n{}\n{TRUSTED_PREFIX}{comment}\n{}\n",
        STANDARD.encode(line),
        STANDARD.encode(global.to_bytes())
    ))
}

/// Reads a `.minisig`: four lines ending in `\n` or `\r\n` (the last may lack it). Anything else,
/// the legacy mode included, is unreadable.
pub fn parse(text: &str) -> Result<Parsed, SigError> {
    let mut lines = text.split('\n').map(|line| line.strip_suffix('\r').unwrap_or(line));
    let mut next = || lines.next().ok_or(SigError::UnreadableSignature);
    let (untrusted, signature_line, trusted, global_line) = (next()?, next()?, next()?, next()?);
    if lines.any(|rest| !rest.is_empty()) || !untrusted.starts_with(UNTRUSTED_PREFIX) {
        return Err(SigError::UnreadableSignature);
    }
    let trusted_comment = trusted.strip_prefix(TRUSTED_PREFIX).ok_or(SigError::UnreadableSignature)?;
    let bytes: [u8; SIGNATURE_LINE_LENGTH] = decode(signature_line)?;
    if &bytes[..2] != PREHASHED {
        return Err(SigError::UnreadableSignature);
    }
    let mut key_id = [0u8; KEY_ID_LENGTH];
    key_id.copy_from_slice(&bytes[2..2 + KEY_ID_LENGTH]);
    let signature = Signature::from_slice(&bytes[2 + KEY_ID_LENGTH..]).map_err(|_| SigError::UnreadableSignature)?;
    let global = Signature::from_bytes(&decode(global_line)?);
    Ok(Parsed { key_id, signature, trusted_comment: trusted_comment.to_owned(), global })
}

/// The check made before the download: the trusted comment is `comment` (the statement built from
/// the offered version, asset name and `.b3` hash, `trusted_comment`), and its signature holds for
/// one of `keys` whose ID matches. Returns that key, for the file check.
pub fn accept(keys: &[VerifyingKey], parsed: &Parsed, comment: &str) -> Result<VerifyingKey, SigError> {
    if parsed.trusted_comment != comment {
        return Err(SigError::NotSigned);
    }
    let message = global_message(&parsed.signature, &parsed.trusted_comment);
    keys.iter()
        .find(|key| key_id(key) == parsed.key_id && key.verify_strict(&message, &parsed.global).is_ok())
        .copied()
        .ok_or(SigError::NotSigned)
}

/// The check made after the download: the first signature holds, for `key`, over the BLAKE2b-512
/// hash of the bytes received.
pub fn check_file(key: &VerifyingKey, parsed: &Parsed, blake2b: &[u8; BLAKE2B_LENGTH]) -> Result<(), SigError> {
    key.verify_strict(blake2b, &parsed.signature).map_err(|_| SigError::NotSigned)
}

/// Both checks on a file whose hashes are known, as `version`'s `asset`: what
/// `avalio-lite-sign verify` does.
pub fn verify(
    keys: &[VerifyingKey],
    text: &str,
    (version, asset): (&semver::Version, &str),
    hashes: &Hashes,
) -> Result<(), SigError> {
    let parsed = parse(text)?;
    let key = accept(keys, &parsed, &trusted_comment(version, asset, &hashes.blake3))?;
    check_file(&key, &parsed, &hashes.blake2b)
}

fn global_message(signature: &Signature, comment: &str) -> Vec<u8> {
    [signature.to_bytes().as_slice(), comment.as_bytes()].concat()
}

fn decode<const N: usize>(line: &str) -> Result<[u8; N], SigError> {
    let bytes = STANDARD.decode(line).map_err(|_| SigError::UnreadableSignature)?;
    bytes.try_into().map_err(|_| SigError::UnreadableSignature)
}
