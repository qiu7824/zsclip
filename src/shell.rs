use std::collections::{BTreeSet, HashMap};
use std::os::windows::process::CommandExt;
use std::path::Path;
use std::ptr::{null_mut};
use std::process::Command;
use std::sync::{Mutex, OnceLock};

use windows_sys::Win32::{
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::{
            CreateIconFromResourceEx, LR_DEFAULTCOLOR, SW_SHOWNORMAL,
        },
    },
};

use crate::app::{ClipItem, ClipKind, Icons};
use crate::i18n::tr;
use crate::win_system_ui::to_wide;

const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;
const HKEY_CURRENT_USER_VAL: isize = -2147483647i32 as isize;
const KEY_READ_VAL: u32 = 0x20019;
const KEY_SET_VALUE_VAL: u32 = 0x0002;
const REG_SZ_VAL: u32 = 1;
const ERROR_FILE_NOT_FOUND: i32 = 2;
const DISABLED_HOTKEYS_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced";
const DISABLED_HOTKEYS_VALUE: &str = "DisabledHotkeys";

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
    fn RegDeleteValueW(hkey: isize, lpvaluename: *const u16) -> i32;
    fn RegCloseKey(hkey: isize) -> i32;
}

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
static ICO_SEARCH: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_SETTING: OnceLock<Vec<u8>> = OnceLock::new();
static ICO_MIN: OnceLock<Vec<u8>> = OnceLock::new();
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

fn icon_handle_cache() -> &'static Mutex<HashMap<(u8, i32), isize>> {
    ICON_HANDLE_CACHE.get_or_init(|| Mutex::new(HashMap::new()))
}

macro_rules! icon_png_pack {
    ($dir:literal, $name:literal) => {
        build_ico_from_png_entries(&[
            (16, include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_16x16.png")) as &[u8]),
            (24, include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_24x24.png")) as &[u8]),
            (32, include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_32x32.png")) as &[u8]),
            (48, include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_48x48.png")) as &[u8]),
            (64, include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_64x64.png")) as &[u8]),
            (128, include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_128x128.png")) as &[u8]),
            (0, include_bytes!(concat!("../assets/icons/", $dir, "/", $name, "_256x256.png")) as &[u8]),
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
    ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), wp.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
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

pub(crate) fn open_source_url() -> &'static str {
    option_env!("CARGO_PKG_REPOSITORY").unwrap_or("")
}

pub(crate) fn open_source_url_display() -> &'static str {
    if open_source_url().trim().is_empty() {
        tr(
            "未配置（可在 Cargo.toml 的 package.repository 中配置）",
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
            return Err(format!("打开注册表失败: {open}"));
        }

        if txt.trim().is_empty() {
            let delete = RegDeleteValueW(key, value.as_ptr());
            RegCloseKey(key);
            if delete == 0 || delete == ERROR_FILE_NOT_FOUND {
                return Ok(());
            }
            return Err(format!("删除注册表值失败: {delete}"));
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
            Err(format!("写入注册表失败: {set}"))
        }
    }
}

pub(crate) fn toggle_disabled_hotkey_char(ch: char, disable: bool) -> Result<(), String> {
    if !ch.is_ascii_alphanumeric() {
        return Err("无效按键".to_string());
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
            if item.file_paths.as_ref().and_then(|v| v.first()).map(|p| Path::new(p).is_dir()).unwrap_or(false) {
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
            app:    load_icon_from_bytes(ICO_APP,       64, 64),
            search: 0,
            setting:0,
            min:    0,
            close:  0,
            text:   0,
            image:  0,
            file:   0,
            folder: 0,
            pin:    0,
            del:    0,
        }
    }
}

fn icon_bytes_for(kind: IconAssetKind) -> &'static [u8] {
    match kind {
        IconAssetKind::App => ICO_APP,
        IconAssetKind::Search => ICO_SEARCH.get_or_init(|| icon_png_pack!("search", "search")).as_slice(),
        IconAssetKind::Setting => ICO_SETTING.get_or_init(|| icon_png_pack!("setting", "setting")).as_slice(),
        IconAssetKind::Min => ICO_MIN.get_or_init(|| icon_png_pack!("min", "min")).as_slice(),
        IconAssetKind::Close => ICO_EXIT.get_or_init(|| icon_png_pack!("exit", "exit")).as_slice(),
        IconAssetKind::Text => ICO_TEXT.get_or_init(|| icon_png_pack!("text", "text")).as_slice(),
        IconAssetKind::Image => ICO_IMAGE.get_or_init(|| icon_png_pack!("image", "image")).as_slice(),
        IconAssetKind::File => ICO_FILE.get_or_init(|| icon_png_pack!("file", "file")).as_slice(),
        IconAssetKind::Folder => ICO_FOLDER.get_or_init(|| icon_png_pack!("fold", "fold")).as_slice(),
        IconAssetKind::Pin => ICO_TOP.get_or_init(|| icon_png_pack!("top", "top")).as_slice(),
        IconAssetKind::Delete => ICO_DEL.get_or_init(|| icon_png_pack!("del", "del")).as_slice(),
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

/// 从 ICO 文件字节流加载指定尺寸的图标句柄。
unsafe fn load_icon_from_bytes(data: &[u8], w: i32, h: i32) -> isize {
    if data.len() < 6 { return 0; }
    let count = u16::from_le_bytes([data[4], data[5]]) as usize;
    let mut exact = Vec::new();
    let mut larger = Vec::new();
    let mut smaller = Vec::new();
    for i in 0..count {
        let base = 6 + i * 16;
        if base + 16 > data.len() { break; }
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
    if base + 16 > data.len() { return None; }
    let size   = u32::from_le_bytes([data[base+8],  data[base+9],  data[base+10], data[base+11]]) as usize;
    let offset = u32::from_le_bytes([data[base+12], data[base+13], data[base+14], data[base+15]]) as usize;
    if offset == 0 || size == 0 || offset + size > data.len() { return None; }
    let slice = &data[offset..offset + size];
    let handle = CreateIconFromResourceEx(
        slice.as_ptr(), slice.len() as u32, 1, 0x00030000, w, h, LR_DEFAULTCOLOR,
    );
    if !handle.is_null() { Some(handle as isize) } else { None }
}
