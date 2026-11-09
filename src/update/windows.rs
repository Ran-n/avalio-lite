// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.960717
//+ Revised: 	2026/11/07 20:25:34.960717
// -----------------------------------------------------------------------------

//! What updating needs from Windows: where the installer put the app, and starting the new
//! installer or exe so it outlives this process.

use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;

use super::install::{Copy, Installation, Pending, copy_kind};

/// The installer's uninstall entry (`installer/avalio-lite.iss`, AppId), per user or for all users.
const UNINSTALL_KEY: &str =
    r"Software\Microsoft\Windows\CurrentVersion\Uninstall\{6F2B8A1E-3C4D-4E5F-9A7B-8C1D2E3F4A5B}_is1";

/// The installer's flags for an update: no wizard but a progress window, no questions, close a
/// copy still exiting, and open the app again afterwards (`/RELAUNCH=1`, see the installer).
pub const INSTALLER_ARGUMENTS: &[&str] =
    &["/SILENT", "/SUPPRESSMSGBOXES", "/NORESTART", "/CLOSEAPPLICATIONS", "/RELAUNCH=1"];

/// The running exe and how it was installed.
pub fn this_installation() -> Option<Installation> {
    let exe = match std::env::current_exe() {
        Ok(exe) => exe,
        Err(error) => {
            log::warn!("couldn't tell where the app runs from, so it can't update itself: {error}");
            return None;
        }
    };
    let copy = copy_kind(&exe, &install_locations());
    log::info!("this copy is {}", if copy == Copy::Installed { "installed" } else { "portable" });
    Some(Installation { exe, copy })
}

/// Where the installer put the app, per user and for all users; a key that can't be read counts
/// as not installed there (logged).
pub fn install_locations() -> Vec<PathBuf> {
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE};
    [HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE].into_iter().filter_map(install_location).collect()
}

fn install_location(root: windows_sys::Win32::System::Registry::HKEY) -> Option<PathBuf> {
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{RRF_RT_REG_SZ, RegGetValueW};
    let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (key, value) = (wide(UNINSTALL_KEY), wide("InstallLocation"));
    let mut buffer = vec![0u16; 1024];
    let mut size = (buffer.len() * 2) as u32;
    // SAFETY: NUL-terminated key and value names, and a buffer whose size in bytes is passed.
    let status = unsafe {
        RegGetValueW(
            root,
            key.as_ptr(),
            value.as_ptr(),
            RRF_RT_REG_SZ,
            std::ptr::null_mut(),
            buffer.as_mut_ptr().cast(),
            &mut size,
        )
    };
    if status == ERROR_FILE_NOT_FOUND {
        return None;
    }
    if status != ERROR_SUCCESS {
        log::warn!("couldn't read where the installer put the app (error {status})");
        return None;
    }
    let length = buffer.iter().position(|&unit| unit == 0).unwrap_or(buffer.len());
    Some(PathBuf::from(String::from_utf16_lossy(&buffer[..length])))
}

/// Starts what is pending so it outlives this process, which is about to exit.
pub fn start(pending: &Pending) -> std::io::Result<()> {
    match pending {
        Pending::RunInstaller(installer) => start_detached(installer, INSTALLER_ARGUMENTS),
        Pending::StartNew(exe) => start_detached(exe, &[]),
    }
}

fn start_detached(program: &Path, arguments: &[&str]) -> std::io::Result<()> {
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    Command::new(program).args(arguments).creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP).spawn().map(drop)
}
