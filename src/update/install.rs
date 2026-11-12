// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.606999
//+ Revised: 	2026/11/12 14:39:55.383558
// -----------------------------------------------------------------------------

//! Putting a downloaded, verified update in place: which file this copy needs (the installer for an
//! installed copy, the exe for a portable one), swapping the portable exe, and what is left to
//! start once the app has shut down.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use super::{Asset, Release, UpdateError};
use crate::platform::storage::with_suffix;

/// How this copy of the app was put on the computer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Copy {
    /// by the installer: updated by running the new installer
    Installed,
    /// the portable exe at this path: updated by swapping the file
    Portable(PathBuf),
}

/// The running exe and how it was installed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Installation {
    pub exe: PathBuf,
    pub copy: Copy,
}

/// Installed when the exe's folder is one the installer recorded (any letter case, with or
/// without a trailing separator); portable otherwise.
pub fn copy_kind(exe: &Path, install_locations: &[PathBuf]) -> Copy {
    let folder = exe.parent().map(comparable);
    if folder.is_some_and(|folder| install_locations.iter().any(|location| comparable(location) == folder)) {
        Copy::Installed
    } else {
        Copy::Portable(exe.to_path_buf())
    }
}

fn comparable(path: &Path) -> String {
    let canonical = fs::canonicalize(path).unwrap_or_else(|_| path.to_path_buf());
    let text = canonical.to_string_lossy().to_lowercase().replace('/', "\\");
    text.trim_start_matches(r"\\?\").trim_end_matches('\\').to_owned()
}

/// The file this copy updates from, and its signature.
pub fn asset_for<'a>(release: &'a Release, copy: &Copy) -> Result<(&'a Asset, &'a Asset), UpdateError> {
    let kind = match copy {
        Copy::Installed => "setup",
        Copy::Portable(_) => "portable",
    };
    let name = format!("avalio-lite-{}-{kind}.exe", release.version);
    let find = |name: &str| {
        release.assets.iter().find(|asset| asset.name == name).ok_or_else(|| UpdateError::MissingAsset(name.to_owned()))
    };
    Ok((find(&name)?, find(&format!("{name}.sig"))?))
}

/// Whether files can be created in `folder`: a portable copy on a locked USB stick can't update.
pub fn writable(folder: &Path) -> io::Result<()> {
    let probe = folder.join("avalio-lite.write-test");
    fs::write(&probe, b"")?;
    fs::remove_file(&probe)
}

/// `<exe>.new`, where a portable copy downloads its update.
pub fn new_exe(exe: &Path) -> PathBuf {
    with_suffix(exe, ".new")
}

/// `<exe>.old`, where the replaced exe waits until the next start deletes it.
pub fn old_exe(exe: &Path) -> PathBuf {
    with_suffix(exe, ".old")
}

/// Puts `<exe>.new` in the running exe's place, keeping the old one as `<exe>.old`. Windows lets a
/// running exe be renamed, not overwritten. If the second rename fails, the first is undone; if
/// that fails too, the error says where the old exe is, since the app's shortcut no longer finds it.
pub fn swap_portable(exe: &Path) -> Result<(), UpdateError> {
    swap_portable_with(exe, |from, to| fs::rename(from, to))
}

/// `swap_portable` with the renaming done by `rename`, so the tests can make a rename fail.
#[doc(hidden)]
pub fn swap_portable_with(
    exe: &Path,
    mut rename: impl FnMut(&Path, &Path) -> io::Result<()>,
) -> Result<(), UpdateError> {
    let (new, old) = (new_exe(exe), old_exe(exe));
    if old.exists() {
        fs::remove_file(&old).map_err(|error| UpdateError::Swap(format!("{}: {error}", old.display())))?;
    }
    rename(exe, &old).map_err(|error| UpdateError::Swap(format!("{}: {error}", exe.display())))?;
    if let Err(error) = rename(&new, exe) {
        let detail = format!("{}: {error}", new.display());
        if let Err(undo) = rename(&old, exe) {
            log::error!("couldn't put {} back after a failed update: {undo}", exe.display());
            return Err(UpdateError::SwapLeftAside { old, detail });
        }
        return Err(UpdateError::Swap(detail));
    }
    Ok(())
}

/// What to start once the app has shut down.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Pending {
    /// the new installer, run silently; it opens the app again. `hash` is its BLAKE3 hash when it
    /// was verified, checked again just before it runs.
    RunInstaller { installer: PathBuf, hash: String },
    /// the new portable exe, already in place
    StartNew(PathBuf),
}

/// The folder an installed copy downloads its installer to.
pub fn download_folder() -> PathBuf {
    std::env::temp_dir().join("avalio-lite-update")
}

/// Removes what an earlier update left: the old portable exe, a half-downloaded one, and the
/// downloaded installer. A failure is logged; the next start tries again.
pub fn clean_leftovers(exe: &Path, folder: &Path) {
    for file in [old_exe(exe), new_exe(exe)] {
        if file.exists()
            && let Err(error) = fs::remove_file(&file)
        {
            log::warn!("couldn't remove {} left by an update: {error}", file.display());
        }
    }
    if folder.exists()
        && let Err(error) = fs::remove_dir_all(folder)
    {
        log::warn!("couldn't remove {} left by an update: {error}", folder.display());
    }
}
