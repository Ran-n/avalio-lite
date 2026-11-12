// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.812932
//+ Revised: 	2026/11/12 14:39:53.941661
// -----------------------------------------------------------------------------

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result, io_at};

/// Moves a file, also across drives: copy, check the copy, then delete the original. When any
/// step fails the copy is removed again, so `destination` is left as it was, and the original is
/// only deleted once a complete copy exists.
pub fn move_file(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_file() {
        return Err(AppError::NotFound(source.to_path_buf()));
    }
    if destination.exists() {
        return Err(AppError::AlreadyExists(destination.to_path_buf()));
    }
    ensure_parent(destination)?;
    // `destination` did not exist a moment ago, so whatever is there after a failure is this
    // copy: a partial one, or a complete one of an original that couldn't be deleted. Either way
    // it goes, so the move failed as a whole and can simply be tried again.
    let moved = copy_complete(source, destination).and_then(|()| fs::remove_file(source).map_err(io_at(source)));
    if moved.is_err()
        && let Err(removal) = fs::remove_file(destination)
        && removal.kind() != std::io::ErrorKind::NotFound
    {
        log::error!("could not remove the copy {}: {removal}", destination.display());
    }
    moved
}

/// Copies `source` to `destination` and checks the copy is whole and on the disk, since the caller
/// deletes the original next.
fn copy_complete(source: &Path, destination: &Path) -> Result<()> {
    let expected = fs::metadata(source).map_err(io_at(source))?.len();
    fs::copy(source, destination).map_err(io_at(destination))?;
    fs::File::options().write(true).open(destination).and_then(|copy| copy.sync_all()).map_err(io_at(destination))?;
    let copied = fs::metadata(destination).map_err(io_at(destination))?.len();
    if copied != expected {
        let reason = format!("the copy has {copied} bytes, the original {expected}");
        return Err(io_at(destination)(std::io::Error::other(reason)));
    }
    Ok(())
}

/// Creates the folder a file at `path` goes in, unless it has none (a drive root, an empty path).
pub fn ensure_parent(path: &Path) -> Result<()> {
    match path.parent() {
        Some(dir) => fs::create_dir_all(dir).map_err(io_at(dir)),
        None => Ok(()),
    }
}

/// True when both paths name the same location, even if spelled differently.
pub fn same_location(first: &Path, second: &Path) -> bool {
    match (fs::canonicalize(first), fs::canonicalize(second)) {
        (Ok(first), Ok(second)) => first == second,
        _ => first == second,
    }
}

/// Added to a file's name while it is being written, before it takes its place.
const PARTIAL_SUFFIX: &str = ".partial";

/// Where a file is written before it is moved over `path`, so a failure never leaves half a
/// file there: `1A.xlsx` gives `1A.xlsx.partial`.
pub fn partial_path(path: &Path) -> PathBuf {
    with_suffix(path, PARTIAL_SUFFIX)
}

/// `path` with `suffix` added to its file name: `a.db` and `.partial` give `a.db.partial`.
pub fn with_suffix(path: &Path, suffix: &str) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(suffix);
    PathBuf::from(name)
}
