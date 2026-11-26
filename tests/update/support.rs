// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:34.169017
//+ Revised: 	2026/11/23 13:00:34.169017
// -----------------------------------------------------------------------------

//! Helpers shared by the update tests: fake sources, keys and release files.

use crate::common::release_key;
use crate::common::releases::{TARGET, version};
use avalio_lite::update::release::{Asset, Release};
use avalio_lite::update::signature::hashes::{self, Hashes};
use avalio_lite::update::signature::minisign;
use avalio_lite::update::target::Build;
use ed25519_dalek::{SigningKey, VerifyingKey};
use std::fs;
use std::path::Path;

pub(crate) const BUILD: Build = Build::Release(TARGET);

pub(crate) fn stranger() -> SigningKey {
    SigningKey::from_bytes(&[9; 32])
}

pub(crate) fn keys() -> Vec<VerifyingKey> {
    vec![release_key().verifying_key()]
}

pub(crate) fn hash_of(content: &[u8]) -> String {
    blake3::hash(content).to_hex().to_string()
}

// ---------------------------------------------------------------------------------------------
// The check

pub(crate) fn release(text: &str, names: &[String]) -> Release {
    let assets =
        names.iter().map(|name| Asset { name: name.clone(), url: format!("https://example.org/{name}") }).collect();
    Release { version: version(text), page: "https://example.org/page".into(), assets }
}

// ---------------------------------------------------------------------------------------------
// Signatures

pub(crate) fn hashes_of(content: &[u8]) -> Hashes {
    hashes::of_bytes(content)
}

pub(crate) fn signed(key: &SigningKey, version_text: &str, name: &str, content: &[u8]) -> String {
    minisign::sign(key, &version(version_text), name, &hashes_of(content)).unwrap()
}

// ---------------------------------------------------------------------------------------------
// The bridge: 0.6.0 to 0.10.0 update themselves from the old-named files

/// 0.10.0's own check (src/update/signature.rs then), copied word for word apart from names: the
/// bridge files must pass it as those copies run it.
pub(crate) mod as_0_10_0 {
    use base64::Engine;
    use base64::engine::general_purpose::URL_SAFE_NO_PAD;
    use ed25519_dalek::{SIGNATURE_LENGTH, Signature, VerifyingKey};

    pub const PUBLIC_KEY: &str = "8b8n9Uqg_5-ReHLUbp5xyfL6HwdVgYNXiJQdP0Uex5o";

    pub fn public_key(text: &str) -> VerifyingKey {
        let bytes: [u8; 32] = URL_SAFE_NO_PAD.decode(text.trim()).unwrap().try_into().unwrap();
        VerifyingKey::from_bytes(&bytes).unwrap()
    }

    fn message(file_name: &str, blake3_hex: &str) -> String {
        format!("avalio-lite-release-v1\n{file_name}\n{blake3_hex}")
    }

    pub fn verify(public_key: &VerifyingKey, file_name: &str, blake3_hex: &str, sig_line: &str) -> bool {
        let Some((encoded, signed_name)) = sig_line.trim_end().split_once("  ") else { return false };
        let Ok(bytes) = URL_SAFE_NO_PAD.decode(encoded) else { return false };
        let Ok(bytes) = <[u8; SIGNATURE_LENGTH]>::try_from(bytes) else { return false };
        if signed_name != file_name {
            return false;
        }
        use ed25519_dalek::Verifier;
        public_key.verify(message(file_name, blake3_hex).as_bytes(), &Signature::from_bytes(&bytes)).is_ok()
    }

    /// The file 0.10.0 asks for: format!("avalio-lite-{}-{kind}.exe", release.version).
    pub fn asset_name(version: &semver::Version, kind: &str) -> String {
        format!("avalio-lite-{version}-{kind}.exe")
    }
}

pub(crate) fn files_in(folder: &Path) -> Vec<String> {
    fs::read_dir(folder).map_or_else(
        |_| Vec::new(),
        |entries| entries.map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect(),
    )
}
