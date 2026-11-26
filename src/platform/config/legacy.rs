// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 11:49:16.150653
//+ Revised: 	2026/11/23 11:49:16.150653
// -----------------------------------------------------------------------------

//! Moving the settings of 0.10.0 and earlier, from their own folder to the publisher's folder. The
//! copy is checked before the original is deleted, and the original stays where it is when the copy
//! fails.

use std::fs;
use std::path::{Path, PathBuf};

use super::CONFIG_FILE_NAME;
use super::store::write_flushed;

/// Moves the settings of 0.10.0 and earlier from `legacy` to `dir`: copy, check the copy, then
/// delete the original (and `legacy` itself, once empty). Returns the folder to use: `dir`, or
/// `legacy` when the move failed (logged), so the settings are never lost on the way. Nothing is
/// moved when `dir` already has a settings file or `legacy` has none.
pub fn move_legacy_settings(legacy: &Path, dir: &Path) -> PathBuf {
    let (old, new) = (legacy.join(CONFIG_FILE_NAME), dir.join(CONFIG_FILE_NAME));
    if new.exists() || !old.is_file() {
        return dir.to_path_buf();
    }
    if let Err(error) = copy_checked(&old, &new, dir) {
        log::error!(
            "couldn't move the settings from {} to {}: {error}; using them where they are",
            old.display(),
            new.display()
        );
        remove_half_made(&new);
        return legacy.to_path_buf();
    }
    log::info!("moved the settings from {} to {}", old.display(), new.display());
    // a copy kept from a settings file that once failed to load goes along with it
    move_bad_copy(legacy, dir);
    if let Err(error) = fs::remove_file(&old) {
        log::warn!("couldn't remove the old settings {} after moving them: {error}", old.display());
    }
    // only when nothing else is left in it
    if fs::remove_dir(legacy).is_err() {
        log::info!("{} isn't empty, so it stays", legacy.display());
    }
    dir.to_path_buf()
}

/// Copies the settings file `old` to `new` in `dir` (made when missing), then reads the copy back
/// and checks it holds the same bytes.
fn copy_checked(old: &Path, new: &Path, dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(dir)?;
    let bytes = fs::read(old)?;
    write_flushed(new, &bytes)?;
    if fs::read(new)? != bytes {
        return Err(std::io::Error::other("the copy differs from the original"));
    }
    Ok(())
}

/// Deletes the half-made copy left by a failed `copy_checked`; a failure is logged.
fn remove_half_made(new: &Path) {
    if new.exists()
        && let Err(error) = fs::remove_file(new)
    {
        log::warn!("couldn't remove the half-made copy {}: {error}", new.display());
    }
}

/// Moves the kept bad copy (`config.toml.bad`) from `legacy` to `dir`, unless `dir` already has one.
/// A failure is logged; the copy then stays in `legacy`.
fn move_bad_copy(legacy: &Path, dir: &Path) {
    let (from, to) = (bad_copy(legacy), bad_copy(dir));
    if from.is_file()
        && !to.exists()
        && let Err(error) = fs::copy(&from, &to).and_then(|_| fs::remove_file(&from))
    {
        log::warn!("couldn't move {}: {error}", from.display());
    }
}

fn bad_copy(folder: &Path) -> PathBuf {
    folder.join(format!("{CONFIG_FILE_NAME}.bad"))
}
