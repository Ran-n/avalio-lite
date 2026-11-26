// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.675075
//+ Revised: 	2026/11/23 16:34:46.133433
// -----------------------------------------------------------------------------

//! The exe's entry point: `--version`, or the app itself. Both are in `app`.

// Release builds are GUI-only; debug builds keep the console so stderr logging is visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    // `avalio-lite --version` prints `avalio lite X.Y.Z` and opens no window
    if std::env::args().nth(1).as_deref() == Some("--version") {
        return avalio_lite::app::print_version();
    }
    avalio_lite::app::run()
}
