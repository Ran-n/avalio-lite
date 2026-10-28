// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/10/27 22:32:30.675075
//+ Revised: 	2026/10/28 10:52:41.641492
// -----------------------------------------------------------------------------

// Release builds are GUI-only; debug builds keep the console so stderr logging is visible.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() -> std::process::ExitCode {
    avalio_lite::app::run()
}
