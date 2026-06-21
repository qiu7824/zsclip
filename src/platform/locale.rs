use std::ptr::null_mut;

use windows_sys::Win32::Globalization::{
    GetThreadPreferredUILanguages, GetUserDefaultUILanguage, GetUserPreferredUILanguages,
    LCIDToLocaleName, MUI_LANGUAGE_NAME,
};

use crate::platform::registry::{self as platform_registry, RegistryKey};

const LOCALE_NAME_CAPACITY: usize = 85;

pub(crate) fn preferred_ui_language_code() -> Option<String> {
    registry_preferred_ui_language_code()
        .or_else(|| {
            preferred_ui_language_code_from(|num, buf, size| unsafe {
                GetUserPreferredUILanguages(MUI_LANGUAGE_NAME, num, buf, size)
            })
        })
        .or_else(|| {
            preferred_ui_language_code_from(|num, buf, size| unsafe {
                GetThreadPreferredUILanguages(MUI_LANGUAGE_NAME, num, buf, size)
            })
        })
}

pub(crate) fn user_default_language_code() -> String {
    let lang = unsafe { GetUserDefaultUILanguage() };

    let mut buf = [0u16; LOCALE_NAME_CAPACITY];
    let written = unsafe { LCIDToLocaleName(lang as u32, buf.as_mut_ptr(), buf.len() as i32, 0) };
    if written > 1 {
        let locale = String::from_utf16_lossy(&buf[..(written as usize - 1)]);
        if let Some(locale) = sanitize_locale_code(&locale) {
            return locale;
        }
    }

    let primary_lang = lang & 0x03ff;
    match primary_lang {
        0x04 => "zh-CN".to_string(),
        0x09 => "en".to_string(),
        _ => "en".to_string(),
    }
}

fn preferred_ui_language_code_from<F>(mut fetch: F) -> Option<String>
where
    F: FnMut(*mut u32, *mut u16, *mut u32) -> i32,
{
    let mut num_langs = 0u32;
    let mut buffer_len = 0u32;
    if fetch(&mut num_langs, null_mut(), &mut buffer_len) == 0 || buffer_len <= 1 {
        return None;
    }

    let mut buffer = vec![0u16; buffer_len as usize];
    if fetch(&mut num_langs, buffer.as_mut_ptr(), &mut buffer_len) == 0 {
        return None;
    }

    first_locale_from_multi_sz(&buffer)
}

fn first_locale_from_multi_sz(buffer: &[u16]) -> Option<String> {
    let end = buffer.iter().position(|&ch| ch == 0)?;
    if end == 0 {
        return None;
    }
    sanitize_locale_code(&String::from_utf16_lossy(&buffer[..end]))
}

fn registry_preferred_ui_language_code() -> Option<String> {
    let key = RegistryKey::open_current_user("Control Panel\\Desktop", platform_registry::KEY_READ)
        .ok()?;
    let value = key.query_value("PreferredUILanguages").ok().flatten()?;
    if value.bytes.len() < 2 {
        return None;
    }
    let buffer = bytes_to_utf16_units(&value.bytes);
    match value.value_type {
        platform_registry::REG_MULTI_SZ => first_locale_from_multi_sz(&buffer),
        platform_registry::REG_SZ => {
            let end = buffer
                .iter()
                .position(|&ch| ch == 0)
                .unwrap_or(buffer.len());
            sanitize_locale_code(&String::from_utf16_lossy(&buffer[..end]))
        }
        _ => None,
    }
}

fn sanitize_locale_code(raw: &str) -> Option<String> {
    let trimmed = raw
        .trim_matches(char::from(0))
        .trim()
        .trim_matches(|ch| ch == '{' || ch == '}' || ch == '"')
        .replace('_', "-");
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed)
    }
}

fn bytes_to_utf16_units(bytes: &[u8]) -> Vec<u16> {
    bytes
        .chunks_exact(2)
        .map(|pair| u16::from_le_bytes([pair[0], pair[1]]))
        .collect()
}
