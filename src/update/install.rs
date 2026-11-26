// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.606999
//+ Revised: 	2026/11/24 17:54:06.167129
// -----------------------------------------------------------------------------

//! Putting a verified download in place of the running program, starting the new one, and
//! cleaning up what earlier updates left. Installed and portable copies are the same binary and
//! update the same way; an installed copy then fixes its uninstall entry (`registration`).

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use super::download::PREFIX;
use super::release::UpdateError;
use super::target::debug_override;
use crate::platform::storage::with_suffix;

/// Debug builds only: the download folder, so tests never write to the real cache.
pub const DIR_VARIABLE: &str = "AVALIO_LITE_UPDATE_DIR";

/// A download file younger than this may be another window's live download: the download limit
/// (30 minutes) plus 10.
pub const LEFTOVER_AGE: Duration = Duration::from_secs(40 * 60);

/// Where downloads go: `<cache dir>\Breren\avalio-lite\updates` (`%LOCALAPPDATA%` on Windows), or the
/// debug override.
pub fn download_folder() -> PathBuf {
    if let Some(folder) = debug_override(DIR_VARIABLE) {
        return PathBuf::from(folder);
    }
    dirs::cache_dir().unwrap_or_else(std::env::temp_dir).join("Breren").join("avalio-lite").join("updates")
}

/// Puts the verified `new` file in place of the program at `exe` (read before the swap: once it
/// is replaced, asking Windows where the program is gives the old file's new place). On a failure
/// the program is left as it was.
pub fn replace(exe: &Path, new: &Path) -> Result<(), UpdateError> {
    let replaced = swap(exe, new);
    // the download is no longer needed either way
    if let Err(error) = fs::remove_file(new)
        && error.kind() != io::ErrorKind::NotFound
    {
        log::warn!(target: "avalio_lite::update", "couldn't remove {}: {error}", new.display());
    }
    replaced.map_err(|error| {
        log::error!(target: "avalio_lite::update", "couldn't replace {}: {error}", exe.display());
        UpdateError::Replace {
            path: exe.display().to_string(),
            denied: error.kind() == io::ErrorKind::PermissionDenied,
        }
    })
}

/// Windows can't overwrite a running exe, but lets it be renamed: the new program is copied whole
/// beside the old one, the old one moves aside, the new one takes its name, and the old one is
/// deleted after exit. `self_replace::self_replace` isn't used for the swap: it moves the program
/// away and plans its deletion before copying, so a full disk would leave no program at all.
fn swap(exe: &Path, new: &Path) -> io::Result<()> {
    let (staged, aside) = (with_suffix(exe, ".new"), with_suffix(exe, ".old"));
    if let Err(error) = stage(new, &staged).and_then(|()| take_place(exe, &staged, &aside)) {
        if staged.exists() {
            remove(&staged);
        }
        return Err(error);
    }
    // what this can't delete, `clean_leftovers` does at the next start
    if let Err(error) = self_replace::self_delete_at(&aside) {
        log::warn!(target: "avalio_lite::update", "{} can't be deleted yet: {error}", aside.display());
    }
    Ok(())
}

/// Copies `new` to `staged`, whole and on the disk.
fn stage(new: &Path, staged: &Path) -> io::Result<()> {
    fs::copy(new, staged)?;
    fs::File::options().write(true).open(staged)?.sync_all()
}

/// Moves the program at `exe` to `aside` and `staged` to `exe`; when the second fails, the program
/// goes back.
fn take_place(exe: &Path, staged: &Path, aside: &Path) -> io::Result<()> {
    fs::rename(exe, aside)?;
    fs::rename(staged, exe).inspect_err(|_| {
        if let Err(error) = fs::rename(aside, exe) {
            log::error!(target: "avalio_lite::update", "couldn't put {} back: {error}", exe.display());
        }
    })
}

/// Starts `program` with `arguments` detached from this process (no inherited input or output; on
/// Windows its own process group), so it outlives this one, which is about to exit.
pub fn relaunch(program: &Path, arguments: &[String]) -> io::Result<()> {
    use std::process::{Command, Stdio};
    let mut command = Command::new(program);
    command.args(arguments).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const DETACHED_PROCESS: u32 = 0x0000_0008;
        const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
        command.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
    }
    command.spawn().map(drop)
}

/// At start: deletes download files (named with the prefix) in `folder` older than
/// `LEFTOVER_AGE`; other files and folders are left, and one that can't be deleted is logged and
/// skipped. Also `<exe>.old` and `<exe>.new` next to the program, left by a `replace` that couldn't
/// delete them or by the updater of 0.6.0 to 0.10.0, and that updater's folder in the temp directory.
pub fn clean_leftovers(folder: &Path, exe: Option<&Path>, now: SystemTime) {
    if let Ok(entries) = fs::read_dir(folder) {
        for entry in entries.flatten() {
            let path = entry.path();
            let ours = entry.file_name().to_string_lossy().starts_with(PREFIX);
            let old = entry
                .metadata()
                .ok()
                .filter(std::fs::Metadata::is_file)
                .and_then(|metadata| metadata.modified().ok())
                .is_some_and(|modified| now.duration_since(modified).is_ok_and(|age| age > LEFTOVER_AGE));
            if ours && old {
                remove(&path);
            }
        }
    }
    if let Some(exe) = exe {
        for file in [with_suffix(exe, ".old"), with_suffix(exe, ".new")] {
            if file.is_file() {
                remove(&file);
            }
        }
    }
    let bridge = std::env::temp_dir().join("avalio-lite-update");
    if bridge.is_dir()
        && let Err(error) = fs::remove_dir_all(&bridge)
    {
        log::warn!(target: "avalio_lite::update", "couldn't remove {} left by an earlier update: {error}", bridge.display());
    }
}

fn remove(path: &Path) {
    match fs::remove_file(path) {
        Ok(()) => log::info!(target: "avalio_lite::update", "removed {} left by an earlier update", path.display()),
        Err(error) => log::warn!(target: "avalio_lite::update", "couldn't remove {}: {error}", path.display()),
    }
}
