// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:59.263457
//+ Revised: 	2026/11/22 21:06:11.628019
// -----------------------------------------------------------------------------

//! The signature format of 0.6.0 to 0.10.0, which those copies still check when they update. Only
//! the release tooling uses it: `avalio-lite-sign` writes it for the old-named files every release
//! carries until the bridge ends (docs/development.md, Releases), and checks it before upload. The
//! app itself never accepts it.
//!
//! `.sig` line: `<signature, base64url without padding>  <file name>`; signed message: the context,
//! the file name and its BLAKE3 hash (hex), one per line, without a final line break. It can't be
//! mistaken for a minisign signature: minisign signs a 64-byte BLAKE2b-512 hash or the first
//! signature followed by a trusted comment starting `avalio-lite-release-v2`, never text starting
//! with this format's `avalio-lite-release-v1` line.

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{SIGNATURE_LENGTH, Signature, SigningKey, VerifyingKey};

use super::SigError;

/// Starts every message of this format, as 0.6.0 to 0.10.0 expect it.
pub const CONTEXT: &str = "avalio-lite-release-v1";

/// The key those copies trust, as they embed it (base64url): the first of `BUILT_IN_KEYS`.
pub const BRIDGE_KEY: &str = "8b8n9Uqg_5-ReHLUbp5xyfL6HwdVgYNXiJQdP0Uex5o";

pub fn message(file_name: &str, blake3_hex: &str) -> String {
    format!("{CONTEXT}\n{file_name}\n{blake3_hex}")
}

pub fn sign(key: &SigningKey, file_name: &str, blake3_hex: &str) -> Signature {
    use ed25519_dalek::Signer;
    key.sign(message(file_name, blake3_hex).as_bytes())
}

/// The `.sig` file's text, with its line break.
pub fn signature_text(signature: &Signature, file_name: &str) -> String {
    format!("{}  {file_name}\n", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
}

/// Checks a `.sig` line as 0.10.0 does: the name in it must be `file_name`, and the signature
/// must be `key`'s over the message.
pub fn verify(key: &VerifyingKey, file_name: &str, blake3_hex: &str, line: &str) -> Result<(), SigError> {
    let (encoded, name) = line.trim_end().split_once("  ").ok_or(SigError::UnreadableSignature)?;
    let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| SigError::UnreadableSignature)?;
    let bytes: [u8; SIGNATURE_LENGTH] = bytes.try_into().map_err(|_| SigError::UnreadableSignature)?;
    if name != file_name {
        return Err(SigError::NotSigned);
    }
    use ed25519_dalek::Verifier;
    key.verify(message(file_name, blake3_hex).as_bytes(), &Signature::from_bytes(&bytes))
        .map_err(|_| SigError::NotSigned)
}

/// `BRIDGE_KEY` as a key.
pub fn bridge_key() -> VerifyingKey {
    // a constant, and a test proves it is the first built-in key
    decode_key(BRIDGE_KEY).and_then(|bytes| VerifyingKey::from_bytes(&bytes).ok()).expect("the bridge key is valid")
}

/// 32 bytes written as base64url without padding, as the first key was.
pub fn decode_key(text: &str) -> Option<[u8; 32]> {
    URL_SAFE_NO_PAD.decode(text).ok()?.try_into().ok()
}
