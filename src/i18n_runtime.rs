use std::borrow::Cow;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::OnceLock;

#[cfg(target_os = "windows")]
use crate::platform::locale as platform_locale;

type TranslationMap = HashMap<String, String>;

static LANGUAGE_CODE: OnceLock<String> = OnceLock::new();
static ACTIVE_TRANSLATIONS: OnceLock<TranslationMap> = OnceLock::new();

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
    #[cfg(target_os = "windows")]
    {
        if let Some(locale) = platform_locale::preferred_ui_language_code() {
            return locale;
        }

        return platform_locale::user_default_language_code();
    }

    #[cfg(not(target_os = "windows"))]
    {
        ["LC_ALL", "LC_MESSAGES", "LANG"]
            .into_iter()
            .find_map(|key| std::env::var(key).ok())
            .map(|value| value.split('.').next().unwrap_or(&value).replace('_', "-"))
            .filter(|value| !value.trim().is_empty())
            .unwrap_or_else(|| "en".to_string())
    }
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
