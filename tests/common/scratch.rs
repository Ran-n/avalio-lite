// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/22 21:06:23.677307
//+ Revised: 	2026/11/22 21:06:23.677307
// -----------------------------------------------------------------------------

//! Scratch folders for tests whose files can reach the screen. They live under the shared profile,
//! `%PUBLIC%\avalio-lite-tests\`, never under the user's own profile, so no path a test shows or
//! captures can name the Windows user; each folder checks that of itself when it is made.

use std::path::{Path, PathBuf};

use avalio_lite::platform::personal::names_any;
use tempfile::TempDir;

/// `%PUBLIC%\avalio-lite-tests`, created if needed.
pub fn scratch_root() -> PathBuf {
    let public = std::env::var_os("PUBLIC").expect("PUBLIC is set on every Windows account");
    let root = Path::new(&public).join("avalio-lite-tests");
    std::fs::create_dir_all(&root).unwrap_or_else(|error| panic!("{}: {error}", root.display()));
    root
}

/// A new `run-*` folder under [`scratch_root`], deleted when dropped.
pub fn scratch_dir() -> TempDir {
    scratch_dir_in(&scratch_root(), &avalio_lite::platform::personal::user_names())
}

/// A new `run-*` folder in `root`; panics if its path names any of `names`.
pub fn scratch_dir_in(root: &Path, names: &[String]) -> TempDir {
    let dir = tempfile::Builder::new()
        .prefix("run-")
        .tempdir_in(root)
        .unwrap_or_else(|error| panic!("{}: {error}", root.display()));
    assert!(
        !names_any(&dir.path().to_string_lossy(), names),
        "the test scratch folder {} names the Windows user; it must be outside the user profile",
        dir.path().display()
    );
    dir
}
