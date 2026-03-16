use std::collections::BTreeSet;
use std::os::windows::process::CommandExt;
use std::path::Path;
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

use crate::app::{AppState, ClipItem, ClipKind, Icons};
use crate::i18n::tr;
use crate::win_system_ui::to_wide;

const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;

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

fn update_check_state() -> &'static Mutex<UpdateCheckState> {
    UPDATE_CHECK_STATE.get_or_init(|| Mutex::new(UpdateCheckState::default()))
}

// ── 图标数据嵌入二进制（无需外部文件）──────────────────────────────────────
static ICO_CLIPBOARD: &[u8] = include_bytes!("../assets/icons/clipboard.ico");
static ICO_SEARCH:    &[u8] = include_bytes!("../assets/icons/search.ico");
static ICO_SETTING:   &[u8] = include_bytes!("../assets/icons/setting.ico");
static ICO_MIN:       &[u8] = include_bytes!("../assets/icons/min.ico");
static ICO_EXIT:      &[u8] = include_bytes!("../assets/icons/exit.ico");
static ICO_TEXT:      &[u8] = include_bytes!("../assets/icons/text.ico");
static ICO_IMAGE:     &[u8] = include_bytes!("../assets/icons/image.ico");
static ICO_FILE:      &[u8] = include_bytes!("../assets/icons/file.ico");
static ICO_FOLD:      &[u8] = include_bytes!("../assets/icons/fold.ico");
static ICO_TOP:       &[u8] = include_bytes!("../assets/icons/top.ico");
static ICO_DEL:       &[u8] = include_bytes!("../assets/icons/del.ico");

pub(crate) unsafe fn open_path_with_shell(path: &str) {
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

pub(crate) fn toggle_disabled_hotkey_char(ch: char, disable: bool) -> Result<(), String> {
    if !ch.is_ascii_alphanumeric() {
        return Err("无效按键".to_string());
    }
    let mut chars: BTreeSet<char> = read_disabled_hotkeys_text()
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
    set_disabled_hotkeys_text(&new_text)
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

fn read_disabled_hotkeys_text() -> Option<String> {
    let out = hidden_command("reg")
        .args([
            "query",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
            "/v",
            "DisabledHotkeys",
        ])
        .output()
        .ok()?;
    if !out.status.success() {
        return Some(String::new());
    }
    let stdout = String::from_utf8_lossy(&out.stdout);
    for line in stdout.lines() {
        if line.contains("DisabledHotkeys") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if let Some(v) = parts.last() {
                return Some((*v).trim().to_string());
            }
        }
    }
    Some(String::new())
}

fn set_disabled_hotkeys_text(txt: &str) -> Result<(), String> {
    if txt.trim().is_empty() {
        let out = hidden_command("reg")
            .args([
                "delete",
                r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
                "/v",
                "DisabledHotkeys",
                "/f",
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if stderr.contains("Unable to find") || stderr.contains("系统找不到指定") {
            return Ok(());
        }
        return Err(if stderr.trim().is_empty() {
            "删除注册表值失败".to_string()
        } else {
            stderr
        });
    }
    let out = hidden_command("reg")
        .args([
            "add",
            r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced",
            "/v",
            "DisabledHotkeys",
            "/t",
            "REG_SZ",
            "/d",
            txt,
            "/f",
        ])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() {
        Ok(())
    } else {
        Err(String::from_utf8_lossy(&out.stderr).to_string())
    }
}

pub(crate) fn is_directory_item(item: &ClipItem) -> bool {
    item.file_paths
        .as_ref()
        .and_then(|v| v.first())
        .map(|p| Path::new(p).is_dir())
        .unwrap_or(false)
}

pub(crate) unsafe fn item_icon_handle(state: &mut AppState, item: &ClipItem) -> isize {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => state.icons.text,
        ClipKind::Image => state.icons.image,
        ClipKind::Files => {
            if item.file_paths.as_ref().and_then(|v| v.first()).map(|p| Path::new(p).is_dir()).unwrap_or(false) {
                state.icons.folder
            } else {
                state.icons.file
            }
        }
    }
}

pub(crate) fn load_icons() -> Icons {
    unsafe {
        Icons {
            app:    load_icon_from_bytes(ICO_CLIPBOARD, 32, 32),
            search: load_icon_from_bytes(ICO_SEARCH,    16, 16),
            setting:load_icon_from_bytes(ICO_SETTING,   16, 16),
            min:    load_icon_from_bytes(ICO_MIN,        16, 16),
            close:  load_icon_from_bytes(ICO_EXIT,      16, 16),
            text:   load_icon_from_bytes(ICO_TEXT,      20, 20),
            image:  load_icon_from_bytes(ICO_IMAGE,     20, 20),
            file:   load_icon_from_bytes(ICO_FILE,      20, 20),
            folder: load_icon_from_bytes(ICO_FOLD,      20, 20),
            pin:    load_icon_from_bytes(ICO_TOP,       16, 16),
            del:    load_icon_from_bytes(ICO_DEL,       16, 16),
        }
    }
}

/// 从 ICO 文件字节流加载指定尺寸的图标句柄。
unsafe fn load_icon_from_bytes(data: &[u8], w: i32, h: i32) -> isize {
    if data.len() < 6 { return 0; }
    let count = u16::from_le_bytes([data[4], data[5]]) as usize;
    // 1st pass: exact size match
    for i in 0..count {
        let base = 6 + i * 16;
        if base + 16 > data.len() { break; }
        let icon_w = data[base] as i32;
        let icon_h = data[base + 1] as i32;
        let icon_w = if icon_w == 0 { 256 } else { icon_w };
        let icon_h = if icon_h == 0 { 256 } else { icon_h };
        if icon_w != w || icon_h != h { continue; }
        if let Some(h) = try_create_icon(data, base, w, h) { return h; }
    }
    // 2nd pass: any size, let system scale
    if count > 0 {
        if let Some(h) = try_create_icon(data, 6, w, h) { return h; }
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
