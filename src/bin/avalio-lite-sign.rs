// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:58.992511
//+ Revised: 	2026/11/24 17:54:04.786487
// -----------------------------------------------------------------------------

//! Signs release assets with the avalio-lite release key and checks their signatures; the updater
//! checks the same signatures (src/update/signature.rs). docs/development.md, Releases, has the
//! steps.
//!
//!   avalio-lite-sign keygen <secret-file>     a new key (32 random bytes, hex); never overwrites
//!   avalio-lite-sign public <secret-file>     prints the public half: hex, then minisign's form
//!   avalio-lite-sign sign <secret-file> <asset>...
//!   avalio-lite-sign verify <asset>...
//!   avalio-lite-sign keys                     the built-in keys in minisign's form, signing key first
//!
//! `<secret-file>` may be `-`: at a terminal the key is pasted at a prompt that doesn't show it;
//! otherwise it is read from standard input, so a password manager's command line can pipe it in.
//! Assets are signed as this checkout's version: a name that doesn't start with
//! `avalio-lite-<version>-` is refused, so sign from the tag's checkout. The old-named bridge files
//! (`avalio-lite-<version>-portable.exe` / `-setup.exe`) get the old `.sig` format their readers
//! (0.6.0 to 0.10.0) understand; every other asset a `.minisig`, which `minisign -Vm` checks too.

use std::fs::{self, OpenOptions};
use std::io::{IsTerminal, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use avalio_lite::update::signature::{BUILT_IN_KEYS, bridge, hashes, hex, minisign, readable_keys, secret_key};
use avalio_lite::update::target::{
    APP, Form, bridge_asset_name, bridge_signature_name, current_version, signature_name,
};
use ed25519_dalek::{SigningKey, VerifyingKey};
use zeroize::Zeroizing;

const USAGE: &str = "usage: avalio-lite-sign keygen <secret-file> | public <secret-file> | sign <secret-file> <asset>... | verify <asset>... | keys";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let args: Vec<&str> = args.iter().map(String::as_str).collect();
    let result = match args.as_slice() {
        ["keygen", file] => keygen(Path::new(file)),
        ["public", file] => read_key(file).map(|key| print_public(&key.verifying_key())),
        ["sign", file, assets @ ..] if !assets.is_empty() => read_key(file).and_then(|key| sign_all(&key, assets)),
        ["verify", assets @ ..] if !assets.is_empty() => verify_all(assets),
        ["keys"] => print_keys(),
        _ => Err(USAGE.to_owned()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("avalio-lite-sign: {error}");
            ExitCode::FAILURE
        }
    }
}

fn keygen(path: &Path) -> Result<(), String> {
    let mut bytes = Zeroizing::new([0u8; 32]);
    getrandom::fill(bytes.as_mut()).map_err(|error| format!("no random bytes: {error}"))?;
    // create_new: an existing key is never overwritten
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|error| format!("{}: {error}", path.display()))?;
    file.write_all(format!("{}\n", hex(bytes.as_ref())).as_bytes())
        .map_err(|error| format!("{}: {error}", path.display()))?;
    print_public(&SigningKey::from_bytes(&bytes).verifying_key());
    Ok(())
}

/// A public key in hex (what `BUILT_IN_KEYS` holds), then in minisign's form.
fn print_public(key: &VerifyingKey) {
    println!("{}", hex(key.as_bytes()));
    println!("{}", minisign::public_key_text(key));
}

/// The built-in keys in minisign's form, signing key first: what the README publishes.
fn print_keys() -> Result<(), String> {
    for key in readable_keys(&BUILT_IN_KEYS).map_err(|error| error.to_string())? {
        println!("{}", minisign::public_key_text(&key));
    }
    Ok(())
}

/// The secret key from `file`, or from the terminal or standard input for `-`.
fn read_key(file: &str) -> Result<SigningKey, String> {
    let text = Zeroizing::new(if file == "-" {
        if std::io::stdin().is_terminal() {
            rpassword::prompt_password("Release key (not shown): ")
                .map_err(|error| format!("reading the key: {error}"))?
        } else {
            let mut text = String::new();
            std::io::stdin().read_to_string(&mut text).map_err(|error| format!("reading the key: {error}"))?;
            text
        }
    } else {
        fs::read_to_string(file).map_err(|error| format!("{file}: {error}"))?
    });
    secret_key(&text).map_err(|error| error.to_string())
}

/// What an asset's signature file is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Format {
    /// `<asset>.minisig`
    Current,
    /// `<asset>.sig`, for 0.6.0 to 0.10.0
    Bridge,
}

impl Format {
    fn path(self, asset: &Path) -> PathBuf {
        let name = asset.file_name().unwrap_or_default().to_string_lossy();
        asset.with_file_name(match self {
            Self::Current => signature_name(&name),
            Self::Bridge => bridge_signature_name(&name),
        })
    }
}

/// The asset's name and format, refused unless it belongs to this checkout's version.
fn asset(path: &Path) -> Result<(String, Format), String> {
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| format!("{} has no file name", path.display()))?
        .to_owned();
    let version = current_version();
    let prefix = format!("{APP}-{version}-");
    if !name.starts_with(&prefix) {
        return Err(format!(
            "{name} isn't an asset of {APP} {version}, this checkout's version; sign from the tag's checkout"
        ));
    }
    let bridge = Form::ALL.into_iter().any(|form| bridge_asset_name(&version, form) == name);
    Ok((name, if bridge { Format::Bridge } else { Format::Current }))
}

fn sign_all(key: &SigningKey, assets: &[&str]) -> Result<(), String> {
    let version = current_version();
    // every name is checked before anything is written
    let named: Vec<(PathBuf, String, Format)> = assets
        .iter()
        .map(|file| asset(Path::new(file)).map(|(name, format)| (PathBuf::from(file), name, format)))
        .collect::<Result<_, _>>()?;
    // every signature is made before any is written, so a failure leaves nothing half done
    let mut signed = Vec::with_capacity(named.len());
    for (path, name, format) in named {
        let hashes = hashes::of_file(&path).map_err(|error| format!("{}: {error}", path.display()))?;
        let text = match format {
            Format::Current => {
                minisign::sign(key, &version, &name, &hashes).map_err(|error| format!("{name}: {error}"))?
            }
            Format::Bridge => bridge::signature_text(&bridge::sign(key, &name, &hashes.blake3), &name),
        };
        signed.push((format.path(&path), name, format, text));
    }
    for (file, name, format, text) in signed {
        fs::write(&file, text).map_err(|error| format!("{}: {error}", file.display()))?;
        println!("signed {name}{}", if format == Format::Bridge { " (bridge format)" } else { "" });
    }
    Ok(())
}

fn verify_all(assets: &[&str]) -> Result<(), String> {
    let version = current_version();
    let keys = readable_keys(&BUILT_IN_KEYS).map_err(|error| error.to_string())?;
    let mut failed = Vec::new();
    for file in assets {
        let path = Path::new(file);
        let checked = asset(path).and_then(|(name, format)| {
            let hashes = hashes::of_file(path).map_err(|error| format!("{file}: {error}"))?;
            let sig = format.path(path);
            let text = fs::read_to_string(&sig).map_err(|error| format!("{}: {error}", sig.display()))?;
            match format {
                Format::Current => minisign::verify(&keys, &text, (&version, &name), &hashes),
                Format::Bridge => bridge::verify(&bridge::bridge_key(), &name, &hashes.blake3, &text),
            }
            .map_err(|error| format!("{name}: {error}"))?;
            println!("good signature: {name}");
            Ok(())
        });
        if let Err(error) = checked {
            eprintln!("avalio-lite-sign: {error}");
            failed.push(file.to_string());
        }
    }
    if failed.is_empty() { Ok(()) } else { Err(format!("{} signature(s) didn't verify", failed.len())) }
}
