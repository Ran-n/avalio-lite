// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.326309
//+ Revised: 	2026/11/07 20:25:34.326309
// -----------------------------------------------------------------------------

//! Signs release files with the avalio-lite release key, and checks signatures. scripts/release.ps1
//! runs it for every exe it builds; the updater checks the same signatures (src/update/signature.rs).
//!
//!   cargo run --example release_sign -- new-key <key file>
//!       makes a release key; writes the private key to <key file> (never over an existing one) and
//!       prints only the public key, which goes into src/update/mod.rs
//!   cargo run --example release_sign -- sign <key file> <file>...
//!       writes <file>.sig next to each file
//!   cargo run --example release_sign -- verify <file> [--public-key <base64url>]
//!       checks <file>.sig; without --public-key, against the key the app embeds

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use avalio_lite::update::signature::{self, encode_key, file_hash, private_key, public_key, sig_line};
use ed25519_dalek::SigningKey;

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.iter().map(String::as_str).collect::<Vec<_>>().as_slice() {
        ["new-key", key] => new_key(Path::new(key)),
        ["sign", key, files @ ..] if !files.is_empty() => sign(Path::new(key), files),
        ["verify", file] => verify(Path::new(file), avalio_lite::update::PUBLIC_KEY),
        ["verify", file, "--public-key", key] => verify(Path::new(file), key),
        _ => Err(
            "usage: release_sign new-key <key file> | sign <key file> <file>... | verify <file> [--public-key <key>]"
                .to_owned(),
        ),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn new_key(path: &Path) -> Result<(), String> {
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).map_err(|error| format!("no random bytes: {error}"))?;
    let key = SigningKey::from_bytes(&bytes);
    // create_new: an existing key is never overwritten, since losing it ends in-app updates
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    file.write_all(encode_key(&bytes).as_bytes()).map_err(|error| format!("{}: {error}", path.display()))?;
    println!("public key: {}", encode_key(&key.verifying_key().to_bytes()));
    Ok(())
}

fn sign(key_path: &Path, files: &[&str]) -> Result<(), String> {
    let text = fs::read_to_string(key_path).map_err(|error| format!("{}: {error}", key_path.display()))?;
    let key = private_key(&text).map_err(|error| format!("{}: {error}", key_path.display()))?;
    for file in files {
        let path = Path::new(file);
        let name = file_name(path)?;
        let hash = file_hash(path).map_err(|error| format!("{file}: {error}"))?;
        let line = sig_line(&signature::sign(&key, &name, &hash), &name);
        let sig = sig_path(path);
        fs::write(&sig, format!("{line}\n")).map_err(|error| format!("{}: {error}", sig.display()))?;
        println!("signed {name}");
    }
    Ok(())
}

fn verify(path: &Path, key: &str) -> Result<(), String> {
    let key = public_key(key).map_err(|error| error.to_string())?;
    let name = file_name(path)?;
    let hash = file_hash(path).map_err(|error| format!("{}: {error}", path.display()))?;
    let sig = sig_path(path);
    let line = fs::read_to_string(&sig).map_err(|error| format!("{}: {error}", sig.display()))?;
    signature::verify(&key, &name, &hash, &line).map_err(|error| format!("{name}: {error}"))?;
    println!("good signature: {name}");
    Ok(())
}

fn file_name(path: &Path) -> Result<String, String> {
    path.file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned)
        .ok_or_else(|| format!("{} has no file name", path.display()))
}

fn sig_path(path: &Path) -> PathBuf {
    let mut sig = path.as_os_str().to_owned();
    sig.push(".sig");
    PathBuf::from(sig)
}
