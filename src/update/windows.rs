// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.960717
//+ Revised: 	2026/11/15 12:17:27.047745
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
    [Hive::CurrentUser, Hive::LocalMachine]
        .into_iter()
        .filter_map(|hive| registry_text(hive, UNINSTALL_KEY, "InstallLocation"))
        .map(PathBuf::from)
        .collect()
}

/// The part of the registry a key is under: this user's, or the computer's.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Hive {
    CurrentUser,
    LocalMachine,
}

/// The text `value` of `key` under `hive`; `None` when there is no such value, or (logged) when it
/// can't be read or isn't text.
pub fn registry_text(hive: Hive, key: &str, value: &str) -> Option<String> {
    use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
    use windows_sys::Win32::System::Registry::{HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, RRF_RT_REG_SZ, RegGetValueW};
    let root = match hive {
        Hive::CurrentUser => HKEY_CURRENT_USER,
        Hive::LocalMachine => HKEY_LOCAL_MACHINE,
    };
    let wide = |text: &str| text.encode_utf16().chain(Some(0)).collect::<Vec<u16>>();
    let (key_name, value_name) = (key, value);
    let (key, value) = (wide(key), wide(value));
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
        log::warn!("couldn't read {value_name} of the registry key {key_name} (error {status})");
        return None;
    }
    let length = buffer.iter().position(|&unit| unit == 0).unwrap_or(buffer.len());
    Some(String::from_utf16_lossy(&buffer[..length]))
}

/// Starts what is pending so it outlives this process, which is about to exit. The installer waited
/// in a temp folder since it was verified, so it only runs if it is still the file that was.
pub fn start(pending: &Pending) -> std::io::Result<()> {
    match pending {
        Pending::RunInstaller { installer, hash } => {
            if super::signature::file_hash(installer)? != *hash {
                return Err(std::io::Error::other(format!("{} changed after it was verified", installer.display())));
            }
            start_detached(installer, INSTALLER_ARGUMENTS)
        }
        Pending::StartNew(exe) => start_detached(exe, &[]),
    }
}

fn start_detached(program: &Path, arguments: &[&str]) -> std::io::Result<()> {
    const DETACHED_PROCESS: u32 = 0x0000_0008;
    const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
    Command::new(program).args(arguments).creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP).spawn().map(drop)
}
