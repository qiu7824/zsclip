use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::ptr::null_mut;
use std::sync::OnceLock;

use windows_sys::Win32::Globalization::{
    GetThreadPreferredUILanguages, GetUserDefaultUILanguage, GetUserPreferredUILanguages,
    LCIDToLocaleName, MUI_LANGUAGE_NAME,
};

type TranslationMap = HashMap<String, String>;

static LANGUAGE_CODE: OnceLock<String> = OnceLock::new();
static ACTIVE_TRANSLATIONS: OnceLock<TranslationMap> = OnceLock::new();
const LOCALE_NAME_CAPACITY: usize = 85;
const HKEY_CURRENT_USER_VAL: isize = -2147483647i32 as isize;
const KEY_READ_VAL: u32 = 0x20019;
const REG_SZ_VAL: u32 = 1;
const REG_MULTI_SZ_VAL: u32 = 7;

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
    fn RegCloseKey(hkey: isize) -> i32;
}

pub fn current_language_code() -> &'static str {
    LANGUAGE_CODE
        .get_or_init(detect_system_language_code)
        .as_str()
}

pub fn is_source_language() -> bool {
    current_language_code().starts_with("zh")
}

pub fn tr(source: &'static str, fallback_en: &'static str) -> &'static str {
    if is_source_language() {
        return source;
    }

    active_translations()
        .get(source)
        .map(String::as_str)
        .unwrap_or(fallback_en)
}

pub fn app_title() -> &'static str {
    tr("剪贴板", "Clipboard")
}

pub fn translate<'a>(text: &'a str) -> Cow<'a, str> {
    if is_source_language() {
        return Cow::Borrowed(text);
    }

    active_translations()
        .get(text)
        .map(|value| Cow::Borrowed(value.as_str()))
        .unwrap_or_else(|| Cow::Borrowed(text))
}

fn active_translations() -> &'static TranslationMap {
    ACTIVE_TRANSLATIONS.get_or_init(load_active_translations)
}

fn load_active_translations() -> TranslationMap {
    if is_source_language() {
        return TranslationMap::new();
    }

    for code in translation_search_codes(current_language_code()) {
        if let Some(map) =
            load_translation_from_disk(&code).or_else(|| load_embedded_translation(&code))
        {
            return map;
        }
    }

    TranslationMap::new()
}

fn translation_search_codes(code: &str) -> Vec<String> {
    let mut codes = Vec::new();
    push_unique_code(&mut codes, code);
    if let Some(base) = code.split('-').next() {
        push_unique_code(&mut codes, base);
    }
    push_unique_code(&mut codes, "en");
    codes
}

fn push_unique_code(codes: &mut Vec<String>, code: &str) {
    if !code.is_empty() && !codes.iter().any(|item| item == code) {
        codes.push(code.to_string());
    }
}

fn load_translation_from_disk(code: &str) -> Option<TranslationMap> {
    translation_file_candidates(code)
        .into_iter()
        .find_map(|path| {
            fs::read_to_string(path)
                .ok()
                .and_then(|text| parse_translation_map(&text).ok())
        })
}

fn translation_file_candidates(code: &str) -> Vec<PathBuf> {
    let file_name = format!("{code}.json");
    let mut paths = Vec::new();

    if let Ok(exe_path) = std::env::current_exe() {
        if let Some(dir) = exe_path.parent() {
            paths.push(dir.join("locales").join(&file_name));
        }
    }

    if let Ok(current_dir) = std::env::current_dir() {
        paths.push(current_dir.join("locales").join(&file_name));
    }

    paths
}

fn load_embedded_translation(code: &str) -> Option<TranslationMap> {
    let text = match code {
        "en" => include_str!("../locales/en.json"),
        _ => return None,
    };
    parse_translation_map(text).ok()
}

fn detect_system_language_code() -> String {
    if let Some(locale) = preferred_ui_language_code() {
        return locale;
    }

    let lang = unsafe { GetUserDefaultUILanguage() };

    let mut buf = [0u16; LOCALE_NAME_CAPACITY];
    let written = unsafe { LCIDToLocaleName(lang as u32, buf.as_mut_ptr(), buf.len() as i32, 0) };
    if written > 1 {
        let locale = String::from_utf16_lossy(&buf[..(written as usize - 1)]);
        if !locale.is_empty() {
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

fn preferred_ui_language_code() -> Option<String> {
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
    let key = to_wide("Control Panel\\Desktop");
    let value = to_wide("PreferredUILanguages");
    let mut hkey = 0isize;
    if unsafe {
        RegOpenKeyExW(
            HKEY_CURRENT_USER_VAL,
            key.as_ptr(),
            0,
            KEY_READ_VAL,
            &mut hkey,
        )
    } != 0
    {
        return None;
    }

    let mut reg_type = 0u32;
    let mut byte_len = 0u32;
    let query_len = unsafe {
        RegQueryValueExW(
            hkey,
            value.as_ptr(),
            null_mut(),
            &mut reg_type,
            null_mut(),
            &mut byte_len,
        )
    };
    if query_len != 0 || byte_len < 2 {
        unsafe {
            RegCloseKey(hkey);
        }
        return None;
    }

    let mut buffer = vec![0u16; (byte_len as usize / 2) + 1];
    let query_value = unsafe {
        RegQueryValueExW(
            hkey,
            value.as_ptr(),
            null_mut(),
            &mut reg_type,
            buffer.as_mut_ptr() as *mut u8,
            &mut byte_len,
        )
    };
    unsafe {
        RegCloseKey(hkey);
    }
    if query_value != 0 {
        return None;
    }

    match reg_type {
        REG_MULTI_SZ_VAL => first_locale_from_multi_sz(&buffer),
        REG_SZ_VAL => {
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

fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn parse_translation_map(text: &str) -> Result<TranslationMap, serde_json::Error> {
    serde_json::from_str(text.trim_start_matches('\u{feff}'))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_english_translations_load() {
        let map = parse_translation_map(include_str!("../locales/en.json"))
            .unwrap_or_else(|err| panic!("embedded en.json parse failed: {err}"));
        assert_eq!(map.get("关闭").map(String::as_str), Some("Close"));
        assert_eq!(map.get("保存").map(String::as_str), Some("Save"));
    }

    #[test]
    fn translation_search_includes_base_language() {
        let codes = translation_search_codes("en-GB");
        assert!(codes.iter().any(|code| code == "en"));
    }
}
