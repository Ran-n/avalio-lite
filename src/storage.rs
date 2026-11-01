// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.812932
//+ Revised: 	2026/10/29 15:46:51.718282
// -----------------------------------------------------------------------------

use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{AppError, Result, io_at};

/// Moves a file, also across drives: copy, then delete the original. `fs::copy` fails rather
/// than leave a partial copy, so the original is only removed once the copy is complete.
pub fn move_file(source: &Path, destination: &Path) -> Result<()> {
    if !source.is_file() {
        return Err(AppError::NotFound(source.to_path_buf()));
    }
    if destination.exists() {
        return Err(AppError::AlreadyExists(destination.to_path_buf()));
    }
    ensure_parent(destination)?;
    fs::copy(source, destination).map_err(io_at(destination))?;
    fs::remove_file(source).map_err(io_at(source))
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
