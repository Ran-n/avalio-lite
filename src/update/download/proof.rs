// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.660799
//+ Revised: 	2026/11/23 11:49:16.660799
// -----------------------------------------------------------------------------

//! The proof a download must hold before and after it: the signed BLAKE3 hash and the minisign
//! signature, read and checked before the asset is requested, then checked over the bytes received.

use ed25519_dalek::VerifyingKey;

use super::super::check::Offer;
use super::super::release::UpdateError;
use super::super::signature::hashes::Hashes;
use super::super::signature::minisign::{self, Parsed};
use super::super::signature::parse_checksum;
use super::super::source::{Http, PROOF_LIMIT};

/// What the download is checked against: the signed BLAKE3 hash, the key whose signature holds,
/// and the signature to check over the bytes once they have arrived.
#[derive(Debug, Clone)]
pub struct Proof {
    pub blake3: String,
    pub key: VerifyingKey,
    pub signature: Parsed,
}

impl Proof {
    /// The check after the download: the BLAKE3 hash is the signed one, and the signature holds
    /// over the BLAKE2b-512 hash of the bytes received.
    pub(super) fn check(&self, got: &Hashes, name: &str) -> Result<(), UpdateError> {
        if !got.blake3.eq_ignore_ascii_case(&self.blake3) {
            log::error!(target: "avalio_lite::update", "{name} didn't match its checksum");
            return Err(UpdateError::ChecksumMismatch);
        }
        minisign::check_file(&self.key, &self.signature, &got.blake2b).map_err(|error| {
            log::error!(target: "avalio_lite::update", "{name}'s signature doesn't hold over its bytes");
            UpdateError::NotSigned(error)
        })
    }
}

/// The asset's proof: its `.b3` read, its `.minisig` read and its statement checked against `keys`
/// for this version and name. Nothing of the asset itself is requested here.
pub fn proof(http: &Http, offer: &Offer, keys: &[VerifyingKey]) -> Result<Proof, UpdateError> {
    let checksum = http.text(&offer.checksum.url, "*/*", PROOF_LIMIT).map_err(UpdateError::Network)?;
    let blake3 = parse_checksum(&checksum).map_err(UpdateError::NotSigned)?;
    let text = http.text(&offer.signature.url, "*/*", PROOF_LIMIT).map_err(UpdateError::Network)?;
    let signature = minisign::parse(&text).map_err(UpdateError::NotSigned)?;
    let key =
        minisign::accept(keys, &signature, &minisign::trusted_comment(offer.version(), &offer.asset.name, &blake3))
            .map_err(UpdateError::NotSigned)?;
    Ok(Proof { blake3, key, signature })
}
