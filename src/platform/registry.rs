use std::ptr::null_mut;

use crate::platform::string::to_wide;

const HKEY_CURRENT_USER_VAL: isize = -2147483647i32 as isize;
pub(crate) const KEY_READ: u32 = 0x20019;
pub(crate) const KEY_SET_VALUE: u32 = 0x0002;
pub(crate) const KEY_CREATE_SUB_KEY: u32 = 0x0004;
pub(crate) const ERROR_FILE_NOT_FOUND: i32 = 2;

pub(crate) const REG_SZ: u32 = 1;
pub(crate) const REG_MULTI_SZ: u32 = 7;
const REG_DWORD: u32 = 4;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        hkey: isize,
        lpsubkey: *const u16,
        uloptions: u32,
        samdesired: u32,
        phkresult: *mut isize,
    ) -> i32;
    fn RegQueryValueExW(
        hkey: isize,
        lpvaluename: *const u16,
        lpreserved: *mut u32,
        lptype: *mut u32,
        lpdata: *mut u8,
        lpcbdata: *mut u32,
    ) -> i32;
    fn RegSetValueExW(
        hkey: isize,
        lpvaluename: *const u16,
        reserved: u32,
        dwtype: u32,
        lpdata: *const u8,
        cbdata: u32,
    ) -> i32;
    fn RegCreateKeyExW(
        hkey: isize,
        lpsubkey: *const u16,
        reserved: u32,
        lpclass: *mut u16,
        dwoptions: u32,
        samdesired: u32,
        lpsecurityattributes: *const core::ffi::c_void,
        phkresult: *mut isize,
        lpdwdisposition: *mut u32,
    ) -> i32;
    fn RegDeleteValueW(hkey: isize, lpvaluename: *const u16) -> i32;
    fn RegCloseKey(hkey: isize) -> i32;
}

pub(crate) struct RegistryKey {
    handle: isize,
}

pub(crate) struct RegistryValue {
    pub(crate) value_type: u32,
    pub(crate) bytes: Vec<u8>,
}

impl RegistryKey {
    pub(crate) fn open_current_user(subkey: &str, access: u32) -> Result<Self, i32> {
        let subkey = to_wide(subkey);
        let mut handle = 0isize;
        let code = unsafe {
            RegOpenKeyExW(
                HKEY_CURRENT_USER_VAL,
                subkey.as_ptr(),
                0,
                access,
                &mut handle,
            )
        };
        if code == 0 {
            Ok(Self { handle })
        } else {
            Err(code)
        }
    }

    pub(crate) fn create_current_user(subkey: &str, access: u32) -> Result<Self, i32> {
        let subkey = to_wide(subkey);
        let mut handle = 0isize;
        let mut disposition = 0u32;
        let code = unsafe {
            RegCreateKeyExW(
                HKEY_CURRENT_USER_VAL,
                subkey.as_ptr(),
                0,
                null_mut(),
                0,
                access,
                core::ptr::null(),
                &mut handle,
                &mut disposition,
            )
        };
        if code == 0 {
            Ok(Self { handle })
        } else {
            Err(code)
        }
    }

    pub(crate) fn query_string(&self, value_name: &str) -> Result<Option<String>, i32> {
        let Some(value) = self.query_value(value_name)? else {
            return Ok(None);
        };
        if value.value_type != REG_SZ || value.bytes.len() < 2 {
            return Ok(None);
        }
        let wide = bytes_to_utf16_units(&value.bytes);
        let value_len = wide.iter().position(|&ch| ch == 0).unwrap_or(wide.len());
        Ok(Some(String::from_utf16_lossy(&wide[..value_len])))
    }

    pub(crate) fn query_value(&self, value_name: &str) -> Result<Option<RegistryValue>, i32> {
        let value = to_wide(value_name);
        let mut value_type = 0u32;
        let mut size = 0u32;
        let code = unsafe {
            RegQueryValueExW(
                self.handle,
                value.as_ptr(),
                null_mut(),
                &mut value_type,
                null_mut(),
                &mut size,
            )
        };
        if code == ERROR_FILE_NOT_FOUND {
            return Ok(None);
        }
        if code != 0 {
            return Err(code);
        }
        if size == 0 {
            return Ok(Some(RegistryValue {
                value_type,
                bytes: Vec::new(),
            }));
        }
        let mut bytes = vec![0u8; size as usize];
        let code = unsafe {
            RegQueryValueExW(
                self.handle,
                value.as_ptr(),
                null_mut(),
                &mut value_type,
                bytes.as_mut_ptr(),
                &mut size,
            )
        };
        if code != 0 {
            return Err(code);
        }
        bytes.truncate(size as usize);
        Ok(Some(RegistryValue { value_type, bytes }))
    }

    pub(crate) fn set_string(&self, value_name: &str, value: &str) -> Result<(), i32> {
        let name = to_wide(value_name);
        let data = to_wide(value);
        let code = unsafe {
            RegSetValueExW(
                self.handle,
                name.as_ptr(),
                0,
                REG_SZ,
                data.as_ptr() as *const u8,
                (data.len() * 2) as u32,
            )
        };
        if code == 0 {
            Ok(())
        } else {
            Err(code)
        }
    }

    pub(crate) fn set_dword(&self, value_name: &str, value: u32) -> Result<(), i32> {
        let name = to_wide(value_name);
        let code = unsafe {
            RegSetValueExW(
                self.handle,
                name.as_ptr(),
                0,
                REG_DWORD,
                &value as *const u32 as *const u8,
                core::mem::size_of::<u32>() as u32,
            )
        };
        if code == 0 {
            Ok(())
        } else {
            Err(code)
        }
    }

    pub(crate) fn delete_value(&self, value_name: &str) -> Result<(), i32> {
        let name = to_wide(value_name);
        let code = unsafe { RegDeleteValueW(self.handle, name.as_ptr()) };
        if code == 0 {
            Ok(())
        } else {
            Err(code)
        }
    }
}

impl Drop for RegistryKey {
    fn drop(&mut self) {
        if self.handle != 0 {
            unsafe {
                RegCloseKey(self.handle);
            }
            self.handle = 0;
        }
    }
}

fn bytes_to_utf16_units(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}
