// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/23 13:00:37.414225
//+ Revised: 	2026/11/23 13:00:37.414225
// -----------------------------------------------------------------------------

//! The app while it runs: shutdown releases its files and it keeps working after a restart.

use crate::common::Fixture;
use avalio_lite::platform::config::LOG_FILE_NAME;
use std::fs;

#[test]
fn shutdown_releases_database_and_log_files() {
    let fixture = Fixture::new();
    let mut state = fixture.set_up();
    let config = state.config().unwrap().clone();
    state.shutdown();
    assert!(state.connection().is_none());
    // Windows refuses to delete files that are still open, so this proves both were released.
    avalio_lite::platform::db::remove_files(&config.database_path).unwrap();
    fs::remove_file(config.log_dir.join(LOG_FILE_NAME)).unwrap();
}
