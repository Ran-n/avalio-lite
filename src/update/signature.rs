// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.751522
//+ Revised: 	2026/11/07 20:25:34.751522
// -----------------------------------------------------------------------------

//! Release signatures: each release file ships with `<file>.sig`, an Ed25519 signature made with
//! the avalio-lite release key over the file's name and BLAKE3 hash. The release tool writes them
//! (examples/release_sign.rs) and the updater checks them before running a download.

use std::fs::File;
use std::io;
use std::path::Path;

use base64::Engine;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use ed25519_dalek::{SIGNATURE_LENGTH, Signature, SigningKey, VerifyingKey};

/// Starts every signed message, so the format can change later without being mistaken for this one.
pub const CONTEXT: &str = "avalio-lite-release-v1";

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum SigError {
    #[error("the signature file isn't a signature line")]
    Malformed,
    #[error("the signature is for {found}, not {expected}")]
    WrongFile { expected: String, found: String },
    #[error("the signature doesn't match the file")]
    BadSignature,
    #[error("the key isn't a valid Ed25519 key")]
    BadKey,
}

/// What is signed: the context, the file's name and its BLAKE3 hash (hex), one per line. The name
/// is in it so a valid signature for one file (the setup) can't pass for another (the portable).
pub fn message(file_name: &str, blake3_hex: &str) -> String {
    format!("{CONTEXT}\n{file_name}\n{blake3_hex}")
}

/// The BLAKE3 hash of a file, in hex, read in pieces so a large file isn't held in memory.
pub fn file_hash(path: &Path) -> io::Result<String> {
    let mut hasher = blake3::Hasher::new();
    hasher.update_reader(File::open(path)?)?;
    Ok(hasher.finalize().to_hex().to_string())
}

/// The line of a `.sig` file: `<signature, base64url without padding>  <file name>`, laid out
/// like the `.b3` checksum next to it.
pub fn sig_line(signature: &Signature, file_name: &str) -> String {
    format!("{}  {file_name}", URL_SAFE_NO_PAD.encode(signature.to_bytes()))
}

pub fn parse_sig_line(line: &str) -> Result<(Signature, String), SigError> {
    let (encoded, name) = line.trim_end().split_once("  ").ok_or(SigError::Malformed)?;
    let bytes = URL_SAFE_NO_PAD.decode(encoded).map_err(|_| SigError::Malformed)?;
    let bytes: [u8; SIGNATURE_LENGTH] = bytes.try_into().map_err(|_| SigError::Malformed)?;
    if name.is_empty() {
        return Err(SigError::Malformed);
    }
    Ok((Signature::from_bytes(&bytes), name.to_owned()))
}

pub fn sign(key: &SigningKey, file_name: &str, blake3_hex: &str) -> Signature {
    use ed25519_dalek::Signer;
    key.sign(message(file_name, blake3_hex).as_bytes())
}

/// Checks a `.sig` line against `file_name` and its hash.
pub fn verify(public_key: &VerifyingKey, file_name: &str, blake3_hex: &str, sig_line: &str) -> Result<(), SigError> {
    let (signature, signed_name) = parse_sig_line(sig_line)?;
    if signed_name != file_name {
        return Err(SigError::WrongFile { expected: file_name.to_owned(), found: signed_name });
    }
    public_key.verify_strict(message(file_name, blake3_hex).as_bytes(), &signature).map_err(|_| SigError::BadSignature)
}

/// A public key from its base64url text, as `release_sign new-key` prints it.
pub fn public_key(text: &str) -> Result<VerifyingKey, SigError> {
    let bytes = URL_SAFE_NO_PAD.decode(text.trim()).map_err(|_| SigError::BadKey)?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| SigError::BadKey)?;
    VerifyingKey::from_bytes(&bytes).map_err(|_| SigError::BadKey)
}

/// A private key from its base64url text, as `release_sign new-key` writes it.
pub fn private_key(text: &str) -> Result<SigningKey, SigError> {
    let bytes = URL_SAFE_NO_PAD.decode(text.trim()).map_err(|_| SigError::BadKey)?;
    let bytes: [u8; 32] = bytes.try_into().map_err(|_| SigError::BadKey)?;
    Ok(SigningKey::from_bytes(&bytes))
}

/// The text a key is kept as.
pub fn encode_key(bytes: &[u8; 32]) -> String {
    URL_SAFE_NO_PAD.encode(bytes)
}
