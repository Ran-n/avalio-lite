// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/15 17:41:58.862962
//+ Revised: 	2026/11/23 13:00:37.990041
// -----------------------------------------------------------------------------

//! For tests/update/: replaces its own binary with the file named by its first argument, as the
//! updater does, and exits. The test runs a scratch copy of it, never anything it can't lose.

fn main() -> std::process::ExitCode {
    let Some(new) = std::env::args().nth(1) else { return std::process::ExitCode::FAILURE };
    let exe = std::env::current_exe().expect("the program knows where it is");
    match avalio_lite::update::install::replace(&exe, std::path::Path::new(&new)) {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            std::process::ExitCode::FAILURE
        }
    }
}
