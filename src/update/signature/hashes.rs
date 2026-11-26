// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/22 21:06:22.997040
//+ Revised: 	2026/11/22 21:06:22.997040
// -----------------------------------------------------------------------------

//! The two hashes a release file is checked with, made in one pass: BLAKE3 (the `.b3` line and the
//! signed statement) and BLAKE2b-512 (what minisign's prehashed mode signs).

use std::fs::File;
use std::io::{self, Read};
use std::path::Path;

use blake2::{Blake2b512, Digest};

/// BLAKE2b-512's output length, in bytes.
pub const BLAKE2B_LENGTH: usize = 64;

const CHUNK: usize = 64 * 1024;

/// A file's two hashes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hashes {
    /// lowercase hex
    pub blake3: String,
    pub blake2b: [u8; BLAKE2B_LENGTH],
}

/// Both hashes at once, fed piece by piece, so a download is hashed as it arrives.
#[derive(Default)]
pub struct Hasher {
    blake3: blake3::Hasher,
    blake2b: Blake2b512,
}

impl Hasher {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn update(&mut self, bytes: &[u8]) {
        self.blake3.update(bytes);
        Digest::update(&mut self.blake2b, bytes);
    }

    pub fn finalize(self) -> Hashes {
        Hashes { blake3: self.blake3.finalize().to_hex().to_string(), blake2b: self.blake2b.finalize().into() }
    }
}

/// The hashes of `bytes`.
pub fn of_bytes(bytes: &[u8]) -> Hashes {
    let mut hasher = Hasher::new();
    hasher.update(bytes);
    hasher.finalize()
}

/// The hashes of a file, read in pieces so a large file isn't held in memory.
pub fn of_file(path: &Path) -> io::Result<Hashes> {
    let mut file = File::open(path)?;
    let mut hasher = Hasher::new();
    let mut buffer = vec![0u8; CHUNK];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            return Ok(hasher.finalize());
        }
        hasher.update(&buffer[..read]);
    }
}
