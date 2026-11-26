// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.751522
//+ Revised: 	2026/11/22 21:06:11.268819
// -----------------------------------------------------------------------------

//! Release signatures. Every asset is published with `<asset>.b3` (its BLAKE3 hash, one `b3sum`
//! line) and `<asset>.minisig` (a signature in minisign's format, `minisign`). The updater checks
//! the signature's statement before it fetches one byte of the asset itself, and the signature over
//! the bytes once they have arrived.
//!
//! The old format (0.6.0 to 0.10.0) lives in `bridge`: only the release tooling uses it, to sign
//! the old-named files those copies still look for.

pub mod bridge;
pub mod hashes;
pub mod minisign;

use ed25519_dalek::{SECRET_KEY_LENGTH, SigningKey, VerifyingKey};

use super::target::debug_override;

/// Starts every trusted comment, so the format can change later without being mistaken for this
/// one.
pub const CONTEXT: &str = "avalio-lite-release-v2";

/// The release keys' public halves, as hex. A signature by either is accepted. The first signs
/// every release; the second is the backup, whose secret is kept apart and not used day to day
/// (docs/development.md, Release signing: what to do when a key is lost or stolen).
pub const BUILT_IN_KEYS: [&str; 2] = [
    // the key 0.6.0 to 0.10.0 trust too (as base64url there)
    "f1bf27f54aa0ff9f917872d46e9e71c9f2fa1f075581835788941d3f451ec79a",
    // the backup key; its secret is only in the maintainer's KeePass
    "1c296552be2731bb93904c4ab45b48815ee2490aed66fff1e9b0d6e10b594d1e",
];

/// Debug builds only: trust this key (hex) instead of the built-in ones, so tests sign with a key
/// of their own.
pub const KEY_VARIABLE: &str = "AVALIO_LITE_UPDATE_KEY";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SigError {
    #[error("the signature file isn't a minisign signature")]
    UnreadableSignature,
    #[error("the .b3 file isn't a BLAKE3 checksum line")]
    UnreadableChecksum,
    #[error("the signature isn't by a trusted release key")]
    NotSigned,
    #[error("{0:?} isn't an Ed25519 key in hex")]
    BadKey(String),
    #[error("no readable release key")]
    NoKey,
    #[error("a file name with a line break can't be signed")]
    LineBreak,
}

/// A `.b3` file's hash: the first 64 characters of its first line, which must be hex (either
/// case); lowercase.
pub fn parse_checksum(text: &str) -> Result<String, SigError> {
    let line = text.lines().next().unwrap_or_default();
    let hash = line.get(..64).ok_or(SigError::UnreadableChecksum)?;
    if !hash.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(SigError::UnreadableChecksum);
    }
    Ok(hash.to_ascii_lowercase())
}

/// The `.b3` line `b3sum` writes: the hash, two spaces, the file's name.
pub fn checksum_line(blake3_hex: &str, asset: &str) -> String {
    format!(
        "{blake3_hex}  {asset}
"
    )
}

/// The keys in `texts` that read; an unreadable one is logged and left out, so it hides neither a
/// good signature by another key nor a bad one. `NoKey` when none reads.
pub fn readable_keys(texts: &[&str]) -> Result<Vec<VerifyingKey>, SigError> {
    let keys: Vec<VerifyingKey> = texts
        .iter()
        .filter_map(|text| match public_key(text) {
            Ok(key) => Some(key),
            Err(error) => {
                log::error!("a release key is unreadable: {error}");
                None
            }
        })
        .collect();
    if keys.is_empty() { Err(SigError::NoKey) } else { Ok(keys) }
}

/// The keys this build trusts: the built-in ones, or in a debug build the test key in
/// `AVALIO_LITE_UPDATE_KEY`.
pub fn trusted_keys() -> Result<Vec<VerifyingKey>, SigError> {
    match debug_override(KEY_VARIABLE) {
        Some(key) => {
            log::warn!("trusting the test key in {KEY_VARIABLE} instead of the release keys");
            readable_keys(&[key.as_str()])
        }
        None => readable_keys(&BUILT_IN_KEYS),
    }
}

/// A public key: 64 hex digits (either case).
pub fn public_key(text: &str) -> Result<VerifyingKey, SigError> {
    let bytes: [u8; 32] = unhex(text.trim()).ok_or_else(|| SigError::BadKey(text.to_owned()))?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| SigError::BadKey(text.to_owned()))
}

/// A secret key: 64 hex digits, or the 43-character base64url the first key was written in.
pub fn secret_key(text: &str) -> Result<SigningKey, SigError> {
    let text = text.trim();
    let bytes: [u8; SECRET_KEY_LENGTH] = unhex(text)
        .or_else(|| bridge::decode_key(text))
        .ok_or_else(|| SigError::BadKey("the secret key".to_owned()))?;
    Ok(SigningKey::from_bytes(&bytes))
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn unhex<const N: usize>(text: &str) -> Option<[u8; N]> {
    if text.len() != N * 2 || !text.is_ascii() {
        return None;
    }
    let mut bytes = [0u8; N];
    for (index, byte) in bytes.iter_mut().enumerate() {
        *byte = u8::from_str_radix(&text[index * 2..index * 2 + 2], 16).ok()?;
    }
    Some(bytes)
}
