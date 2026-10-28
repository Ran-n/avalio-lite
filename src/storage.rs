// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.812932
//+ Revised: 	2026/10/28 15:53:08.326490
// -----------------------------------------------------------------------------

use std::fs;
use std::path::Path;

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
    if let Some(dir) = destination.parent() {
        fs::create_dir_all(dir).map_err(io_at(dir))?;
    }
    fs::copy(source, destination).map_err(io_at(destination))?;
    fs::remove_file(source).map_err(io_at(source))
}

/// True when both paths name the same location, even if spelled differently.
pub fn same_location(first: &Path, second: &Path) -> bool {
    match (fs::canonicalize(first), fs::canonicalize(second)) {
        (Ok(first), Ok(second)) => first == second,
        _ => first == second,
    }
}
