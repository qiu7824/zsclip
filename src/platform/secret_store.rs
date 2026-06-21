#[cfg(windows)]
mod imp {
    use std::ptr::{null, null_mut};

    const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

    #[repr(C)]
    struct DataBlob {
        cb_data: u32,
        pb_data: *mut u8,
    }

    #[link(name = "crypt32")]
    unsafe extern "system" {
        fn CryptProtectData(
            p_data_in: *const DataBlob,
            sz_data_descr: *const u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *mut core::ffi::c_void,
            p_prompt_struct: *mut core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;
        fn CryptUnprotectData(
            p_data_in: *const DataBlob,
            ppsz_data_descr: *mut *mut u16,
            p_optional_entropy: *const DataBlob,
            pv_reserved: *mut core::ffi::c_void,
            p_prompt_struct: *mut core::ffi::c_void,
            dw_flags: u32,
            p_data_out: *mut DataBlob,
        ) -> i32;
    }

    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn LocalFree(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    }

    pub(crate) fn protect(bytes: &[u8]) -> Option<Vec<u8>> {
        unsafe { protect_bytes(bytes) }
    }

    pub(crate) fn unprotect(bytes: &[u8]) -> Option<Vec<u8>> {
        unsafe { unprotect_bytes(bytes) }
    }

    unsafe fn protect_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
        let input = DataBlob {
            cb_data: bytes.len() as u32,
            pb_data: bytes.as_ptr() as *mut u8,
        };
        let mut output = DataBlob {
            cb_data: 0,
            pb_data: null_mut(),
        };
        let ok = CryptProtectData(
            &input,
            null(),
            null(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );
        if ok == 0 || output.pb_data.is_null() {
            return None;
        }
        let result = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec();
        let _ = LocalFree(output.pb_data as *mut core::ffi::c_void);
        Some(result)
    }

    unsafe fn unprotect_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
        let input = DataBlob {
            cb_data: bytes.len() as u32,
            pb_data: bytes.as_ptr() as *mut u8,
        };
        let mut output = DataBlob {
            cb_data: 0,
            pb_data: null_mut(),
        };
        let ok = CryptUnprotectData(
            &input,
            null_mut(),
            null(),
            null_mut(),
            null_mut(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        );
        if ok == 0 || output.pb_data.is_null() {
            return None;
        }
        let result = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec();
        let _ = LocalFree(output.pb_data as *mut core::ffi::c_void);
        Some(result)
    }
}

#[cfg(not(windows))]
mod imp {
    pub(crate) fn protect(bytes: &[u8]) -> Option<Vec<u8>> {
        Some(bytes.to_vec())
    }

    pub(crate) fn unprotect(bytes: &[u8]) -> Option<Vec<u8>> {
        Some(bytes.to_vec())
    }
}

pub(crate) fn encrypt_secret_for_storage(secret: &str) -> Option<String> {
    if secret.is_empty() {
        return Some(String::new());
    }
    imp::protect(secret.as_bytes()).map(|bytes| hex_encode(&bytes))
}

pub(crate) fn decrypt_secret_from_storage(encoded: &str) -> Option<String> {
    if encoded.trim().is_empty() {
        return Some(String::new());
    }
    let raw = hex_decode(encoded)?;
    imp::unprotect(&raw).and_then(|bytes| String::from_utf8(bytes).ok())
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    fn nibble(ch: u8) -> Option<u8> {
        match ch {
            b'0'..=b'9' => Some(ch - b'0'),
            b'a'..=b'f' => Some(ch - b'a' + 10),
            b'A'..=b'F' => Some(ch - b'A' + 10),
            _ => None,
        }
    }
    let bytes = text.as_bytes();
    if bytes.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut idx = 0;
    while idx < bytes.len() {
        let hi = nibble(bytes[idx])?;
        let lo = nibble(bytes[idx + 1])?;
        out.push((hi << 4) | lo);
        idx += 2;
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_hex_secret_is_rejected() {
        assert_eq!(decrypt_secret_from_storage("not-hex"), None);
        assert_eq!(decrypt_secret_from_storage("abc"), None);
    }

    #[test]
    fn empty_secret_round_trips_without_platform_call() {
        assert_eq!(encrypt_secret_for_storage(""), Some(String::new()));
        assert_eq!(decrypt_secret_from_storage(""), Some(String::new()));
    }
}
