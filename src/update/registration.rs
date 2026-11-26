// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:59.111885
//+ Revised: 	2026/11/22 21:06:18.931634
// -----------------------------------------------------------------------------

//! The system's record of the installed app (Windows: the installer's uninstall entry under
//! `HKCU`). An in-app update only swaps the binary, so at the next start the new version brings the
//! record up to date, but only when the record is this copy's: a portable copy elsewhere never
//! touches it.

use std::path::Path;

/// The installer's application id (packaging/windows/avalio-lite.iss, `AppId`), which never
/// changes between versions; Inno Setup adds `_is1` to it for the uninstall key.
pub const APP_ID: &str = "{6F2B8A1E-3C4D-4E5F-9A7B-8C1D2E3F4A5B}";

/// What the installed app is listed as: the plain name, which an update doesn't make stale.
pub use crate::about::DISPLAY_NAME;

/// The uninstall key under `HKCU`.
pub fn uninstall_key() -> String {
    format!(r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{APP_ID}_is1")
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Text(String),
    Number(u32),
}

/// The uninstall entry's values; Windows' registry in the app, a map in the tests.
pub trait Record {
    /// `None` when the entry or the value isn't there.
    fn text(&self, name: &str) -> Option<String>;
    fn number(&self, name: &str) -> Option<u32>;
    fn write(&mut self, name: &str, value: &Value) -> std::io::Result<()>;
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Outcome {
    Updated,
    AlreadyRight,
    /// installed, but in another folder: this copy is a portable one
    Elsewhere,
    NotRegistered,
}

/// Brings the record to `version` if it is the record of the copy in `exe_folder`. `DisplayVersion`
/// is written last, so a partial failure is retried at the next start.
pub fn bring_up_to_date(
    record: &mut dyn Record,
    exe_folder: &Path,
    version: &semver::Version,
) -> std::io::Result<Outcome> {
    let Some(location) = record.text("InstallLocation") else { return Ok(Outcome::NotRegistered) };
    if comparable(Path::new(&location)) != comparable(exe_folder) {
        return Ok(Outcome::Elsewhere);
    }
    let wanted = [
        ("DisplayName", Value::Text(DISPLAY_NAME.to_owned())),
        ("VersionMajor", Value::Number(number(version.major))),
        ("MajorVersion", Value::Number(number(version.major))),
        ("VersionMinor", Value::Number(number(version.minor))),
        ("MinorVersion", Value::Number(number(version.minor))),
        ("DisplayVersion", Value::Text(version.to_string())),
    ];
    let current = |name: &str, value: &Value| match value {
        Value::Text(text) => record.text(name).as_deref() == Some(text.as_str()),
        Value::Number(number) => record.number(name) == Some(*number),
    };
    if wanted.iter().all(|(name, value)| current(name, value)) {
        return Ok(Outcome::AlreadyRight);
    }
    for (name, value) in &wanted {
        record.write(name, value)?;
    }
    Ok(Outcome::Updated)
}

fn number(part: u64) -> u32 {
    u32::try_from(part).unwrap_or(u32::MAX)
}

/// A folder compared ignoring letter case, `/` against `\` and trailing separators.
fn comparable(path: &Path) -> String {
    let text = path.to_string_lossy().to_lowercase().replace('/', "\\");
    text.trim_start_matches(r"\\?\").trim_end_matches('\\').to_owned()
}
