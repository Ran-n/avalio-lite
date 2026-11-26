// -----------------------------------------------------------------------------
//+ Authors: 	Ran# <ran.hash@proton.me>
//+ Created: 	2026/11/07 20:25:34.960717
//+ Revised: 	2026/11/15 17:42:01.032100
// -----------------------------------------------------------------------------

//! What updating needs from Windows: the installer's uninstall entry under `HKCU`, read and
//! written for `registration`.

use super::registration::{Record, Value, uninstall_key};

use windows_sys::Win32::Foundation::{ERROR_FILE_NOT_FOUND, ERROR_SUCCESS};
use windows_sys::Win32::System::Registry::{
    HKEY, HKEY_CURRENT_USER, KEY_SET_VALUE, REG_DWORD, REG_SZ, RRF_RT_REG_DWORD, RRF_RT_REG_SZ, RegCloseKey,
    RegGetValueW, RegOpenKeyExW, RegSetValueExW,
};

fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(Some(0)).collect()
}

/// The uninstall entry of this user's installation (`HKCU`, the installer installs per user).
pub struct UninstallEntry {
    key: String,
}

impl Default for UninstallEntry {
    fn default() -> Self {
        Self { key: uninstall_key() }
    }
}

impl UninstallEntry {
    /// Another key under `HKCU`, for the tests.
    pub fn at(key: &str) -> Self {
        Self { key: key.to_owned() }
    }

    /// The raw value into `buffer`; `None` when the key or the value isn't there, or (logged) when
    /// it can't be read.
    fn get(&self, name: &str, kind: u32, buffer: &mut [u8]) -> Option<usize> {
        let (key, value) = (wide(&self.key), wide(name));
        let mut size = buffer.len() as u32;
        // SAFETY: NUL-terminated key and value names, and a buffer whose size in bytes is passed.
        let status = unsafe {
            RegGetValueW(
                HKEY_CURRENT_USER,
                key.as_ptr(),
                value.as_ptr(),
                kind,
                std::ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if status == ERROR_FILE_NOT_FOUND {
            return None;
        }
        if status != ERROR_SUCCESS {
            log::warn!(target: "avalio_lite::update", "couldn't read {name} of {} (error {status})", self.key);
            return None;
        }
        Some(size as usize)
    }
}

impl Record for UninstallEntry {
    fn text(&self, name: &str) -> Option<String> {
        let mut buffer = vec![0u8; 2048];
        let size = self.get(name, RRF_RT_REG_SZ, &mut buffer)?;
        let units: Vec<u16> = buffer[..size].as_chunks::<2>().0.iter().map(|pair| u16::from_le_bytes(*pair)).collect();
        let length = units.iter().position(|&unit| unit == 0).unwrap_or(units.len());
        Some(String::from_utf16_lossy(&units[..length]))
    }

    fn number(&self, name: &str) -> Option<u32> {
        let mut buffer = [0u8; 4];
        self.get(name, RRF_RT_REG_DWORD, &mut buffer)?;
        Some(u32::from_le_bytes(buffer))
    }

    fn write(&mut self, name: &str, value: &Value) -> std::io::Result<()> {
        let key = wide(&self.key);
        let mut handle: HKEY = std::ptr::null_mut();
        // SAFETY: a NUL-terminated key name and a handle to fill in.
        let opened = unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, key.as_ptr(), 0, KEY_SET_VALUE, &mut handle) };
        if opened != ERROR_SUCCESS {
            return Err(std::io::Error::from_raw_os_error(opened as i32));
        }
        let name_wide = wide(name);
        let (kind, data): (u32, Vec<u8>) = match value {
            Value::Text(text) => (REG_SZ, wide(text).iter().flat_map(|unit| unit.to_le_bytes()).collect()),
            Value::Number(number) => (REG_DWORD, number.to_le_bytes().to_vec()),
        };
        // SAFETY: an open key, a NUL-terminated value name, and data whose length in bytes is passed.
        let status = unsafe { RegSetValueExW(handle, name_wide.as_ptr(), 0, kind, data.as_ptr(), data.len() as u32) };
        // SAFETY: the key opened above.
        unsafe { RegCloseKey(handle) };
        if status == ERROR_SUCCESS { Ok(()) } else { Err(std::io::Error::from_raw_os_error(status as i32)) }
    }
}
