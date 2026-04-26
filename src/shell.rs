use std::collections::{BTreeSet, HashMap};
use std::ffi::{c_void, CStr, CString, OsStr};
use std::io::Write;
use std::os::windows::ffi::OsStrExt;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::ptr::null_mut;
use std::sync::{Condvar, Mutex, OnceLock};
use std::time::Duration;

use base64::Engine;
use windows_sys::Win32::{
    System::LibraryLoader::{GetProcAddress, LoadLibraryW},
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::{CreateIconFromResourceEx, LR_DEFAULTCOLOR, SW_SHOWNORMAL},
    },
};

use crate::app::{ClipItem, ClipKind, Icons};
use crate::i18n::tr;
use crate::win_system_ui::to_wide;

const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;
const HKEY_CURRENT_USER_VAL: isize = -2147483647i32 as isize;
const KEY_READ_VAL: u32 = 0x20019;
const KEY_SET_VALUE_VAL: u32 = 0x0002;
const KEY_CREATE_SUB_KEY_VAL: u32 = 0x0004;
const REG_SZ_VAL: u32 = 1;
const REG_DWORD_VAL: u32 = 4;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const DISABLED_HOTKEYS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const DISABLED_HOTKEYS_VALUE: &str = "DisabledHotkeys";
const CLIPBOARD_SETTINGS_KEY: &str = r"Software\Microsoft\Clipboard";
const CLIPBOARD_HISTORY_VALUE: &str = "EnableClipboardHistory";
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

#[link(name = "kernel32")]
unsafe extern "system" {
    fn FreeLibrary(hlibmodule: *mut c_void) -> i32;
}

#[link(name = "winmm")]
unsafe extern "system" {
    fn PlaySoundW(pszsound: *const u16, hmod: isize, fdwsound: u32) -> i32;
}

const SND_ASYNC: u32 = 0x0001;
const SND_FILENAME: u32 = 0x00020000;
const SND_NODEFAULT: u32 = 0x0002;
const SND_MEMORY: u32 = 0x0004;

#[derive(Default, Clone)]
pub(crate) struct UpdateCheckState {
    pub(crate) started: bool,
    pub(crate) checking: bool,
    pub(crate) available: bool,
    pub(crate) latest_tag: String,
    pub(crate) latest_url: String,
    pub(crate) error: String,
}

static UPDATE_CHECK_STATE: OnceLock<Mutex<UpdateCheckState>> = OnceLock::new();
static ICON_HANDLE_CACHE: OnceLock<Mutex<HashMap<(u8, i32), isize>>> = OnceLock::new();
static BAIDU_OCR_TOKEN_CACHE: OnceLock<Mutex<HashMap<String, (String, u64)>>> = OnceLock::new();
static WECHAT_OCR_CALLBACK: OnceLock<(Mutex<Option<String>>, Condvar)> = OnceLock::new();
static WECHAT_RUNTIME_DIR_CACHE: OnceLock<Mutex<Option<PathBuf>>> = OnceLock::new();
static ICO_SEARCH: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_SETTING: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_MIN: OnceLock<Vec<u8>> = OnceLock::new();
static PASTE_SOUND_DEFAULT: &[u8] = include_bytes!("../assets/sounds/paste_default.wav");
static PASTE_SOUND_SOFT: &[u8] = include_bytes!("../assets/sounds/paste_soft.wav");
static PASTE_SOUND_BRIGHT: &[u8] = include_bytes!("../assets/sounds/paste_bright.wav");
static EMBEDDED_WCOCR_DLL: &[u8] = include_bytes!("../assets/plugin/wcocr.dll");
static ICO_EXIT: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_TEXT: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_IMAGE: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_FILE: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_FOLDER: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_TOP: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_DEL: OnceLock<Vec<u8>> = OnceLock::new();

fn update_check_state() -> &'static Mutex<UpdateCheckState> {
    UPDATE_CHECK_STATE.get_or_init(|| Mutex::new(UpdateCheckState::default()))
}

fn baidu_ocr_token_cache() -> &'static Mutex<HashMap<String, (String, u64)>> {
    BAIDU_OCR_TOKEN_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

fn wechat_ocr_callback_state() -> &'static (Mutex<Option<String>>, Condvar) {
    WECHAT_OCR_CALLBACK.get_or_init(|| (Mutex::new(None), Condvar::new()))
}

fn wechat_runtime_dir_cache() -> &'static Mutex<Option<PathBuf>> {
    WECHAT_RUNTIME_DIR_CACHE.get_or_init(|| Mutex::new(None))
}

fn icon_handle_cache() -> &'static Mutex<HashMap<(u8, i32), isize>> {
    ICON_HANDLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

macro_rules! icon_png_pack {
    ($dir:literal, $name:literal) => {
        build_ico_from_png_entries(&[
            (
                16,
                include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_16x16.png"))
                    as &[u8],
            ),
            (
                24,
                include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_24x24.png"))
                    as &[u8],
            ),
            (
                32,
                include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_32x32.png"))
                    as &[u8],
            ),
            (
                48,
                include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_48x48.png"))
                    as &[u8],
            ),
            (
                64,
                include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_64x64.png"))
                    as &[u8],
            ),
            (
                128,
                include_bytes!(concat!(
                    "../assets/icons/",
                    $dir,
                    "/",
                    $name,
                    "_128x128.png"
                )) as &[u8],
            ),
            (
                0,
                include_bytes!(concat!(
                    "../assets/icons/",
                    $dir,
                    "/",
                    $name,
                    "_256x256.png"
                )) as &[u8],
            ),
        ])
    };
}

static ICO_APP: &[u8] = include_bytes!("../assets/icons/icon.ico");

#[derive(Copy, Clone, Debug, Eq, PartialEq, Hash)]
#[repr(u8)]
pub(crate) enum IconAssetKind {
    App = 0,
    Search = 1,
    Setting = 2,
    Min = 3,
    Close = 4,
    Text = 5,
    Image = 6,
    File = 7,
    Folder = 8,
    Pin = 9,
    Delete = 10,
}

pub(crate) unsafe fn open_path_with_shell(path: &str) {
    if let Some((scheme, _)) = path.split_once("://") {
        let scheme = scheme.trim().to_ascii_lowercase();
        if scheme != "http" && scheme != "https" {
            return;
        }
    }
    let op = to_wide("open");
    let wp = to_wide(path);
    ShellExecuteW(
        std::ptr::null_mut(),
        op.as_ptr(),
        wp.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
        SW_SHOWNORMAL,
    );
}

pub(crate) unsafe fn open_parent_folder(path: &str) {
    let p = Path::new(path);
    if p.is_dir() {
        open_path_with_shell(path);
    } else if let Some(parent) = p.parent() {
        if let Some(s) = parent.to_str() {
            open_path_with_shell(s);
        }
    }
}

pub(crate) fn hidden_command(program: &str) -> Command {
    let mut cmd = Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW_FLAG);
    cmd
}

fn hidden_command_path(program: &Path) -> Command {
    let mut cmd = Command::new(program);
    cmd.creation_flags(CREATE_NO_WINDOW_FLAG);
    cmd
}

fn app_exe_dir() -> Option<PathBuf> {
    std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(|p| p.to_path_buf()))
}

fn is_wechat_runtime_dir(path: &Path) -> bool {
    path.join("mmmojo.dll").is_file()
        || path.join("mmmojo_64.dll").is_file()
        || path.join("xwechatwin.dll").is_file()
        || path.join("Weixin.dll").is_file()
        || path.join("WeChatOcr.bin").is_file()
        || path.join("XNet.dll").is_file()
}

fn version_score(path: &Path) -> Vec<u32> {
    path.file_name()
        .and_then(|s| s.to_str())
        .map(|name| {
            name.split('.')
                .filter_map(|part| part.parse::<u32>().ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn embedded_wcocr_extract_path() -> Option<PathBuf> {
    let base = std::env::temp_dir().join("zsclip").join("plugin");
    std::fs::create_dir_all(&base).ok()?;
    let hash = format!("{:x}", md5::compute(EMBEDDED_WCOCR_DLL));
    let dll_path = base.join(format!("wcocr-{}.dll", &hash[..8]));
    if !dll_path.is_file() {
        std::fs::write(&dll_path, EMBEDDED_WCOCR_DLL).ok()?;
    }
    Some(dll_path)
}

fn runtime_dir_from_candidate(path: &Path) -> Option<PathBuf> {
    if !path.exists() {
        return None;
    }
    let direct = if path.is_file() {
        path.parent().map(Path::to_path_buf)
    } else {
        Some(path.to_path_buf())
    }?;
    for candidate in [
        direct.clone(),
        direct
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| direct.clone()),
    ] {
        if is_wechat_runtime_dir(&candidate) {
            return Some(candidate);
        }
        let mut version_dirs = Vec::new();
        if let Ok(entries) = std::fs::read_dir(&candidate) {
            for entry in entries.filter_map(|e| e.ok()) {
                let child = entry.path();
                if child.is_dir() && is_wechat_runtime_dir(&child) {
                    version_dirs.push(child);
                }
            }
        }
        version_dirs.sort_by_key(|path| std::cmp::Reverse(version_score(path)));
        if let Some(best) = version_dirs.into_iter().next() {
            return Some(best);
        }
    }
    None
}

fn cached_wechat_runtime_dir() -> Option<PathBuf> {
    let path = wechat_runtime_dir_cache().lock().ok()?.clone()?;
    runtime_dir_from_candidate(&path)
}

fn cache_wechat_runtime_dir(path: &Path) {
    if let Some(runtime) = runtime_dir_from_candidate(path) {
        if let Ok(mut slot) = wechat_runtime_dir_cache().lock() {
            *slot = Some(runtime);
        }
    }
}

fn resolve_wechat_runtime_dir(path_override: &str) -> Option<PathBuf> {
    let trimmed = path_override.trim();
    if !trimmed.is_empty() {
        if let Some(runtime) = runtime_dir_from_candidate(Path::new(trimmed)) {
            cache_wechat_runtime_dir(&runtime);
            return Some(runtime);
        }
    }
    if let Some(runtime) = cached_wechat_runtime_dir() {
        return Some(runtime);
    }
    let runtime = find_wechat_runtime_dir()?;
    cache_wechat_runtime_dir(&runtime);
    Some(runtime)
}

fn resolve_wechat_runtime_dir_candidates(path_override: &str) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    let mut push_unique = |path: PathBuf| {
        if !candidates.iter().any(|existing| existing == &path) {
            candidates.push(path);
        }
    };
    let trimmed = path_override.trim();
    if !trimmed.is_empty() {
        let input = PathBuf::from(trimmed);
        if let Some(runtime) = runtime_dir_from_candidate(&input) {
            push_unique(runtime);
            return candidates;
        }
    }
    if let Some(runtime) = cached_wechat_runtime_dir() {
        push_unique(runtime);
    }
    if let Some(runtime) = find_wechat_runtime_dir() {
        push_unique(runtime);
    }
    candidates
}

pub(crate) fn detect_wechat_runtime_dir(current: &str) -> Option<String> {
    resolve_wechat_runtime_dir(current).map(|p| p.to_string_lossy().into_owned())
}

fn find_wcocr_wrapper_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(app_dir) = app_exe_dir() {
        candidates.push(app_dir.join("wcocr.dll"));
        candidates.push(app_dir.join("plugins").join("wcocr.dll"));
    }
    for plugin_root in current_user_wechat_ocr_plugin_roots() {
        if let Ok(entries) = std::fs::read_dir(&plugin_root) {
            for entry in entries.filter_map(|e| e.ok()) {
                let extracted = entry.path().join("extracted");
                let dll = extracted.join("wcocr.dll");
                if dll.is_file() {
                    candidates.push(dll);
                }
            }
        }
    }
    candidates
        .into_iter()
        .find(|path| path.is_file())
        .or_else(embedded_wcocr_extract_path)
}

fn current_user_wechat_ocr_plugin_roots() -> Vec<PathBuf> {
    let mut roots = Vec::new();
    if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
        roots.push(appdata.join("Tencent\\xwechat\\xplugin\\Plugins\\WeChatOcr"));
        roots.push(appdata.join("Tencent\\WeChat\\XPlugin\\Plugins\\WeChatOCR"));
    }
    if let Some(profile) = std::env::var_os("USERPROFILE").map(PathBuf::from) {
        roots.push(profile.join("AppData\\Roaming\\Tencent\\xwechat\\xplugin\\Plugins\\WeChatOcr"));
        roots.push(profile.join("AppData\\Roaming\\Tencent\\WeChat\\XPlugin\\Plugins\\WeChatOCR"));
    }
    roots.sort();
    roots.dedup();
    roots
}

fn parse_numeric_dir_name(path: &Path) -> u64 {
    path.file_name()
        .and_then(|name| name.to_str())
        .and_then(|name| name.parse::<u64>().ok())
        .unwrap_or(0)
}

fn find_wechat_ocr_binary_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    for plugin_root in current_user_wechat_ocr_plugin_roots() {
        if let Ok(entries) = std::fs::read_dir(&plugin_root) {
            for entry in entries.filter_map(|e| e.ok()) {
                let extracted = entry.path().join("extracted");
                let dll = extracted.join("wxocr.dll");
                let exe = extracted.join("WeChatOCR.exe");
                if dll.is_file() {
                    candidates.push((parse_numeric_dir_name(&entry.path()), dll));
                } else if exe.is_file() {
                    candidates.push((parse_numeric_dir_name(&entry.path()), exe));
                }
            }
        }
    }
    candidates.sort_by(|a, b| b.0.cmp(&a.0));
    candidates.into_iter().map(|(_, path)| path).next()
}

fn find_wechat_runtime_dir() -> Option<PathBuf> {
    fn push_runtime(candidates: &mut Vec<PathBuf>, path: &Path) {
        if let Some(runtime) = runtime_dir_from_candidate(path) {
            candidates.push(runtime);
        }
    }

    fn running_wechat_install_dirs() -> Vec<PathBuf> {
        let mut dirs = Vec::new();
        let output = hidden_command("powershell.exe")
            .args([
                "-NoProfile",
                "-Command",
                "(Get-Process Weixin -ErrorAction SilentlyContinue | Select-Object -First 8 -ExpandProperty Path)",
            ])
            .output();
        if let Ok(output) = output {
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                for line in stdout.lines().map(str::trim).filter(|s| !s.is_empty()) {
                    let path = PathBuf::from(line);
                    if let Some(parent) = path.parent() {
                        dirs.push(parent.to_path_buf());
                    }
                }
            }
        }
        dirs
    }

    let mut roots = BTreeSet::new();
    for path in [
        std::env::var_os("ProgramFiles").map(PathBuf::from),
        std::env::var_os("ProgramW6432").map(PathBuf::from),
        std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
    ]
    .into_iter()
    .flatten()
    {
        roots.insert(path);
    }
    for drive in ["C", "D", "E", "F", "G", "H"] {
        roots.insert(PathBuf::from(format!("{drive}:\\Program Files")));
        roots.insert(PathBuf::from(format!("{drive}:\\Program Files (x86)")));
    }
    let mut candidates = Vec::new();
    for root in &roots {
        if !root.is_dir() {
            continue;
        }

        // Common install roots first.
        for rel in [
            PathBuf::from("Tencent\\Weixin"),
            PathBuf::from("Tencent\\WeChat"),
            PathBuf::from("Weixin"),
            PathBuf::from("WeChat"),
        ] {
            push_runtime(&mut candidates, &root.join(rel));
        }

        // Scan Tencent subdirs one level deep.
        let tencent_root = root.join("Tencent");
        if let Ok(entries) = std::fs::read_dir(&tencent_root) {
            for entry in entries.filter_map(|e| e.ok()) {
                let path = entry.path();
                if !path.is_dir() {
                    continue;
                }
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or_default()
                    .to_ascii_lowercase();
                if name.contains("weixin") || name.contains("wechat") {
                    push_runtime(&mut candidates, &path);
                }
            }
        }
    }
    for dir in running_wechat_install_dirs() {
        push_runtime(&mut candidates, &dir);
    }
    candidates.sort_by(|a, b| {
        version_score(b)
            .cmp(&version_score(a))
            .then_with(|| a.cmp(b))
    });
    candidates.dedup();
    candidates.into_iter().next()
}

pub(crate) fn image_ocr_status_text(
    provider: &str,
    primary: &str,
    secondary: &str,
    wechat_dir: &str,
) -> String {
    match provider {
        "baidu" => {
            if primary.trim().is_empty() || secondary.trim().is_empty() {
                tr(
                    "百度 OCR：未配置 API Key / Secret Key",
                    "Baidu OCR: API Key / Secret Key not configured",
                )
                .to_string()
            } else {
                tr("百度 OCR：已配置", "Baidu OCR: configured").to_string()
            }
        }
        "winocr" => {
            let wrapper = find_wcocr_wrapper_path();
            let ocr_bin = find_wechat_ocr_binary_path();
            let runtime = resolve_wechat_runtime_dir(wechat_dir);
            match (wrapper, ocr_bin, runtime) {
                (Some(wrapper), Some(ocr_bin), Some(runtime)) => format!(
                    "{} {} / {} / {}",
                    tr("WinOCR：已就绪", "WinOCR: ready"),
                    wrapper
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("wcocr.dll"),
                    ocr_bin
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("wxocr.dll"),
                    runtime
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("Weixin")
                ),
                (None, Some(_), _) => tr(
                    "WinOCR：已找到 wxocr.dll，但缺少兼容的 wcocr.dll 桥接层",
                    "WinOCR: wxocr.dll found, but compatible wcocr.dll bridge is missing",
                )
                .to_string(),
                (None, _, _) => tr(
                    "WinOCR：未找到兼容的 wcocr.dll",
                    "WinOCR: compatible wcocr.dll not found",
                )
                .to_string(),
                (_, None, _) => tr(
                    "WinOCR：未找到微信 OCR 插件",
                    "WinOCR: WeChat OCR plugin not found",
                )
                .to_string(),
                (_, _, None) => tr(
                    "WinOCR：未找到微信运行时目录",
                    "WinOCR: WeChat runtime directory not found",
                )
                .to_string(),
            }
        }
        _ => tr(
            "\u{56fe}\u{7247} OCR\u{ff1a}\u{5df2}\u{5173}\u{95ed}",
            "Image OCR: disabled",
        )
        .to_string(),
    }
}

fn baidu_translate_target_name(key: &str) -> &'static str {
    match key {
        "en" => tr("英语", "English"),
        "jp" => tr("日语", "Japanese"),
        "kor" => tr("韩语", "Korean"),
        _ => tr("简体中文", "Simplified Chinese"),
    }
}

pub(crate) fn text_translate_status_text(
    provider: &str,
    app_id: &str,
    secret: &str,
    target_lang: &str,
) -> String {
    match provider {
        "baidu" => {
            if app_id.trim().is_empty() || secret.trim().is_empty() {
                tr(
                    "百度翻译：请配置 APP ID / 密钥",
                    "Baidu Translate: please configure APP ID / Secret",
                )
                .to_string()
            } else {
                format!(
                    "{} {}",
                    tr(
                        "百度翻译：已就绪，目标语言：",
                        "Baidu Translate: ready, target language: "
                    ),
                    baidu_translate_target_name(target_lang),
                )
            }
        }
        _ => tr("文本翻译：已关闭", "Text translation: disabled").to_string(),
    }
}

fn url_encode_form_component(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 16);
    for b in input.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            out.push('%');
            out.push_str(&format!("{:02X}", b));
        }
    }
    out
}

fn temp_unique_path(prefix: &str, ext: &str) -> PathBuf {
    let ts = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "zsclip_{}_{}_{}.{}",
        prefix,
        std::process::id(),
        ts,
        ext.trim_start_matches('.')
    ))
}

fn curl_config_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn curl_config_option(name: &str, value: &str) -> String {
    format!("{} = {}\n", name, curl_config_quote(value))
}

fn run_curl_config(config: &str, prefix: &str) -> Result<Output, String> {
    let config_path = temp_unique_path(prefix, "curl");
    std::fs::write(&config_path, config).map_err(|e| e.to_string())?;
    let config_arg = config_path.to_string_lossy().to_string();
    let output = hidden_command("curl.exe")
        .args(["--config", config_arg.as_str()])
        .output()
        .map_err(|e| e.to_string());
    let _ = std::fs::remove_file(&config_path);
    output
}

fn parse_baidu_access_token(body: &str) -> Result<(String, u64), String> {
    let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    let token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            tr(
                "百度 OCR access_token 返回无效",
                "Baidu OCR access_token response is invalid",
            )
            .to_string()
        })?;
    let expires_in = json
        .get("expires_in")
        .and_then(|v| v.as_u64())
        .unwrap_or(2_592_000);
    Ok((token, expires_in))
}

fn cached_baidu_access_token(api_key: &str, secret_key: &str) -> Option<String> {
    let cache_key = format!("{}\0{}", api_key.trim(), secret_key.trim());
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    baidu_ocr_token_cache()
        .lock()
        .ok()
        .and_then(|cache| cache.get(&cache_key).cloned())
        .and_then(|(token, expires_at)| {
            if expires_at > now + 60 {
                Some(token)
            } else {
                None
            }
        })
}

fn fetch_baidu_access_token(api_key: &str, secret_key: &str) -> Result<String, String> {
    if let Some(token) = cached_baidu_access_token(api_key, secret_key) {
        return Ok(token);
    }
    let api_key = api_key.trim();
    let secret_key = secret_key.trim();
    if api_key.is_empty() || secret_key.is_empty() {
        return Err(tr(
            "请先在设置-插件中配置百度 OCR 的 API Key / Secret Key",
            "Please configure the Baidu OCR API Key / Secret Key in Settings > Plugins",
        )
        .to_string());
    }
    let body = format!(
        "grant_type=client_credentials&client_id={}&client_secret={}",
        url_encode_form_component(api_key),
        url_encode_form_component(secret_key),
    );
    let body_path = temp_unique_path("baidu_token_body", "txt");
    std::fs::write(&body_path, body).map_err(|e| e.to_string())?;
    let mut config = String::from("silent\nshow-error\nlocation\n");
    config.push_str(&curl_config_option("request", "POST"));
    config.push_str(&curl_config_option(
        "url",
        "https://aip.baidubce.com/oauth/2.0/token",
    ));
    config.push_str(&curl_config_option(
        "header",
        "Content-Type: application/x-www-form-urlencoded",
    ));
    config.push_str(&curl_config_option(
        "data-binary",
        &format!("@{}", body_path.to_string_lossy()),
    ));
    let output = run_curl_config(&config, "baidu_token");
    let _ = std::fs::remove_file(&body_path);
    let output = output?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            tr(
                "百度 OCR 获取 access_token 失败",
                "Failed to obtain Baidu OCR access_token",
            )
            .to_string()
        } else {
            stderr
        });
    }
    let (token, expires_in) = parse_baidu_access_token(&String::from_utf8_lossy(&output.stdout))?;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    if let Ok(mut cache) = baidu_ocr_token_cache().lock() {
        cache.insert(
            format!("{}\0{}", api_key, secret_key),
            (token.clone(), now + expires_in),
        );
    }
    Ok(token)
}

pub(crate) fn run_baidu_ocr_api(
    api_key: &str,
    secret_key: &str,
    image_bytes: &[u8],
) -> Result<String, String> {
    let access_token = fetch_baidu_access_token(api_key, secret_key)?;
    let form_body = format!(
        "image={}",
        url_encode_form_component(&base64::engine::general_purpose::STANDARD.encode(image_bytes))
    );
    let request_path = temp_unique_path("baidu_ocr_body", "txt");
    std::fs::write(&request_path, form_body).map_err(|e| e.to_string())?;
    let request_url = format!(
        "https://aip.baidubce.com/rest/2.0/ocr/v1/general_basic?access_token={}",
        access_token
    );
    let mut config = String::from("silent\nshow-error\nlocation\n");
    config.push_str(&curl_config_option("request", "POST"));
    config.push_str(&curl_config_option("url", &request_url));
    config.push_str(&curl_config_option(
        "header",
        "Content-Type: application/x-www-form-urlencoded",
    ));
    config.push_str(&curl_config_option(
        "data-binary",
        &format!("@{}", request_path.to_string_lossy()),
    ));
    let output = run_curl_config(&config, "baidu_ocr");
    let _ = std::fs::remove_file(&request_path);
    let output = output?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            tr("百度 OCR 请求失败", "Baidu OCR request failed").to_string()
        } else {
            stderr
        });
    }
    parse_baidu_ocr_text(&String::from_utf8_lossy(&output.stdout))
}

// ── Structured OCR result (with bounding boxes) ─────────────────────────────

#[derive(Clone, Default)]
pub(crate) struct OcrLine {
    pub text: String,
    pub left: u32,
    pub top: u32,
    pub width: u32,
    pub height: u32,
}

fn parse_baidu_ocr_lines(body: &str) -> Result<Vec<OcrLine>, String> {
    let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if let Some(err) = json
        .get("error_msg")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Err(err.to_string());
    }
    let lines: Vec<OcrLine> = json
        .get("words_result")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    let text = item
                        .get("words")
                        .and_then(|v| v.as_str())
                        .map(str::trim)
                        .filter(|s| !s.is_empty())?;
                    let loc = item.get("location")?;
                    let left = loc.get("left").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let top = loc.get("top").and_then(|v| v.as_u64()).unwrap_or(0) as u32;
                    let width = loc.get("width").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                    let height = loc.get("height").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
                    Some(OcrLine {
                        text: text.to_string(),
                        left,
                        top,
                        width,
                        height,
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    if lines.is_empty() {
        Err(tr(
            "百度 OCR 返回中未找到可用文本字段",
            "Baidu OCR response does not contain recognized text",
        )
        .to_string())
    } else {
        Ok(lines)
    }
}

/// Like `run_baidu_ocr_api` but returns per-line text with bounding boxes.
pub(crate) fn run_baidu_ocr_api_lines(
    api_key: &str,
    secret_key: &str,
    image_bytes: &[u8],
) -> Result<Vec<OcrLine>, String> {
    let access_token = fetch_baidu_access_token(api_key, secret_key)?;
    let form_body = format!(
        "image={}",
        url_encode_form_component(&base64::engine::general_purpose::STANDARD.encode(image_bytes))
    );
    let request_path = temp_unique_path("baidu_ocr_lines_body", "txt");
    std::fs::write(&request_path, &form_body).map_err(|e| e.to_string())?;
    let request_url = format!(
        "https://aip.baidubce.com/rest/2.0/ocr/v1/general?access_token={}",
        access_token
    );
    let mut config = String::from("silent\nshow-error\nlocation\n");
    config.push_str(&curl_config_option("request", "POST"));
    config.push_str(&curl_config_option("url", &request_url));
    config.push_str(&curl_config_option(
        "header",
        "Content-Type: application/x-www-form-urlencoded",
    ));
    config.push_str(&curl_config_option(
        "data-binary",
        &format!("@{}", request_path.to_string_lossy()),
    ));
    let output = run_curl_config(&config, "baidu_ocr_lines");
    let _ = std::fs::remove_file(&request_path);
    let output = output?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            tr("百度 OCR 请求失败", "Baidu OCR request failed").to_string()
        } else {
            stderr
        });
    }
    parse_baidu_ocr_lines(&String::from_utf8_lossy(&output.stdout))
}

fn parse_baidu_ocr_text(body: &str) -> Result<String, String> {
    let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if let Some(err_msg) = json
        .get("error_msg")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Err(err_msg.to_string());
    }
    let lines: Vec<String> = json
        .get("words_result")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("words").and_then(|w| w.as_str()))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if lines.is_empty() {
        return Err(tr(
            "百度 OCR 返回中未找到可用文本字段",
            "Baidu OCR response does not contain recognized text",
        )
        .to_string());
    }
    Ok(lines.join("\r\n"))
}

fn parse_baidu_translate_text(body: &str) -> Result<String, String> {
    let json: serde_json::Value = serde_json::from_str(body).map_err(|e| e.to_string())?;
    if let Some(err_msg) = json
        .get("error_msg")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        return Err(err_msg.to_string());
    }
    let lines: Vec<String> = json
        .get("trans_result")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.get("dst").and_then(|w| w.as_str()))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if lines.is_empty() {
        return Err(tr(
            "百度翻译返回中未找到可用文本字段",
            "Baidu Translate response does not contain translated text",
        )
        .to_string());
    }
    Ok(lines.join("\r\n"))
}

pub(crate) fn run_baidu_translate_api(
    app_id: &str,
    secret: &str,
    text: &str,
    target_lang: &str,
) -> Result<String, String> {
    let app_id = app_id.trim();
    let secret = secret.trim();
    let text = text.trim();
    let target_lang = target_lang.trim();
    if app_id.is_empty() || secret.is_empty() {
        return Err(tr(
            "请先在设置-插件中配置百度翻译的 APP ID / 密钥",
            "Please configure the Baidu Translate APP ID / Secret in Settings > Plugins",
        )
        .to_string());
    }
    if text.is_empty() {
        return Err(tr(
            "当前记录没有可翻译的文本",
            "This item does not contain translatable text",
        )
        .to_string());
    }
    let salt = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis().to_string())
        .unwrap_or_else(|_| format!("{}", std::process::id()));
    let sign = format!(
        "{:x}",
        md5::compute(format!("{}{}{}{}", app_id, text, salt, secret))
    );
    let body = format!(
        "q={}&from=auto&to={}&appid={}&salt={}&sign={}",
        url_encode_form_component(text),
        url_encode_form_component(if target_lang.is_empty() {
            "zh"
        } else {
            target_lang
        }),
        url_encode_form_component(app_id),
        url_encode_form_component(&salt),
        sign
    );
    let body_path = temp_unique_path("baidu_translate_body", "txt");
    std::fs::write(&body_path, body).map_err(|e| e.to_string())?;
    let mut config = String::from("silent\nshow-error\nlocation\n");
    config.push_str(&curl_config_option("request", "POST"));
    config.push_str(&curl_config_option(
        "url",
        "https://fanyi-api.baidu.com/api/trans/vip/translate",
    ));
    config.push_str(&curl_config_option(
        "header",
        "Content-Type: application/x-www-form-urlencoded",
    ));
    config.push_str(&curl_config_option(
        "data-binary",
        &format!("@{}", body_path.to_string_lossy()),
    ));
    let output = run_curl_config(&config, "baidu_translate");
    let _ = std::fs::remove_file(&body_path);
    let output = output?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            tr("百度翻译请求失败", "Baidu Translate request failed").to_string()
        } else {
            stderr
        });
    }
    parse_baidu_translate_text(&String::from_utf8_lossy(&output.stdout))
}

type WeChatOcrFn =
    unsafe extern "C" fn(*const u16, *const u16, *const i8, extern "C" fn(*const i8)) -> bool;
type WeChatStopOcrFn = unsafe extern "C" fn() -> i32;

unsafe fn winocr_get_proc<T: Sized>(module: *mut c_void, names: &[&[u8]]) -> Option<T> {
    for name in names {
        let ptr = GetProcAddress(module, name.as_ptr());
        if let Some(proc) = ptr {
            return Some(std::mem::transmute_copy(&proc));
        }
    }
    None
}

extern "C" fn wechat_ocr_callback(ptr: *const i8) {
    if ptr.is_null() {
        return;
    }
    let text = unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned();
    let (lock, cvar) = wechat_ocr_callback_state();
    if let Ok(mut slot) = lock.lock() {
        *slot = Some(text);
        cvar.notify_all();
    }
}

fn parse_wechat_ocr_json(payload: &str) -> Result<String, String> {
    let json: serde_json::Value =
        serde_json::from_str(payload).map_err(|_| payload.trim().to_string())?;
    let errcode = json.get("errcode").and_then(|v| v.as_i64()).unwrap_or(0);
    if errcode != 0 {
        if let Some(err_msg) = json
            .get("msg")
            .and_then(|v| v.as_str())
            .filter(|s| !s.trim().is_empty())
        {
            return Err(err_msg.to_string());
        }
        return Err(format!("errcode={}", errcode));
    }
    let lines = json
        .get("ocr_response")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|row| row.get("text").and_then(|v| v.as_str()))
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if lines.is_empty() {
        return Err(tr(
            "WinOCR 未识别到文字",
            "WinOCR did not return any recognized text",
        )
        .to_string());
    }
    Ok(lines.join("\r\n"))
}

pub(crate) fn run_winocr_dll_ocr(image_path: &Path, wechat_dir: &str) -> Result<String, String> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let output = hidden_command_path(&exe)
        .arg("--wechat-ocr-helper")
        .arg(image_path.as_os_str())
        .arg(wechat_dir)
        .output()
        .map_err(|e| e.to_string())?;
    if output.status.success() {
        let text = String::from_utf8_lossy(&output.stdout).into_owned();
        let trimmed = text.trim_matches(['\r', '\n']);
        if trimmed.is_empty() {
            Err(tr("WinOCR 未返回结果", "WinOCR did not return a result").to_string())
        } else {
            Ok(trimmed.to_string())
        }
    } else {
        let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if err.is_empty() {
            Err(tr("WinOCR 子进程执行失败", "WinOCR helper process failed").to_string())
        } else {
            Err(err)
        }
    }
}

fn run_wechat_wcocr_with_runtime(image_path: &Path, wechat_dir: &str) -> Result<String, String> {
    let wrapper_path = find_wcocr_wrapper_path().ok_or_else(|| {
        tr(
            "WinOCR：未找到兼容的 wcocr.dll",
            "WinOCR: compatible wcocr.dll not found",
        )
        .to_string()
    })?;
    let ocr_bin = find_wechat_ocr_binary_path().ok_or_else(|| {
        tr(
            "WinOCR：未找到微信 OCR 插件",
            "WinOCR: WeChat OCR plugin not found",
        )
        .to_string()
    })?;
    let runtime_dirs = resolve_wechat_runtime_dir_candidates(wechat_dir);
    if runtime_dirs.is_empty() {
        return Err(tr(
            "WinOCR：未找到微信运行时目录",
            "WinOCR: WeChat runtime directory not found",
        )
        .to_string());
    }
    let wrapper_wide = to_wide(&wrapper_path.to_string_lossy());
    let ocr_bin_wide = to_wide(&ocr_bin.to_string_lossy());
    let image_c =
        CString::new(image_path.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
    unsafe {
        let module = LoadLibraryW(wrapper_wide.as_ptr());
        if module.is_null() {
            return Err(tr("WinOCR DLL 加载失败", "Failed to load WinOCR DLL").to_string());
        }
        let wechat_ocr: WeChatOcrFn = match winocr_get_proc(module, &[b"wechat_ocr\0"]) {
            Some(func) => func,
            None => {
                FreeLibrary(module);
                return Err(tr(
                    "WinOCR DLL 未导出可用的 OCR 接口",
                    "WinOCR DLL does not export a supported OCR entry point",
                )
                .to_string());
            }
        };
        let stop_ocr: Option<WeChatStopOcrFn> = winocr_get_proc(module, &[b"stop_ocr\0"]);
        let (lock, cvar) = wechat_ocr_callback_state();
        if let Ok(mut slot) = lock.lock() {
            *slot = None;
        }
        let mut last_rc = None;
        let mut last_timeout = false;
        for runtime_dir in runtime_dirs {
            if let Ok(mut slot) = lock.lock() {
                *slot = None;
            }
            let runtime_wide = to_wide(&runtime_dir.to_string_lossy());
            let ok = wechat_ocr(
                ocr_bin_wide.as_ptr(),
                runtime_wide.as_ptr(),
                image_c.as_ptr(),
                wechat_ocr_callback,
            );
            if !ok {
                last_rc = Some((1, runtime_dir));
                continue;
            }
            let received = {
                let guard = lock.lock().map_err(|_| {
                    tr("WinOCR 回调等待失败", "WinOCR callback wait failed").to_string()
                })?;
                let (mut guard, timeout) = cvar
                    .wait_timeout_while(guard, Duration::from_secs(15), |slot| slot.is_none())
                    .map_err(|_| {
                        tr("WinOCR 回调等待失败", "WinOCR callback wait failed").to_string()
                    })?;
                let value = guard.take();
                (value, timeout.timed_out())
            };
            match received {
                (Some(payload), _) => {
                    if let Some(stop) = stop_ocr {
                        let _ = stop();
                    }
                    FreeLibrary(module);
                    return parse_wechat_ocr_json(&payload);
                }
                (None, true) => {
                    last_timeout = true;
                }
                (None, false) => {
                    last_rc = Some((0, runtime_dir));
                }
            }
        }
        if let Some(stop) = stop_ocr {
            let _ = stop();
        }
        FreeLibrary(module);
        if last_timeout {
            Err(tr("WinOCR 识别超时", "WinOCR recognition timed out").to_string())
        } else if let Some((rc, runtime_dir)) = last_rc {
            Err(format!(
                "{} ({}) [{}]",
                tr("WinOCR 识别失败", "WinOCR recognition failed"),
                rc,
                runtime_dir.to_string_lossy()
            ))
        } else {
            Err(tr("WinOCR 未返回结果", "WinOCR did not return a result").to_string())
        }
    }
}

pub(crate) fn maybe_run_wechat_ocr_helper_from_args() -> Option<i32> {
    let mut args = std::env::args_os();
    let _exe = args.next();
    let mode = args.next()?;
    if mode != OsStr::new("--wechat-ocr-helper") {
        return None;
    }
    let image_path = args.next().map(PathBuf::from);
    let wechat_dir = args.next().unwrap_or_default();
    let result = match image_path {
        Some(path) => run_wechat_wcocr_with_runtime(&path, &wechat_dir.to_string_lossy()),
        None => Err(tr(
            "WinOCR 缺少图片路径参数",
            "WinOCR helper missing image path argument",
        )
        .to_string()),
    };
    match result {
        Ok(text) => {
            let mut stdout = std::io::stdout().lock();
            let _ = stdout.write_all(text.as_bytes());
            let _ = stdout.flush();
            Some(0)
        }
        Err(err) => {
            let mut stderr = std::io::stderr().lock();
            let _ = stderr.write_all(err.as_bytes());
            let _ = stderr.flush();
            Some(1)
        }
    }
}

pub(crate) fn open_source_url() -> &'static str {
    option_env!("CARGO_PKG_REPOSITORY").unwrap_or("")
}

pub(crate) fn open_source_url_display() -> &'static str {
    if open_source_url().trim().is_empty() {
        tr(
            "\u{672a}\u{914d}\u{7f6e}\u{ff08}\u{53ef}\u{5728} Cargo.toml \u{7684} package.repository \u{4e2d}\u{914d}\u{7f6e}\u{ff09}",
            "Not configured (set package.repository in Cargo.toml)",
        )
    } else {
        open_source_url()
    }
}

pub(crate) fn latest_release_url() -> String {
    format!("{}/latest", releases_url())
}

pub(crate) fn update_check_state_snapshot() -> UpdateCheckState {
    update_check_state()
        .lock()
        .ok()
        .map(|guard| guard.clone())
        .unwrap_or_default()
}

pub(crate) fn update_check_latest_url_or_default() -> String {
    update_check_state()
        .lock()
        .ok()
        .and_then(|state| {
            if !state.latest_url.trim().is_empty() {
                Some(state.latest_url.clone())
            } else {
                None
            }
        })
        .unwrap_or_else(latest_release_url)
}

pub(crate) fn update_check_available() -> bool {
    update_check_state()
        .lock()
        .ok()
        .map(|state| state.available)
        .unwrap_or(false)
}

pub(crate) fn start_update_check<F>(notify: F)
where
    F: FnOnce() + Send + 'static,
{
    let should_start = {
        let Ok(mut state) = update_check_state().lock() else {
            return;
        };
        if state.checking || open_source_url().trim().is_empty() {
            false
        } else {
            state.started = true;
            state.checking = true;
            state.error.clear();
            true
        }
    };
    if !should_start {
        return;
    }

    std::thread::spawn(move || {
        let api = if let Some(path) = releases_url().strip_prefix("https://github.com/") {
            format!(
                "https://api.github.com/repos/{}/releases/latest",
                path.trim_end_matches("/releases")
            )
        } else {
            String::new()
        };

        let result = if api.is_empty() {
            Err("missing repository".to_string())
        } else {
            hidden_command("curl.exe")
                .args([
                    "-sL",
                    "-H",
                    "User-Agent: zsclip",
                    "-H",
                    "Accept: application/vnd.github+json",
                    &api,
                ])
                .output()
                .map_err(|e| e.to_string())
                .and_then(|out| {
                    if !out.status.success() {
                        Err(String::from_utf8_lossy(&out.stderr).trim().to_string())
                    } else {
                        Ok(String::from_utf8_lossy(&out.stdout).to_string())
                    }
                })
        };

        let mut next = UpdateCheckState {
            started: true,
            checking: false,
            available: false,
            latest_tag: String::new(),
            latest_url: latest_release_url(),
            error: String::new(),
        };

        match result {
            Ok(body) => {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    next.latest_tag = json
                        .get("tag_name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    next.latest_url = json
                        .get("html_url")
                        .and_then(|v| v.as_str())
                        .unwrap_or(next.latest_url.as_str())
                        .to_string();
                    next.available = !next.latest_tag.trim().is_empty()
                        && version_is_newer(&next.latest_tag, env!("CARGO_PKG_VERSION"));
                } else {
                    next.error = "invalid response".to_string();
                }
            }
            Err(err) => {
                next.error = err;
            }
        }

        if let Ok(mut state) = update_check_state().lock() {
            *state = next;
        }

        notify();
    });
}

fn read_disabled_hotkeys_registry() -> Option<String> {
    unsafe {
        let mut key = 0isize;
        let subkey = to_wide(DISABLED_HOTKEYS_KEY);
        let value = to_wide(DISABLED_HOTKEYS_VALUE);
        let open = RegOpenKeyExW(
            HKEY_CURRENT_USER_VAL,
            subkey.as_ptr(),
            0,
            KEY_READ_VAL,
            &mut key,
        );
        if open != 0 {
            return Some(String::new());
        }

        let mut ty = 0u32;
        let mut size = 0u32;
        let query_size = RegQueryValueExW(
            key,
            value.as_ptr(),
            null_mut(),
            &mut ty,
            null_mut(),
            &mut size,
        );
        if query_size != 0 || size == 0 {
            RegCloseKey(key);
            return Some(String::new());
        }

        let mut buf = vec![0u8; size as usize];
        let query = RegQueryValueExW(
            key,
            value.as_ptr(),
            null_mut(),
            &mut ty,
            buf.as_mut_ptr(),
            &mut size,
        );
        RegCloseKey(key);
        if query != 0 || ty != REG_SZ_VAL {
            return Some(String::new());
        }

        let wide_len = (size as usize / 2).saturating_sub(1);
        let wide = std::slice::from_raw_parts(buf.as_ptr() as *const u16, wide_len);
        Some(String::from_utf16_lossy(wide))
    }
}

fn set_disabled_hotkeys_registry(txt: &str) -> Result<(), String> {
    unsafe {
        let mut key = 0isize;
        let subkey = to_wide(DISABLED_HOTKEYS_KEY);
        let value = to_wide(DISABLED_HOTKEYS_VALUE);
        let open = RegOpenKeyExW(
            HKEY_CURRENT_USER_VAL,
            subkey.as_ptr(),
            0,
            KEY_SET_VALUE_VAL | KEY_READ_VAL,
            &mut key,
        );
        if open != 0 {
            return Err(format!(
                "\u{6253}\u{5f00}\u{6ce8}\u{518c}\u{8868}\u{5931}\u{8d25}: {open}"
            ));
        }

        if txt.trim().is_empty() {
            let delete = RegDeleteValueW(key, value.as_ptr());
            RegCloseKey(key);
            if delete == 0 || delete == ERROR_FILE_NOT_FOUND {
                return Ok(());
            }
            return Err(format!(
                "\u{5220}\u{9664}\u{6ce8}\u{518c}\u{8868}\u{503c}\u{5931}\u{8d25}: {delete}"
            ));
        }

        let mut wide = to_wide(txt);
        if *wide.last().unwrap_or(&0) != 0 {
            wide.push(0);
        }
        let set = RegSetValueExW(
            key,
            value.as_ptr(),
            0,
            REG_SZ_VAL,
            wide.as_ptr() as *const u8,
            (wide.len() * 2) as u32,
        );
        RegCloseKey(key);
        if set == 0 {
            Ok(())
        } else {
            Err(format!(
                "\u{5199}\u{5165}\u{6ce8}\u{518c}\u{8868}\u{5931}\u{8d25}: {set}"
            ))
        }
    }
}

fn set_clipboard_history_enabled_registry(enabled: bool) -> Result<(), String> {
    unsafe {
        let mut key = 0isize;
        let mut disposition = 0u32;
        let subkey = to_wide(CLIPBOARD_SETTINGS_KEY);
        let value = to_wide(CLIPBOARD_HISTORY_VALUE);
        let open = RegCreateKeyExW(
            HKEY_CURRENT_USER_VAL,
            subkey.as_ptr(),
            0,
            null_mut(),
            0,
            KEY_READ_VAL | KEY_SET_VALUE_VAL | KEY_CREATE_SUB_KEY_VAL,
            core::ptr::null(),
            &mut key,
            &mut disposition,
        );
        if open != 0 {
            return Err(format!("打开剪贴板设置失败: {open}"));
        }

        let data: u32 = if enabled { 1 } else { 0 };
        let set = RegSetValueExW(
            key,
            value.as_ptr(),
            0,
            REG_DWORD_VAL,
            &data as *const u32 as *const u8,
            core::mem::size_of::<u32>() as u32,
        );
        RegCloseKey(key);
        if set == 0 {
            Ok(())
        } else {
            Err(format!("写入剪贴板历史设置失败: {set}"))
        }
    }
}

pub(crate) fn toggle_disabled_hotkey_char(ch: char, disable: bool) -> Result<(), String> {
    if !ch.is_ascii_alphanumeric() {
        return Err("\u{65e0}\u{6548}\u{6309}\u{952e}".to_string());
    }
    let mut chars: BTreeSet<char> = read_disabled_hotkeys_registry()
        .unwrap_or_default()
        .to_uppercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .collect();
    let up = ch.to_ascii_uppercase();
    if disable {
        chars.insert(up);
    } else {
        chars.remove(&up);
    }
    let new_text: String = chars.into_iter().collect();
    set_disabled_hotkeys_registry(&new_text)
}

pub(crate) fn set_system_clipboard_history_enabled(enabled: bool) -> Result<(), String> {
    set_clipboard_history_enabled_registry(enabled)?;
    toggle_disabled_hotkey_char('V', !enabled)?;
    Ok(())
}

pub(crate) fn restart_explorer_shell() -> Result<(), String> {
    let _ = hidden_command("taskkill")
        .args(["/f", "/im", "explorer.exe"])
        .output();
    Command::new("explorer.exe")
        .spawn()
        .map(|_| ())
        .map_err(|e| e.to_string())
}

fn releases_url() -> String {
    let repo = open_source_url().trim().trim_end_matches('/');
    if repo.is_empty() {
        "https://github.com/qiu7824/zsclip/releases".to_string()
    } else {
        format!("{repo}/releases")
    }
}

fn encode_powershell_script(script: &str) -> String {
    let utf16: Vec<u16> = OsStr::new(script).encode_wide().collect();
    let bytes = unsafe { std::slice::from_raw_parts(utf16.as_ptr() as *const u8, utf16.len() * 2) };
    base64::engine::general_purpose::STANDARD.encode(bytes)
}

fn run_hidden_powershell_encoded(script: &str, args: &[&str]) -> Result<String, String> {
    let encoded = encode_powershell_script(script);
    let out = Command::new("powershell")
        .creation_flags(CREATE_NO_WINDOW_FLAG)
        .arg("-NoProfile")
        .arg("-NonInteractive")
        .arg("-ExecutionPolicy")
        .arg("Bypass")
        .arg("-EncodedCommand")
        .arg(encoded)
        .args(args)
        .output()
        .map_err(|e| format!("启动 PowerShell 失败: {e}"))?;
    if out.status.success() {
        Ok(String::from_utf8_lossy(&out.stdout).trim().to_string())
    } else {
        let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
        Err(if !stderr.is_empty() {
            stderr
        } else if !stdout.is_empty() {
            stdout
        } else {
            "PowerShell 执行失败".to_string()
        })
    }
}

pub(crate) fn pick_paste_sound_file(current: &str) -> Result<Option<String>, String> {
    let script = r#"
Add-Type -AssemblyName System.Windows.Forms
[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
$dlg = New-Object System.Windows.Forms.OpenFileDialog
$dlg.Filter = 'Wave Files|*.wav|All Files|*.*'
$dlg.Title = '选择提示音文件'
$dlg.Multiselect = $false
if ($args.Count -gt 0 -and -not [string]::IsNullOrWhiteSpace($args[0])) {
  $current = $args[0]
  if (Test-Path $current) {
    $dlg.FileName = $current
    $parent = Split-Path -Parent $current
    if (Test-Path $parent) { $dlg.InitialDirectory = $parent }
  } else {
    $parent = Split-Path -Parent $current
    if (Test-Path $parent) { $dlg.InitialDirectory = $parent }
  }
}
if ($dlg.ShowDialog() -eq [System.Windows.Forms.DialogResult]::OK) {
  Write-Output $dlg.FileName
}
"#;
    let out = run_hidden_powershell_encoded(script, &[current])?;
    if out.trim().is_empty() {
        Ok(None)
    } else {
        Ok(Some(out))
    }
}

pub(crate) unsafe fn play_paste_success_sound(kind: &str, custom_path: &str) {
    if kind.trim() == "custom" {
        let path = custom_path.trim();
        if !path.is_empty() && Path::new(path).is_file() {
            let wide = to_wide(path);
            if PlaySoundW(wide.as_ptr(), 0, SND_ASYNC | SND_FILENAME | SND_NODEFAULT) != 0 {
                return;
            }
        }
    }
    let bytes = match kind.trim() {
        "soft" => PASTE_SOUND_SOFT,
        "bright" => PASTE_SOUND_BRIGHT,
        _ => PASTE_SOUND_DEFAULT,
    };
    let _ = PlaySoundW(
        bytes.as_ptr() as *const u16,
        0,
        SND_ASYNC | SND_MEMORY | SND_NODEFAULT,
    );
}

pub(crate) fn plugin_downloads_url() -> String {
    releases_url()
}

fn parse_version_parts(value: &str) -> Vec<u32> {
    value
        .trim()
        .trim_start_matches(['v', 'V'])
        .split('.')
        .map(|part| part.parse::<u32>().ok().unwrap_or(0))
        .collect()
}

fn version_is_newer(latest: &str, current: &str) -> bool {
    let mut a = parse_version_parts(latest);
    let mut b = parse_version_parts(current);
    let max_len = a.len().max(b.len()).max(3);
    a.resize(max_len, 0);
    b.resize(max_len, 0);
    a > b
}

pub(crate) fn is_directory_item(item: &ClipItem) -> bool {
    item.file_paths
        .as_ref()
        .and_then(|v| v.first())
        .map(|p| Path::new(p).is_dir())
        .unwrap_or(false)
}

pub(crate) unsafe fn item_icon_handle(item: &ClipItem, target_px: i32) -> isize {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => icon_handle_for(IconAssetKind::Text, target_px),
        ClipKind::Image => icon_handle_for(IconAssetKind::Image, target_px),
        ClipKind::Files => {
            if item
                .file_paths
                .as_ref()
                .and_then(|v| v.first())
                .map(|p| Path::new(p).is_dir())
                .unwrap_or(false)
            {
                icon_handle_for(IconAssetKind::Folder, target_px)
            } else {
                icon_handle_for(IconAssetKind::File, target_px)
            }
        }
    }
}

pub(crate) fn load_icons() -> Icons {
    unsafe {
        Icons {
            app: load_icon_from_bytes(ICO_APP, 64, 64),
            search: 0,
            setting: 0,
            min: 0,
            close: 0,
            text: 0,
            image: 0,
            file: 0,
            folder: 0,
            pin: 0,
            del: 0,
        }
    }
}

fn icon_bytes_for(kind: IconAssetKind) -> &'static [u8] {
    match kind {
        IconAssetKind::App => ICO_APP,
        IconAssetKind::Search => ICO_SEARCH
            .get_or_init(|| icon_png_pack!("search", "search"))
            .as_slice(),
        IconAssetKind::Setting => ICO_SETTING
            .get_or_init(|| icon_png_pack!("setting", "setting"))
            .as_slice(),
        IconAssetKind::Min => ICO_MIN
            .get_or_init(|| icon_png_pack!("min", "min"))
            .as_slice(),
        IconAssetKind::Close => ICO_EXIT
            .get_or_init(|| icon_png_pack!("exit", "exit"))
            .as_slice(),
        IconAssetKind::Text => ICO_TEXT
            .get_or_init(|| icon_png_pack!("text", "text"))
            .as_slice(),
        IconAssetKind::Image => ICO_IMAGE
            .get_or_init(|| icon_png_pack!("image", "image"))
            .as_slice(),
        IconAssetKind::File => ICO_FILE
            .get_or_init(|| icon_png_pack!("file", "file"))
            .as_slice(),
        IconAssetKind::Folder => ICO_FOLDER
            .get_or_init(|| icon_png_pack!("fold", "fold"))
            .as_slice(),
        IconAssetKind::Pin => ICO_TOP
            .get_or_init(|| icon_png_pack!("top", "top"))
            .as_slice(),
        IconAssetKind::Delete => ICO_DEL
            .get_or_init(|| icon_png_pack!("del", "del"))
            .as_slice(),
    }
}

fn normalize_requested_icon_size(size: i32) -> i32 {
    size.clamp(8, 256)
}

pub(crate) unsafe fn icon_handle_for(kind: IconAssetKind, target_px: i32) -> isize {
    let normalized = normalize_requested_icon_size(target_px);
    if let Ok(mut cache) = icon_handle_cache().lock() {
        if let Some(handle) = cache.get(&(kind as u8, normalized)) {
            return *handle;
        }
        let handle = load_icon_from_bytes(icon_bytes_for(kind), normalized, normalized);
        if handle != 0 {
            cache.insert((kind as u8, normalized), handle);
        }
        handle
    } else {
        load_icon_from_bytes(icon_bytes_for(kind), normalized, normalized)
    }
}

fn build_ico_from_png_entries(entries: &[(u8, &[u8])]) -> Vec<u8> {
    let header_len = 6 + entries.len() * 16;
    let total_data_len: usize = entries.iter().map(|(_, data)| data.len()).sum();
    let mut out = Vec::with_capacity(header_len + total_data_len);
    out.extend_from_slice(&0u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&(entries.len() as u16).to_le_bytes());

    let mut offset = header_len as u32;
    for (size, data) in entries {
        out.push(*size);
        out.push(*size);
        out.push(0);
        out.push(0);
        out.extend_from_slice(&1u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&(data.len() as u32).to_le_bytes());
        out.extend_from_slice(&offset.to_le_bytes());
        offset += data.len() as u32;
    }

    for (_, data) in entries {
        out.extend_from_slice(data);
    }
    out
}

/// Load icon handle from ICO bytes.
unsafe fn load_icon_from_bytes(data: &[u8], w: i32, h: i32) -> isize {
    if data.len() < 6 {
        return 0;
    }
    let count = u16::from_le_bytes([data[4], data[5]]) as usize;
    let mut exact = Vec::new();
    let mut larger = Vec::new();
    let mut smaller = Vec::new();
    for i in 0..count {
        let base = 6 + i * 16;
        if base + 16 > data.len() {
            break;
        }
        let icon_w = data[base] as i32;
        let icon_h = data[base + 1] as i32;
        let icon_w = if icon_w == 0 { 256 } else { icon_w };
        let icon_h = if icon_h == 0 { 256 } else { icon_h };
        if icon_w == w && icon_h == h {
            exact.push(base);
        } else if icon_w >= w && icon_h >= h {
            larger.push((icon_w * icon_h, base));
        } else {
            smaller.push((-(icon_w * icon_h), base));
        }
    }
    for base in exact {
        if let Some(icon) = try_create_icon(data, base, w, h) {
            return icon;
        }
    }
    larger.sort_by_key(|entry| entry.0);
    for (_, base) in larger {
        if let Some(icon) = try_create_icon(data, base, w, h) {
            return icon;
        }
    }
    smaller.sort_by_key(|entry| entry.0);
    for (_, base) in smaller {
        if let Some(icon) = try_create_icon(data, base, w, h) {
            return icon;
        }
    }
    0
}

unsafe fn try_create_icon(data: &[u8], base: usize, w: i32, h: i32) -> Option<isize> {
    if base + 16 > data.len() {
        return None;
    }
    let size = u32::from_le_bytes([
        data[base + 8],
        data[base + 9],
        data[base + 10],
        data[base + 11],
    ]) as usize;
    let offset = u32::from_le_bytes([
        data[base + 12],
        data[base + 13],
        data[base + 14],
        data[base + 15],
    ]) as usize;
    if offset == 0 || size == 0 || offset + size > data.len() {
        return None;
    }
    let slice = &data[offset..offset + size];
    let handle = CreateIconFromResourceEx(
        slice.as_ptr(),
        slice.len() as u32,
        1,
        0x00030000,
        w,
        h,
        LR_DEFAULTCOLOR,
    );
    if !handle.is_null() {
        Some(handle as isize)
    } else {
        None
    }
}
