// BUILD_MARKER: v26b_hotkey_status_group_layout_registry_cleanup
// BUILD_MARKER: v24_hotkey_registry_autolayout
// BUILD_MARKER: v25_settings_ui_framework_registry_lazy_pages
// BUILD_MARKER: v21_tab_style_title_align_warning_cleanup
// BUILD_MARKER: v20_schemeA_bufferedpaint_deferwindowpos
// BUILD_MARKER: v19_hotkey_scroll_datadir
// BUILD_MARKER: v18c_compile_type_fixes
// BUILD_MARKER: v18b_compile_fixes
// BUILD_MARKER: v18_db_theme_sendinput_refactor
use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use std::cmp::{max, min};
use std::collections::BTreeSet;
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::mem::{size_of, zeroed};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::process::Command;
use rusqlite::params;
use serde::{Deserialize, Serialize};
use std::ptr::{null, null_mut};
#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterHotKey(hwnd: HWND, id: i32, fsmodifiers: u32, vk: u32) -> i32;
    fn UnregisterHotKey(hwnd: HWND, id: i32) -> i32;
    fn EnableWindow(hwnd: HWND, benable: i32) -> i32;
    fn IsWindow(hwnd: HWND) -> i32;
    fn TrackMouseEvent(lpeventtrack: *mut TRACKMOUSEEVENT) -> i32;
    fn SendInput(cinputs: u32, pinputs: *const INPUT, cbsize: i32) -> u32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
    fn GetCurrentProcess() -> *mut core::ffi::c_void;
    fn GlobalAlloc(uflags: u32, dwbytes: usize) -> *mut core::ffi::c_void;
    fn GlobalLock(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn GlobalUnlock(hmem: *mut core::ffi::c_void) -> i32;
    fn GlobalFree(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
}

#[link(name = "psapi")]
unsafe extern "system" {
    fn EmptyWorkingSet(hprocess: *mut core::ffi::c_void) -> i32;
}

// Registry API for autostart
const KEY_SET_VALUE: u32 = 0x0002;
const KEY_READ_VAL: u32 = 0x20019;
const REG_SZ: u32 = 1;
const HKEY_CURRENT_USER_VAL: isize = -2147483647i32 as isize;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(hkey: isize, lpsubkey: *const u16, uloptions: u32, samdesired: u32, phkresult: *mut isize) -> i32;
    fn RegQueryValueExW(hkey: isize, lpvaluename: *const u16, lpreserved: *mut u32, lptype: *mut u32, lpdata: *mut u8, lpcbdata: *mut u32) -> i32;
    fn RegSetValueExW(hkey: isize, lpvaluename: *const u16, reserved: u32, dwtype: u32, lpdata: *const u8, cbdata: u32) -> i32;
    fn RegDeleteValueW(hkey: isize, lpvaluename: *const u16) -> i32;
    fn RegCloseKey(hkey: isize) -> i32;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(hwnd: HWND, dwattribute: u32, pvattribute: *const core::ffi::c_void, cbattribute: u32) -> i32;
}

#[link(name = "uxtheme")]
unsafe extern "system" {
    fn SetWindowTheme(hwnd: HWND, pszsubappid: *const u16, pszsubidlist: *const u16) -> i32;
}

#[repr(C)]
struct TRACKMOUSEEVENT {
    cb_size: u32,
    dw_flags: u32,
    hwnd_track: HWND,
    dw_hover_time: u32,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct KEYBDINPUT {
    w_vk: u16,
    w_scan: u16,
    dw_flags: u32,
    time: u32,
    dw_extra_info: usize,
}

#[repr(C)]
#[derive(Copy, Clone)]
union INPUT_UNION {
    ki: KEYBDINPUT,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct INPUT {
    r#type: u32,
    anonymous: INPUT_UNION,
}

const INPUT_KEYBOARD: u32 = 1;
const ERROR_HOTKEY_ALREADY_REGISTERED: u32 = 1409;

pub(crate) use crate::ui::{ClipGroup, ClipItem, ClipKind};
use crate::ui::{draw_icon_tinted, draw_main_segment_bar, draw_round_fill, draw_round_rect, draw_text, draw_text_ex, is_dark_mode, rgb, settings_nav_item_rect, ClipListState, MainUiLayout, Theme, SETTINGS_CONTENT_Y, SETTINGS_H, SETTINGS_NAV_W, SETTINGS_PAGES, SETTINGS_W, DT_LEFT, DT_CENTER, DT_VCENTER, DT_SINGLELINE};
use crate::shell::{is_directory_item, item_icon_handle, load_icons, open_parent_folder, open_path_with_shell};
use crate::sticker::show_image_sticker;
use crate::mail_merge_native::{launch_mail_merge_window, launch_mail_merge_window_with_excel};
use crate::tray::{add_tray_icon, handle_tray, position_main_window, remember_window_pos, remove_tray_icon, toggle_window_visibility, toggle_window_visibility_hotkey};
use crate::db_runtime::{ensure_db, with_db, with_db_mut};
use crate::time_utils::{format_created_at_local, format_local_time_for_image_preview, now_utc_sqlite};
use crate::win_buffered_paint::{begin_buffered_paint, end_buffered_paint};
use crate::win_system_params::{settings_section_body_rect, CF_HDROP, DropFiles, GMEM_MOVEABLE, GMEM_ZEROINIT, IDC_SET_AUTOSTART, IDC_SET_BTN_OPENCFG, IDC_SET_BTN_OPENDB, IDC_SET_BTN_OPENDATA, IDC_SET_CLICK_HIDE, IDC_SET_CLOSE, IDC_SET_CLOSETRAY, IDC_SET_CLOUD_APPLY_CFG, IDC_SET_CLOUD_DIR, IDC_SET_CLOUD_ENABLE, IDC_SET_CLOUD_INTERVAL, IDC_SET_CLOUD_PASS, IDC_SET_CLOUD_RESTORE_BACKUP, IDC_SET_CLOUD_SYNC_NOW, IDC_SET_CLOUD_UPLOAD_CFG, IDC_SET_CLOUD_URL, IDC_SET_CLOUD_USER, IDC_SET_DX, IDC_SET_DY, IDC_SET_EDGEHIDE, IDC_SET_FX, IDC_SET_FY, IDC_SET_GROUP_ADD, IDC_SET_GROUP_DELETE, IDC_SET_GROUP_DOWN, IDC_SET_GROUP_ENABLE, IDC_SET_GROUP_LIST, IDC_SET_GROUP_RENAME, IDC_SET_GROUP_UP, IDC_SET_HOVERPREVIEW, IDC_SET_MAX, IDC_SET_PLUGIN_MAILMERGE, IDC_SET_POSMODE, IDC_SET_SAVE, IID_IDATAOBJECT_RAW, RPC_E_CHANGED_MODE_HR, SCROLL_BAR_MARGIN, SCROLL_BAR_W, SCROLL_BAR_W_ACTIVE, SETTINGS_CLASS, SETTINGS_CONTENT_TOTAL_H, SETTINGS_FORM_ROW_GAP, SETTINGS_FORM_ROW_H};
use crate::win_system_ui::{apply_dark_mode_to_window, apply_theme_to_menu, apply_window_corner_preference, create_drop_source, create_settings_component, create_settings_edit as host_create_settings_edit, create_settings_label as host_create_settings_label, create_settings_label_auto as host_create_settings_label_auto, create_settings_listbox as host_create_settings_listbox, create_settings_password_edit as host_create_settings_password_edit, draw_settings_button_component, draw_settings_nav_item, draw_settings_page_cards, draw_settings_page_content, draw_settings_toggle_component, get_window_text, get_x_lparam, get_y_lparam, init_dark_mode_for_process, nav_divider_x, release_raw_com, settings_child_visible, settings_dropdown_index_for_max_items, settings_dropdown_index_for_pos_mode, settings_dropdown_label_for_max_items, settings_dropdown_label_for_pos_mode, settings_dropdown_max_items_from_label, settings_dropdown_pos_mode_from_label, settings_safe_paint_rect, settings_title_rect_win as settings_title_rect, settings_viewport_mask_rect, settings_viewport_rect, show_settings_dropdown_popup, to_wide, SettingsComponentKind, SettingsCtrlReg, SettingsPage, SettingsUiRegistry, WM_SETTINGS_DROPDOWN_SELECTED};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
        CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
        FillRect, GetStockObject, InvalidateRect, IntersectClipRect, LineTo, MoveToEx, PAINTSTRUCT, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow, RestoreDC, RoundRect, SaveDC, ScreenToClient,
        SelectObject, SetBkColor, SetBkMode, SetTextColor,
        DEFAULT_GUI_FONT, NULL_PEN, SRCCOPY,
    },
    System::{
        DataExchange::{AddClipboardFormatListener, RemoveClipboardFormatListener, OpenClipboard, CloseClipboard, GetClipboardData, EmptyClipboard, SetClipboardData},
        LibraryLoader::GetModuleHandleW,
        Ole::{DoDragDrop, OleInitialize, OleUninitialize, DROPEFFECT, DROPEFFECT_COPY},
    },
    UI::{
        Controls::{DRAWITEMSTRUCT, ODS_SELECTED},
        Input::KeyboardAndMouse::{
            GetAsyncKeyState, SetFocus, SetCapture, ReleaseCapture, keybd_event,
            KEYEVENTF_KEYUP, VK_CONTROL, VK_DELETE, VK_BACK, VK_INSERT, VK_HOME, VK_END, VK_LBUTTON, VK_MENU, VK_PRIOR, VK_NEXT, VK_SPACE,
            VK_DOWN, VK_ESCAPE, VK_RETURN, VK_UP, VK_V, VK_SHIFT, VK_TAB, VK_LEFT, VK_RIGHT,
        },
        Shell::{DragQueryFileW, ILClone, ILCreateFromPathW, ILFindLastID, ILFree, SHCreateDataObject},
        WindowsAndMessaging::*,
    },
};

// Edit control message constants (not re-exported by windows-sys wildcard)
const EM_SETSEL: u32 = 0x00B1;
const EM_GETLINECOUNT: u32 = 0x00BA;
const EM_GETFIRSTVISIBLELINE: u32 = 0x00CE;
const EN_VSCROLL: u32 = 0x0602;
const EM_SETMARGINS: u32 = 0x00D3;
const EC_LEFTMARGIN: usize = 0x0001;
const EC_RIGHTMARGIN: usize = 0x0002;

const APP_TITLE: &str = "剪贴板";
const CLASS_NAME: &str = "ZsClipMain";

pub(crate) const WIN_W: i32 = 300;
pub(crate) const WIN_H: i32 = 615;
const TITLE_H: i32 = 35;
const SEG_X: i32 = 6;
const SEG_Y: i32 = 36;
const SEG_W: i32 = 288;
const SEG_H: i32 = 30;
const LIST_X: i32 = 6;
const LIST_Y: i32 = 70;
const LIST_W: i32 = 288;
const LIST_H: i32 = 538;
const LIST_PAD: i32 = 4;
const ROW_H: i32 = 44;
const SEARCH_LEFT: i32 = 58;
const SEARCH_TOP: i32 = 4;
const SEARCH_W: i32 = 112;
const SEARCH_H: i32 = 30;
const SCROLL_STEP: i32 = ROW_H * 2;
const IDC_SEARCH: isize = 1001;
const ID_TIMER_CARET: usize = 1;
const ID_TIMER_PASTE: usize = 2;
const ID_TIMER_SCROLL_FADE: usize = 3;
const ID_TIMER_SETTINGS_SCROLLBAR: usize = 4; // settings 滚动条自动隐藏
pub(crate) const WM_TRAYICON: u32 = WM_APP + 1;
pub(crate) const TRAY_UID: u32 = 1;
pub(crate) const IDM_TRAY_TOGGLE: usize = 40001;
pub(crate) const IDM_TRAY_EXIT: usize = 40002;
const IDM_ROW_PASTE: usize = 41001;
const IDM_ROW_COPY: usize = 41002;
const IDM_ROW_PIN: usize = 41003;
const IDM_ROW_DELETE: usize = 41004;
const IDM_ROW_TO_PHRASE: usize = 41005;
const IDM_ROW_STICKER: usize = 41006;
const IDM_ROW_SAVE_IMAGE: usize = 41007;
const IDM_ROW_OPEN_PATH: usize = 41008;
const IDM_ROW_OPEN_FOLDER: usize = 41009;
const IDM_ROW_COPY_PATH: usize = 41010;
const IDM_ROW_GROUP_REMOVE: usize = 41011;
const IDM_ROW_EDIT: usize = 41012;
const IDM_ROW_QUICK_SEARCH: usize = 41013;
const IDM_ROW_EXPORT_FILE: usize = 41014;
const IDM_ROW_MAIL_MERGE: usize = 41015;
const IDM_ROW_GROUP_BASE: usize = 41100;
const IDM_GROUP_FILTER_ALL: usize = 41200;
const IDM_GROUP_FILTER_BASE: usize = 41210;
const HOTKEY_ID: i32 = 1;
const MAIN_UI_LAYOUT: MainUiLayout = MainUiLayout::zsclip();

const EN_CHANGE_CODE: u16 = 0x0300;
const MOD_ALT: u32 = 0x0001;
const MOD_CONTROL: u32 = 0x0002;
const MOD_SHIFT: u32 = 0x0004;
const MOD_WIN: u32 = 0x0008;
const MOD_NOREPEAT: u32 = 0x4000;
const TME_LEAVE: u32 = 0x00000002;
const WM_MOUSELEAVE: u32 = 0x02A3;

type AppResult<T> = Result<T, io::Error>;

const HOTKEY_MOD_OPTIONS: [&str; 8] = ["Win", "Ctrl", "Alt", "Shift", "Ctrl+Alt", "Ctrl+Shift", "Alt+Shift", "Ctrl+Alt+Shift"];
const HOTKEY_KEY_OPTIONS: [&str; 51] = [
    "A","B","C","D","E","F","G","H","I","J","K","L","M","N","O","P","Q","R","S","T","U","V","W","X","Y","Z",
    "0","1","2","3","4","5","6","7","8","9",
    "Space","Enter","Tab","Esc","Backspace","Delete","Insert","Up","Down","Left","Right","Home","End","PageUp","PageDown",
];
const SEARCH_ENGINE_PRESETS: [(&str, &str, &str); 6] = [
    ("jzxx",   "筑森搜索（jzxx.vip）", "https://jzxx.vip/search/more.html?type=11&key={q}&se=2"),
    ("bing",   "必应",                 "https://www.bing.com/search?q={q}"),
    ("baidu",  "百度",                 "https://www.baidu.com/s?wd={q}"),
    ("google", "Google",              "https://www.google.com/search?q={q}"),
    ("sogou",  "搜狗",                 "https://www.sogou.com/web?query={q}"),
    ("custom", "自定义",               "https://example.com/search?q={q}"),
];

#[derive(Clone, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct AppSettings {
    pub(crate) hotkey_enabled: bool,
    pub(crate) hotkey_mod: String,
    pub(crate) hotkey_key: String,
    pub(crate) click_hide: bool,
    pub(crate) edge_auto_hide: bool,
    pub(crate) hover_preview: bool,
    pub(crate) auto_start: bool,
    pub(crate) close_without_exit: bool,
    pub(crate) max_items: usize,
    pub(crate) show_pos_mode: String,
    pub(crate) show_mouse_dx: i32,
    pub(crate) show_mouse_dy: i32,
    pub(crate) show_fixed_x: i32,
    pub(crate) show_fixed_y: i32,
    pub(crate) quick_search_enabled: bool,
    pub(crate) search_engine: String,
    pub(crate) search_template: String,
    pub(crate) ai_clean_enabled: bool,
    pub(crate) super_mail_merge_enabled: bool,
    pub(crate) grouping_enabled: bool,
    pub(crate) cloud_sync_enabled: bool,
    pub(crate) cloud_sync_interval: String,
    pub(crate) cloud_webdav_url: String,
    pub(crate) cloud_webdav_user: String,
    pub(crate) cloud_webdav_pass: String,
    pub(crate) cloud_remote_dir: String,
    pub(crate) cloud_last_sync_status: String,
    pub(crate) last_window_x: i32,
    pub(crate) last_window_y: i32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            hotkey_enabled: true,
            hotkey_mod: "Win".to_string(),
            hotkey_key: "V".to_string(),
            click_hide: true,
            edge_auto_hide: false,
            hover_preview: false,
            auto_start: false,
            close_without_exit: true,
            max_items: 200,
            show_pos_mode: "last".to_string(),
            show_mouse_dx: 12,
            show_mouse_dy: 12,
            show_fixed_x: 120,
            show_fixed_y: 120,
            quick_search_enabled: true,
            search_engine: "jzxx".to_string(),
            search_template: search_engine_template("jzxx").to_string(),
            ai_clean_enabled: false,
            super_mail_merge_enabled: true,
            grouping_enabled: true,
            cloud_sync_enabled: false,
            cloud_sync_interval: "1小时".to_string(),
            cloud_webdav_url: String::new(),
            cloud_webdav_user: String::new(),
            cloud_webdav_pass: String::new(),
            cloud_remote_dir: "ZSClip".to_string(),
            cloud_last_sync_status: "未同步".to_string(),
            last_window_x: -1,
            last_window_y: -1,
        }
    }
}

fn search_engine_template(key: &str) -> &'static str {
    SEARCH_ENGINE_PRESETS.iter().find(|(k,_,_)| *k == key).map(|(_,_,tpl)| *tpl).unwrap_or(SEARCH_ENGINE_PRESETS[0].2)
}

fn search_engine_display(key: &str) -> &'static str {
    SEARCH_ENGINE_PRESETS.iter().find(|(k,_,_)| *k == key).map(|(_,name,_)| *name).unwrap_or(SEARCH_ENGINE_PRESETS[0].1)
}

fn search_engine_key_from_display(label: &str) -> &'static str {
    SEARCH_ENGINE_PRESETS.iter().find(|(_,name,_)| *name == label).map(|(k,_,_)| *k).unwrap_or("jzxx")
}

fn normalize_hotkey_mod(s: &str) -> String {
    let t = s.trim();
    if HOTKEY_MOD_OPTIONS.iter().any(|x| *x == t) { t.to_string() } else { "Win".to_string() }
}

fn normalize_hotkey_key(s: &str) -> String {
    let t = s.trim();
    if HOTKEY_KEY_OPTIONS.iter().any(|x| *x == t) { t.to_string() } else { "V".to_string() }
}

fn hotkey_preview_text(mod_label: &str, key_label: &str) -> String {
    format!("当前设置：{} + {}", normalize_hotkey_mod(mod_label), normalize_hotkey_key(key_label))
}

fn hotkey_mods_from_label(label: &str) -> u32 {
    match normalize_hotkey_mod(label).as_str() {
        "Ctrl" => MOD_CONTROL,
        "Alt" => MOD_ALT,
        "Shift" => MOD_SHIFT,
        "Ctrl+Alt" => MOD_CONTROL | MOD_ALT,
        "Ctrl+Shift" => MOD_CONTROL | MOD_SHIFT,
        "Alt+Shift" => MOD_ALT | MOD_SHIFT,
        "Ctrl+Alt+Shift" => MOD_CONTROL | MOD_ALT | MOD_SHIFT,
        _ => MOD_WIN,
    }
}

fn hotkey_vk_from_label(label: &str) -> u32 {
    let k = normalize_hotkey_key(label);
    if k.len() == 1 {
        let ch = k.as_bytes()[0] as char;
        if ch.is_ascii_alphabetic() { return ch.to_ascii_uppercase() as u32; }
        if ch.is_ascii_digit() { return ch as u32; }
    }
    match k.as_str() {
        "Space" => VK_SPACE as u32,
        "Enter" => VK_RETURN as u32,
        "Tab" => VK_TAB as u32,
        "Esc" => VK_ESCAPE as u32,
        "Backspace" => VK_BACK as u32,
        "Delete" => VK_DELETE as u32,
        "Insert" => VK_INSERT as u32,
        "Up" => VK_UP as u32,
        "Down" => VK_DOWN as u32,
        "Left" => VK_LEFT as u32,
        "Right" => VK_RIGHT as u32,
        "Home" => VK_HOME as u32,
        "End" => VK_END as u32,
        "PageUp" => VK_PRIOR as u32,
        "PageDown" => VK_NEXT as u32,
        _ => 'V' as u32,
    }
}

fn read_disabled_hotkeys_text() -> Option<String> {
    let out = Command::new("reg")
        .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "/v", "DisabledHotkeys"])
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
        let out = Command::new("reg")
            .args(["delete", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "/v", "DisabledHotkeys", "/f"])
            .output()
            .map_err(|e| e.to_string())?;
        if out.status.success() { return Ok(()); }
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        if stderr.contains("Unable to find") || stderr.contains("系统找不到指定") {
            return Ok(());
        }
        return Err(if stderr.trim().is_empty() { "删除注册表值失败".to_string() } else { stderr });
    }
    let out = Command::new("reg")
        .args(["add", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Explorer\Advanced", "/v", "DisabledHotkeys", "/t", "REG_SZ", "/d", txt, "/f"])
        .output()
        .map_err(|e| e.to_string())?;
    if out.status.success() { Ok(()) } else { Err(String::from_utf8_lossy(&out.stderr).to_string()) }
}

fn toggle_disabled_hotkey_char(ch: char, disable: bool) -> Result<(), String> {
    if !ch.is_ascii_alphanumeric() {
        return Err("无效按键".to_string());
    }
    let mut chars: BTreeSet<char> = read_disabled_hotkeys_text().unwrap_or_default().to_uppercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let up = ch.to_ascii_uppercase();
    if disable { chars.insert(up); } else { chars.remove(&up); }
    let new_text: String = chars.into_iter().collect();
    set_disabled_hotkeys_text(&new_text)
}

fn restart_explorer_shell() -> Result<(), String> {
    let _ = Command::new("taskkill").args(["/f", "/im", "explorer.exe"]).output();
    Command::new("explorer.exe").spawn().map(|_| ()).map_err(|e| e.to_string())
}

unsafe fn quick_search_open(settings: &AppSettings, text: &str) {
    if !settings.quick_search_enabled { return; }
    let mut raw = text.trim().to_string();
    if raw.is_empty() { return; }
    if raw.chars().count() > 200 { raw = raw.chars().take(200).collect(); }
    let q = raw.replace('\r', " ").replace('\n', " ");
    let enc = url_encode_component(&q);
    let tpl = if settings.search_template.trim().is_empty() { search_engine_template(&settings.search_engine).to_string() } else { settings.search_template.clone() };
    let url = tpl.replace("{key}", &enc).replace("{q}", &enc).replace("{raw}", &q);
    open_path_with_shell(&url);
}

fn url_encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3 / 2);
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => out.push(*b as char),
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", *b)),
        }
    }
    out
}

#[derive(Debug)]
struct DbItem {
    id: i64,
    kind: String,
    preview: String,
    text: Option<String>,
    file_paths: Option<String>,
    image_bytes: Option<Vec<u8>>,
    image_path: Option<String>,
    image_width: i64,
    image_height: i64,
    pinned: i64,
    group_id: i64,
    created_at: String,
}

#[derive(Clone, Copy)]
#[derive(Default)]
pub(crate) struct Icons {
    pub(crate) app: isize,
    pub(crate) search: isize,
    pub(crate) setting: isize,
    pub(crate) min: isize,
    pub(crate) close: isize,
    pub(crate) text: isize,
    pub(crate) image: isize,
    pub(crate) file: isize,
    pub(crate) folder: isize,
    pub(crate) pin: isize,
    pub(crate) del: isize,
}

impl Icons {
    fn destroy(&mut self) {
        unsafe {
            for icon in [
                &mut self.app,
                &mut self.search,
                &mut self.setting,
                &mut self.min,
                &mut self.close,
                &mut self.text,
                &mut self.image,
                &mut self.file,
                &mut self.folder,
                &mut self.pin,
                &mut self.del,
            ] {
                if *icon != 0 {
                    DestroyIcon(*icon as _);
                    *icon = 0;
                }
            }
        }
    }
}

pub(crate) struct AppState {
    pub(crate) search_hwnd: HWND,
    pub(crate) theme: Theme,
    pub(crate) icons: Icons,
    pub(crate) records: Vec<ClipItem>,
    pub(crate) phrases: Vec<ClipItem>,
    pub(crate) groups: Vec<ClipGroup>,
    pub(crate) list: ClipListState,
    pub(crate) hover_btn: &'static str,
    pub(crate) down_btn: &'static str,
    pub(crate) down_row: i32,
    pub(crate) down_x: i32,
    pub(crate) down_y: i32,
    pub(crate) hover_tab: i32,
    pub(crate) last_signature: String,
    pub(crate) ignore_clipboard_until: Option<Instant>,
    pub(crate) settings: AppSettings,
    pub(crate) hotkey_registered: bool,
    pub(crate) hotkey_conflict_notified: bool,
    pub(crate) settings_hwnd: HWND,
    pub(crate) hover_scroll: bool,   // 鼠标是否在滚动条区域
    pub(crate) scroll_fade_alpha: u8, // 滚动条透明度 0-255
    pub(crate) scroll_fade_timer: bool, // 渐隐 timer 是否运行中
    pub(crate) paste_return_to_main: bool,
}

impl Deref for AppState {
    type Target = ClipListState;

    fn deref(&self) -> &Self::Target {
        &self.list
    }
}

impl DerefMut for AppState {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.list
    }
}


impl AppState {
    fn new(_hwnd: HWND, search_hwnd: HWND, icons: Icons) -> Self {
        let mut s = Self {
            search_hwnd,
            theme: Theme::default(),
            icons,
            records: Vec::new(),
            phrases: Vec::new(),
            groups: Vec::new(),
            list: ClipListState::default(),
            hover_btn: "",
            down_btn: "",
            down_row: -1,
            down_x: 0,
            down_y: 0,
            hover_tab: -1,
            last_signature: String::new(),
            ignore_clipboard_until: None,
            settings: load_settings(),
            hotkey_registered: false,
            hotkey_conflict_notified: false,
            settings_hwnd: null_mut(),
            hover_scroll: false,
            scroll_fade_alpha: 0,
            scroll_fade_timer: false,
            paste_return_to_main: false,
        };
        s.refilter();
        s
    }

    fn active_items(&self) -> &Vec<ClipItem> {
        if self.tab_index == 0 {
            &self.records
        } else {
            &self.phrases
        }
    }

    fn active_items_mut(&mut self) -> &mut Vec<ClipItem> {
        if self.tab_index == 0 {
            &mut self.records
        } else {
            &mut self.phrases
        }
    }

    fn current_item(&self) -> Option<&ClipItem> {
        if self.sel_idx < 0 {
            return None;
        }
        let visible_idx = self.sel_idx as usize;
        let src_idx = *self.filtered_indices.get(visible_idx)?;
        self.active_items().get(src_idx)
    }

    fn current_item_mut(&mut self) -> Option<&mut ClipItem> {
        if self.sel_idx < 0 {
            return None;
        }
        let visible_idx = self.sel_idx as usize;
        let src_idx = *self.filtered_indices.get(visible_idx)?;
        self.active_items_mut().get_mut(src_idx)
    }

    fn refilter(&mut self) {
        let grouping_enabled = self.settings.grouping_enabled;
        let items = if self.list.tab_index == 0 {
            &self.records
        } else {
            &self.phrases
        };
        self.list.refilter_with(items, grouping_enabled);
        self.clamp_scroll();
    }

    fn clear_selection(&mut self) {
        self.list.clear_selection();
        self.down_row = -1;
        self.down_x = 0;
        self.down_y = 0;
    }

    fn row_is_selected(&self, visible_idx: i32) -> bool {
        self.list.row_is_selected(visible_idx)
    }

    fn selected_source_indices(&self) -> Vec<usize> {
        self.list.selected_source_indices()
    }

    fn delete_selected_rows(&mut self) {
        let mut src = self.selected_source_indices();
        if src.is_empty() {
            self.delete_selected();
            return;
        }
        src.sort_unstable_by(|a,b| b.cmp(a));
        for src_idx in src {
            if src_idx < self.active_items().len() {
                let iid = self.active_items()[src_idx].id;
                if iid > 0 { let _ = db_delete_item(iid); }
                self.active_items_mut().remove(src_idx);
            }
        }
        self.clear_selection();
        self.refilter();
    }

    fn toggle_pin_rows(&mut self) {
        let src = self.selected_source_indices();
        if src.is_empty() {
            self.toggle_pin_selected();
            return;
        }
        let make_pinned = src.iter().filter_map(|&i| self.active_items().get(i)).any(|it| !it.pinned);
        for src_idx in src {
            if let Some(it) = self.active_items_mut().get_mut(src_idx) {
                it.pinned = make_pinned;
                if it.id > 0 { let _ = db_update_item_pinned(it.id, it.pinned); }
            }
        }
        self.sort_active_items();
        self.refilter();
    }

    fn selected_items_owned(&self) -> Vec<ClipItem> {
        self.selected_source_indices().into_iter().filter_map(|i| self.active_items().get(i).cloned()).collect()
    }

    fn context_selection_count(&self) -> usize {
        self.list.context_selection_count()
    }

    fn context_selection_has_unpinned(&self) -> bool {
        let items = self.selected_items_owned();
        if items.is_empty() {
            self.current_item().map(|it| !it.pinned).unwrap_or(false)
        } else {
            items.iter().any(|it| !it.pinned)
        }
    }

    fn sort_active_items(&mut self) {
        self.active_items_mut().sort_by(|a, b| b.pinned.cmp(&a.pinned));
    }

    fn add_clip_item(&mut self, mut item: ClipItem, signature: String) {
        if self.tab_index == 0 && self.last_signature == signature {
            return;
        }
        self.last_signature = signature;

        let records = &mut self.records;
        if let Some(first) = records.first() {
            if first.preview == item.preview && first.kind == item.kind {
                return;
            }
        }
        item.id = db_insert_item(0, &item).unwrap_or(0);
        // 回填内存中的 created_at（DB 由 CURRENT_TIMESTAMP 自动填写，内存补齐以便时间分组标头正常工作）
        if item.created_at.is_empty() {
            item.created_at = now_utc_sqlite();
        }
        records.insert(0, item);
        let max_items = self.settings.max_items; // 0 = 无限制；仅限制非置顶条目
        if max_items > 0 {
            let unpinned_count = records.iter().filter(|r| !r.pinned).count();
            if unpinned_count > max_items {
                let mut need_remove = unpinned_count - max_items;
                let mut to_remove = Vec::new();
                for rec in records.iter().rev() {
                    if need_remove == 0 {
                        break;
                    }
                    if !rec.pinned {
                        to_remove.push(rec.id);
                        need_remove -= 1;
                    }
                }
                if !to_remove.is_empty() {
                    for id in &to_remove {
                        let _ = db_delete_item(*id);
                    }
                    records.retain(|r| !to_remove.contains(&r.id));
                }
            }
        }
        if self.tab_index == 0 {
            self.sel_idx = 0;
        }
        self.refilter();
    }


    fn list_view_height(&self) -> i32 {
        MAIN_UI_LAYOUT.list_view_height()
    }

    fn total_content_height(&self) -> i32 {
        MAIN_UI_LAYOUT.total_content_height(self.filtered_indices.len())
    }

    fn clamp_scroll(&mut self) {
        self.scroll_y = MAIN_UI_LAYOUT.clamp_scroll(self.scroll_y, self.filtered_indices.len());
    }

    fn ensure_visible(&mut self, idx: i32) {
        self.scroll_y = MAIN_UI_LAYOUT.ensure_visible(self.scroll_y, idx, self.filtered_indices.len());
    }

    fn row_rect(&self, visible_idx: i32) -> Option<RECT> {
        MAIN_UI_LAYOUT
            .row_rect(visible_idx, self.filtered_indices.len(), self.scroll_y)
            .map(Into::into)
    }

    fn quick_delete_rect(&self, visible_idx: i32) -> Option<RECT> {
        MAIN_UI_LAYOUT
            .quick_delete_rect(visible_idx, self.filtered_indices.len(), self.scroll_y)
            .map(Into::into)
    }

    fn search_rect(&self) -> RECT {
        MAIN_UI_LAYOUT.search_rect().into()
    }

    fn title_button_rect(&self, key: &str) -> RECT {
        MAIN_UI_LAYOUT.title_button_rect(key).into()
    }

    fn segment_rects(&self) -> (RECT, RECT) {
        let (left, right) = MAIN_UI_LAYOUT.segment_rects();
        (left.into(), right.into())
    }

    fn scrollbar_track_rect(&self) -> Option<RECT> {
        MAIN_UI_LAYOUT
            .scrollbar_track_rect(self.filtered_indices.len())
            .map(Into::into)
    }

    fn scrollbar_thumb_rect(&self) -> Option<RECT> {
        MAIN_UI_LAYOUT
            .scrollbar_thumb_rect(self.filtered_indices.len(), self.scroll_y)
            .map(Into::into)
    }

    fn delete_selected(&mut self) {
        if self.sel_idx < 0 {
            return;
        }
        let visible_idx = self.sel_idx as usize;
        if let Some(src_idx) = self.filtered_indices.get(visible_idx).copied() {
            if src_idx < self.active_items().len() {
                let iid = self.active_items()[src_idx].id;
                if iid > 0 {
                    let _ = db_delete_item(iid);
                }
                self.active_items_mut().remove(src_idx);
                self.refilter();
                if self.sel_idx >= self.filtered_indices.len() as i32 {
                    self.sel_idx = self.filtered_indices.len() as i32 - 1;
                }
            }
        }
    }

    fn toggle_pin_selected(&mut self) {
        if let Some(it) = self.current_item_mut() {
            it.pinned = !it.pinned;
            if it.id > 0 {
                let _ = db_update_item_pinned(it.id, it.pinned);
            }
        }
        self.sort_active_items();
        self.refilter();
    }

    fn selected_db_ids(&self) -> Vec<i64> {
        self.selected_source_indices()
            .into_iter()
            .filter_map(|i| self.active_items().get(i).map(|it| it.id))
            .filter(|id| *id > 0)
            .collect()
    }
}

fn current_exe_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

fn data_dir() -> PathBuf {
    let dir = current_exe_path()
        .and_then(|p| p.parent().map(|d| d.join("data")))
        .unwrap_or_else(|| PathBuf::from("data"));
    let _ = fs::create_dir_all(&dir);
    dir
}

pub(crate) fn db_file() -> PathBuf {
    data_dir().join("clipboard.db")
}

fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

fn load_settings() -> AppSettings {

    match fs::read_to_string(settings_file()) {
        Ok(txt) => serde_json::from_str(&txt).unwrap_or_default(),
        Err(_) => AppSettings::default(),
    }
}

pub(crate) fn save_settings(s: &AppSettings) {
    let _ = fs::create_dir_all(data_dir());
    if let Ok(txt) = serde_json::to_string_pretty(s) {
        let _ = fs::write(settings_file(), txt);
    }
}

fn autostart_command_for_current_exe() -> Option<String> {
    current_exe_path().map(|p| format!("\"{}\"", p.to_string_lossy()))
}

fn normalize_run_target(value: &str) -> String {
    let s = value.trim();
    let target = if let Some(rest) = s.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else {
        s.split_whitespace().next().unwrap_or("")
    };
    target
        .trim_matches('"')
        .replace('/', "\\")
        .trim()
        .to_ascii_lowercase()
}

/// 读取注册表判断当前是否已启用开机自启
fn is_autostart_enabled() -> bool {
    unsafe {
        let run_key = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let val_name = to_wide(APP_TITLE);
        let mut hkey: isize = 0;
        if RegOpenKeyExW(HKEY_CURRENT_USER_VAL, run_key.as_ptr(), 0, KEY_READ_VAL, &mut hkey) != 0 {
            return false;
        }

        let mut data_size = 0u32;
        let mut reg_type = 0u32;
        let ret = RegQueryValueExW(hkey, val_name.as_ptr(), null_mut(), &mut reg_type, null_mut(), &mut data_size);
        if ret != 0 || reg_type != REG_SZ || data_size < 2 {
            RegCloseKey(hkey);
            return false;
        }

        let mut data = vec![0u8; data_size as usize];
        let ret = RegQueryValueExW(hkey, val_name.as_ptr(), null_mut(), &mut reg_type, data.as_mut_ptr(), &mut data_size);
        RegCloseKey(hkey);
        if ret != 0 || reg_type != REG_SZ {
            return false;
        }

        let wide = std::slice::from_raw_parts(data.as_ptr() as *const u16, (data_size as usize) / 2);
        let value_len = wide.iter().position(|&c| c == 0).unwrap_or(wide.len());
        let reg_value = String::from_utf16_lossy(&wide[..value_len]);

        if let Some(exe) = current_exe_path() {
            normalize_run_target(&reg_value) == normalize_run_target(&exe.to_string_lossy())
        } else {
            !reg_value.trim().is_empty()
        }
    }
}

/// 设置/取消开机自启（写 HKCU\Software\Microsoft\Windows\CurrentVersion\Run）
fn apply_autostart(enabled: bool) {
    unsafe {
        let run_key = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let val_name = to_wide(APP_TITLE);
        let mut hkey: isize = 0;
        let flags = KEY_READ_VAL | KEY_SET_VALUE;
        if RegOpenKeyExW(HKEY_CURRENT_USER_VAL, run_key.as_ptr(), 0, flags, &mut hkey) != 0 {
            return;
        }
        if enabled {
            if let Some(cmdline) = autostart_command_for_current_exe() {
                let wide = to_wide(&cmdline);
                let bytes = std::slice::from_raw_parts(
                    wide.as_ptr() as *const u8,
                    wide.len() * 2,
                );
                RegSetValueExW(hkey, val_name.as_ptr(), 0, REG_SZ, bytes.as_ptr(), bytes.len() as u32);
            }
        } else {
            RegDeleteValueW(hkey, val_name.as_ptr());
        }
        RegCloseKey(hkey);
    }
}

fn row_to_clip_item(row: DbItem) -> ClipItem {
    ClipItem {
        id: row.id,
        kind: match row.kind.as_str() {
            "image" => ClipKind::Image,
            "phrase" => ClipKind::Phrase,
            "files" => ClipKind::Files,
            _ => ClipKind::Text,
        },
        preview: row.preview,
        text: row.text,
        file_paths: row.file_paths.map(|v| v.split("\n").map(|s| s.to_string()).collect()),
        image_bytes: row.image_bytes,
        image_path: row.image_path,
        image_width: row.image_width.max(0) as usize,
        image_height: row.image_height.max(0) as usize,
        pinned: row.pinned == 1,
        group_id: row.group_id,
        created_at: row.created_at,
    }
}

fn db_load_items(category: i64) -> Vec<ClipItem> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, kind, preview, text_data, file_paths, image_path, image_width, image_height, pinned, group_id, \
             COALESCE(created_at, '') as created_at \
             FROM items WHERE category=? ORDER BY pinned DESC, id DESC"
        )?;
        let rows = stmt.query_map(params![category], |r| {
            Ok(DbItem {
                id: r.get(0)?,
                kind: r.get(1)?,
                preview: r.get(2)?,
                text: r.get(3)?,
                file_paths: r.get(4)?,
                image_path: r.get(5)?,
                image_bytes: None,
                image_width: r.get(6)?,
                image_height: r.get(7)?,
                pinned: r.get(8)?,
                group_id: r.get(9)?,
                created_at: r.get(10)?,
            })
        })?;
        Ok(rows.filter_map(|r| r.ok().map(row_to_clip_item)).collect())
    }).unwrap_or_default()
}

/// 按需加载完整数据（粘贴/复制时调用）
fn db_load_item_full(id: i64) -> Option<ClipItem> {
    with_db(|conn| {
        conn.query_row(
            "SELECT id, kind, preview, text_data, file_paths, image_data, image_width, image_height, pinned, group_id, image_path, \
             COALESCE(created_at, '') as created_at \
             FROM items WHERE id=?",
            params![id],
            |r| Ok(row_to_clip_item(DbItem {
                id: r.get(0)?,
                kind: r.get(1)?,
                preview: r.get(2)?,
                text: r.get(3)?,
                file_paths: r.get(4)?,
                image_bytes: r.get(5)?,
                image_path: r.get(10)?,
                image_width: r.get(6)?,
                image_height: r.get(7)?,
                pinned: r.get(8)?,
                group_id: r.get(9)?,
                created_at: r.get(11)?,
            }))
        )
    }).ok()
}

fn db_insert_item(category: i64, item: &ClipItem) -> rusqlite::Result<i64> {
    let kind = match item.kind {
        ClipKind::Image => "image",
        ClipKind::Phrase => "phrase",
        ClipKind::Files => "files",
        _ => "text",
    };
    let preview = item.preview.clone();
    let text_data = item.text.clone();
    let file_paths = item.file_paths.as_ref().map(|v| v.join("\n"));
    let image_data = item.image_bytes.clone();
    let image_path = item.image_path.clone();
    with_db(|conn| {
        conn.execute(
            "INSERT INTO items(category, kind, preview, text_data, file_paths, image_data, image_path, image_width, image_height, pinned, group_id)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                category, kind, preview, text_data, file_paths, image_data, image_path,
                item.image_width as i64, item.image_height as i64, if item.pinned { 1 } else { 0 }, item.group_id,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    })
}

fn db_update_item_pinned(id: i64, pinned: bool) -> rusqlite::Result<()> {
    with_db(|conn| {
        conn.execute("UPDATE items SET pinned=? WHERE id=?", params![if pinned {1} else {0}, id])?;
        Ok(())
    })
}

/// 当条目数量超过上限时，从 DB 删除最旧的非置顶记录及其图片数据
fn db_prune_items(max_items: usize) {
    if max_items == 0 { return; }
    let _ = with_db(|conn| {
        let count: i64 = conn.query_row("SELECT COUNT(*) FROM items WHERE pinned=0", [], |r| r.get::<_, i64>(0)).unwrap_or(0);
        let excess = count - max_items as i64;
        if excess > 0 {
            conn.execute(
                "DELETE FROM items WHERE id IN (SELECT id FROM items WHERE pinned=0 ORDER BY id ASC LIMIT ?)",
                params![excess],
            )?;
        }
        Ok(())
    });
}

fn db_delete_item(id: i64) -> rusqlite::Result<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM items WHERE id=?", params![id])?;
        Ok(())
    })
}

fn db_load_groups() -> Vec<ClipGroup> {
    with_db(|conn| {
        let mut stmt = conn.prepare("SELECT id, name FROM clip_groups ORDER BY sort_order ASC, id ASC")?;
        let rows = stmt.query_map([], |r| Ok(ClipGroup { id: r.get(0)?, name: r.get(1)? }))?;
        Ok(rows.filter_map(|r| r.ok()).collect())
    }).unwrap_or_default()
}



fn db_delete_group(group_id: i64) -> rusqlite::Result<()> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        tx.execute("UPDATE items SET group_id=0 WHERE group_id=?", params![group_id])?;
        tx.execute("DELETE FROM clip_groups WHERE id=?", params![group_id])?;
        tx.commit()?;
        Ok(())
    })
}

fn db_set_groups_order(group_ids: &[i64]) -> rusqlite::Result<()> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        for (idx, gid) in group_ids.iter().enumerate() {
            tx.execute("UPDATE clip_groups SET sort_order=? WHERE id=?", params![idx as i64 + 1, *gid])?;
        }
        tx.commit()?;
        Ok(())
    })
}

fn db_assign_group(item_ids: &[i64], group_id: i64) -> rusqlite::Result<()> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        for item_id in item_ids {
            tx.execute("UPDATE items SET group_id=? WHERE id=?", params![group_id, item_id])?;
        }
        tx.commit()?;
        Ok(())
    })
}


fn db_rename_group(group_id: i64, new_name: &str) -> rusqlite::Result<()> {
    with_db(|conn| {
        conn.execute("UPDATE clip_groups SET name=? WHERE id=?", params![new_name, group_id])?;
        Ok(())
    })
}

fn db_create_named_group(name: &str) -> rusqlite::Result<ClipGroup> {
    with_db(|conn| {
        let next_sort: i64 = conn.query_row("SELECT COALESCE(MAX(sort_order), 0) + 1 FROM clip_groups", [], |r| r.get(0)).unwrap_or(1);
        conn.execute("INSERT INTO clip_groups(name, sort_order) VALUES(?, ?)", params![name, next_sort])?;
        Ok(ClipGroup { id: conn.last_insert_rowid(), name: name.to_string() })
    })
}

fn db_update_item_text(item_id: i64, new_text: &str) -> rusqlite::Result<()> {
    let preview: String = new_text.chars().take(120).collect();
    with_db(|conn| {
        conn.execute("UPDATE items SET text_data=?, preview=? WHERE id=?", params![new_text, preview, item_id])?;
        Ok(())
    })
}

fn db_add_phrase_from_item(item: &ClipItem) -> rusqlite::Result<i64> {
    let mut clone = item.clone();
    clone.id = 0;
    clone.file_paths = None;
    clone.image_bytes = None;
    clone.image_width = 0;
    clone.image_height = 0;
    clone.kind = ClipKind::Phrase;
    if clone.text.is_none() {
        clone.text = Some(clone.preview.clone());
    }
    db_insert_item(1, &clone)
}

fn register_hotkey_for(hwnd: HWND, state: &mut AppState) {
    unregister_hotkey_for(hwnd, state);
    if !state.settings.hotkey_enabled {
        return;
    }
    let mods = hotkey_mods_from_label(&state.settings.hotkey_mod);
    let vk = hotkey_vk_from_label(&state.settings.hotkey_key);
    let ok = unsafe { RegisterHotKey(hwnd, HOTKEY_ID, mods | MOD_NOREPEAT, vk) };
    if ok != 0 {
        state.hotkey_registered = true;
        state.hotkey_conflict_notified = false;
        return;
    }

    state.hotkey_registered = false;
    let err = unsafe { GetLastError() };
    if err == ERROR_HOTKEY_ALREADY_REGISTERED && !state.hotkey_conflict_notified {
        state.hotkey_conflict_notified = true;
        let hk = hotkey_preview_text(&state.settings.hotkey_mod, &state.settings.hotkey_key).replace("当前设置：", "");
        unsafe {
            MessageBoxW(
                hwnd,
                to_wide(&format!("快捷键 {} 已被其他程序或系统占用，当前不会注册全局热键。请在设置-快捷键中改用其他组合。", hk)).as_ptr(),
                to_wide("快捷键冲突").as_ptr(),
                MB_OK | MB_ICONWARNING,
            );
        }
    }
}

fn unregister_hotkey_for(hwnd: HWND, state: &mut AppState) {
    if state.hotkey_registered {
        unsafe { UnregisterHotKey(hwnd, HOTKEY_ID); }
        state.hotkey_registered = false;
    }
}


struct SettingsWndState {
    parent_hwnd: HWND,
    cur_page: usize,
    nav_hot: i32,
    content_scroll_y: i32,           // 内容区当前滚动偏移（像素）
    scroll_dragging: bool,            // 正在拖拽滚动条拇指
    scroll_drag_start_y: i32,         // 拖拽起始鼠标Y
    scroll_drag_start_scroll: i32,    // 拖拽起始scroll_y
    scroll_bar_visible: bool,         // 滚动条当前是否可见（auto-hide）
    scroll_hide_timer: bool,          // 渐隐 timer 是否运行中
    ui: SettingsUiRegistry,
    btn_save: HWND,
    btn_close: HWND,
    btn_open_cfg: HWND,
    btn_open_db: HWND,
    btn_open_data: HWND,
    chk_autostart: HWND,
    chk_close_tray: HWND,
    chk_click_hide: HWND,
    chk_edge_hide: HWND,
    chk_hover_preview: HWND,
    chk_group_enable: HWND,
    lb_group_current: HWND,
    lb_groups: HWND,
    btn_group_add: HWND,
    btn_group_rename: HWND,
    btn_group_delete: HWND,
    btn_group_up: HWND,
    btn_group_down: HWND,
    groups_cache: Vec<ClipGroup>,
    chk_hk_enable: HWND,
    cb_hk_mod: HWND,
    cb_hk_key: HWND,
    lb_hk_preview: HWND,
    btn_clip_hist_block: HWND,
    btn_clip_hist_restore: HWND,
    btn_restart_explorer: HWND,
    chk_qs: HWND,
    cb_engine: HWND,
    ed_tpl: HWND,
    chk_ai: HWND,
    chk_mm: HWND,
    chk_cloud_enable: HWND,
    cb_cloud_interval: HWND,
    ed_cloud_url: HWND,
    ed_cloud_user: HWND,
    ed_cloud_pass: HWND,
    ed_cloud_dir: HWND,
    lb_cloud_status: HWND,
    cb_max: HWND,
    cb_pos: HWND,
    ed_dx: HWND,
    ed_dy: HWND,
    ed_fx: HWND,
    ed_fy: HWND,
    nav_font: *mut core::ffi::c_void,
    ui_font: *mut core::ffi::c_void,
    title_font: *mut core::ffi::c_void,
    draft: AppSettings,
    ownerdraw_ctrls: Vec<HWND>,
    hot_ownerdraw: HWND,
    bg_brush: *mut core::ffi::c_void,
    surface_brush: *mut core::ffi::c_void,
    control_brush: *mut core::ffi::c_void,
    nav_brush: *mut core::ffi::c_void,
    dropdown_popup: HWND,
}




unsafe fn settings_set_text(hwnd: HWND, s: &str) {
    SetWindowTextW(hwnd, to_wide(s).as_ptr());
}

unsafe fn settings_group_current_filter_text(st: &SettingsWndState) -> String {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() { return "全部记录".to_string(); }
    let app = &*pst;
    let gid = app.current_group_filter;
    if gid == 0 {
        return if app.tab_index == 0 { "全部记录".to_string() } else { "全部短语".to_string() };
    }
    app.groups.iter().find(|g| g.id == gid).map(|g| g.name.clone()).unwrap_or_else(|| format!("分组 #{}", gid))
}

unsafe fn settings_invalidate_page_ctrls(hwnd: HWND, st: &SettingsWndState, page: usize) {
    for reg in st.ui.page_regs(page) {
        if !reg.hwnd.is_null() { InvalidateRect(reg.hwnd, null(), 1); }
    }
    let mut rc: RECT = core::mem::zeroed();
    if GetClientRect(hwnd, &mut rc) != 0 {
        let viewport = settings_viewport_rect(&rc);
        InvalidateRect(hwnd, &viewport, 0);
    }
}

unsafe fn settings_sync_page_state(st: &mut SettingsWndState, page: usize) {
    match SettingsPage::from_index(page) {
        SettingsPage::General => {
            settings_sync_pos_fields_enabled(st);
        }
        SettingsPage::Hotkey => {
            let s = &st.draft;
            settings_set_text(st.cb_hk_mod, &normalize_hotkey_mod(&s.hotkey_mod));
            settings_set_text(st.cb_hk_key, &normalize_hotkey_key(&s.hotkey_key));
            settings_set_text(st.lb_hk_preview, &hotkey_preview_text(&s.hotkey_mod, &s.hotkey_key));
        }
        SettingsPage::Plugin => {
            let s = &st.draft;
            settings_set_text(st.cb_engine, search_engine_display(&s.search_engine));
            settings_set_text(st.ed_tpl, &s.search_template);
        }
        SettingsPage::Group => {
            let text = format!("当前分组：{}", settings_group_current_filter_text(st));
            settings_set_text(st.lb_group_current, &text);
            let pst = get_state_ptr(st.parent_hwnd);
            let gid = if pst.is_null() { 0 } else { (&*pst).current_group_filter };
            settings_groups_refresh_list(st, gid);
        }
        SettingsPage::Cloud => {
            let s = &st.draft;
            settings_set_text(st.cb_cloud_interval, &s.cloud_sync_interval);
            settings_set_text(st.ed_cloud_url, &s.cloud_webdav_url);
            settings_set_text(st.ed_cloud_user, &s.cloud_webdav_user);
            settings_set_text(st.ed_cloud_pass, &s.cloud_webdav_pass);
            settings_set_text(st.ed_cloud_dir, &s.cloud_remote_dir);
            settings_set_text(st.lb_cloud_status, &format!("上次同步：{}", s.cloud_last_sync_status));
        }
        SettingsPage::About => {}
    }
    settings_invalidate_page_ctrls(st.parent_hwnd, st, page);
}

unsafe fn settings_refresh_theme_resources(st: &mut SettingsWndState) {
    if !st.bg_brush.is_null() { DeleteObject(st.bg_brush as _); }
    if !st.surface_brush.is_null() { DeleteObject(st.surface_brush as _); }
    if !st.control_brush.is_null() { DeleteObject(st.control_brush as _); }
    if !st.nav_brush.is_null() { DeleteObject(st.nav_brush as _); }
    let th = Theme::default();
    st.bg_brush = CreateSolidBrush(th.bg) as _;
    st.surface_brush = CreateSolidBrush(th.surface) as _;
    st.control_brush = CreateSolidBrush(th.control_bg) as _;
    st.nav_brush = CreateSolidBrush(th.nav_bg) as _;
}

unsafe fn settings_set_font(hwnd: HWND, hfont: *mut core::ffi::c_void) {
    if !hwnd.is_null() && !hfont.is_null() {
        SendMessageW(hwnd, WM_SETFONT, hfont as usize, 1);
    }
}

unsafe fn settings_create_label(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32, font: *mut core::ffi::c_void) -> HWND {
    host_create_settings_label(parent, text, x, y, w, h, font)
}

unsafe fn settings_create_label_auto(parent: HWND, text: &str, x: i32, y: i32, w: i32, min_h: i32, font: *mut core::ffi::c_void) -> (HWND, i32) {
    host_create_settings_label_auto(parent, text, x, y, w, min_h, font)
}

unsafe fn settings_create_btn(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    create_settings_component(parent, text, id, SettingsComponentKind::Button, x, y, w, 32, font)
}

unsafe fn settings_create_small_btn(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    settings_create_btn(parent, text, id, x, y, w, font)
}

unsafe fn settings_create_dropdown_btn(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    create_settings_component(parent, text, id, SettingsComponentKind::Dropdown, x, y, w, 32, font)
}

unsafe fn settings_create_toggle_plain(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> (HWND, HWND, i32, i32, i32, i32, i32, i32) {
    const SS_CENTERIMAGE: u32 = 0x0200;
    let toggle_w = 44;
    let toggle_h = 24;
    let row_h    = 32;
    let gap      = 12;
    let label_w  = max(40, w - toggle_w - gap);
    let label = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        to_wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | SS_CENTERIMAGE,
        x, y, label_w, row_h,
        parent, null_mut(), GetModuleHandleW(null()), null(),
    );
    settings_set_font(label, font);

    let btn_x = x + w - toggle_w;
    let btn_y = y + max(0, (row_h - toggle_h) / 2);
    let btn = create_settings_component(parent, "", id, SettingsComponentKind::Toggle, btn_x, btn_y, toggle_w, toggle_h, font);
    (label, btn, x, y, label_w, row_h, btn_x, btn_y)
}

unsafe fn settings_create_toggle(parent: HWND, st: &mut SettingsWndState, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    let (label, btn, lx, ly, lw, lh, btn_x, btn_y) = settings_create_toggle_plain(parent, text, id, x, y, w, font);
    settings_page0_push_ctrl(st, label, lx, ly, lw, lh);
    settings_page0_push_ctrl(st, btn, btn_x, btn_y, 44, 24);
    if !btn.is_null() { st.ownerdraw_ctrls.push(btn); }
    btn
}



unsafe fn settings_create_edit(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    host_create_settings_edit(parent, text, id, x, y, w, font)
}

unsafe fn settings_create_password_edit(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    host_create_settings_password_edit(parent, text, id, x, y, w, font)
}


/// 计算最大可滚动量
fn settings_max_scroll(view_h: i32) -> i32 {
    (SETTINGS_CONTENT_TOTAL_H - view_h).max(0)
}


/// 向 page0_ctrls 和 ctrl_origins 同时注册控件（在创建时调用，用创建参数记录坐标）
unsafe fn settings_register_ctrl(st: &mut SettingsWndState, page: usize, hwnd: HWND, x: i32, y: i32, w: i32, h: i32, scrollable: bool) {
    if hwnd.is_null() { return; }
    st.ui.register(SettingsCtrlReg::new(hwnd, page, x, y, w, h, scrollable));
}

unsafe fn settings_page_push_ctrl(st: &mut SettingsWndState, page: usize, hwnd: HWND) {
    settings_register_ctrl(st, page, hwnd, 0, 0, 0, 0, false);
}

unsafe fn settings_page0_push_ctrl(st: &mut SettingsWndState, hwnd: HWND, x: i32, y: i32, w: i32, h: i32) {
    settings_register_ctrl(st, 0, hwnd, x, y, w, h, true);
}


/// 根据 content_scroll_y 批量重定位「当前页」内容区控件
/// 超出内容视口的控件自动隐藏，防止溢出到标题区
unsafe fn settings_repos_controls(hwnd: HWND, st: &SettingsWndState) {
    if st.ui.scroll_ctrls().is_empty() { return; }
    if st.cur_page != SettingsPage::General.index() { return; }

    let mut crc: RECT = core::mem::zeroed();
    GetClientRect(hwnd, &mut crc);
    let viewport = settings_viewport_rect(&crc);

    // 记录旧/新区域，滚动后精确失效，避免遗留白块
    let mut dirty: Vec<RECT> = Vec::with_capacity(st.ui.scroll_ctrls().len() * 2);

    let hdwp = BeginDeferWindowPos(st.ui.scroll_ctrls().len() as i32);
    if hdwp.is_null() { return; }
    let mut hdwp = hdwp;
    for slot in st.ui.scroll_ctrls() {
        let hchild = slot.hwnd;
        let ox = slot.bounds.left;
        let oy = slot.bounds.top;
        let ow = slot.bounds.right - slot.bounds.left;
        let oh = slot.bounds.bottom - slot.bounds.top;
        if hchild.is_null() { continue; }

        let mut wr: RECT = core::mem::zeroed();
        if GetWindowRect(hchild, &mut wr) != 0 {
            let mut tl = POINT { x: wr.left, y: wr.top };
            let mut br = POINT { x: wr.right, y: wr.bottom };
            ScreenToClient(hwnd, &mut tl);
            ScreenToClient(hwnd, &mut br);
            dirty.push(RECT { left: tl.x, top: tl.y, right: br.x, bottom: br.y });
        }

        let new_y = oy - st.content_scroll_y;
        let visible = settings_child_visible(new_y, oh, &viewport);
        dirty.push(RECT { left: ox, top: new_y, right: ox + ow, bottom: new_y + oh });

        let flags = SWP_NOZORDER | SWP_NOACTIVATE | SWP_NOCOPYBITS
            | if visible { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW };
        let r = DeferWindowPos(hdwp, hchild, null_mut(), ox, new_y, ow, oh, flags);
        if !r.is_null() { hdwp = r; }
    }
    EndDeferWindowPos(hdwp);

    // 立即刷新当前页子控件，避免滚动过程先出现白框、结束后才补画
    for slot in st.ui.scroll_ctrls() {
        let hchild = slot.hwnd;
        let oy = slot.bounds.top;
        let oh = slot.bounds.bottom - slot.bounds.top;
        if hchild.is_null() { continue; }
        let new_y = oy - st.content_scroll_y;
        if settings_child_visible(new_y, oh, &viewport) {
            InvalidateRect(hchild, null(), 0);
        }
    }

    for mut rc in dirty {
        if rc.right <= rc.left || rc.bottom <= rc.top { continue; }
        if rc.left < viewport.left { rc.left = viewport.left; }
        if rc.top < viewport.top { rc.top = viewport.top; }
        if rc.right > viewport.right { rc.right = viewport.right; }
        if rc.bottom > viewport.bottom { rc.bottom = viewport.bottom; }
        if rc.right > rc.left && rc.bottom > rc.top {
            InvalidateRect(hwnd, &rc, 0);
        }
    }
}

/// 滚动到指定位置，重定位控件并重绘
unsafe fn settings_scroll_to(hwnd: HWND, st: &mut SettingsWndState, new_y: i32) {
    let mut crc: RECT = core::mem::zeroed();
    GetClientRect(hwnd, &mut crc);
    let view_h = (crc.bottom - crc.top) - SETTINGS_CONTENT_Y;
    let new_y = new_y.clamp(0, settings_max_scroll(view_h));
    let old_y = st.content_scroll_y;
    if new_y == old_y { return; }
    st.content_scroll_y = new_y;
    settings_scrollbar_show(hwnd, st);

    let viewport = settings_viewport_rect(&crc);

    // 不再依赖 ScrollWindowEx 复制旧像素，直接重定位子控件并立即精确重绘，避免滚动白框残留
    settings_repos_controls(hwnd, st);

    InvalidateRect(hwnd, &viewport, 0);
    let mask = settings_viewport_mask_rect(&crc);
    InvalidateRect(hwnd, &mask, 0);
    let scroll_strip = RECT {
        left: crc.right - SCROLL_BAR_W_ACTIVE - SCROLL_BAR_MARGIN - 4,
        top: SETTINGS_CONTENT_Y,
        right: crc.right,
        bottom: crc.bottom,
    };
    InvalidateRect(hwnd, &scroll_strip, 0);
    InvalidateRect(hwnd, &viewport, 0);
}

/// 显示设置滚动条，并启动（重置）自动隐藏 timer（1.5秒后隐藏）
unsafe fn settings_scrollbar_show(hwnd: HWND, st: &mut SettingsWndState) {
    st.scroll_bar_visible = true;
    if st.scroll_hide_timer {
        KillTimer(hwnd, ID_TIMER_SETTINGS_SCROLLBAR);
    }
    st.scroll_hide_timer = true;
    SetTimer(hwnd, ID_TIMER_SETTINGS_SCROLLBAR, 1500, None);
}

unsafe fn settings_scroll(hwnd: HWND, st: &mut SettingsWndState, delta: i32) {
    settings_scroll_to(hwnd, st, st.content_scroll_y + delta);
}

/// 计算自绘滚动条拇指矩形（宽度可变：正常=SCROLL_BAR_W，拖拽=SCROLL_BAR_W_ACTIVE）
fn settings_scrollbar_thumb_w(crc: &RECT, scroll_y: i32, bar_w: i32) -> Option<RECT> {
    let view_h = (crc.bottom - crc.top) - SETTINGS_CONTENT_Y;
    let max_s = settings_max_scroll(view_h);
    if max_s <= 0 { return None; }
    let track_top    = SETTINGS_CONTENT_Y + 8;
    let track_bottom = crc.bottom - 8;
    let track_h = (track_bottom - track_top).max(1);
    let thumb_h = ((view_h as f32 / SETTINGS_CONTENT_TOTAL_H as f32) * track_h as f32) as i32;
    let thumb_h = thumb_h.max(24);
    let thumb_top = track_top
        + ((scroll_y as f32 / max_s as f32) * (track_h - thumb_h) as f32) as i32;
    let right = crc.right - SCROLL_BAR_MARGIN;
    Some(RECT {
        left:   right - bar_w,
        top:    thumb_top,
        right,
        bottom: thumb_top + thumb_h,
    })
}

unsafe fn settings_show_page(hwnd: HWND, st: &mut SettingsWndState, page: usize) {
    let page = page.min(SETTINGS_PAGES.len().saturating_sub(1));
    let old_page = st.cur_page;
    if old_page == page && st.ui.is_built(page) {
        settings_sync_page_state(st, page);
        return;
    }

    SendMessageW(hwnd, WM_SETREDRAW, 0, 0);
    settings_ensure_page(hwnd, st, page);
    st.cur_page = page;

    for reg in st.ui.page_regs(old_page) {
        if !reg.hwnd.is_null() { ShowWindow(reg.hwnd, SW_HIDE); }
    }
    for reg in st.ui.page_regs(st.cur_page) {
        if !reg.hwnd.is_null() { ShowWindow(reg.hwnd, SW_SHOW); }
    }

    st.content_scroll_y = 0;
    if st.cur_page == SettingsPage::General.index() {
        settings_repos_controls(hwnd, st);
    }

    settings_sync_page_state(st, page);
    SendMessageW(hwnd, WM_SETREDRAW, 1, 0);
    let mut rc: RECT = core::mem::zeroed();
    if GetClientRect(hwnd, &mut rc) != 0 {
        let viewport = settings_viewport_rect(&rc);
        RedrawWindow(hwnd, &viewport, null_mut(), RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW);
    } else {
        InvalidateRect(hwnd, null(), 1);
    }
}

unsafe fn settings_apply_from_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() { return; }
    let app = &mut *pst;
    // 每次打开设置时，从注册表同步自启状态（避免外部修改不一致）
    app.settings.auto_start = is_autostart_enabled();
    st.draft = app.settings.clone();
    let s = &st.draft;
    settings_set_text(st.cb_max, settings_dropdown_label_for_max_items(s.max_items));
    settings_set_text(st.ed_dx, &s.show_mouse_dx.to_string());
    settings_set_text(st.ed_dy, &s.show_mouse_dy.to_string());
    settings_set_text(st.ed_fx, &s.show_fixed_x.to_string());
    settings_set_text(st.ed_fy, &s.show_fixed_y.to_string());
    settings_set_text(st.cb_pos, settings_dropdown_label_for_pos_mode(&s.show_pos_mode));
    settings_sync_page_state(st, SettingsPage::General.index());
    if st.ui.is_built(SettingsPage::Hotkey.index()) { settings_sync_page_state(st, SettingsPage::Hotkey.index()); }
    if st.ui.is_built(SettingsPage::Plugin.index()) { settings_sync_page_state(st, SettingsPage::Plugin.index()); }
    if st.ui.is_built(SettingsPage::Group.index()) { settings_sync_page_state(st, SettingsPage::Group.index()); }
    if st.ui.is_built(SettingsPage::Cloud.index()) { settings_sync_page_state(st, SettingsPage::Cloud.index()); }
}

unsafe fn settings_sync_pos_fields_enabled(st: &SettingsWndState) {
    let mode = settings_dropdown_pos_mode_from_label(&get_window_text(st.cb_pos));
    let is_follow = mode == "mouse";
    let is_fixed = mode == "fixed";
    if !st.ed_dx.is_null() { EnableWindow(st.ed_dx, if is_follow { 1 } else { 0 }); }
    if !st.ed_dy.is_null() { EnableWindow(st.ed_dy, if is_follow { 1 } else { 0 }); }
    if !st.ed_fx.is_null() { EnableWindow(st.ed_fx, if is_fixed { 1 } else { 0 }); }
    if !st.ed_fy.is_null() { EnableWindow(st.ed_fy, if is_fixed { 1 } else { 0 }); }
}

unsafe fn settings_collect_to_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() { return; }
    st.draft.max_items = settings_dropdown_max_items_from_label(&get_window_text(st.cb_max));
    st.draft.show_mouse_dx = get_window_text(st.ed_dx).parse::<i32>().ok().unwrap_or(12);
    st.draft.show_mouse_dy = get_window_text(st.ed_dy).parse::<i32>().ok().unwrap_or(12);
    st.draft.show_fixed_x = get_window_text(st.ed_fx).parse::<i32>().ok().unwrap_or(120);
    st.draft.show_fixed_y = get_window_text(st.ed_fy).parse::<i32>().ok().unwrap_or(120);
    st.draft.show_pos_mode = settings_dropdown_pos_mode_from_label(&get_window_text(st.cb_pos));
    st.draft.hotkey_mod = normalize_hotkey_mod(&get_window_text(st.cb_hk_mod));
    st.draft.hotkey_key = normalize_hotkey_key(&get_window_text(st.cb_hk_key));
    st.draft.search_engine = search_engine_key_from_display(&get_window_text(st.cb_engine)).to_string();
    st.draft.search_template = {
        let tpl = get_window_text(st.ed_tpl);
        if tpl.trim().is_empty() { search_engine_template(&st.draft.search_engine).to_string() } else { tpl }
    };
    st.draft.cloud_sync_interval = {
        let label = get_window_text(st.cb_cloud_interval);
        if label.trim().is_empty() { "1小时".to_string() } else { label }
    };
    st.draft.cloud_webdav_url = get_window_text(st.ed_cloud_url);
    st.draft.cloud_webdav_user = get_window_text(st.ed_cloud_user);
    st.draft.cloud_webdav_pass = get_window_text(st.ed_cloud_pass);
    st.draft.cloud_remote_dir = {
        let dir = get_window_text(st.ed_cloud_dir);
        if dir.trim().is_empty() { "ZSClip".to_string() } else { dir }
    };
    let app = &mut *pst;
    let grouping_old = app.settings.grouping_enabled;
    let autostart_old = app.settings.auto_start;
    let hotkey_old = format!("{}+{}+{}", app.settings.hotkey_enabled, app.settings.hotkey_mod, app.settings.hotkey_key);
    app.settings = st.draft.clone();
    if !app.settings.grouping_enabled {
        app.current_group_filter = 0;
        app.tab_group_filters = [0, 0];
    }
    save_settings(&app.settings);
    // 开机自启：同步写注册表
    if autostart_old != app.settings.auto_start {
        apply_autostart(app.settings.auto_start);
    }
    if grouping_old != app.settings.grouping_enabled {
        app.clear_selection();
    }
    let hotkey_new = format!("{}+{}+{}", app.settings.hotkey_enabled, app.settings.hotkey_mod, app.settings.hotkey_key);
    if hotkey_old != hotkey_new {
        register_hotkey_for(st.parent_hwnd, app);
    }
    // 保存后按新的上限清理 DB 中多余条目（0=无限制不清理）
    let new_max = app.settings.max_items;
    if new_max > 0 {
        db_prune_items(new_max);
        // 同步刷新内存列表
        reload_state_from_db(app);
    }
    app.refilter();
    InvalidateRect(st.parent_hwnd, null(), 1);
}

unsafe fn settings_toggle_get(st: &SettingsWndState, cid: isize) -> bool {
    match cid {
        IDC_SET_AUTOSTART    => st.draft.auto_start,
        IDC_SET_CLOSETRAY    => st.draft.close_without_exit,
        IDC_SET_CLICK_HIDE   => st.draft.click_hide,
        IDC_SET_EDGEHIDE     => st.draft.edge_auto_hide,
        IDC_SET_HOVERPREVIEW => st.draft.hover_preview,
        IDC_SET_GROUP_ENABLE => st.draft.grouping_enabled,
        IDC_SET_CLOUD_ENABLE => st.draft.cloud_sync_enabled,
        6101 => st.draft.hotkey_enabled,
        7102 => st.draft.quick_search_enabled,
        7101 => st.draft.ai_clean_enabled,
        7103 => st.draft.super_mail_merge_enabled,
        _ => false,
    }
}

unsafe fn settings_toggle_flip(st: &mut SettingsWndState, cid: isize) {
    match cid {
        IDC_SET_AUTOSTART    => st.draft.auto_start = !st.draft.auto_start,
        IDC_SET_CLOSETRAY    => st.draft.close_without_exit = !st.draft.close_without_exit,
        IDC_SET_CLICK_HIDE   => st.draft.click_hide = !st.draft.click_hide,
        IDC_SET_EDGEHIDE     => st.draft.edge_auto_hide = !st.draft.edge_auto_hide,
        IDC_SET_HOVERPREVIEW => st.draft.hover_preview = !st.draft.hover_preview,
        IDC_SET_GROUP_ENABLE => st.draft.grouping_enabled = !st.draft.grouping_enabled,
        IDC_SET_CLOUD_ENABLE => st.draft.cloud_sync_enabled = !st.draft.cloud_sync_enabled,
        6101 => st.draft.hotkey_enabled = !st.draft.hotkey_enabled,
        7102 => st.draft.quick_search_enabled = !st.draft.quick_search_enabled,
        7101 => st.draft.ai_clean_enabled = !st.draft.ai_clean_enabled,
        7103 => st.draft.super_mail_merge_enabled = !st.draft.super_mail_merge_enabled,
        _ => {}
    }
}

unsafe fn settings_create_fonts() -> (*mut core::ffi::c_void, *mut core::ffi::c_void, *mut core::ffi::c_void) {
    let nav: *mut core::ffi::c_void = CreateFontW(-18, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, to_wide("Segoe Fluent Icons").as_ptr()) as _;
    // WinUI3 body font = 14px Segoe UI Variable Text Regular
    let ui: *mut core::ffi::c_void = CreateFontW(-14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, to_wide("Segoe UI Variable Text").as_ptr()) as _;
    // WinUI3 title = 20px Segoe UI Variable Display Semibold
    let title: *mut core::ffi::c_void = CreateFontW(-20, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0, to_wide("Segoe UI Variable Display").as_ptr()) as _;
    let default_ui: *mut core::ffi::c_void = if ui.is_null() { GetStockObject(DEFAULT_GUI_FONT) as _ } else { ui };
    let default_title: *mut core::ffi::c_void = if title.is_null() { GetStockObject(DEFAULT_GUI_FONT) as _ } else { title };
    (nav, default_ui, default_title)
}

unsafe fn settings_create_general_page(hwnd: HWND, st: &mut SettingsWndState) {
    let ui_font = st.ui_font;
    let sec0 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 0, 0);
    let sec1 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 1, 130);
    let sec2 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 2, 138);
    let sec3 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 3, 0);

    st.chk_autostart = settings_create_toggle(hwnd, st, "开机自启", IDC_SET_AUTOSTART, sec0.left(), sec0.row_y(0), sec0.full_w(), ui_font);
    st.chk_close_tray = settings_create_toggle(hwnd, st, "关闭不退出（托盘驻留）", IDC_SET_CLOSETRAY, sec0.left(), sec0.row_y(1), sec0.full_w(), ui_font);
    st.chk_click_hide = settings_create_toggle(hwnd, st, "单击后隐藏主窗口", IDC_SET_CLICK_HIDE, sec0.left(), sec0.row_y(2), sec0.full_w(), ui_font);
    st.chk_edge_hide = settings_create_toggle(hwnd, st, "贴边自动隐藏", IDC_SET_EDGEHIDE, sec0.left(), sec0.row_y(3), sec0.full_w(), ui_font);
    st.chk_hover_preview = settings_create_toggle(hwnd, st, "悬停预览", IDC_SET_HOVERPREVIEW, sec0.left(), sec0.row_y(4), sec0.full_w(), ui_font);

    let lbl_max = settings_create_label(hwnd, "最大保存条数：", sec1.left(), sec1.label_y(0, 24), sec1.label_w, 24, ui_font);
    settings_page0_push_ctrl(st, lbl_max, sec1.left(), sec1.label_y(0, 24), sec1.label_w, 24);
    st.cb_max = settings_create_dropdown_btn(hwnd, "200", IDC_SET_MAX, sec1.field_x(), sec1.row_y(0), 150, ui_font);
    settings_page0_push_ctrl(st, st.cb_max, sec1.field_x(), sec1.row_y(0), 150, 32);
    if !st.cb_max.is_null() { st.ownerdraw_ctrls.push(st.cb_max); }

    let lbl_pos = settings_create_label(hwnd, "弹出位置：", sec2.left(), sec2.label_y(0, 24), sec2.label_w, 24, ui_font);
    settings_page0_push_ctrl(st, lbl_pos, sec2.left(), sec2.label_y(0, 24), sec2.label_w, 24);
    st.cb_pos = settings_create_dropdown_btn(hwnd, "跟随鼠标", IDC_SET_POSMODE, sec2.field_x(), sec2.row_y(0), 170, ui_font);
    settings_page0_push_ctrl(st, st.cb_pos, sec2.field_x(), sec2.row_y(0), 170, 32);
    if !st.cb_pos.is_null() { st.ownerdraw_ctrls.push(st.cb_pos); }

    let lbl_mouse = settings_create_label(hwnd, "鼠标偏移 dx/dy：", sec2.left(), sec2.label_y(1, 24), sec2.label_w, 24, ui_font);
    settings_page0_push_ctrl(st, lbl_mouse, sec2.left(), sec2.label_y(1, 24), sec2.label_w, 24);
    let mouse_x = sec2.field_x();
    st.ed_dx = settings_create_edit(hwnd, "", IDC_SET_DX, mouse_x, sec2.row_y(1), 64, ui_font);
    st.ed_dy = settings_create_edit(hwnd, "", IDC_SET_DY, mouse_x + 74, sec2.row_y(1), 64, ui_font);
    settings_page0_push_ctrl(st, st.ed_dx, mouse_x, sec2.row_y(1), 64, 28);
    settings_page0_push_ctrl(st, st.ed_dy, mouse_x + 74, sec2.row_y(1), 64, 28);

    let lbl_fixed = settings_create_label(hwnd, "固定位置 x/y：", sec2.left(), sec2.label_y(2, 24), sec2.label_w, 24, ui_font);
    settings_page0_push_ctrl(st, lbl_fixed, sec2.left(), sec2.label_y(2, 24), sec2.label_w, 24);
    let fixed_x = sec2.field_x();
    st.ed_fx = settings_create_edit(hwnd, "", IDC_SET_FX, fixed_x, sec2.row_y(2), 64, ui_font);
    st.ed_fy = settings_create_edit(hwnd, "", IDC_SET_FY, fixed_x + 74, sec2.row_y(2), 64, ui_font);
    settings_page0_push_ctrl(st, st.ed_fx, fixed_x, sec2.row_y(2), 64, 28);
    settings_page0_push_ctrl(st, st.ed_fy, fixed_x + 74, sec2.row_y(2), 64, 28);

    let btn_y = sec3.row_y(0);
    st.btn_open_cfg = settings_create_small_btn(hwnd, "打开设置文件", IDC_SET_BTN_OPENCFG, sec3.action_x(0, 130), btn_y, 130, ui_font);
    st.btn_open_db = settings_create_small_btn(hwnd, "打开数据库文件", IDC_SET_BTN_OPENDB, sec3.action_x(1, 130), btn_y, 130, ui_font);
    st.btn_open_data = settings_create_small_btn(hwnd, "打开数据目录", IDC_SET_BTN_OPENDATA, sec3.action_x(2, 130), btn_y, 130, ui_font);
    settings_page0_push_ctrl(st, st.btn_open_cfg, sec3.action_x(0, 130), btn_y, 130, 32);
    settings_page0_push_ctrl(st, st.btn_open_db, sec3.action_x(1, 130), btn_y, 130, 32);
    settings_page0_push_ctrl(st, st.btn_open_data, sec3.action_x(2, 130), btn_y, 130, 32);
    for &hh in &[st.btn_open_cfg, st.btn_open_db, st.btn_open_data] {
        if !hh.is_null() { st.ownerdraw_ctrls.push(hh); }
    }
    st.ui.mark_built(SettingsPage::General.index());
}

unsafe fn settings_create_listbox(parent: HWND, id: isize, x: i32, y: i32, w: i32, h: i32, font: *mut core::ffi::c_void) -> HWND {
    host_create_settings_listbox(parent, id, x, y, w, h, font)
}

unsafe fn settings_groups_refresh_list(st: &mut SettingsWndState, select_gid: i64) {
    if st.lb_groups.is_null() { return; }
    SendMessageW(st.lb_groups, LB_RESETCONTENT, 0, 0);
    st.groups_cache = db_load_groups();
    let mut sel_idx: i32 = -1;
    for (i, g) in st.groups_cache.iter().enumerate() {
        SendMessageW(st.lb_groups, LB_ADDSTRING, 0, to_wide(&g.name).as_ptr() as LPARAM);
        if g.id == select_gid {
            sel_idx = i as i32;
        }
    }
    if sel_idx < 0 && !st.groups_cache.is_empty() {
        sel_idx = 0;
    }
    if sel_idx >= 0 {
        SendMessageW(st.lb_groups, LB_SETCURSEL, sel_idx as WPARAM, 0);
    }
}

unsafe fn settings_groups_selected(st: &SettingsWndState) -> Option<(usize, ClipGroup)> {
    if st.lb_groups.is_null() { return None; }
    let row = SendMessageW(st.lb_groups, LB_GETCURSEL, 0, 0) as i32;
    if row < 0 { return None; }
    st.groups_cache.get(row as usize).cloned().map(|g| (row as usize, g))
}

unsafe fn settings_groups_sync_name(_st: &mut SettingsWndState) {
}

unsafe fn settings_groups_move(st: &mut SettingsWndState, step: i32) {
    let Some((idx, _)) = settings_groups_selected(st) else { return; };
    let new_idx = idx as i32 + step;
    if new_idx < 0 || new_idx >= st.groups_cache.len() as i32 {
        return;
    }
    let mut ids: Vec<i64> = st.groups_cache.iter().map(|g| g.id).collect();
    let item = ids.remove(idx);
    ids.insert(new_idx as usize, item);
    if db_set_groups_order(&ids).is_ok() {
        settings_groups_refresh_list(st, ids[new_idx as usize]);
        let pst = get_state_ptr(st.parent_hwnd);
        if !pst.is_null() {
            reload_state_from_db(&mut *pst);
            InvalidateRect(st.parent_hwnd, null(), 1);
        }
    }
}


// ─── WinUI3风格输入对话框 ──────────────────────────────────────────────────────
// 用于"新建分组"和"分组重命名"，返回用户输入的字符串，None表示取消
struct InputDlgData {
    result: Option<String>,
    initial: [u16; 256],
    title_w: Vec<u16>,
    label_w: Vec<u16>,
    ui_font: *mut core::ffi::c_void,
    surface_brush: *mut core::ffi::c_void,
    control_brush: *mut core::ffi::c_void,
}

unsafe fn input_dialog_refresh_theme(data: &mut InputDlgData) {
    if !data.surface_brush.is_null() { DeleteObject(data.surface_brush as _); }
    if !data.control_brush.is_null() { DeleteObject(data.control_brush as _); }
    let th = Theme::default();
    data.surface_brush = CreateSolidBrush(th.surface) as _;
    data.control_brush = CreateSolidBrush(th.control_bg) as _;
}

const IDC_INPUT_EDIT: usize = 9001;
const IDC_INPUT_OK: usize = 9002;
const IDC_INPUT_CANCEL: usize = 9003;
const INPUT_DLG_CLASS: &str = "ZsClipInputDlg";

unsafe extern "system" fn input_dlg_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let data = cs.lpCreateParams as *mut InputDlgData;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, data as isize);

            let d = &mut *data;
            let hmod = GetModuleHandleW(null());
            d.ui_font = CreateFontW(-14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0,
                to_wide("Segoe UI Variable Text").as_ptr()) as _;
            input_dialog_refresh_theme(d);

            // 标签
            let lbl = CreateWindowExW(0, to_wide("STATIC").as_ptr(), d.label_w.as_ptr(),
                WS_CHILD | WS_VISIBLE, 20, 58, 320, 22, hwnd, null_mut(), hmod, null());
            SendMessageW(lbl, WM_SETFONT, d.ui_font as usize, 1);

            // 输入框
            let ed = CreateWindowExW(WS_EX_CLIENTEDGE, to_wide("EDIT").as_ptr(), d.initial.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32),
                20, 84, 320, 32, hwnd, IDC_INPUT_EDIT as _, hmod, null());
            SendMessageW(ed, WM_SETFONT, d.ui_font as usize, 1);
            SetWindowTheme(ed, to_wide("Explorer").as_ptr(), null());
            // 全选
            SendMessageW(ed, EM_SETSEL, 0, -1isize);
            SetFocus(ed);

            // 取消按钮（左）
            let btn_cancel = CreateWindowExW(0, to_wide("BUTTON").as_ptr(), to_wide("取消").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                148, 132, 88, 30, hwnd, IDC_INPUT_CANCEL as _, hmod, null());
            SendMessageW(btn_cancel, WM_SETFONT, d.ui_font as usize, 1);

            // 保存按钮（右）
            let btn_ok = CreateWindowExW(0, to_wide("BUTTON").as_ptr(), to_wide("保存").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                248, 132, 88, 30, hwnd, IDC_INPUT_OK as _, hmod, null());
            SendMessageW(btn_ok, WM_SETFONT, d.ui_font as usize, 1);

            // DWM圆角
            let corner: u32 = 2; // DWMWCP_ROUND
            DwmSetWindowAttribute(hwnd, 33, &corner as *const u32 as _, 4);
            // 背景色
            let _ = (btn_cancel, btn_ok);
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORDLG => {
            let hdc = wparam as *mut core::ffi::c_void;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            let th = Theme::default();
            SetBkMode(hdc, 1);
            SetBkColor(hdc, th.surface);
            SetTextColor(hdc, th.text);
            if !data_ptr.is_null() {
                return (*data_ptr).surface_brush as isize;
            }
            0
        }
        WM_CTLCOLOREDIT => {
            let hdc = wparam as *mut core::ffi::c_void;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            let th = Theme::default();
            SetBkColor(hdc, th.control_bg);
            SetTextColor(hdc, th.text);
            if !data_ptr.is_null() {
                return (*data_ptr).control_brush as isize;
            }
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let th = Theme::default();
            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            // 背景
            let bg = CreateSolidBrush(th.surface);
            FillRect(hdc, &rc, bg);
            DeleteObject(bg as _);
            // 顶部accent条
            let top_rc = RECT { left: 0, top: 0, right: rc.right, bottom: 48 };
            let top_br = CreateSolidBrush(th.surface);
            FillRect(hdc, &top_rc, top_br);
            DeleteObject(top_br as _);
            // 标题
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            if !data_ptr.is_null() {
                let d = &*data_ptr;
                let title_rc = RECT { left: 20, top: 12, right: rc.right - 20, bottom: 46 };
                let title_font: *mut core::ffi::c_void = CreateFontW(-16, 0, 0, 0, 600, 0, 0, 0, 1, 0, 0, 5, 0,
                    to_wide("Segoe UI Variable Display").as_ptr()) as _;
                let old = SelectObject(hdc, title_font as _);
                SetBkMode(hdc, 1);
                SetTextColor(hdc, th.text);
                DrawTextW(hdc, d.title_w.as_ptr(), -1, &title_rc as *const _ as *mut _,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE);
                SelectObject(hdc, old);
                DeleteObject(title_font as _);
            }
            // 分隔线
            let sep_br = CreateSolidBrush(th.stroke);
            let sep_rc = RECT { left: 0, top: 47, right: rc.right, bottom: 48 };
            FillRect(hdc, &sep_rc, sep_br);
            DeleteObject(sep_br as _);
            EndPaint(hwnd, &ps);
            0
        }
        WM_DRAWITEM => {
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            let hdc = dis.hDC;
            let rc = dis.rcItem;
            let th = Theme::default();
            let pressed = (dis.itemState & (ODS_SELECTED as u32)) != 0;
            let cid = dis.CtlID as usize;
            let text_w = get_ctrl_text_w(dis.hwndItem);
            let rr = RECT { left: rc.left+1, top: rc.top+1, right: rc.right-1, bottom: rc.bottom-1 };
            if cid == IDC_INPUT_OK {
                let fill = if pressed {
                    let r = (th.accent & 0xFF) as i32;
                    let g = ((th.accent >> 8) & 0xFF) as i32;
                    let b = ((th.accent >> 16) & 0xFF) as i32;
                    rgb((r-18).max(0) as u8, (g-18).max(0) as u8, (b-18).max(0) as u8)
                } else { th.accent };
                draw_round_rect(hdc as _, &rr, fill, fill, 4);
                draw_text_wide(hdc as _, &text_w, &rr, rgb(255,255,255), 14, "Segoe UI Variable Text");
            } else {
                let fill = if pressed { th.button_pressed } else { th.button_bg };
                let border = if pressed { rgb(180,180,180) } else { rgb(196,196,196) };
                draw_round_rect(hdc as _, &rr, fill, border, 4);
                draw_text_wide(hdc as _, &text_w, &rr, th.text, 14, "Segoe UI Variable Text");
            }
            1
        }
        WM_COMMAND => {
            let cid = (wparam & 0xffff) as usize;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            if data_ptr.is_null() { return 0; }
            let d = &mut *data_ptr;
            if cid == IDC_INPUT_OK {
                let ed = GetDlgItem(hwnd, IDC_INPUT_EDIT as i32);
                if !ed.is_null() {
                    let len = GetWindowTextLengthW(ed);
                    let mut buf = vec![0u16; (len as usize) + 2];
                    GetWindowTextW(ed, buf.as_mut_ptr(), buf.len() as i32);
                    let s = String::from_utf16_lossy(&buf[..len as usize]).trim().to_string();
                    if !s.is_empty() {
                        d.result = Some(s);
                        DestroyWindow(hwnd);
                    }
                }
            } else if cid == IDC_INPUT_CANCEL {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_RETURN as usize {
                SendMessageW(hwnd, WM_COMMAND, IDC_INPUT_OK, 0);
            } else if wparam == VK_ESCAPE as usize {
                DestroyWindow(hwnd);
            }
            0
        }
        WM_THEMECHANGED | WM_SETTINGCHANGE => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            if !data_ptr.is_null() {
                input_dialog_refresh_theme(&mut *data_ptr);
                InvalidateRect(hwnd, null(), 1);
            }
            0
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }
        WM_NCDESTROY => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            if !data_ptr.is_null() {
                if !(*data_ptr).ui_font.is_null() && (*data_ptr).ui_font != GetStockObject(DEFAULT_GUI_FONT) { DeleteObject((*data_ptr).ui_font as _); }
                if !(*data_ptr).surface_brush.is_null() { DeleteObject((*data_ptr).surface_brush as _); }
                if !(*data_ptr).control_brush.is_null() { DeleteObject((*data_ptr).control_brush as _); }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// 注册并显示输入对话框，返回用户输入文本
unsafe fn input_name_dialog(parent: HWND, title: &str, label: &str, initial: &str) -> Option<String> {
    let hmod = GetModuleHandleW(null());
    // 注册窗口类
    let cls_w = to_wide(INPUT_DLG_CLASS);
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(input_dlg_proc);
    wc.hInstance = hmod;
    wc.lpszClassName = cls_w.as_ptr();
    wc.hbrBackground = CreateSolidBrush(Theme::default().surface) as _;
    let _ = RegisterClassExW(&wc);

    let mut init_arr = [0u16; 256];
    let iw: Vec<u16> = initial.encode_utf16().collect();
    let copy_len = iw.len().min(255);
    init_arr[..copy_len].copy_from_slice(&iw[..copy_len]);

    let mut data = InputDlgData {
        result: None,
        initial: init_arr,
        title_w: title.encode_utf16().chain(std::iter::once(0)).collect(),
        label_w: label.encode_utf16().chain(std::iter::once(0)).collect(),
        ui_font: null_mut(),
        surface_brush: null_mut(),
        control_brush: null_mut(),
    };

    // 计算居中位置
    let (dw, dh) = (360i32, 180i32);
    let mut parent_rc: RECT = zeroed();
    GetWindowRect(parent, &mut parent_rc);
    let cx = parent_rc.left + (parent_rc.right - parent_rc.left - dw) / 2;
    let cy = parent_rc.top + (parent_rc.bottom - parent_rc.top - dh) / 2;

    let title_w = to_wide(title);
    let hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
        cls_w.as_ptr(),
        title_w.as_ptr(),
        WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN,
        cx, cy, dw, dh,
        parent, null_mut(), hmod,
        &mut data as *mut InputDlgData as _,
    );
    if hwnd.is_null() { return None; }
    EnableWindow(parent, 0);

    let mut msg: MSG = zeroed();
    loop {
        if GetMessageW(&mut msg, null_mut(), 0, 0) == 0 { break; }
        if msg.message == WM_KEYDOWN && (msg.wParam == VK_RETURN as usize || msg.wParam == VK_ESCAPE as usize) {
            SendMessageW(hwnd, WM_KEYDOWN, msg.wParam, msg.lParam);
            continue;
        }
        if IsDialogMessageW(hwnd, &mut msg) == 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if IsWindow(hwnd) == 0 { break; }
    }
    EnableWindow(parent, 1);
    SetForegroundWindow(parent);
    data.result
}

// ─── WinUI3风格文本编辑对话框 ─────────────────────────────────────────────────
const IDC_EDIT_TEXTAREA: usize = 9010;
const IDC_EDIT_LINENO: usize = 9011;
const IDC_EDIT_SAVE: usize = 9012;
const IDC_EDIT_CANCEL: usize = 9013;
const EDIT_DLG_CLASS: &str = "ZsClipEditDlg";

struct EditDlgData {
    item_id: i64,
    saved: bool,
    ui_font: *mut core::ffi::c_void,
    btn_font: *mut core::ffi::c_void,
    surface_brush: *mut core::ffi::c_void,
    control_brush: *mut core::ffi::c_void,
    gutter_brush: *mut core::ffi::c_void,
}

unsafe fn edit_dialog_refresh_theme(data: &mut EditDlgData) {
    if !data.surface_brush.is_null() { DeleteObject(data.surface_brush as _); }
    if !data.control_brush.is_null() { DeleteObject(data.control_brush as _); }
    if !data.gutter_brush.is_null() { DeleteObject(data.gutter_brush as _); }
    let th = Theme::default();
    data.surface_brush = CreateSolidBrush(th.surface) as _;
    data.control_brush = CreateSolidBrush(th.control_bg) as _;
    data.gutter_brush = CreateSolidBrush(if is_dark_mode() { rgb(38, 42, 48) } else { rgb(246, 248, 250) }) as _;
}

unsafe fn sync_line_numbers(lineno_hwnd: HWND, textarea_hwnd: HWND) {
    let line_count = SendMessageW(textarea_hwnd, EM_GETLINECOUNT, 0, 0) as i32;
    let first_visible = SendMessageW(textarea_hwnd, EM_GETFIRSTVISIBLELINE, 0, 0) as i32;
    let mut lines = String::new();
    // 获取可见行数（近似：控件高度 / 行高）
    let mut rc: RECT = zeroed();
    GetClientRect(textarea_hwnd, &mut rc);
    let line_h = SendMessageW(textarea_hwnd, EM_GETLINECOUNT, 0, 0); // reuse
    let _ = line_h;
    let visible_lines = (rc.bottom - rc.top) / 16 + 2;
    let end = (first_visible + visible_lines).min(line_count);
    for i in first_visible..end {
        lines.push_str(&format!("{}\r\n", i + 1));
    }
    // pad remaining
    SetWindowTextW(lineno_hwnd, to_wide(&lines).as_ptr());
}

unsafe extern "system" fn edit_dlg_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let data = cs.lpCreateParams as *mut EditDlgData;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, data as isize);
            let d = &mut *data;
            let hmod = GetModuleHandleW(null());

            d.ui_font = CreateFontW(-14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0,
                to_wide("Cascadia Mono").as_ptr()) as _;
            edit_dialog_refresh_theme(d);
            if (d.ui_font as usize) == 0 {
                d.ui_font = CreateFontW(-14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0,
                    to_wide("Consolas").as_ptr()) as _;
            }
            d.btn_font = CreateFontW(-14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0,
                to_wide("Segoe UI Variable Text").as_ptr()) as _;

            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            let w = rc.right;
            let h = rc.bottom;
            let lineno_w = 44i32;
            let edit_x = lineno_w;
            let edit_w = w - lineno_w;
            let edit_h = h - 56;

            // 行号面板（只读编辑框，右对齐）
            let lineno = CreateWindowExW(0, to_wide("EDIT").as_ptr(), to_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | (ES_MULTILINE as u32) | (ES_READONLY as u32) | (ES_RIGHT as u32),
                0, 0, lineno_w, edit_h, hwnd, IDC_EDIT_LINENO as _, hmod, null());
            SendMessageW(lineno, WM_SETFONT, d.ui_font as usize, 1);

            // 主编辑区
            let ed = CreateWindowExW(0, to_wide("EDIT").as_ptr(), to_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_VSCROLL | WS_TABSTOP
                | (ES_MULTILINE as u32) | (ES_AUTOVSCROLL as u32) | (ES_WANTRETURN as u32) | (ES_NOHIDESEL as u32),
                edit_x, 0, edit_w, edit_h, hwnd, IDC_EDIT_TEXTAREA as _, hmod, null());
            SendMessageW(ed, WM_SETFONT, d.ui_font as usize, 1);
            SetWindowTheme(ed, to_wide("Explorer").as_ptr(), null());

            // 加载文本
            if let Ok(t) = with_db(|conn| {
                conn.query_row(
                    "SELECT COALESCE(text_data,'') FROM items WHERE id=?", params![d.item_id], |r| r.get::<_, String>(0))
            }) {
                let crlf = t.replace('\n', "\r\n");
                SetWindowTextW(ed, to_wide(&crlf).as_ptr());
            }
            SendMessageW(ed, EM_SETSEL, 0, 0);
            SetFocus(ed);

            // 取消按钮
            let btn_cancel = CreateWindowExW(0, to_wide("BUTTON").as_ptr(), to_wide("取消").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                w - 210, h - 44, 90, 30, hwnd, IDC_EDIT_CANCEL as _, hmod, null());
            SendMessageW(btn_cancel, WM_SETFONT, d.btn_font as usize, 1);

            // 保存按钮
            let btn_save = CreateWindowExW(0, to_wide("BUTTON").as_ptr(), to_wide("保存").as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                w - 110, h - 44, 90, 30, hwnd, IDC_EDIT_SAVE as _, hmod, null());
            SendMessageW(btn_save, WM_SETFONT, d.btn_font as usize, 1);

            let _ = (btn_cancel, btn_save);

            // DWM圆角
            let corner: u32 = 2;
            DwmSetWindowAttribute(hwnd, 33, &corner as *const u32 as _, 4);

            // 初始化行号
            sync_line_numbers(lineno, ed);
            0
        }
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let th = Theme::default();
            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            let bg = CreateSolidBrush(th.surface);
            FillRect(hdc, &rc, bg);
            DeleteObject(bg as _);
            // 底部工具栏背景
            let bar_rc = RECT { left: 0, top: rc.bottom - 56, right: rc.right, bottom: rc.bottom };
            let bar_br = CreateSolidBrush(th.surface);
            FillRect(hdc, &bar_rc, bar_br);
            DeleteObject(bar_br as _);
            // 分隔线
            let sep_br = CreateSolidBrush(th.stroke);
            let sep_rc = RECT { left: 0, top: rc.bottom - 56, right: rc.right, bottom: rc.bottom - 55 };
            FillRect(hdc, &sep_rc, sep_br);
            DeleteObject(sep_br as _);
            EndPaint(hwnd, &ps);
            0
        }
        WM_THEMECHANGED | WM_SETTINGCHANGE => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditDlgData;
            if !data_ptr.is_null() {
                edit_dialog_refresh_theme(&mut *data_ptr);
                InvalidateRect(hwnd, null(), 1);
            }
            0
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as *mut core::ffi::c_void;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditDlgData;
            let th = Theme::default();
            SetBkMode(hdc, 1);
            SetBkColor(hdc, th.surface);
            SetTextColor(hdc, rgb(140, 148, 160));
            if !data_ptr.is_null() {
                return (*data_ptr).surface_brush as isize;
            }
            0
        }
        WM_CTLCOLOREDIT => {
            let hdc = wparam as *mut core::ffi::c_void;
            let child = lparam as HWND;
            let cid = GetDlgCtrlID(child) as usize;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditDlgData;
            let th = Theme::default();
            if !data_ptr.is_null() {
                if cid == IDC_EDIT_LINENO {
                    SetBkColor(hdc, if is_dark_mode() { rgb(38, 42, 48) } else { rgb(246, 248, 250) });
                    SetTextColor(hdc, rgb(140, 148, 160));
                    return (*data_ptr).gutter_brush as isize;
                }
                SetBkColor(hdc, th.control_bg);
                SetTextColor(hdc, th.text);
                return (*data_ptr).control_brush as isize;
            }
            0
        }
        WM_DRAWITEM => {
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            let hdc = dis.hDC;
            let rc = dis.rcItem;
            let th = Theme::default();
            let pressed = (dis.itemState & (ODS_SELECTED as u32)) != 0;
            let cid = dis.CtlID as usize;
            let text_w = get_ctrl_text_w(dis.hwndItem);
            let rr = RECT { left: rc.left+1, top: rc.top+1, right: rc.right-1, bottom: rc.bottom-1 };
            if cid == IDC_EDIT_SAVE {
                let fill = if pressed {
                    let r = (th.accent & 0xFF) as i32;
                    let g = ((th.accent >> 8) & 0xFF) as i32;
                    let b = ((th.accent >> 16) & 0xFF) as i32;
                    rgb((r-18).max(0) as u8, (g-18).max(0) as u8, (b-18).max(0) as u8)
                } else { th.accent };
                draw_round_rect(hdc as _, &rr, fill, fill, 4);
                draw_text_wide(hdc as _, &text_w, &rr, rgb(255,255,255), 14, "Segoe UI Variable Text");
            } else {
                let fill = if pressed { th.button_pressed } else { th.button_bg };
                let border = if pressed { rgb(180,180,180) } else { rgb(196,196,196) };
                draw_round_rect(hdc as _, &rr, fill, border, 4);
                draw_text_wide(hdc as _, &text_w, &rr, th.text, 14, "Segoe UI Variable Text");
            }
            1
        }
        WM_COMMAND => {
            let cid = (wparam & 0xffff) as usize;
            let notify = ((wparam >> 16) & 0xffff) as u32;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditDlgData;
            if data_ptr.is_null() { return 0; }
            let d = &mut *data_ptr;

            // 文本区滚动时同步行号
            if cid == IDC_EDIT_TEXTAREA && notify == EN_VSCROLL as u32 {
                let ed = GetDlgItem(hwnd, IDC_EDIT_TEXTAREA as i32);
                let ln = GetDlgItem(hwnd, IDC_EDIT_LINENO as i32);
                sync_line_numbers(ln, ed);
            }

            if cid == IDC_EDIT_SAVE {
                let ed = GetDlgItem(hwnd, IDC_EDIT_TEXTAREA as i32);
                if !ed.is_null() {
                    let len = GetWindowTextLengthW(ed);
                    let mut buf = vec![0u16; (len as usize) + 2];
                    GetWindowTextW(ed, buf.as_mut_ptr(), buf.len() as i32);
                    let raw = String::from_utf16_lossy(&buf[..len as usize]);
                    // 将\r\n转回\n
                    let normalized = raw.replace("\r\n", "\n").replace('\r', "\n");
                    let _ = db_update_item_text(d.item_id, &normalized);
                    d.saved = true;
                }
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            } else if cid == IDC_EDIT_CANCEL {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
            0
        }
        WM_VSCROLL => {
            // 文本框滚动时同步行号
            let ed = GetDlgItem(hwnd, IDC_EDIT_TEXTAREA as i32);
            let ln = GetDlgItem(hwnd, IDC_EDIT_LINENO as i32);
            if !ed.is_null() && !ln.is_null() {
                sync_line_numbers(ln, ed);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SIZE => {
            let w = (lparam & 0xffff) as i32;
            let h = ((lparam >> 16) & 0xffff) as i32;
            let lineno_w = 44i32;
            let edit_h = h - 56;
            let lineno = GetDlgItem(hwnd, IDC_EDIT_LINENO as i32);
            let ed = GetDlgItem(hwnd, IDC_EDIT_TEXTAREA as i32);
            let btn_cancel = GetDlgItem(hwnd, IDC_EDIT_CANCEL as i32);
            let btn_save = GetDlgItem(hwnd, IDC_EDIT_SAVE as i32);
            if !lineno.is_null() {
                SetWindowPos(lineno, null_mut(), 0, 0, lineno_w, edit_h, SWP_NOMOVE | SWP_NOZORDER);
            }
            if !ed.is_null() {
                SetWindowPos(ed, null_mut(), lineno_w, 0, w - lineno_w, edit_h, SWP_NOZORDER);
            }
            if !btn_cancel.is_null() {
                SetWindowPos(btn_cancel, null_mut(), w - 210, h - 44, 90, 30, SWP_NOZORDER);
            }
            if !btn_save.is_null() {
                SetWindowPos(btn_save, null_mut(), w - 110, h - 44, 90, 30, SWP_NOZORDER);
            }
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_ESCAPE as usize {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_NCDESTROY => {
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn show_edit_item_dialog(parent: HWND, item_id: i64, title: &str) -> bool {
    let hmod = GetModuleHandleW(null());
    let cls_w = to_wide(EDIT_DLG_CLASS);
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(edit_dlg_proc);
    wc.hInstance = hmod;
    wc.lpszClassName = cls_w.as_ptr();
    wc.hbrBackground = CreateSolidBrush(Theme::default().surface) as _;
    wc.style = CS_HREDRAW | CS_VREDRAW;
    let _ = RegisterClassExW(&wc);

    let mut parent_rc: RECT = zeroed();
    GetWindowRect(parent, &mut parent_rc);
    let pw = parent_rc.right - parent_rc.left;
    let ph = parent_rc.bottom - parent_rc.top;
    let dw = (pw * 3).max(640);
    let dh = (ph * 4 / 3).max(500);
    let cx = parent_rc.left + (pw - dw) / 2;
    let cy = parent_rc.top + (ph - dh) / 2;

    let mut data = EditDlgData { item_id, saved: false, ui_font: null_mut(), btn_font: null_mut(), surface_brush: null_mut(), control_brush: null_mut(), gutter_brush: null_mut() };
    let title_w = to_wide(title);
    let hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
        cls_w.as_ptr(),
        title_w.as_ptr(),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE | WS_CLIPCHILDREN,
        cx, cy, dw, dh,
        parent, null_mut(), hmod,
        &mut data as *mut EditDlgData as _,
    );
    if hwnd.is_null() { return false; }
    EnableWindow(parent, 0);

    let mut msg: MSG = zeroed();
    loop {
        let r = GetMessageW(&mut msg, null_mut(), 0, 0);
        if r == 0 || r == -1 { break; }
        if IsDialogMessageW(hwnd, &mut msg) == 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if IsWindow(hwnd) == 0 { break; }
    }
    EnableWindow(parent, 1);
    SetForegroundWindow(parent);
    data.saved
}

// 辅助函数：获取控件文字宽字节
unsafe fn get_ctrl_text_w(hwnd: HWND) -> Vec<u16> {
    let len = GetWindowTextLengthW(hwnd);
    let mut buf = vec![0u16; (len as usize) + 2];
    GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    buf.truncate(len as usize);
    buf
}

// 辅助函数：绘制宽字节文字居中
#[derive(Clone, Copy)]
struct SettingsPageBuilder {
    hwnd: HWND,
    page: usize,
    font: *mut core::ffi::c_void,
}

#[derive(Clone, Copy)]
struct SettingsFormSectionLayout {
    body: RECT,
    label_w: i32,
}

impl SettingsFormSectionLayout {
    fn new(page: usize, index: usize, label_w: i32) -> Self {
        Self {
            body: settings_section_body_rect(page, index, 18).into(),
            label_w,
        }
    }

    fn left(&self) -> i32 { self.body.left }
    fn full_w(&self) -> i32 { self.body.right - self.body.left }
    fn row_y(&self, row: i32) -> i32 { self.body.top + row * (SETTINGS_FORM_ROW_H + SETTINGS_FORM_ROW_GAP) }
    fn label_y(&self, row: i32, h: i32) -> i32 { self.row_y(row) + ((SETTINGS_FORM_ROW_H - h).max(0) / 2) }
    fn field_x(&self) -> i32 { self.body.left + self.label_w }
    fn field_w(&self) -> i32 { (self.body.right - self.field_x()).max(40) }
    fn field_w_from(&self, x: i32) -> i32 { (self.body.right - x).max(40) }
    fn action_x(&self, slot: i32, w: i32) -> i32 { self.body.left + slot * (w + 14) }
}

impl SettingsPageBuilder {
    unsafe fn add(&self, st: &mut SettingsWndState, hwnd: HWND) -> HWND {
        if !hwnd.is_null() { settings_page_push_ctrl(st, self.page, hwnd); }
        hwnd
    }

    unsafe fn label(&self, st: &mut SettingsWndState, text: &str, x: i32, y: i32, w: i32, h: i32) -> HWND {
        self.add(st, settings_create_label(self.hwnd, text, x, y, w, h, self.font))
    }

    unsafe fn label_auto(&self, st: &mut SettingsWndState, text: &str, x: i32, y: i32, w: i32, min_h: i32) -> (HWND, i32) {
        let (hwnd, h) = settings_create_label_auto(self.hwnd, text, x, y, w, min_h, self.font);
        (self.add(st, hwnd), h)
    }

    unsafe fn button(&self, st: &mut SettingsWndState, text: &str, id: isize, x: i32, y: i32, w: i32) -> HWND {
        self.add(st, settings_create_small_btn(self.hwnd, text, id, x, y, w, self.font))
    }

    unsafe fn dropdown(&self, st: &mut SettingsWndState, text: &str, id: isize, x: i32, y: i32, w: i32) -> HWND {
        self.add(st, settings_create_dropdown_btn(self.hwnd, text, id, x, y, w, self.font))
    }

    unsafe fn edit(&self, st: &mut SettingsWndState, text: &str, id: isize, x: i32, y: i32, w: i32) -> HWND {
        self.add(st, settings_create_edit(self.hwnd, text, id, x, y, w, self.font))
    }

    unsafe fn toggle_row(&self, st: &mut SettingsWndState, text: &str, id: isize, x: i32, y: i32, w: i32) -> (HWND, HWND) {
        let (label, btn, ..) = settings_create_toggle_plain(self.hwnd, text, id, x, y, w, self.font);
        (self.add(st, label), self.add(st, btn))
    }
}

unsafe fn draw_text_wide(hdc: *mut core::ffi::c_void, text_w: &[u16], rc: &RECT, color: u32, size: i32, font_name: &str) {
    let hdc = hdc as _;
    let font: *mut core::ffi::c_void = CreateFontW(-size, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0,
        to_wide(font_name).as_ptr()) as _;
    let old = SelectObject(hdc, font as _);
    SetBkMode(hdc, 1);
    SetTextColor(hdc, color);
    DrawTextW(hdc, text_w.as_ptr(), text_w.len() as i32, rc as *const _ as *mut _, DT_CENTER | DT_VCENTER | DT_SINGLELINE);
    SelectObject(hdc, old);
    DeleteObject(font as _);
}

unsafe fn settings_create_hotkey_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Hotkey.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 86);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 0);
    let sec2 = SettingsFormSectionLayout::new(page, 2, 0);

    let (_hk_lbl, hk_btn) = b.toggle_row(st, "启用快捷键", 6101, sec0.left(), sec0.row_y(0), sec0.full_w());
    st.chk_hk_enable = hk_btn;
    if !st.chk_hk_enable.is_null() { st.ownerdraw_ctrls.push(st.chk_hk_enable); }

    b.label(st, "修饰键：", sec0.left(), sec0.label_y(1, 24), 70, 24);
    st.cb_hk_mod = b.dropdown(st, "Win", 6102, sec0.field_x(), sec0.row_y(1), 170);
    if !st.cb_hk_mod.is_null() { st.ownerdraw_ctrls.push(st.cb_hk_mod); }
    let key_label_x = sec0.field_x() + 186;
    b.label(st, "按键：", key_label_x, sec0.label_y(1, 24), 50, 24);
    st.cb_hk_key = b.dropdown(st, "V", 6103, key_label_x + 50, sec0.row_y(1), 120);
    if !st.cb_hk_key.is_null() { st.ownerdraw_ctrls.push(st.cb_hk_key); }
    st.lb_hk_preview = b.label(st, "当前设置：Win + V", sec0.left(), sec0.label_y(2, 24), sec0.full_w(), 24);

    let _ = b.label_auto(st, "说明：通过注册表 DisabledHotkeys 屏蔽或恢复 Win+V。修改后通常需要重启资源管理器或重新登录。", sec1.left(), sec1.row_y(0), sec1.full_w(), 40);
    st.btn_clip_hist_block = b.button(st, "屏蔽 Win+V", 6111, sec1.action_x(0, 110), sec1.row_y(1), 110);
    st.btn_clip_hist_restore = b.button(st, "恢复 Win+V", 6112, sec1.action_x(1, 110), sec1.row_y(1), 110);
    st.btn_restart_explorer = b.button(st, "重启资源管理器", 6113, sec1.action_x(2, 130), sec1.row_y(1), 130);
    for &hh in &[st.btn_clip_hist_block, st.btn_clip_hist_restore, st.btn_restart_explorer] {
        if !hh.is_null() { st.ownerdraw_ctrls.push(hh); }
    }

    let (_desc1, d1h) = b.label_auto(st, "说明：保存后会立即重新注册主快捷键。", sec2.left(), sec2.row_y(0), sec2.full_w(), 24);
    let _ = b.label_auto(st, "建议避免使用 Ctrl+C / Ctrl+V 等系统级常用组合。", sec2.left(), sec2.row_y(0) + d1h + 6, sec2.full_w(), 24);

    st.ui.mark_built(page);
}

unsafe fn settings_create_plugin_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Plugin.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 110);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 0);
    let sec2 = SettingsFormSectionLayout::new(page, 2, 0);

    let (_qs_lbl, qs_btn) = b.toggle_row(st, "启用快速搜索", 7102, sec0.left(), sec0.row_y(0), sec0.full_w());
    st.chk_qs = qs_btn;
    if !st.chk_qs.is_null() { st.ownerdraw_ctrls.push(st.chk_qs); }
    b.label(st, "搜索引擎：", sec0.left(), sec0.label_y(1, 24), sec0.label_w, 24);
    st.cb_engine = b.dropdown(st, "筑森搜索（jzxx.vip）", 7201, sec0.field_x(), sec0.row_y(1), 240);
    if !st.cb_engine.is_null() { st.ownerdraw_ctrls.push(st.cb_engine); }
    b.label(st, "URL 模板：", sec0.left(), sec0.label_y(2, 24), sec0.label_w, 24);
    st.ed_tpl = b.edit(st, "", 7202, sec0.field_x(), sec0.row_y(2), sec0.field_w());
    let btn_restore_tpl = b.button(st, "恢复预设模板", 7203, sec0.left(), sec0.row_y(3), 130);
    if !btn_restore_tpl.is_null() { st.ownerdraw_ctrls.push(btn_restore_tpl); }
    let _ = b.label_auto(st, "占位符：{q}=编码后关键字，{raw}=原文", sec0.left() + 146, sec0.row_y(3) + 4, sec0.field_w_from(sec0.left() + 146), 24);
    let (_ai_lbl, ai_btn) = b.toggle_row(st, "AI 文本清洗（预留）", 7101, sec1.left(), sec1.row_y(0), sec1.full_w());
    st.chk_ai = ai_btn;
    if !st.chk_ai.is_null() { st.ownerdraw_ctrls.push(st.chk_ai); }
    let (_mm_lbl, mm_btn) = b.toggle_row(st, "启用超级邮件合并", 7103, sec2.left(), sec2.row_y(0), sec2.full_w());
    st.chk_mm = mm_btn;
    if !st.chk_mm.is_null() { st.ownerdraw_ctrls.push(st.chk_mm); }
    let btn_mail_merge = b.button(st, "打开超级邮件合并", IDC_SET_PLUGIN_MAILMERGE, sec2.left(), sec2.row_y(1), 170);
    if !btn_mail_merge.is_null() { st.ownerdraw_ctrls.push(btn_mail_merge); }
    st.ui.mark_built(page);
}

unsafe fn settings_create_group_page(hwnd: HWND, st: &mut SettingsWndState) {
    let ui_font = st.ui_font;
    let page = SettingsPage::Group.index();
    let sec0 = SettingsFormSectionLayout::new(page, 0, 0);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 0);

    let push = |st: &mut SettingsWndState, hh: HWND| {
        if !hh.is_null() { settings_page_push_ctrl(st, page, hh); }
    };

    let (group_lbl, group_btn, _lx, _ly, _lw, _lh, _bx, _by) = settings_create_toggle_plain(hwnd, "启用分组功能", IDC_SET_GROUP_ENABLE, sec0.left(), sec0.row_y(0), sec0.full_w(), ui_font);
    push(st, group_lbl);
    st.chk_group_enable = group_btn;
    settings_set_font(st.chk_group_enable, ui_font);
    push(st, st.chk_group_enable);
    if !st.chk_group_enable.is_null() { st.ownerdraw_ctrls.push(st.chk_group_enable); }

    st.lb_group_current = settings_create_label(hwnd, "当前分组：全部记录", sec1.left(), sec1.row_y(0), sec1.full_w(), 24, ui_font);
    push(st, st.lb_group_current);

    let lbl3 = settings_create_label(hwnd, "分组列表：", sec1.left(), sec1.row_y(1), 220, 22, ui_font);
    push(st, lbl3);

    st.lb_groups = settings_create_listbox(hwnd, IDC_SET_GROUP_LIST, sec1.left(), sec1.row_y(2), sec1.full_w(), 210, ui_font);
    if !st.lb_groups.is_null() { settings_page_push_ctrl(st, page, st.lb_groups); }

    let btn_y = sec1.row_y(2) + 226;
    let bw = 90;
    let gap = 10;
    let x0 = sec1.left();
    st.btn_group_add = settings_create_small_btn(hwnd, "新建分组", IDC_SET_GROUP_ADD, x0 + (bw + gap) * 0, btn_y, bw, ui_font);
    st.btn_group_rename = settings_create_small_btn(hwnd, "重命名", IDC_SET_GROUP_RENAME, x0 + (bw + gap) * 1, btn_y, bw, ui_font);
    st.btn_group_delete = settings_create_small_btn(hwnd, "删除", IDC_SET_GROUP_DELETE, x0 + (bw + gap) * 2, btn_y, bw, ui_font);
    st.btn_group_up = settings_create_small_btn(hwnd, "上移", IDC_SET_GROUP_UP, x0 + (bw + gap) * 3, btn_y, bw, ui_font);
    st.btn_group_down = settings_create_small_btn(hwnd, "下移", IDC_SET_GROUP_DOWN, x0 + (bw + gap) * 4, btn_y, bw, ui_font);
    for &hh in &[st.btn_group_add, st.btn_group_rename, st.btn_group_delete, st.btn_group_up, st.btn_group_down] {
        if !hh.is_null() {
            settings_page_push_ctrl(st, page, hh);
            st.ownerdraw_ctrls.push(hh);
        }
    }

    st.ui.mark_built(page);
}

unsafe fn settings_create_cloud_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Cloud.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 110);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 110);
    let sec2 = SettingsFormSectionLayout::new(page, 2, 0);

    let (_, toggle) = b.toggle_row(st, "启用自动同步", IDC_SET_CLOUD_ENABLE, sec0.left(), sec0.row_y(0), sec0.full_w());
    st.chk_cloud_enable = toggle;
    if !st.chk_cloud_enable.is_null() {
        st.ownerdraw_ctrls.push(st.chk_cloud_enable);
    }
    b.label(st, "同步间隔：", sec0.left(), sec0.label_y(1, 24), sec0.label_w, 24);
    st.cb_cloud_interval = b.dropdown(st, "1小时", IDC_SET_CLOUD_INTERVAL, sec0.field_x(), sec0.row_y(1), 150);
    st.lb_cloud_status = b.label(st, "上次同步：未同步", sec0.left(), sec0.label_y(2, 24), sec0.full_w(), 24);

    b.label(st, "WebDAV 地址：", sec1.left(), sec1.label_y(0, 24), sec1.label_w, 24);
    st.ed_cloud_url = b.edit(st, "", IDC_SET_CLOUD_URL, sec1.field_x(), sec1.row_y(0), sec1.field_w());
    b.label(st, "用户名：", sec1.left(), sec1.label_y(1, 24), sec1.label_w, 24);
    st.ed_cloud_user = b.edit(st, "", IDC_SET_CLOUD_USER, sec1.field_x(), sec1.row_y(1), sec1.field_w());
    b.label(st, "密码：", sec1.left(), sec1.label_y(2, 24), sec1.label_w, 24);
    st.ed_cloud_pass = b.add(st, settings_create_password_edit(hwnd, "", IDC_SET_CLOUD_PASS, sec1.field_x(), sec1.row_y(2), sec1.field_w(), st.ui_font));
    b.label(st, "远程目录：", sec1.left(), sec1.label_y(3, 24), sec1.label_w, 24);
    st.ed_cloud_dir = b.edit(st, "", IDC_SET_CLOUD_DIR, sec1.field_x(), sec1.row_y(3), sec1.field_w());

    let btn_w = 130;
    let gap = 14;
    let x0 = sec2.left();
    let x1 = x0 + btn_w + gap;
    let btn_sync = b.button(st, "立即同步", IDC_SET_CLOUD_SYNC_NOW, x0, sec2.row_y(0), btn_w);
    let btn_upload = b.button(st, "上传配置", IDC_SET_CLOUD_UPLOAD_CFG, x1, sec2.row_y(0), btn_w);
    let btn_apply = b.button(st, "应用云端配置", IDC_SET_CLOUD_APPLY_CFG, x0, sec2.row_y(1), btn_w);
    let btn_restore = b.button(st, "云备份恢复", IDC_SET_CLOUD_RESTORE_BACKUP, x1, sec2.row_y(1), btn_w);
    for &hh in &[btn_sync, btn_upload, btn_apply, btn_restore] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }

    st.ui.mark_built(page);
}

unsafe fn settings_create_about_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::About.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let sec = SettingsFormSectionLayout::new(page, 0, 0);
    let lines = [
        format!("版本：{}", env!("CARGO_PKG_VERSION")),
        "设置界面现在统一使用同一套 section/form 布局。".to_string(),
        "新增设置项时可以直接复用卡片、字段列、按钮行和统一间距。".to_string(),
        format!("数据目录：{}", data_dir().to_string_lossy()),
        format!("数据库：{}", db_file().to_string_lossy()),
    ];
    let mut y = sec.row_y(0);
    for line in lines.iter() {
        let (_, h) = b.label_auto(st, line, sec.left(), y, sec.full_w(), 24);
        y += h + 10;
    }
    st.ui.mark_built(page);
}

unsafe fn settings_button_hover(st: &SettingsWndState, hwnd_item: HWND) -> bool {
    if hwnd_item.is_null() { return false; }
    let mut pt: POINT = zeroed();
    if GetCursorPos(&mut pt) == 0 { return false; }
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd_item, &mut rc) == 0 { return false; }
    pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom && st.hot_ownerdraw == hwnd_item
}

unsafe fn settings_ensure_page(hwnd: HWND, st: &mut SettingsWndState, page: usize) {
    let page = page.min(SETTINGS_PAGES.len().saturating_sub(1));
    if st.ui.is_built(page) { return; }
    match SettingsPage::from_index(page) {
        SettingsPage::General => {
            settings_create_general_page(hwnd, st);
            st.ui.mark_built(page);
        }
        SettingsPage::Hotkey => settings_create_hotkey_page(hwnd, st),
        SettingsPage::Plugin => settings_create_plugin_page(hwnd, st),
        SettingsPage::Group => settings_create_group_page(hwnd, st),
        SettingsPage::Cloud => settings_create_cloud_page(hwnd, st),
        SettingsPage::About => settings_create_about_page(hwnd, st),
    }
}

unsafe fn settings_draw_button_item(st: &SettingsWndState, dis: &DRAWITEMSTRUCT) {
    let th = Theme::default();
    let hdc = dis.hDC;
    let rc = dis.rcItem;
    let cid = dis.CtlID as isize;
    let pressed = (dis.itemState & (ODS_SELECTED as u32)) != 0;
    let hover = settings_button_hover(st, dis.hwndItem);
    let text = get_window_text(dis.hwndItem);

    if cid == IDC_SET_AUTOSTART || cid == IDC_SET_CLOSETRAY
        || cid == IDC_SET_CLICK_HIDE || cid == IDC_SET_EDGEHIDE
        || cid == IDC_SET_HOVERPREVIEW || cid == IDC_SET_GROUP_ENABLE
        || cid == IDC_SET_CLOUD_ENABLE
        || cid == 6101 || cid == 7102 || cid == 7101 || cid == 7103
    {
        let checked = settings_toggle_get(st, cid);
        draw_settings_toggle_component(hdc as _, &rc, hover, checked, th);
        return;
    }

    let kind = if cid == IDC_SET_MAX || cid == IDC_SET_POSMODE || cid == IDC_SET_CLOUD_INTERVAL || cid == 6102 || cid == 6103 || cid == 7201 {
        SettingsComponentKind::Dropdown
    } else if cid == IDC_SET_SAVE {
        SettingsComponentKind::AccentButton
    } else {
        SettingsComponentKind::Button
    };
    draw_settings_button_component(hdc as _, &rc, &text, kind, hover, pressed, th);
}

unsafe extern "system" fn settings_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let parent_hwnd = cs.lpCreateParams as HWND;
            let (nav_font, ui_font, title_font) = settings_create_fonts();
            let mut st = Box::new(SettingsWndState {
                parent_hwnd,
                cur_page: 0,
                content_scroll_y: 0,
                scroll_dragging: false,
                scroll_drag_start_y: 0,
                scroll_drag_start_scroll: 0,
                scroll_bar_visible: false,
                scroll_hide_timer: false,
                ui: SettingsUiRegistry::new(),
                nav_hot: -1,
                btn_save: null_mut(),
                btn_close: null_mut(),
                btn_open_cfg: null_mut(),
                btn_open_db: null_mut(),
                btn_open_data: null_mut(),
                chk_autostart: null_mut(),
                chk_close_tray: null_mut(),
                chk_click_hide: null_mut(),
                chk_edge_hide: null_mut(),
                chk_hover_preview: null_mut(),
                chk_group_enable: null_mut(),
                lb_group_current: null_mut(),
                lb_groups: null_mut(),
                btn_group_add: null_mut(),
                btn_group_rename: null_mut(),
                btn_group_delete: null_mut(),
                btn_group_up: null_mut(),
                btn_group_down: null_mut(),
                groups_cache: Vec::new(),
                chk_hk_enable: null_mut(),
                cb_hk_mod: null_mut(),
                cb_hk_key: null_mut(),
                lb_hk_preview: null_mut(),
                btn_clip_hist_block: null_mut(),
                btn_clip_hist_restore: null_mut(),
                btn_restart_explorer: null_mut(),
                chk_qs: null_mut(),
                cb_engine: null_mut(),
                ed_tpl: null_mut(),
                chk_ai: null_mut(),
                chk_mm: null_mut(),
                chk_cloud_enable: null_mut(),
                cb_cloud_interval: null_mut(),
                ed_cloud_url: null_mut(),
                ed_cloud_user: null_mut(),
                ed_cloud_pass: null_mut(),
                ed_cloud_dir: null_mut(),
                lb_cloud_status: null_mut(),
                cb_max: null_mut(),
                cb_pos: null_mut(),
                ed_dx: null_mut(),
                ed_dy: null_mut(),
                ed_fx: null_mut(),
                ed_fy: null_mut(),
                nav_font,
                ui_font,
                title_font,
                draft: AppSettings::default(),
                ownerdraw_ctrls: Vec::new(),
                hot_ownerdraw: null_mut(),
                bg_brush: null_mut(),
                surface_brush: null_mut(),
                control_brush: null_mut(),
                nav_brush: null_mut(),
                dropdown_popup: null_mut(),
            });

            settings_refresh_theme_resources(&mut st);

            // 顶部按钮（Fluent 自绘）
            st.btn_save = settings_create_btn(hwnd, "保存", IDC_SET_SAVE, 984, 24, 72, st.ui_font);
            st.btn_close = settings_create_btn(hwnd, "关闭", IDC_SET_CLOSE, 900, 24, 64, st.ui_font);
            for &hh in &[st.btn_save, st.btn_close] {
                if !hh.is_null() { st.ownerdraw_ctrls.push(hh); }
            }
            settings_ensure_page(hwnd, &mut st, SettingsPage::General.index());
            settings_apply_from_app(&mut st);
            settings_show_page(hwnd, &mut st, 0);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(st) as isize);
            apply_window_corner_preference(hwnd);
            apply_dark_mode_to_window(hwnd);
            0
        }
        WM_MOUSEMOVE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            ensure_mouse_leave_tracking(hwnd);
            let st = &mut *st_ptr;
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            // 滚动条拖拽
            if st.scroll_dragging {
                let my = y;
                let mut crc: RECT = core::mem::zeroed();
                GetClientRect(hwnd, &mut crc);
                let view_h = (crc.bottom - crc.top) - SETTINGS_CONTENT_Y;
                let max_s = settings_max_scroll(view_h);
                // 与 settings_scrollbar_thumb_w 保持一致的轨道范围
                let track_top    = SETTINGS_CONTENT_Y + 8;
                let track_bottom = crc.bottom - 8;
                let track_h = (track_bottom - track_top).max(1);
                let thumb_h = ((view_h as f32 / SETTINGS_CONTENT_TOTAL_H as f32) * track_h as f32) as i32;
                let thumb_h = thumb_h.max(24);
                let drag_range = (track_h - thumb_h).max(1);
                let dy = my - st.scroll_drag_start_y;
                let new_y = st.scroll_drag_start_scroll + (dy as f32 / drag_range as f32 * max_s as f32) as i32;
                settings_scroll_to(hwnd, st, new_y);
                return 0;
            }
            // 导航 hover 高亮
            let mut hot = -1;
            for i in 0..SETTINGS_PAGES.len() {
                let rc = settings_nav_item_rect(i);
                if x >= rc.left && x <= rc.right && y >= rc.top && y <= rc.bottom {
                    hot = i as i32;
                    break;
                }
            }
            if hot != st.nav_hot {
                let old_hot = st.nav_hot;
                st.nav_hot = hot;
                if old_hot >= 0 {
                    let old_rc = settings_nav_item_rect(old_hot as usize);
                    InvalidateRect(hwnd, &old_rc, 0);
                }
                if hot >= 0 {
                    let new_rc = settings_nav_item_rect(hot as usize);
                    InvalidateRect(hwnd, &new_rc, 0);
                }
            }
            let pt = POINT { x, y };
            let mut hot_ctrl = ChildWindowFromPointEx(hwnd, pt, CWP_SKIPDISABLED | CWP_SKIPINVISIBLE);
            if !st.ownerdraw_ctrls.iter().any(|&hh| hh == hot_ctrl) {
                hot_ctrl = null_mut();
            }
            if hot_ctrl != st.hot_ownerdraw {
                if !st.hot_ownerdraw.is_null() { InvalidateRect(st.hot_ownerdraw, null(), 1); }
                st.hot_ownerdraw = hot_ctrl;
                if !st.hot_ownerdraw.is_null() { InvalidateRect(st.hot_ownerdraw, null(), 1); }
            }
            0
        }
        WM_MOUSELEAVE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                let st = &mut *st_ptr;
                if st.nav_hot >= 0 {
                    let rc = settings_nav_item_rect(st.nav_hot as usize);
                    st.nav_hot = -1;
                    InvalidateRect(hwnd, &rc, 0);
                }
                if !st.hot_ownerdraw.is_null() {
                    let old = st.hot_ownerdraw;
                    st.hot_ownerdraw = null_mut();
                    InvalidateRect(old, null(), 1);
                }
            }
            0
        }
        WM_LBUTTONDOWN => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() { return 0; }
            let st = &mut *st_ptr;
            let mx = get_x_lparam(lparam);
            let my = get_y_lparam(lparam);
            if !st.dropdown_popup.is_null() && IsWindow(st.dropdown_popup) != 0 {
                let mut prc: RECT = core::mem::zeroed();
                GetWindowRect(st.dropdown_popup, &mut prc);
                let mut pt = POINT { x: mx, y: my };
                ClientToScreen(hwnd, &mut pt);
                if !(pt.x >= prc.left && pt.x <= prc.right && pt.y >= prc.top && pt.y <= prc.bottom) {
                    DestroyWindow(st.dropdown_popup);
                    st.dropdown_popup = null_mut();
                }
            }
            // 导航项点击
            for i in 0..SETTINGS_PAGES.len() {
                let rc = settings_nav_item_rect(i);
                if mx >= rc.left && mx <= rc.right && my >= rc.top && my <= rc.bottom {
                    settings_show_page(hwnd, st, i);
                    let mut rc2: RECT = core::mem::zeroed();
                    GetClientRect(hwnd, &mut rc2);
                    let viewport = settings_viewport_rect(&rc2);
                    InvalidateRect(hwnd, &viewport, 0);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
            }
            // 滚动条点击（用 ACTIVE 宽度扩大命中区域，便于点击）
            let mut crc: RECT = core::mem::zeroed();
            GetClientRect(hwnd, &mut crc);
            if let Some(thumb) = settings_scrollbar_thumb_w(&crc, st.content_scroll_y, SCROLL_BAR_W_ACTIVE) {
                if mx >= thumb.left - 4 && mx <= thumb.right + 4 && my >= thumb.top && my <= thumb.bottom {
                    st.scroll_dragging = true;
                    st.scroll_drag_start_y = my;
                    st.scroll_drag_start_scroll = st.content_scroll_y;
                    settings_scrollbar_show(hwnd, st);
                    SetCapture(hwnd);
                    InvalidateRect(hwnd, null(), 0); // 立即重绘为粗条
                    return 0;
                }
            }
            // 点击轨道区域（右侧8px内）跳转
            let right_edge = crc.right - SCROLL_BAR_MARGIN;
            let left_edge  = right_edge - SCROLL_BAR_W_ACTIVE - 4;
            if mx >= left_edge && mx <= right_edge + 2 && my >= SETTINGS_CONTENT_Y + 4 && my < crc.bottom - 4 {
                let view_h = (crc.bottom - crc.top) - SETTINGS_CONTENT_Y;
                let max_s = settings_max_scroll(view_h);
                let track_h = (crc.bottom - 8 - (SETTINGS_CONTENT_Y + 8)).max(1);
                let new_y = ((my - SETTINGS_CONTENT_Y - 8) as f32 / track_h as f32 * max_s as f32) as i32;
                settings_scroll_to(hwnd, st, new_y);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEWHEEL => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() { return 0; }
            let st = &mut *st_ptr;
            let delta = ((wparam >> 16) & 0xffff) as u16 as i16 as i32;
            settings_scroll(hwnd, st, if delta > 0 { -60 } else { 60 });
            0
        }
        WM_LBUTTONUP => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() && (*st_ptr).scroll_dragging {
                (*st_ptr).scroll_dragging = false;
                ReleaseCapture();
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_SETTINGS_DROPDOWN_SELECTED => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() { return 0; }
            let st = &mut *st_ptr;
            st.dropdown_popup = null_mut();
            let idx = lparam as usize;
            match wparam as isize {
                IDC_SET_MAX => {
                    let items = ["100", "200", "500", "1000", "3000", "无限制"];
                    if let Some(label) = items.get(idx) {
                        settings_set_text(st.cb_max, label);
                        InvalidateRect(st.cb_max, null(), 1);
                    }
                }
                IDC_SET_POSMODE => {
                    let items = ["跟随鼠标", "固定位置", "上次位置"];
                    if let Some(label) = items.get(idx) {
                        settings_set_text(st.cb_pos, label);
                        InvalidateRect(st.cb_pos, null(), 1);
                        settings_sync_pos_fields_enabled(st);
                    }
                }
                IDC_SET_CLOUD_INTERVAL => {
                    let items = ["15分钟", "30分钟", "1小时", "6小时", "12小时", "24小时"];
                    if let Some(label) = items.get(idx) {
                        settings_set_text(st.cb_cloud_interval, label);
                        InvalidateRect(st.cb_cloud_interval, null(), 1);
                    }
                }
                6102 => {
                    if let Some(label) = HOTKEY_MOD_OPTIONS.get(idx) {
                        settings_set_text(st.cb_hk_mod, label);
                        settings_set_text(st.lb_hk_preview, &hotkey_preview_text(label, &get_window_text(st.cb_hk_key)));
                        InvalidateRect(st.cb_hk_mod, null(), 1);
                    }
                }
                6103 => {
                    if let Some(label) = HOTKEY_KEY_OPTIONS.get(idx) {
                        settings_set_text(st.cb_hk_key, label);
                        settings_set_text(st.lb_hk_preview, &hotkey_preview_text(&get_window_text(st.cb_hk_mod), label));
                        InvalidateRect(st.cb_hk_key, null(), 1);
                    }
                }
                7201 => {
                    if let Some((_, label, tpl)) = SEARCH_ENGINE_PRESETS.get(idx) {
                        settings_set_text(st.cb_engine, label);
                        if get_window_text(st.ed_tpl).trim().is_empty() || get_window_text(st.ed_tpl) == search_engine_template(search_engine_key_from_display(&get_window_text(st.cb_engine))) {
                            settings_set_text(st.ed_tpl, tpl);
                        }
                        InvalidateRect(st.cb_engine, null(), 1);
                    }
                }
                _ => {}
            }
            0
        }
        WM_DRAWITEM => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() { return 0; }
            let st = &mut *st_ptr;
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            let rc0 = dis.rcItem;
            let w = (rc0.right - rc0.left).max(1);
            let h = (rc0.bottom - rc0.top).max(1);
            let memdc = CreateCompatibleDC(dis.hDC);
            let bmp = CreateCompatibleBitmap(dis.hDC, w, h);
            let oldbmp = SelectObject(memdc, bmp as _);
            let th = Theme::default();
            let bg_fill = if dis.CtlID as isize == IDC_SET_AUTOSTART || dis.CtlID as isize == IDC_SET_CLOSETRAY || dis.CtlID as isize == IDC_SET_CLICK_HIDE || dis.CtlID as isize == IDC_SET_EDGEHIDE || dis.CtlID as isize == IDC_SET_HOVERPREVIEW || dis.CtlID as isize == IDC_SET_GROUP_ENABLE || dis.CtlID as isize == IDC_SET_CLOUD_ENABLE || dis.CtlID as isize == 6101 || dis.CtlID as isize == 7102 || dis.CtlID as isize == 7101 || dis.CtlID as isize == 7103 { th.surface } else { th.bg };
            let bg = CreateSolidBrush(bg_fill);
            let local = RECT { left: 0, top: 0, right: w, bottom: h };
            FillRect(memdc, &local, bg);
            DeleteObject(bg as _);
            let mut dis2 = *dis;
            dis2.hDC = memdc;
            dis2.rcItem = local;
            settings_draw_button_item(st, &dis2);
            BitBlt(dis.hDC, rc0.left, rc0.top, w, h, memdc, 0, 0, SRCCOPY);
            SelectObject(memdc, oldbmp);
            DeleteObject(bmp as _);
            DeleteDC(memdc);
            1
        }
        WM_COMMAND => {
            let cmd = loword(wparam as u32) as isize;
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() { return 0; }
            let st = &mut *st_ptr;
            match cmd {
                IDC_SET_AUTOSTART | IDC_SET_CLOSETRAY | IDC_SET_CLICK_HIDE | IDC_SET_EDGEHIDE | IDC_SET_HOVERPREVIEW | IDC_SET_GROUP_ENABLE | IDC_SET_CLOUD_ENABLE | 6101 | 7102 | 7101 | 7103 => {
                    settings_toggle_flip(st, cmd);
                    let sender = lparam as HWND;
                    if !sender.is_null() { InvalidateRect(sender, null(), 1); }
                }
                IDC_SET_GROUP_ADD => {
                    if let Some(name) = input_name_dialog(hwnd, "新建分组", "请输入分组名称：", "新分组") {
                        match db_create_named_group(&name) {
                            Ok(group) => {
                                settings_groups_refresh_list(st, group.id);
                                let pst = get_state_ptr(st.parent_hwnd);
                                if !pst.is_null() { reload_state_from_db(&mut *pst); InvalidateRect(st.parent_hwnd, null(), 1); }
                            }
                            Err(e) => {
                                MessageBoxW(hwnd, to_wide(&format!("新建分组失败：{}", e)).as_ptr(), to_wide("分组").as_ptr(), MB_OK | MB_ICONERROR);
                            }
                        }
                    }
                }
                IDC_SET_GROUP_RENAME => {
                    if let Some((_, g)) = settings_groups_selected(st) {
                        if let Some(new_name) = input_name_dialog(hwnd, "重命名分组", "请输入新名称：", &g.name) {
                            if let Err(e) = db_rename_group(g.id, &new_name) {
                                MessageBoxW(hwnd, to_wide(&format!("重命名失败：{}", e)).as_ptr(), to_wide("分组").as_ptr(), MB_OK | MB_ICONERROR);
                            } else {
                                settings_groups_refresh_list(st, g.id);
                                let pst = get_state_ptr(st.parent_hwnd);
                                if !pst.is_null() { reload_state_from_db(&mut *pst); InvalidateRect(st.parent_hwnd, null(), 1); }
                            }
                        }
                    } else {
                        MessageBoxW(hwnd, to_wide("请先选择一个分组。").as_ptr(), to_wide("分组").as_ptr(), MB_OK | MB_ICONINFORMATION);
                    }
                }
                IDC_SET_GROUP_DELETE => {
                    if let Some((_, g)) = settings_groups_selected(st) {
                        let ask = format!("确认删除分组“{}”？\n不会删除记录，只会清空这些记录的分组。", g.name);
                        if MessageBoxW(hwnd, to_wide(&ask).as_ptr(), to_wide("分组").as_ptr(), MB_YESNO | MB_ICONQUESTION) == IDYES {
                            if let Err(e) = db_delete_group(g.id) {
                                MessageBoxW(hwnd, to_wide(&format!("删除分组失败：{}", e)).as_ptr(), to_wide("分组").as_ptr(), MB_OK | MB_ICONERROR);
                            } else {
                                settings_groups_refresh_list(st, 0);
                                let pst = get_state_ptr(st.parent_hwnd);
                                if !pst.is_null() { reload_state_from_db(&mut *pst); InvalidateRect(st.parent_hwnd, null(), 1); }
                            }
                        }
                    }
                }
                IDC_SET_GROUP_UP => { settings_groups_move(st, -1); }
                IDC_SET_GROUP_DOWN => { settings_groups_move(st, 1); }
                IDC_SET_GROUP_LIST => {
                    if hiword(wparam as u32) as u32 == LBN_SELCHANGE {
                        settings_groups_sync_name(st);
                    }
                }
                IDC_SET_MAX => {
                    if !st.dropdown_popup.is_null() { if IsWindow(st.dropdown_popup) != 0 { DestroyWindow(st.dropdown_popup); } st.dropdown_popup = null_mut(); }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_max, &mut rc);
                    let current = settings_dropdown_index_for_max_items(settings_dropdown_max_items_from_label(&get_window_text(st.cb_max)));
                    st.dropdown_popup = show_settings_dropdown_popup(hwnd, IDC_SET_MAX, &rc, &["100", "200", "500", "1000", "3000", "无限制"], current, 180);
                }
                IDC_SET_POSMODE => {
                    if !st.dropdown_popup.is_null() { if IsWindow(st.dropdown_popup) != 0 { DestroyWindow(st.dropdown_popup); } st.dropdown_popup = null_mut(); }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_pos, &mut rc);
                    let current = settings_dropdown_index_for_pos_mode(&settings_dropdown_pos_mode_from_label(&get_window_text(st.cb_pos)));
                    st.dropdown_popup = show_settings_dropdown_popup(hwnd, IDC_SET_POSMODE, &rc, &["跟随鼠标", "固定位置", "上次位置"], current, 180);
                }
                IDC_SET_CLOUD_INTERVAL => {
                    if !st.dropdown_popup.is_null() { if IsWindow(st.dropdown_popup) != 0 { DestroyWindow(st.dropdown_popup); } st.dropdown_popup = null_mut(); }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_cloud_interval, &mut rc);
                    let items = ["15分钟", "30分钟", "1小时", "6小时", "12小时", "24小时"];
                    let current = items.iter().position(|x| *x == get_window_text(st.cb_cloud_interval)).unwrap_or(2);
                    st.dropdown_popup = show_settings_dropdown_popup(hwnd, IDC_SET_CLOUD_INTERVAL, &rc, &items, current, 180);
                }
                6102 => {
                    if !st.dropdown_popup.is_null() { if IsWindow(st.dropdown_popup) != 0 { DestroyWindow(st.dropdown_popup); } st.dropdown_popup = null_mut(); }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_hk_mod, &mut rc);
                    let current = HOTKEY_MOD_OPTIONS.iter().position(|x| *x == get_window_text(st.cb_hk_mod)).unwrap_or(0);
                    st.dropdown_popup = show_settings_dropdown_popup(hwnd, 6102, &rc, &HOTKEY_MOD_OPTIONS, current, 200);
                }
                6103 => {
                    if !st.dropdown_popup.is_null() { if IsWindow(st.dropdown_popup) != 0 { DestroyWindow(st.dropdown_popup); } st.dropdown_popup = null_mut(); }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_hk_key, &mut rc);
                    let current = HOTKEY_KEY_OPTIONS.iter().position(|x| *x == get_window_text(st.cb_hk_key)).unwrap_or(21);
                    st.dropdown_popup = show_settings_dropdown_popup(hwnd, 6103, &rc, &HOTKEY_KEY_OPTIONS, current, 220);
                }
                7201 => {
                    if !st.dropdown_popup.is_null() { if IsWindow(st.dropdown_popup) != 0 { DestroyWindow(st.dropdown_popup); } st.dropdown_popup = null_mut(); }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_engine, &mut rc);
                    let current = SEARCH_ENGINE_PRESETS.iter().position(|(_,name,_)| *name == get_window_text(st.cb_engine)).unwrap_or(0);
                    let labels: Vec<&str> = SEARCH_ENGINE_PRESETS.iter().map(|(_,name,_)| *name).collect();
                    st.dropdown_popup = show_settings_dropdown_popup(hwnd, 7201, &rc, &labels, current, 260);
                }
                7203 => {
                    let key = search_engine_key_from_display(&get_window_text(st.cb_engine));
                    settings_set_text(st.ed_tpl, search_engine_template(key));
                }
                IDC_SET_PLUGIN_MAILMERGE => {
                    launch_mail_merge_window(hwnd);
                }
                6111 => {
                    if let Err(e) = toggle_disabled_hotkey_char('V', true) {
                        MessageBoxW(hwnd, to_wide(&format!("屏蔽 Win+V 失败：{}", e)).as_ptr(), to_wide("系统剪贴板历史").as_ptr(), MB_OK | MB_ICONERROR);
                    }
                }
                6112 => {
                    if let Err(e) = toggle_disabled_hotkey_char('V', false) {
                        MessageBoxW(hwnd, to_wide(&format!("恢复 Win+V 失败：{}", e)).as_ptr(), to_wide("系统剪贴板历史").as_ptr(), MB_OK | MB_ICONERROR);
                    }
                }
                6113 => {
                    if let Err(e) = restart_explorer_shell() {
                        MessageBoxW(hwnd, to_wide(&format!("重启资源管理器失败：{}", e)).as_ptr(), to_wide("系统剪贴板历史").as_ptr(), MB_OK | MB_ICONERROR);
                    }
                }
                IDC_SET_CLOUD_SYNC_NOW | IDC_SET_CLOUD_UPLOAD_CFG | IDC_SET_CLOUD_APPLY_CFG | IDC_SET_CLOUD_RESTORE_BACKUP => {
                    let msg = match cmd {
                        IDC_SET_CLOUD_SYNC_NOW => "云同步执行入口已经迁到统一框架，下一步继续接真实同步逻辑。",
                        IDC_SET_CLOUD_UPLOAD_CFG => "上传配置入口已经迁到统一框架，下一步继续接真实上传逻辑。",
                        IDC_SET_CLOUD_APPLY_CFG => "应用云端配置入口已经迁到统一框架，下一步继续接真实下载逻辑。",
                        _ => "云备份恢复入口已经迁到统一框架，下一步继续接数据库与资源恢复逻辑。",
                    };
                    MessageBoxW(hwnd, to_wide(msg).as_ptr(), to_wide("云同步").as_ptr(), MB_OK | MB_ICONINFORMATION);
                }
                IDC_SET_SAVE => {
                    settings_collect_to_app(st);
                    // 刷新标题栏按钮（开机自启 UI 无需弹窗，直接关闭即可）
                    DestroyWindow(hwnd);
                }
                IDC_SET_CLOSE => { DestroyWindow(hwnd); }
                IDC_SET_BTN_OPENCFG => {
                    let pst = get_state_ptr(st.parent_hwnd);
                    if !pst.is_null() {
                        save_settings(&(*pst).settings);
                    } else if !settings_file().exists() {
                        save_settings(&AppSettings::default());
                    }
                    let path = settings_file();
                    open_path_with_shell(path.to_string_lossy().as_ref());
                }
                IDC_SET_BTN_OPENDATA => {
                    let dir = data_dir();
                    let _ = fs::create_dir_all(&dir);
                    open_path_with_shell(dir.to_string_lossy().as_ref());
                }
                IDC_SET_BTN_OPENDB => {
                    ensure_db();
                    let db = db_file();
                    open_path_with_shell(db.to_string_lossy().as_ref());
                }
                _ => {}
            }
            0
        }
        WM_THEMECHANGED | WM_SETTINGCHANGE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                settings_refresh_theme_resources(&mut *st_ptr);
                apply_dark_mode_to_window(hwnd);
                InvalidateRect(hwnd, null(), 1);
            }
            0
        }
        WM_CTLCOLORSTATIC => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            let hdc = wparam as *mut core::ffi::c_void;
            if !st_ptr.is_null() {
                SetBkMode(hdc, 2);
                SetBkColor(hdc, Theme::default().surface);
                SetTextColor(hdc, Theme::default().text);
                return (*st_ptr).surface_brush as isize;
            }
            0
        }
        WM_CTLCOLOREDIT => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            let hdc = wparam as *mut core::ffi::c_void;
            if !st_ptr.is_null() {
                SetBkColor(hdc, Theme::default().control_bg);
                SetTextColor(hdc, Theme::default().text);
                return (*st_ptr).control_brush as isize;
            }
            0
        }
        WM_CTLCOLORLISTBOX => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            let hdc = wparam as *mut core::ffi::c_void;
            if !st_ptr.is_null() {
                SetBkColor(hdc, Theme::default().surface);
                SetTextColor(hdc, Theme::default().text);
                return (*st_ptr).surface_brush as isize;
            }
            0
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !hdc.is_null() {
                let th = Theme::default();
                let mut rc: RECT = zeroed();
                GetClientRect(hwnd, &mut rc);
                let paint_target = begin_buffered_paint(hdc, &rc);
                let memdc = if let Some((_, pdc)) = paint_target { pdc } else { hdc };

                let bg = CreateSolidBrush(th.bg);
                FillRect(memdc, &rc, bg);
                DeleteObject(bg as _);

                let nav_rc = RECT { left: 0, top: 0, right: SETTINGS_NAV_W, bottom: rc.bottom };
                draw_round_rect(memdc as _, &nav_rc, th.nav_bg, 0, 0);
                let line_pen = CreatePen(0, 1, th.stroke);
                let old_pen = SelectObject(memdc, line_pen as _);
                MoveToEx(memdc, nav_divider_x(), 0, null_mut());
                LineTo(memdc, nav_divider_x(), rc.bottom);
                SelectObject(memdc, old_pen);
                DeleteObject(line_pen as _);

                let menu_rc = RECT { left: 22, top: 18, right: 50, bottom: 46 };
                draw_text_ex(memdc as _, "", &menu_rc, th.text_muted, 16, false, false, "Segoe Fluent Icons");
                let title_rc = RECT { left: 56, top: 18, right: 220, bottom: 50 };
                draw_text_ex(memdc as _, "设置", &title_rc, th.text, 15, true, false, "Segoe UI Variable Text");
                let cur_page = if st_ptr.is_null() { 0 } else { (*st_ptr).cur_page.min(SETTINGS_PAGES.len()-1) };
                let scroll_y = if st_ptr.is_null() { 0 } else { (*st_ptr).content_scroll_y };
                let sub_rc = settings_title_rect();
                draw_text_ex(memdc as _, SETTINGS_PAGES[cur_page], &sub_rc, th.text, 24, true, false, "Segoe UI Variable Display");

                for i in 0..SETTINGS_PAGES.len() {
                    let selected = !st_ptr.is_null() && (*st_ptr).cur_page == i;
                    let hover = !st_ptr.is_null() && (*st_ptr).nav_hot == i as i32;
                    draw_settings_nav_item(memdc as _, i, selected, hover, th);
                }

                // 内容区裁剪（防止滚动时卡片溢出到标题栏区域）
                let content_clip = settings_safe_paint_rect(&rc);
                SaveDC(memdc);
                IntersectClipRect(memdc, content_clip.left, content_clip.top, content_clip.right, content_clip.bottom);
                draw_settings_page_cards(memdc as _, cur_page, scroll_y, th);
                draw_settings_page_content(memdc as _, cur_page, th);
                RestoreDC(memdc, -1);

                let mask_rc = settings_viewport_mask_rect(&rc);
                let mask_br = CreateSolidBrush(th.bg);
                FillRect(memdc, &mask_rc, mask_br);
                DeleteObject(mask_br as _);
                let mask_line = CreateSolidBrush(th.stroke);
                let mask_sep = RECT { left: mask_rc.left + 12, top: mask_rc.bottom - 1, right: mask_rc.right - 12, bottom: mask_rc.bottom };
                FillRect(memdc, &mask_sep, mask_line);
                DeleteObject(mask_line as _);

                // 自绘 WinUI 细条滚动条（仅在滚动时短暂显示，1.5秒后自动隐藏）
                let view_h = (rc.bottom - rc.top) - SETTINGS_CONTENT_Y;
                let show_bar = !st_ptr.is_null() && (*st_ptr).scroll_bar_visible
                    && settings_max_scroll(view_h) > 0;
                if show_bar {
                    let dragging = !st_ptr.is_null() && (*st_ptr).scroll_dragging;
                    let bar_w = if dragging { SCROLL_BAR_W_ACTIVE } else { SCROLL_BAR_W };
                    let track_rc = RECT {
                        left:   rc.right - bar_w - SCROLL_BAR_MARGIN,
                        top:    SETTINGS_CONTENT_Y + 8,
                        right:  rc.right - SCROLL_BAR_MARGIN,
                        bottom: rc.bottom - 8,
                    };
                    if dragging {
                        let track_color = if th.bg == rgb(32,32,32) { rgb(70,70,70) } else { rgb(200,200,200) };
                        let track_br = CreateSolidBrush(track_color);
                        let old_br = SelectObject(memdc, track_br as _);
                        let old_pn = SelectObject(memdc, GetStockObject(NULL_PEN));
                        RoundRect(memdc, track_rc.left, track_rc.top, track_rc.right+1, track_rc.bottom+1, bar_w, bar_w);
                        SelectObject(memdc, old_pn);
                        SelectObject(memdc, old_br);
                        DeleteObject(track_br as _);
                    }
                    if let Some(thumb) = settings_scrollbar_thumb_w(&rc, scroll_y, bar_w) {
                        let thumb_color = if dragging { th.accent }
                            else if th.bg == rgb(32,32,32) { rgb(120,120,120) }
                            else { rgb(160,160,160) };
                        let thumb_br = CreateSolidBrush(thumb_color);
                        let old_b2 = SelectObject(memdc, thumb_br as _);
                        let old_p2 = SelectObject(memdc, GetStockObject(NULL_PEN));
                        RoundRect(memdc, thumb.left, thumb.top, thumb.right+1, thumb.bottom+1, bar_w, bar_w);
                        SelectObject(memdc, old_p2);
                        SelectObject(memdc, old_b2);
                        DeleteObject(thumb_br as _);
                    }
                }

                if let Some((paint_buf, _)) = paint_target {
                    end_buffered_paint(paint_buf, true);
                }
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_TIMER => {
            if wparam == ID_TIMER_SETTINGS_SCROLLBAR {
                let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
                if !st_ptr.is_null() {
                    let st = &mut *st_ptr;
                    KillTimer(hwnd, ID_TIMER_SETTINGS_SCROLLBAR);
                    st.scroll_hide_timer = false;
                    st.scroll_bar_visible = false;
                    InvalidateRect(hwnd, null(), 0);
                }
                return 0;
            }
            0
        }
        WM_CLOSE => { DestroyWindow(hwnd); 0 }
        WM_DESTROY => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                let parent = (*st_ptr).parent_hwnd;
                if !(*st_ptr).dropdown_popup.is_null() { if IsWindow((*st_ptr).dropdown_popup) != 0 { DestroyWindow((*st_ptr).dropdown_popup); } (*st_ptr).dropdown_popup = null_mut(); }
                if !(*st_ptr).nav_font.is_null() { DeleteObject((*st_ptr).nav_font as _); }
                if !(*st_ptr).ui_font.is_null() && (*st_ptr).ui_font != GetStockObject(DEFAULT_GUI_FONT) { DeleteObject((*st_ptr).ui_font as _); }
                if !(*st_ptr).title_font.is_null() && (*st_ptr).title_font != GetStockObject(DEFAULT_GUI_FONT) { DeleteObject((*st_ptr).title_font as _); }
                if !(*st_ptr).bg_brush.is_null() { DeleteObject((*st_ptr).bg_brush as _); }
                if !(*st_ptr).surface_brush.is_null() { DeleteObject((*st_ptr).surface_brush as _); }
                if !(*st_ptr).control_brush.is_null() { DeleteObject((*st_ptr).control_brush as _); }
                if !(*st_ptr).nav_brush.is_null() { DeleteObject((*st_ptr).nav_brush as _); }
                drop(Box::from_raw(st_ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let pst = get_state_ptr(parent);
                if !pst.is_null() { (*pst).settings_hwnd = null_mut(); }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_settings_class() {
    let hinstance = GetModuleHandleW(null());
    let cname = to_wide(SETTINGS_CLASS);
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(settings_wnd_proc);
    wc.hInstance = hinstance;
    wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
    wc.hIcon = null_mut();
    wc.hIconSm = null_mut();
    wc.hbrBackground = null_mut();
    wc.lpszClassName = cname.as_ptr();
    RegisterClassExW(&wc);
}

unsafe fn open_settings_window(hwnd: HWND) {
    let pst = get_state_ptr(hwnd);
    if pst.is_null() { return; }
    let app = &mut *pst;
    if !app.settings_hwnd.is_null() {
        ShowWindow(app.settings_hwnd, SW_SHOW);
        SetForegroundWindow(app.settings_hwnd);
        return;
    }
    ensure_settings_class();
    let hinstance = GetModuleHandleW(null());
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let sh = GetSystemMetrics(SM_CYSCREEN);
    let x = max(0, (sw - SETTINGS_W) / 2);
    let y = max(0, (sh - SETTINGS_H) / 2);
    let whd = CreateWindowExW(
        WS_EX_APPWINDOW | WS_EX_DLGMODALFRAME,
        to_wide(SETTINGS_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_VISIBLE | WS_CLIPCHILDREN,
        x,
        y,
        SETTINGS_W,
        SETTINGS_H,
        hwnd,
        null_mut(),
        hinstance,
        hwnd as _,
    );
    if !whd.is_null() {
        if app.icons.app != 0 {
            SendMessageW(whd, WM_SETICON, ICON_SMALL as usize, app.icons.app as LPARAM);
            SendMessageW(whd, WM_SETICON, ICON_BIG as usize, app.icons.app as LPARAM);
            SetClassLongPtrW(whd, GCLP_HICON, app.icons.app);
            SetClassLongPtrW(whd, GCLP_HICONSM, app.icons.app);
        }
        apply_window_corner_preference(whd);
        apply_dark_mode_to_window(whd);
        app.settings_hwnd = whd;
    }
}

fn reload_state_from_db(state: &mut AppState) {
    ensure_db();
    state.groups = db_load_groups();
    state.records = db_load_items(0);
    state.phrases = db_load_items(1);
    for i in 0..state.tab_group_filters.len() {
        let gid = state.tab_group_filters[i];
        if gid > 0 && !state.groups.iter().any(|g| g.id == gid) {
            state.tab_group_filters[i] = 0;
        }
    }
    if state.tab_index < state.tab_group_filters.len() {
        state.current_group_filter = state.tab_group_filters[state.tab_index];
    }
    state.refilter();
}

pub fn run() -> AppResult<()> {
    // ── 单实例保护：若已有实例运行则激活它并退出 ──
    unsafe {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn CreateMutexW(lp_attributes: *const core::ffi::c_void, b_initial_owner: i32, lp_name: *const u16) -> *mut core::ffi::c_void;
            fn GetLastError() -> u32;
        }
        const ERROR_ALREADY_EXISTS: u32 = 183;
        let name: Vec<u16> = "Global\\ZsClipSingleInstance".encode_utf16().chain(std::iter::once(0u16)).collect();
        let _mutex = CreateMutexW(core::ptr::null(), 0, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // 已有实例：找到主窗口并激活
            use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, ShowWindow, SetForegroundWindow, SW_RESTORE};
            let cls = to_wide("ZsClipMain");
            let hwnd = FindWindowW(cls.as_ptr(), core::ptr::null());
            if !hwnd.is_null() {
                ShowWindow(hwnd, SW_RESTORE);
                SetForegroundWindow(hwnd);
            }
            return Ok(());
        }
    }
    unsafe {
        // 进程级深色模式初始化：让系统菜单、滚动条、控件跟随主题
        init_dark_mode_for_process();

        let hinstance = GetModuleHandleW(null());
        if hinstance.is_null() {
            return Err(io::Error::last_os_error());
        }

        let cursor = LoadCursorW(null_mut(), IDC_ARROW);
        if cursor.is_null() {
            return Err(io::Error::last_os_error());
        }

        let class_name = to_wide(CLASS_NAME);
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance,
            hCursor: cursor,
            hbrBackground: null_mut(),
            lpszClassName: class_name.as_ptr(),
            ..zeroed()
        };

        let atom = RegisterClassExW(&wc);
        if atom == 0 {
            return Err(io::Error::last_os_error());
        }

        let title = to_wide(APP_TITLE);
        let hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            class_name.as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WIN_W,
            WIN_H,
            null_mut(),
            null_mut(),
            hinstance,
            null(),
        );
        if hwnd.is_null() {
            return Err(io::Error::last_os_error());
        }

        ShowWindow(hwnd, SW_SHOW);
        SetTimer(hwnd, ID_TIMER_CARET, 500, None);

        let mut msg: MSG = zeroed();
        loop {
            let code = GetMessageW(&mut msg, null_mut(), 0, 0);
            if code == -1 {
                return Err(io::Error::last_os_error());
            }
            if code == 0 {
                break;
            }
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
    }

    Ok(())
}


unsafe fn apply_main_window_region(hwnd: HWND) {
    apply_window_corner_preference(hwnd);
}

unsafe extern "system" fn wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_CREATE => match on_create(hwnd) {
            Ok(_) => 0,
            Err(_) => -1,
        },
        WM_PAINT => {
            paint(hwnd);
            0
        }
        WM_ERASEBKGND => 1,
        WM_SIZE => {
            apply_main_window_region(hwnd);
            layout_children(hwnd);
            InvalidateRect(hwnd, null(), 1);
            0
        }
        WM_TIMER => {
            if wparam as usize == ID_TIMER_PASTE {
                KillTimer(hwnd, ID_TIMER_PASTE);
                send_ctrl_v();
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    if state.paste_return_to_main {
                        SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
                        state.paste_return_to_main = false;
                    }
                }
                return 0;
            }
            if wparam as usize == ID_TIMER_SCROLL_FADE {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    if state.hover_scroll {
                        // 鼠标悬停时保持完全不透明
                        state.scroll_fade_alpha = 255;
                    } else {
                        // 每帧减少 25（约10帧渐隐）
                        state.scroll_fade_alpha = state.scroll_fade_alpha.saturating_sub(30);
                        if state.scroll_fade_alpha == 0 {
                            KillTimer(hwnd, ID_TIMER_SCROLL_FADE);
                            state.scroll_fade_timer = false;
                        }
                    }
                    InvalidateRect(hwnd, null(), 0);
                }
                return 0;
            }
            InvalidateRect(hwnd, null(), 0);
            0
        }
        WM_COMMAND => {
            handle_command(hwnd, wparam, lparam);
            0
        }
        WM_CLIPBOARDUPDATE => {
            capture_clipboard(hwnd);
            0
        }
        WM_MOUSEWHEEL => {
            handle_mouse_wheel(hwnd, wparam);
            0
        }
        WM_MOUSEMOVE => {
            handle_mouse_move(hwnd, lparam);
            0
        }
        WM_MOUSELEAVE => {
            handle_mouse_leave_main(hwnd);
            0
        }
        WM_LBUTTONDOWN => {
            handle_lbutton_down(hwnd, lparam);
            0
        }
        WM_LBUTTONUP => {
            handle_lbutton_up(hwnd, lparam);
            0
        }
        WM_LBUTTONDBLCLK => {
            handle_lbutton_dblclk(hwnd, lparam);
            0
        }
        WM_RBUTTONUP => {
            handle_rbutton_up(hwnd, lparam);
            0
        }
        WM_KEYDOWN => {
            handle_keydown(hwnd, wparam as u32);
            0
        }
        WM_HOTKEY => {
            if wparam as i32 == HOTKEY_ID {
                toggle_window_visibility_hotkey(hwnd);
                return 0;
            }
            0
        }
        WM_ACTIVATEAPP => {
            if wparam == 0 {
                clear_main_hover_state(hwnd);
            }
            0
        }
        WM_TRAYICON => {
            handle_tray(hwnd, lparam as u32);
            0
        }
        WM_MOVE => {
            remember_window_pos(hwnd);
            0
        }
        WM_NCHITTEST => handle_nchittest(hwnd, lparam),
        WM_DESTROY => {
            KillTimer(hwnd, ID_TIMER_CARET);
            KillTimer(hwnd, ID_TIMER_PASTE);
            RemoveClipboardFormatListener(hwnd);
            if let Some(state) = unsafe { get_state_mut(hwnd) } {
                unregister_hotkey_for(hwnd, state);
            }
            remove_tray_icon(hwnd);
            PostQuitMessage(0);
            0
        }
        WM_NCDESTROY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                (*ptr).icons.destroy();
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_create(hwnd: HWND) -> AppResult<()> {
    let hinstance = GetModuleHandleW(null());
    if hinstance.is_null() {
        return Err(io::Error::last_os_error());
    }

    let search_hwnd = CreateWindowExW(
        0,
        to_wide("EDIT").as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32),
        SEARCH_LEFT + 10,
        SEARCH_TOP + 3,
        SEARCH_W - 20,
        SEARCH_H - 6,
        hwnd,
        IDC_SEARCH as usize as _,
        hinstance,
        null(),
    );
    if search_hwnd.is_null() {
        return Err(io::Error::last_os_error());
    }
    let search_font: *mut core::ffi::c_void = CreateFontW(-14, 0, 0, 0, 400, 0, 0, 0, 1, 0, 0, 5, 0, to_wide("Segoe UI Variable Text").as_ptr()) as _;
    let font: *mut core::ffi::c_void = if search_font.is_null() { GetStockObject(DEFAULT_GUI_FONT) as _ } else { search_font };
    SendMessageW(search_hwnd, WM_SETFONT, font as WPARAM, 1 as LPARAM);
    SendMessageW(search_hwnd, EM_SETMARGINS, (EC_LEFTMARGIN | EC_RIGHTMARGIN) as WPARAM, 0);

    let icons = load_icons();
    let tray_icon = icons.app;
    if icons.app != 0 {
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as WPARAM, icons.app as LPARAM);
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as WPARAM, icons.app as LPARAM);
    }

    let state = Box::new(AppState::new(hwnd, search_hwnd, icons));
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
    if let Some(state) = unsafe { get_state_mut(hwnd) } {
        ensure_db();
        reload_state_from_db(state);
        register_hotkey_for(hwnd, state);
        position_main_window(hwnd, &state.settings, false);
    }

    AddClipboardFormatListener(hwnd);
    apply_main_window_region(hwnd);
    apply_dark_mode_to_window(hwnd);
    add_tray_icon(hwnd, tray_icon);
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
    Ok(())
}

unsafe fn layout_children(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let rc = state.search_rect();
    MoveWindow(
        state.search_hwnd,
        rc.left + 10,
        rc.top + 5,
        rc.right - rc.left - 20,
        rc.bottom - rc.top - 10,
        1,
    );
    ShowWindow(state.search_hwnd, if state.search_on { SW_SHOW } else { SW_HIDE });
}

unsafe fn execute_row_command(hwnd: HWND, state: &mut AppState, cmd: usize) {
    match cmd {
        IDM_ROW_PASTE => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                paste_selected(hwnd, state);
                InvalidateRect(hwnd, null(), 0);
            }
        }
        IDM_ROW_COPY => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if state.context_selection_count() > 1 {
                    copy_selected_rows_combined(state);
                } else {
                    apply_selected_to_clipboard(state);
                }
                state.clear_selection();
                InvalidateRect(hwnd, null(), 0);
            }
        }
        IDM_ROW_PIN => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                state.toggle_pin_rows();
                InvalidateRect(hwnd, null(), 1);
            }
        }
        IDM_ROW_DELETE => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                state.delete_selected_rows();
                InvalidateRect(hwnd, null(), 1);
            }
        }
        IDM_ROW_TO_PHRASE => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                let items = state.selected_items_owned();
                if items.is_empty() {
                    if let Some(item) = state.current_item() {
                        let _ = db_add_phrase_from_item(item);
                    }
                } else {
                    for item in &items {
                        let _ = db_add_phrase_from_item(item);
                    }
                }
                state.phrases = db_load_items(1);
                InvalidateRect(hwnd, null(), 1);
            }
        }
        IDM_ROW_STICKER => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item() {
                    if item.kind == ClipKind::Image {
                        show_image_sticker(item);
                    }
                }
            }
        }
        IDM_ROW_SAVE_IMAGE => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item() {
                    if item.kind == ClipKind::Image {
                        if let Some(path) = save_image_item(item) {
                            if let Some(parent) = path.parent().and_then(|p| p.to_str()) {
                                open_path_with_shell(parent);
                            }
                        }
                    }
                }
            }
        }
        IDM_ROW_OPEN_PATH => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item() {
                    if let Some(paths) = &item.file_paths {
                        for p in paths { open_path_with_shell(p); }
                    }
                }
            }
        }
        IDM_ROW_OPEN_FOLDER => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item() {
                    if let Some(paths) = &item.file_paths {
                        for p in paths { open_parent_folder(p); }
                    }
                }
            }
        }
        IDM_ROW_COPY_PATH => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                let items = state.selected_items_owned();
                let mut lines = Vec::new();
                if items.is_empty() {
                    if let Some(item) = state.current_item() {
                        if let Some(paths) = &item.file_paths { lines.extend(paths.clone()); }
                    }
                } else {
                    for item in &items {
                        if let Some(paths) = &item.file_paths { lines.extend(paths.clone()); }
                    }
                }
                if !lines.is_empty() {
                    if let Ok(mut cb) = Clipboard::new() { let _ = cb.set_text(lines.join("\n")); }
                }
            }
        }
        IDM_ROW_QUICK_SEARCH => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item() {
                    let text = match item.kind {
                        ClipKind::Text | ClipKind::Phrase => item.text.clone().unwrap_or_else(|| item.preview.clone()),
                        ClipKind::Files => item.file_paths.as_ref().map(|v| v.join(" ")).unwrap_or_else(|| item.preview.clone()),
                        ClipKind::Image => item.preview.clone(),
                    };
                    quick_search_open(&state.settings, &text);
                }
            }
        }
        IDM_ROW_EXPORT_FILE => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item() {
                    if let Some(path) = materialize_item_as_file(item) {
                        if let Some(parent) = path.parent().and_then(|p| p.to_str()) { open_path_with_shell(parent); }
                    }
                }
            }
        }
        IDM_ROW_MAIL_MERGE => {
            if !state.settings.super_mail_merge_enabled {
                return;
            }
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item() {
                    if let Some(path) = item.file_paths.as_ref().and_then(|v| v.first()) {
                        launch_mail_merge_window_with_excel(hwnd, Some(path));
                    } else {
                        launch_mail_merge_window(hwnd);
                    }
                }
            }
        }
        IDM_ROW_EDIT => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item() {
                    let item_id = item.id;
                    let title = format!("编辑 — {}", item.preview.chars().take(40).collect::<String>());
                    let saved = show_edit_item_dialog(hwnd, item_id, &title);
                    if saved {
                        reload_state_from_db(state);
                        state.refilter();
                        InvalidateRect(hwnd, null(), 1);
                    }
                }
            }
        }
        IDM_ROW_GROUP_REMOVE => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                let ids = state.selected_db_ids();
                if !ids.is_empty() {
                    let _ = db_assign_group(&ids, 0);
                    reload_state_from_db(state);
                    InvalidateRect(hwnd, null(), 1);
                }
            }
        }
        IDM_GROUP_FILTER_ALL => {
            state.current_group_filter = 0;
            let tab_index = state.tab_index;
            if tab_index < state.tab_group_filters.len() {
                state.tab_group_filters[tab_index] = 0;
            }
            state.scroll_y = 0;
            state.clear_selection();
            state.refilter();
            InvalidateRect(hwnd, null(), 1);
        }
        _ if (IDM_ROW_GROUP_BASE..IDM_ROW_GROUP_BASE + 2000).contains(&cmd) => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                let idx = cmd - IDM_ROW_GROUP_BASE;
                if let Some(group_id) = state.groups.get(idx).map(|g| g.id) {
                    let ids = state.selected_db_ids();
                    if !ids.is_empty() {
                        let _ = db_assign_group(&ids, group_id);
                        reload_state_from_db(state);
                        state.refilter();
                        InvalidateRect(hwnd, null(), 1);
                    }
                }
            }
        }
        _ if (IDM_GROUP_FILTER_BASE..IDM_GROUP_FILTER_BASE + 2000).contains(&cmd) => {
            let idx = cmd - IDM_GROUP_FILTER_BASE;
            if let Some(group_id) = state.groups.get(idx).map(|g| g.id) {
                state.current_group_filter = group_id;
                let tab_index = state.tab_index;
                if tab_index < state.tab_group_filters.len() {
                    state.tab_group_filters[tab_index] = group_id;
                }
                state.scroll_y = 0;
                state.clear_selection();
                state.refilter();
                InvalidateRect(hwnd, null(), 1);
            }
        }
        _ => {}
    }
}

unsafe fn handle_command(hwnd: HWND, wparam: WPARAM, _lparam: LPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let id = loword(wparam as u32) as usize;
    let code = hiword(wparam as u32) as u16;

    if id == IDC_SEARCH as usize && code == EN_CHANGE_CODE {
        state.search_text = get_window_text(state.search_hwnd);
        state.sel_idx = 0;
        state.scroll_y = 0;
        state.refilter();
        InvalidateRect(hwnd, null(), 1);
        return;
    }

    match id {
        IDM_TRAY_TOGGLE => {
            toggle_window_visibility(hwnd);
        }
        IDM_TRAY_EXIT => {
            DestroyWindow(hwnd);
        }
        IDM_ROW_PASTE | IDM_ROW_COPY | IDM_ROW_PIN | IDM_ROW_DELETE | IDM_ROW_TO_PHRASE | IDM_ROW_STICKER | IDM_ROW_SAVE_IMAGE | IDM_ROW_OPEN_PATH | IDM_ROW_OPEN_FOLDER | IDM_ROW_COPY_PATH | IDM_ROW_GROUP_REMOVE | IDM_ROW_EDIT | IDM_ROW_QUICK_SEARCH | IDM_ROW_EXPORT_FILE | IDM_ROW_MAIL_MERGE | IDM_GROUP_FILTER_ALL => {
            execute_row_command(hwnd, state, id);
        }
        _ if (IDM_ROW_GROUP_BASE..IDM_ROW_GROUP_BASE + 2000).contains(&id) || (IDM_GROUP_FILTER_BASE..IDM_GROUP_FILTER_BASE + 2000).contains(&id) => {
            execute_row_command(hwnd, state, id);
        }
        _ => {}
    }

    state.context_row = -1;
}


unsafe fn handle_mouse_wheel(hwnd: HWND, wparam: WPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let delta = ((wparam >> 16) & 0xffff) as u16 as i16 as i32;
    if delta > 0 {
        state.scroll_y -= SCROLL_STEP;
    } else {
        state.scroll_y += SCROLL_STEP;
    }
    state.clamp_scroll();
    // 显示滚动条并启动渐隐 timer
    state.scroll_fade_alpha = 255;
    if !state.scroll_fade_timer {
        state.scroll_fade_timer = true;
        SetTimer(hwnd, ID_TIMER_SCROLL_FADE, 50, None); // 50ms ≈ 20fps
    }
    InvalidateRect(hwnd, null(), 1);
}

unsafe fn ensure_mouse_leave_tracking(hwnd: HWND) {
    let mut tme = TRACKMOUSEEVENT {
        cb_size: core::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dw_flags: TME_LEAVE,
        hwnd_track: hwnd,
        dw_hover_time: 0,
    };
    TrackMouseEvent(&mut tme);
}

unsafe fn handle_mouse_move(hwnd: HWND, lparam: LPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    ensure_mouse_leave_tracking(hwnd);
    let state = &mut *ptr;
    let x = get_x_lparam(lparam);
    let y = get_y_lparam(lparam);

    if state.down_row >= 0 && (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0 {
        let dx = (x - state.down_x).abs();
        let dy = (y - state.down_y).abs();
        let drag_cx = GetSystemMetrics(SM_CXDRAG).max(4);
        let drag_cy = GetSystemMetrics(SM_CYDRAG).max(4);
        if dx >= drag_cx || dy >= drag_cy {
            let drag_row = state.down_row;
            state.down_row = -1;
            if begin_row_drag_export(hwnd, state, drag_row) {
                InvalidateRect(hwnd, null(), 0);
                return;
            }
        }
    }

    let old_btn = state.hover_btn;
    let old_tab = state.hover_tab;
    let old_scroll = state.hover_scroll;
    state.hover_btn = "";
    for key in ["search", "setting", "min", "close"] {
        if pt_in_rect(x, y, &state.title_button_rect(key)) {
            state.hover_btn = key;
            break;
        }
    }

    let (tab0, tab1) = state.segment_rects();
    state.hover_tab = if pt_in_rect(x, y, &tab0) {
        0
    } else if pt_in_rect(x, y, &tab1) {
        1
    } else {
        -1
    };

    // 滚动条 hover 检测
    let was_hover_scroll = state.hover_scroll;
    state.hover_scroll = state.scrollbar_track_rect()
        .map(|tr| {
            let hot = RECT { left: tr.left - 8, top: tr.top, right: tr.right + 2, bottom: tr.bottom };
            pt_in_rect(x, y, &hot)
        })
        .unwrap_or(false);
    // 悬停时立即显示滚动条（满透明）
    if state.hover_scroll && !was_hover_scroll {
        state.scroll_fade_alpha = 255;
        if !state.scroll_fade_timer {
            state.scroll_fade_timer = true;
            SetTimer(hwnd, ID_TIMER_SCROLL_FADE, 50, None);
        }
    }

    let old_hover = state.hover_idx;
    state.hover_idx = hit_test_row(state, x, y);

    if old_btn != state.hover_btn || old_hover != state.hover_idx || old_tab != state.hover_tab || old_scroll != state.hover_scroll {
        InvalidateRect(hwnd, null(), 0);
    }
}

unsafe fn handle_mouse_leave_main(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let mut dirty = false;
    if !state.hover_btn.is_empty() { state.hover_btn = ""; dirty = true; }
    if state.hover_tab != -1 { state.hover_tab = -1; dirty = true; }
    if state.hover_idx != -1 { state.hover_idx = -1; dirty = true; }
    if state.hover_scroll { state.hover_scroll = false; dirty = true; }
    // 保留当前选择集，避免右键菜单弹出时因为 WM_MOUSELEAVE 把多选清空。
    if dirty {
        InvalidateRect(hwnd, null(), 0);
    }
}

unsafe fn clear_main_hover_state(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let mut dirty = false;
    if !state.hover_btn.is_empty() { state.hover_btn = ""; dirty = true; }
    if state.hover_tab != -1 { state.hover_tab = -1; dirty = true; }
    if state.hover_idx != -1 { state.hover_idx = -1; dirty = true; }
    if state.down_row != -1 { state.down_row = -1; state.down_x = 0; state.down_y = 0; dirty = true; }
    if dirty {
        InvalidateRect(hwnd, null(), 0);
    }
}

unsafe fn handle_lbutton_down(hwnd: HWND, lparam: LPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let x = get_x_lparam(lparam);
    let y = get_y_lparam(lparam);

    if 0 <= y && y < TITLE_H {
        let mut blocked = false;
        for key in ["search", "setting", "min", "close"] {
            if pt_in_rect(x, y, &state.title_button_rect(key)) {
                blocked = true;
                break;
            }
        }
        if !blocked && (!state.search_on || !pt_in_rect(x, y, &state.search_rect())) {
            SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as WPARAM, 0);
            return;
        }
    }

    state.down_btn = "";
    for key in ["search", "setting", "min", "close"] {
        if pt_in_rect(x, y, &state.title_button_rect(key)) {
            state.down_btn = key;
            InvalidateRect(hwnd, null(), 0);
            return;
        }
    }

    let (tab0, tab1) = state.segment_rects();
    if pt_in_rect(x, y, &tab0) {
        state.tab_index = 0;
        state.current_group_filter = state.tab_group_filters[0];
        state.clear_selection();
        state.scroll_y = 0;
        state.refilter();
        InvalidateRect(hwnd, null(), 1);
        return;
    }
    if pt_in_rect(x, y, &tab1) {
        state.tab_index = 1;
        state.current_group_filter = state.tab_group_filters[1];
        state.clear_selection();
        state.scroll_y = 0;
        state.refilter();
        InvalidateRect(hwnd, null(), 1);
        return;
    }

    let idx = hit_test_row(state, x, y);
    state.down_row = -1;
    if idx >= 0 {
        state.sel_idx = idx;
        state.ensure_visible(idx);
        state.down_row = idx;
        state.down_x = x;
        state.down_y = y;
        InvalidateRect(hwnd, null(), 0);
    }
}

unsafe fn handle_lbutton_up(hwnd: HWND, lparam: LPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let x = get_x_lparam(lparam);
    let y = get_y_lparam(lparam);
    let key = state.down_btn;
    state.down_btn = "";
    if !key.is_empty() {
        if !pt_in_rect(x, y, &state.title_button_rect(key)) {
            InvalidateRect(hwnd, null(), 0);
            return;
        }

        match key {
            "search" => {
                state.search_on = !state.search_on;
                if !state.search_on {
                    state.search_text.clear();
                    SetWindowTextW(state.search_hwnd, to_wide("").as_ptr());
                    state.clear_selection();
                    state.refilter();
                }
                layout_children(hwnd);
                if state.search_on {
                    SetFocus(state.search_hwnd);
                }
            }
            "setting" => {
                open_settings_window(hwnd);
            }
            "min" => {
                ShowWindow(hwnd, SW_HIDE);
            }
            _ => {
                if state.settings.close_without_exit {
                    ShowWindow(hwnd, SW_HIDE);
                } else {
                    DestroyWindow(hwnd);
                }
            }
        }
        state.hover_btn = "";
        InvalidateRect(hwnd, null(), 0);
        return;
    }

    let down_row = state.down_row;
    state.down_row = -1;
    state.down_x = 0;
    state.down_y = 0;
    if down_row < 0 {
        return;
    }
    let idx = hit_test_row(state, x, y);
    if idx != down_row {
        InvalidateRect(hwnd, null(), 0);
        return;
    }
    state.sel_idx = idx;
    if let Some(del_rc) = state.quick_delete_rect(idx) {
        if pt_in_rect(x, y, &del_rc) {
            state.delete_selected();
            InvalidateRect(hwnd, null(), 1);
            return;
        }
    }
    let ctrl = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;
    let shift = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
    if ctrl || shift {
        if ctrl {
            if !state.selected_rows.insert(idx) {
                state.selected_rows.remove(&idx);
            }
            state.selection_anchor = idx;
        } else if shift {
            if state.selection_anchor < 0 {
                state.selection_anchor = idx;
            }
            state.selected_rows.clear();
            let a = min(state.selection_anchor, idx);
            let b = max(state.selection_anchor, idx);
            for i in a..=b {
                state.selected_rows.insert(i);
            }
        }
        state.sel_idx = idx;
        InvalidateRect(hwnd, null(), 0);
        return;
    }

    // 单击逻辑：
    // - 开启“单击后隐藏主窗口”：复制到剪贴板后切到目标窗口并粘贴，同时隐藏主窗口
    // - 关闭时：仍执行粘贴，但主窗口保持显示，仅在粘贴完成后重新置于最上层
    if !apply_selected_to_clipboard(state) {
        return;
    }
    state.clear_selection();
    clear_main_hover_state(hwnd);
    paste_after_clipboard_ready(hwnd, state, state.settings.click_hide);
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn handle_lbutton_dblclk(hwnd: HWND, lparam: LPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let x = get_x_lparam(lparam);
    let y = get_y_lparam(lparam);
    let idx = hit_test_row(state, x, y);
    if idx >= 0 {
        state.sel_idx = idx;
        paste_selected(hwnd, state);
        state.sel_idx = -1;
        state.hover_idx = -1;
        InvalidateRect(hwnd, null(), 0);
    }
}


unsafe fn handle_rbutton_up(hwnd: HWND, lparam: LPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let x = get_x_lparam(lparam);
    let y = get_y_lparam(lparam);
    let (tab0, tab1) = state.segment_rects();
    if state.settings.grouping_enabled && (pt_in_rect(x, y, &tab0) || pt_in_rect(x, y, &tab1)) {
        let target_tab = if pt_in_rect(x, y, &tab1) { 1usize } else { 0usize };
        state.tab_index = target_tab;
        state.current_group_filter = state.tab_group_filters[target_tab];
        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        let cmd = show_group_filter_menu(hwnd, pt.x, pt.y, target_tab, state);
        if cmd == IDM_GROUP_FILTER_ALL {
            state.tab_group_filters[target_tab] = 0;
            state.current_group_filter = 0;
            state.scroll_y = 0;
            state.clear_selection();
            state.refilter();
        } else if (IDM_GROUP_FILTER_BASE..IDM_GROUP_FILTER_BASE + 2000).contains(&cmd) {
            let idx = cmd - IDM_GROUP_FILTER_BASE;
            if let Some(group_id) = state.groups.get(idx).map(|g| g.id) {
                state.tab_group_filters[target_tab] = group_id;
                state.current_group_filter = group_id;
                state.scroll_y = 0;
                state.clear_selection();
                state.refilter();
            }
        }
        InvalidateRect(hwnd, null(), 1);
        return;
    }

    let idx = hit_test_row(state, x, y);
    if idx < 0 {
        return;
    }

    let ctrl = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;
    let shift = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;

    if shift && state.selection_anchor >= 0 {
        state.selected_rows.clear();
        let a = min(state.selection_anchor, idx);
        let b = max(state.selection_anchor, idx);
        for i in a..=b {
            state.selected_rows.insert(i);
        }
        state.sel_idx = idx;
    } else if ctrl {
        if !state.selected_rows.insert(idx) {
            state.selected_rows.remove(&idx);
        }
        state.sel_idx = idx;
        if state.selection_anchor < 0 {
            state.selection_anchor = idx;
        }
    } else {
        let already_multi_selected = state.selected_rows.len() > 1 && state.selected_rows.contains(&idx);
        if !already_multi_selected {
            state.selected_rows.clear();
            state.sel_idx = idx;
            state.selection_anchor = idx;
        } else {
            state.sel_idx = idx;
        }
    }

    state.context_row = idx;
    state.ensure_visible(idx);
    let current_item = state.current_item().cloned();
    let current_kind = current_item.as_ref().map(|it| it.kind).unwrap_or(ClipKind::Text);
    let current_is_dir = current_item.as_ref().map(|it| is_directory_item(it)).unwrap_or(false);
    let current_is_excel = current_item
        .as_ref()
        .and_then(|it| it.file_paths.as_ref())
        .and_then(|paths| paths.first())
        .map(|path| {
            let lower = path.to_ascii_lowercase();
            lower.ends_with(".xls") || lower.ends_with(".xlsx") || lower.ends_with(".xlsm") || lower.ends_with(".csv")
        })
        .unwrap_or(false);
    let cmd = show_row_menu(
        hwnd,
        x,
        y,
        state,
        state.context_selection_count(),
        state.context_selection_has_unpinned(),
        current_kind,
        current_is_dir,
        current_is_excel,
    );
    if cmd != 0 {
        execute_row_command(hwnd, state, cmd);
    }
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn handle_keydown(hwnd: HWND, vk: u32) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let ctrl = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;
    let shift = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
    match vk {
        x if x == VK_UP as u32 => {
            if state.filtered_indices.is_empty() { return; }
            let new_idx = if state.sel_idx <= 0 { 0 } else { state.sel_idx - 1 };
            if shift {
                // Shift+Up: 扩展选择
                if state.selection_anchor < 0 { state.selection_anchor = state.sel_idx; }
                state.sel_idx = new_idx;
                state.selected_rows.clear();
                let a = min(state.selection_anchor, state.sel_idx);
                let b = max(state.selection_anchor, state.sel_idx);
                for i in a..=b { state.selected_rows.insert(i); }
            } else {
                state.sel_idx = new_idx;
                state.selected_rows.clear();
                state.selection_anchor = -1;
            }
            state.ensure_visible(state.sel_idx);
            InvalidateRect(hwnd, null(), 0);
        }
        x if x == VK_DOWN as u32 => {
            if state.filtered_indices.is_empty() { return; }
            let new_idx = if state.sel_idx < 0 { 0 }
                else { min(state.filtered_indices.len() as i32 - 1, state.sel_idx + 1) };
            if shift {
                if state.selection_anchor < 0 { state.selection_anchor = state.sel_idx; }
                state.sel_idx = new_idx;
                state.selected_rows.clear();
                let a = min(state.selection_anchor, state.sel_idx);
                let b = max(state.selection_anchor, state.sel_idx);
                for i in a..=b { state.selected_rows.insert(i); }
            } else {
                state.sel_idx = new_idx;
                state.selected_rows.clear();
                state.selection_anchor = -1;
            }
            state.ensure_visible(state.sel_idx);
            InvalidateRect(hwnd, null(), 0);
        }
        x if x == VK_RETURN as u32 => {
            // Enter: 如果多选则合并粘贴，否则粘贴当前项
            if state.context_selection_count() > 1 {
                copy_selected_rows_combined(state);
                state.clear_selection();
                paste_after_clipboard_ready(hwnd, state, state.settings.click_hide);
            } else {
                paste_selected(hwnd, state);
            }
            InvalidateRect(hwnd, null(), 0);
        }
        // Ctrl+A: 全选
        0x41 if ctrl => {
            state.selected_rows.clear();
            for i in 0..state.filtered_indices.len() as i32 {
                state.selected_rows.insert(i);
            }
            state.selection_anchor = 0;
            InvalidateRect(hwnd, null(), 0);
        }
        // Ctrl+C: 复制（多选合并，单选复制）
        0x43 if ctrl => {
            if state.context_selection_count() > 1 {
                copy_selected_rows_combined(state);
            } else {
                apply_selected_to_clipboard(state);
            }
            state.clear_selection();
            InvalidateRect(hwnd, null(), 0);
        }
        x if x == VK_DELETE as u32 => {
            state.delete_selected_rows();
            InvalidateRect(hwnd, null(), 1);
        }
        x if x == VK_ESCAPE as u32 => {
            if !state.selected_rows.is_empty() {
                // 先清除多选
                state.clear_selection();
                InvalidateRect(hwnd, null(), 0);
            } else if state.search_on {
                state.search_on = false;
                state.search_text.clear();
                SetWindowTextW(state.search_hwnd, to_wide("").as_ptr());
                state.refilter();
                layout_children(hwnd);
                InvalidateRect(hwnd, null(), 1);
            } else {
                ShowWindow(hwnd, SW_HIDE);
            }
        }
        // Ctrl+P: 固定/取消固定
        0x50 if ctrl => {
            state.toggle_pin_rows();
            InvalidateRect(hwnd, null(), 1);
        }
        // Ctrl+F: 搜索
        0x46 if ctrl => {
            state.search_on = true;
            layout_children(hwnd);
            SetFocus(state.search_hwnd);
            InvalidateRect(hwnd, null(), 1);
        }
        _ => {}
    }
}

unsafe fn handle_nchittest(hwnd: HWND, lparam: LPARAM) -> LRESULT {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return DefWindowProcW(hwnd, WM_NCHITTEST, 0, lparam);
    }
    let state = &mut *ptr;

    let mut pt = POINT {
        x: get_x_lparam(lparam),
        y: get_y_lparam(lparam),
    };
    ScreenToClient(hwnd, &mut pt);

    if pt.y >= 0 && pt.y < TITLE_H {
        if pt_in_rect(pt.x, pt.y, &state.search_rect()) {
            return HTCLIENT as LRESULT;
        }
        for key in ["search", "setting", "min", "close"] {
            if pt_in_rect(pt.x, pt.y, &state.title_button_rect(key)) {
                return HTCLIENT as LRESULT;
            }
        }
        return HTCAPTION as LRESULT;
    }

    HTCLIENT as LRESULT
}

unsafe fn capture_clipboard(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    if let Some(until) = state.ignore_clipboard_until {
        if Instant::now() < until {
            return;
        }
        state.ignore_clipboard_until = None;
    }
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return,
    };

    if let Ok(text) = clipboard.get_text() {
        let normalized = text.trim().replace("\r\n", "\n");
        if !normalized.is_empty() {
            let preview = build_preview(&normalized);
            let sig = format!("txt:{}", hash_bytes(normalized.as_bytes()));
            state.add_clip_item(
                ClipItem {
                    id: 0,
                    kind: ClipKind::Text,
                    preview,
                    text: Some(normalized),
                    file_paths: None,
                    image_bytes: None,
                    image_path: None,
                    image_width: 0,
                    image_height: 0,
                    pinned: false,
                    group_id: 0,
                    created_at: String::new(),
                },
                sig,
            );
            InvalidateRect(hwnd, null(), 1);
            return;
        }
    }

    if let Ok(img) = clipboard.get_image() {
        let bytes = img.bytes.into_owned();
        let image_path = write_image_bytes_to_output_path(&bytes, img.width as u32, img.height as u32);
        let image_bytes = if image_path.is_none() { Some(bytes.clone()) } else { None };
        let sig = format!(
            "img:{}:{}:{}",
            img.width,
            img.height,
            hash_bytes(bytes.as_slice())
        );
        // 图片预览：用当前本地时间作为标识（HH:MM:SS），便于用户识别截图时间
        let preview = format_local_time_for_image_preview();
        state.add_clip_item(
            ClipItem {
                id: 0,
                kind: ClipKind::Image,
                preview,
                text: None,
                file_paths: None,
                image_bytes,
                image_path: image_path.map(|p| p.to_string_lossy().to_string()),
                image_width: img.width,
                image_height: img.height,
                pinned: false,
                group_id: 0,
                created_at: String::new(),
            },
            sig,
        );
        trim_process_working_set();
        InvalidateRect(hwnd, null(), 1);
        return;
    }

    if let Some(paths) = clipboard_file_paths() {
        let preview = build_files_preview(&paths);
        let sig = file_paths_signature(&paths);
        state.add_clip_item(
            ClipItem {
                id: 0,
                kind: ClipKind::Files,
                preview,
                text: Some(paths.join("\n")),
                file_paths: Some(paths),
                image_bytes: None,
                image_path: None,
                image_width: 0,
                image_height: 0,
                pinned: false,
                group_id: 0,
                created_at: String::new(),
            },
            sig,
        );
        InvalidateRect(hwnd, null(), 1);
    }
}

unsafe fn copy_selected_rows_combined(state: &mut AppState) -> bool {
    let items = state.selected_items_owned();
    if items.is_empty() {
        return apply_selected_to_clipboard(state);
    }
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let mut parts: Vec<String> = Vec::new();
    for item in &items {
        match item.kind {
            ClipKind::Text | ClipKind::Phrase => {
                if let Some(text) = &item.text {
                    let t = text.trim();
                    if !t.is_empty() {
                        parts.push(t.to_string());
                    }
                }
            }
            ClipKind::Image => {
                parts.push(item.preview.clone());
            }
            ClipKind::Files => {
                if let Some(paths) = &item.file_paths {
                    for p in paths {
                        if !p.trim().is_empty() {
                            parts.push(p.trim().to_string());
                        }
                    }
                }
            }
        }
    }
    let merged = parts.join("\n");
    if merged.trim().is_empty() {
        return false;
    }
    let ok = clipboard.set_text(merged).is_ok();
    if ok {
        state.ignore_clipboard_until = Some(Instant::now() + std::time::Duration::from_millis(450));
    }
    ok
}

unsafe fn apply_selected_to_clipboard(state: &mut AppState) -> bool {
    let Some(item_ref) = state.current_item() else {
        return false;
    };
    let item_id = item_ref.id;
    let item_kind = item_ref.kind;

    // 对图片类型，image_bytes 在列表中为 None，需按需从 DB 加载完整数据
    let full_item;
    let item: &ClipItem = if item_kind == ClipKind::Image && item_ref.image_bytes.is_none() {
        match db_load_item_full(item_id) {
            Some(fi) => { full_item = fi; &full_item }
            None => return false,
        }
    } else {
        // text/phrase/files 的 text 字段已在初始加载中包含
        item_ref
    };

    let ok = match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let mut clipboard = match Clipboard::new() {
                Ok(c) => c,
                Err(_) => return false,
            };
            if let Some(text) = &item.text {
                clipboard.set_text(text.clone()).is_ok()
            } else {
                false
            }
        }
        ClipKind::Image => {
            let mut clipboard = match Clipboard::new() {
                Ok(c) => c,
                Err(_) => return false,
            };
            if let Some((bytes, width, height)) = ensure_item_image_bytes(item) {
                clipboard
                    .set_image(ImageData {
                        width,
                        height,
                        bytes: Cow::Owned(bytes),
                    })
                    .is_ok()
            } else {
                false
            }
        }
        ClipKind::Files => {
            if let Some(paths) = &item.file_paths {
                set_file_paths_to_clipboard(paths)
            } else if let Some(text) = &item.text {
                let mut clipboard = match Clipboard::new() {
                    Ok(c) => c,
                    Err(_) => return false,
                };
                clipboard.set_text(text.clone()).is_ok()
            } else {
                false
            }
        }
    };
    if ok {
        state.ignore_clipboard_until = Some(Instant::now() + std::time::Duration::from_millis(450));
    }
    ok
}

unsafe fn paste_selected(hwnd: HWND, state: &mut AppState) {
    if !apply_selected_to_clipboard(state) {
        return;
    }
    state.clear_selection();
    clear_main_hover_state(hwnd);
    paste_after_clipboard_ready(hwnd, state, state.settings.click_hide);
}

unsafe fn create_shell_data_object(paths: &[PathBuf]) -> Option<*mut core::ffi::c_void> {
    if paths.is_empty() {
        return None;
    }
    let parent = paths.first()?.parent()?.to_path_buf();
    if paths.iter().any(|p| p.parent() != Some(parent.as_path())) {
        return None;
    }

    let parent_wide = to_wide(parent.to_string_lossy().as_ref());
    let parent_pidl = ILCreateFromPathW(parent_wide.as_ptr());
    if parent_pidl.is_null() {
        return None;
    }

    let mut child_pidls: Vec<*mut windows_sys::Win32::UI::Shell::Common::ITEMIDLIST> = Vec::new();
    for path in paths {
        let wide = to_wide(path.to_string_lossy().as_ref());
        let abs_pidl = ILCreateFromPathW(wide.as_ptr());
        if abs_pidl.is_null() {
            continue;
        }
        let child = ILClone(ILFindLastID(abs_pidl));
        ILFree(abs_pidl);
        if !child.is_null() {
            child_pidls.push(child);
        }
    }

    if child_pidls.is_empty() {
        ILFree(parent_pidl);
        return None;
    }

    let mut data_obj: *mut core::ffi::c_void = null_mut();
    let hr = SHCreateDataObject(
        parent_pidl,
        child_pidls.len() as u32,
        child_pidls.as_ptr() as *const *const windows_sys::Win32::UI::Shell::Common::ITEMIDLIST,
        null_mut(),
        &IID_IDATAOBJECT_RAW,
        &mut data_obj,
    );
    for child in child_pidls {
        ILFree(child);
    }
    ILFree(parent_pidl);
    if hr >= 0 && !data_obj.is_null() {
        Some(data_obj)
    } else {
        None
    }
}

unsafe fn begin_file_drag(_hwnd: HWND, paths: &[PathBuf]) -> bool {
    let Some(data_obj) = create_shell_data_object(paths) else {
        return false;
    };
    let drop_source = create_drop_source();
    let init_hr = OleInitialize(null());
    if init_hr < 0 && init_hr != RPC_E_CHANGED_MODE_HR {
        release_raw_com(data_obj);
        release_raw_com(drop_source);
        return false;
    }

    let mut effect: DROPEFFECT = 0;
    let drag_hr = DoDragDrop(data_obj, drop_source, DROPEFFECT_COPY, &mut effect);
    release_raw_com(data_obj);
    release_raw_com(drop_source);
    if init_hr >= 0 {
        OleUninitialize();
    }
    let _ = effect;
    drag_hr >= 0
}


fn export_dir() -> PathBuf {
    let dir = data_dir().join("exports");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn sanitize_export_name(name: &str, fallback: &str) -> String {
    let mut s = name.chars().take(40).map(|ch| {
        if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') { '_' } else { ch }
    }).collect::<String>().trim().to_string();
    if s.is_empty() { s = fallback.to_string(); }
    s
}

fn materialize_item_as_file(item: &ClipItem) -> Option<PathBuf> {
    let base = export_dir();
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_millis();
    let suffix = if item.id > 0 { item.id.to_string() } else { ts.to_string() };
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let name = sanitize_export_name(&item.preview, "text");
            let path = base.join(format!("{}_{}_{}.txt", name, ts, suffix));
            let text = item.text.as_ref().map(|s| s.as_str()).unwrap_or(&item.preview);
            fs::write(&path, text).ok()?;
            Some(path)
        }
        ClipKind::Image => {
            let name = sanitize_export_name(&item.preview, "image");
            let path = base.join(format!("{}_{}_{}.png", name, ts, suffix));
            if let Some(existing) = save_image_item(item) {
                if existing != path {
                    fs::copy(existing, &path).ok()?;
                }
            } else {
                return None;
            }
            Some(path)
        }
        ClipKind::Files => item.file_paths.as_ref().and_then(|v| v.first()).map(PathBuf::from),
    }
}

fn drag_export_paths_for_item(item: &ClipItem) -> Vec<PathBuf> {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase | ClipKind::Image => materialize_item_as_file(item).into_iter().collect(),
        ClipKind::Files => Vec::new(),
    }
}

unsafe fn begin_row_drag_export(hwnd: HWND, state: &mut AppState, visible_idx: i32) -> bool {
    if visible_idx < 0 {
        return false;
    }
    let Some(src_idx) = state.filtered_indices.get(visible_idx as usize).copied() else {
        return false;
    };
    let Some(item) = state.active_items().get(src_idx).cloned() else {
        return false;
    };
    let paths = drag_export_paths_for_item(&item);
    if paths.is_empty() {
        return false;
    }
    begin_file_drag(hwnd, &paths)
}

unsafe fn show_row_menu(
    hwnd: HWND,
    x: i32,
    y: i32,
    state: &AppState,
    selected_count: usize,
    has_unpinned: bool,
    current_kind: ClipKind,
    current_is_dir: bool,
    current_is_excel: bool,
) -> usize {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return 0;
    }
    apply_theme_to_menu(menu as _);
    let group_menu = if state.settings.grouping_enabled { CreatePopupMenu() } else { null_mut() };
    if !group_menu.is_null() {
        apply_theme_to_menu(group_menu as _);
        if state.groups.is_empty() {
            AppendMenuW(group_menu, MF_GRAYED | MF_STRING, 0xFFFFusize, to_wide("（暂无分组）").as_ptr());
        } else {
            for (idx, g) in state.groups.iter().enumerate() {
                AppendMenuW(group_menu, MF_STRING, IDM_ROW_GROUP_BASE + idx, to_wide(&g.name).as_ptr());
            }
        }
    }
    if selected_count > 1 {
        AppendMenuW(menu, MF_STRING, IDM_ROW_COPY, to_wide("合并复制").as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let pin_text = if has_unpinned { "置顶所选" } else { "取消置顶" };
        AppendMenuW(menu, MF_STRING, IDM_ROW_PIN, to_wide(pin_text).as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_ROW_TO_PHRASE, to_wide("添加到短语").as_ptr());
        if !group_menu.is_null() {
            AppendMenuW(menu, MF_POPUP, group_menu as usize, to_wide("添加到分组").as_ptr());
        }
        AppendMenuW(menu, MF_STRING, IDM_ROW_GROUP_REMOVE, to_wide("移出分组").as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_ROW_DELETE, to_wide("删除所选").as_ptr());
    } else {
        match current_kind {
            ClipKind::Image => {
                AppendMenuW(menu, MF_STRING, IDM_ROW_STICKER, to_wide("贴图").as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_SAVE_IMAGE, to_wide("另存为 PNG").as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_EXPORT_FILE, to_wide("导出为文件").as_ptr());
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                let pin_text = if has_unpinned { "置顶" } else { "取消置顶" };
                AppendMenuW(menu, MF_STRING, IDM_ROW_PIN, to_wide(pin_text).as_ptr());
                if !group_menu.is_null() {
                    AppendMenuW(menu, MF_POPUP, group_menu as usize, to_wide("添加到分组").as_ptr());
                }
                AppendMenuW(menu, MF_STRING, IDM_ROW_GROUP_REMOVE, to_wide("移出分组").as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_DELETE, to_wide("删除").as_ptr());
            }
            ClipKind::Files => {
                let open_text = if current_is_dir { "打开文件夹" } else { "打开文件" };
                AppendMenuW(menu, MF_STRING, IDM_ROW_OPEN_PATH, to_wide(open_text).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_OPEN_FOLDER, to_wide("打开所在文件夹").as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_COPY_PATH, to_wide("复制路径").as_ptr());
                if current_is_excel && state.settings.super_mail_merge_enabled {
                    AppendMenuW(menu, MF_STRING, IDM_ROW_MAIL_MERGE, to_wide("超级邮件合并").as_ptr());
                }
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                let pin_text = if has_unpinned { "置顶" } else { "取消置顶" };
                AppendMenuW(menu, MF_STRING, IDM_ROW_PIN, to_wide(pin_text).as_ptr());
                if !group_menu.is_null() {
                    AppendMenuW(menu, MF_POPUP, group_menu as usize, to_wide("添加到分组").as_ptr());
                }
                AppendMenuW(menu, MF_STRING, IDM_ROW_GROUP_REMOVE, to_wide("移出分组").as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_DELETE, to_wide("删除").as_ptr());
            }
            _ => {
                let pin_text = if has_unpinned { "置顶" } else { "取消置顶" };
                AppendMenuW(menu, MF_STRING, IDM_ROW_EDIT, to_wide("编辑").as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_QUICK_SEARCH, to_wide("快速搜索").as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_EXPORT_FILE, to_wide("导出为文件").as_ptr());
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                AppendMenuW(menu, MF_STRING, IDM_ROW_PIN, to_wide(pin_text).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_TO_PHRASE, to_wide("添加到短语").as_ptr());
                if !group_menu.is_null() {
                    AppendMenuW(menu, MF_POPUP, group_menu as usize, to_wide("添加到分组").as_ptr());
                }
                AppendMenuW(menu, MF_STRING, IDM_ROW_GROUP_REMOVE, to_wide("移出分组").as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_DELETE, to_wide("删除").as_ptr());
            }
        }
    }

    let mut rc: RECT = zeroed();
    GetWindowRect(hwnd, &mut rc);
    let pt = POINT { x: rc.left + x, y: rc.top + y };
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_TOPALIGN | TPM_LEFTALIGN | TPM_RETURNCMD,
        pt.x,
        pt.y,
        0,
        hwnd,
        null(),
    ) as usize;
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
    cmd
}

unsafe fn show_group_filter_menu(hwnd: HWND, x: i32, y: i32, tab_index: usize, state: &AppState) -> usize {
    if !state.settings.grouping_enabled {
        return 0;
    }
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return 0;
    }
    apply_theme_to_menu(menu as _);
    let cur_gid = if tab_index < state.tab_group_filters.len() {
        state.tab_group_filters[tab_index]
    } else {
        state.current_group_filter
    };
    let all_flags = if cur_gid == 0 { MF_STRING | MF_CHECKED } else { MF_STRING };
    AppendMenuW(menu, all_flags, IDM_GROUP_FILTER_ALL, to_wide("全部").as_ptr());
    if !state.groups.is_empty() {
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        for (idx, g) in state.groups.iter().enumerate() {
            let flags = if cur_gid == g.id { MF_STRING | MF_CHECKED } else { MF_STRING };
            AppendMenuW(menu, flags, IDM_GROUP_FILTER_BASE + idx, to_wide(&g.name).as_ptr());
        }
    }
    SetForegroundWindow(hwnd);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN | TPM_RETURNCMD,
        x,
        y,
        0,
        hwnd,
        null(),
    ) as usize;
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
    cmd
}


unsafe fn send_ctrl_v() {
    keybd_event(VK_CONTROL as u8, 0, 0, 0);
    keybd_event(VK_V as u8, 0, 0, 0);
    keybd_event(VK_V as u8, 0, KEYEVENTF_KEYUP, 0);
    keybd_event(VK_CONTROL as u8, 0, KEYEVENTF_KEYUP, 0);
}

unsafe fn send_alt_tap() {
    let inputs = [
        INPUT { r#type: INPUT_KEYBOARD, anonymous: INPUT_UNION { ki: KEYBDINPUT { w_vk: VK_MENU as u16, w_scan: 0, dw_flags: 0, time: 0, dw_extra_info: 0 } } },
        INPUT { r#type: INPUT_KEYBOARD, anonymous: INPUT_UNION { ki: KEYBDINPUT { w_vk: VK_MENU as u16, w_scan: 0, dw_flags: KEYEVENTF_KEYUP, time: 0, dw_extra_info: 0 } } },
    ];
    let _ = SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32);
}

unsafe fn force_foreground_window(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    if SetForegroundWindow(hwnd) != 0 {
        return true;
    }
    send_alt_tap();
    SetForegroundWindow(hwnd) != 0
}

unsafe fn paste_after_clipboard_ready(hwnd: HWND, state: &mut AppState, hide_main: bool) {
    state.paste_return_to_main = false;
    let target = find_next_paste_target(hwnd);
    if !target.is_null() {
        if hide_main {
            ShowWindow(hwnd, SW_HIDE);
        }
        let _ = force_foreground_window(target);
        KillTimer(hwnd, ID_TIMER_PASTE);
        SetTimer(hwnd, ID_TIMER_PASTE, 150, None);
    } else {
        SetForegroundWindow(hwnd);
        if state.search_on {
            SetFocus(state.search_hwnd);
        }
    }
}

unsafe fn is_window_enabled_compat(hwnd: HWND) -> bool {
    (GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_DISABLED) == 0
}

unsafe extern "system" fn enum_visible_windows(hwnd: HWND, lparam: LPARAM) -> i32 {
    let list = &mut *(lparam as *mut Vec<HWND>);
    if hwnd.is_null() || IsWindowVisible(hwnd) == 0 || !is_window_enabled_compat(hwnd) || IsIconic(hwnd) != 0 {
        return 1;
    }
    list.push(hwnd);
    1
}

unsafe fn is_viable_paste_window(hwnd: HWND, app_hwnd: HWND) -> bool {
    if hwnd.is_null() || hwnd == app_hwnd {
        return false;
    }
    if IsWindowVisible(hwnd) == 0 || !is_window_enabled_compat(hwnd) || IsIconic(hwnd) != 0 {
        return false;
    }
    if GetAncestor(hwnd, GA_ROOT) != hwnd {
        return false;
    }
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    (ex_style & WS_EX_TOOLWINDOW) == 0
}

unsafe fn find_next_paste_target(app_hwnd: HWND) -> HWND {
    let mut wins: Vec<HWND> = Vec::new();
    EnumWindows(Some(enum_visible_windows), &mut wins as *mut _ as LPARAM);

    let fg = GetForegroundWindow();
    let start = wins
        .iter()
        .position(|&h| h == fg)
        .map(|idx| idx + 1)
        .unwrap_or(0);

    for &h in wins.iter().skip(start) {
        if !is_viable_paste_window(h, app_hwnd) {
            continue;
        }
        let title = get_window_text(h);
        if matches!(
            title.trim(),
            "" | "开始" | "dummyLayeredWnd" | "Float" | "屏幕录制" | "RecBackgroundForm"
        ) {
            continue;
        }
        return h;
    }
    null_mut()
}

unsafe fn paint(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let th = state.theme;
    let dark = is_dark_mode();

    let mut ps: PAINTSTRUCT = zeroed();
    let hdc = BeginPaint(hwnd, &mut ps);
    if hdc.is_null() {
        return;
    }

    let mut rc_client: RECT = zeroed();
    GetClientRect(hwnd, &mut rc_client);
    let w = rc_client.right - rc_client.left;
    let h = rc_client.bottom - rc_client.top;

    let memdc = CreateCompatibleDC(hdc);
    let membmp = CreateCompatibleBitmap(hdc, w, h);
    let oldbmp = SelectObject(memdc, membmp as _);

    let bg_br = CreateSolidBrush(th.bg);
    FillRect(memdc, &rc_client, bg_br);
    DeleteObject(bg_br as _);

    if state.icons.app != 0 {
        draw_icon_tinted(memdc as _, 10, 9, state.icons.app, 18, 18, dark);
    }
    for key in ["search", "setting", "min", "close"] {
        let rc = state.title_button_rect(key);
        let hover = state.hover_btn == key;
        let down = state.down_btn == key;
        if hover || down {
            if key == "close" {
                let br = CreateSolidBrush(th.close_hover);
                FillRect(memdc, &rc, br);
                DeleteObject(br as _);
            } else {
                let col = if down { th.button_pressed } else { th.button_hover };
                draw_round_rect(memdc as _, &inflate_rect(&rc, -2, -2), col, 0, 6);
            }
        }
        let icon = match key {
            "search" => state.icons.search,
            "setting" => state.icons.setting,
            "min" => state.icons.min,
            _ => state.icons.close,
        };
        if icon != 0 {
            let iw = 16;
            let ih = 16;
            let ix = rc.left + ((rc.right - rc.left - iw) / 2);
            let iy = rc.top + ((rc.bottom - rc.top - ih) / 2);
            draw_icon_tinted(memdc as _, ix, iy, icon, iw, ih, dark);
        }
    }

    if state.search_on {
        let search_bg = state.search_rect();
        draw_round_rect(memdc as _, &search_bg, th.control_bg, th.control_stroke, 10);
    }

    let seg_rc = RECT {
        left: SEG_X,
        top: SEG_Y,
        right: SEG_X + SEG_W,
        bottom: SEG_Y + SEG_H,
    };
    let (tab0, tab1) = state.segment_rects();
    draw_main_segment_bar(memdc as _, &seg_rc, &tab0, &tab1, state.tab_index as i32, state.hover_tab, th);


    let list_rc = RECT {
        left: LIST_X,
        top: LIST_Y,
        right: LIST_X + LIST_W,
        bottom: LIST_Y + LIST_H,
    };
    draw_round_rect(memdc as _, &list_rc, th.surface, th.stroke, 10);

    let saved_clip = SaveDC(memdc);
    IntersectClipRect(memdc, LIST_X + 1, LIST_Y + 1, LIST_X + LIST_W - 1, LIST_Y + LIST_H - 1);
    if state.filtered_indices.is_empty() {
        let tr = RECT {
            left: LIST_X + 20,
            top: LIST_Y + 20,
            right: LIST_X + LIST_W - 20,
            bottom: LIST_Y + LIST_H - 20,
        };
        let msg = if state.settings.grouping_enabled && state.current_group_filter != 0 {
            "当前分组暂无记录"
        } else if state.tab_index == 0 {
            "暂无剪贴板记录"
        } else {
            "暂无短语"
        };
        draw_text(memdc as _, msg, &tr, th.text_muted, 12, false, true);
    } else {
        let view_top = LIST_Y + LIST_PAD;
        let view_bottom = LIST_Y + LIST_H - LIST_PAD;
        let start_idx = max(0, state.scroll_y / ROW_H);
        let end_idx = min(
            state.filtered_indices.len() as i32,
            (state.scroll_y + state.list_view_height()) / ROW_H + 2,
        );

        for i in start_idx..end_idx {
            let Some(mut row_rc) = state.row_rect(i) else {
                continue;
            };
            if row_rc.bottom <= view_top || row_rc.top >= view_bottom {
                continue;
            }
            let src_idx = state.filtered_indices[i as usize];
            let item = state.active_items()[src_idx].clone();

            if state.row_is_selected(i) {
                let br = CreateSolidBrush(th.item_selected);
                FillRect(memdc, &row_rc, br);
                DeleteObject(br as _);
            } else if i == state.hover_idx {
                let br = CreateSolidBrush(th.item_hover);
                FillRect(memdc, &row_rc, br);
                DeleteObject(br as _);
            }

            let icon = item_icon_handle(state, &item);
            if icon != 0 {
                draw_icon_tinted(memdc as _, row_rc.left + 10, row_rc.top + 12, icon, 20, 20, dark);
            }

            if item.pinned && state.icons.pin != 0 {
                let mut pin_y = row_rc.top + 3;
                if pin_y < (view_top + 2) {
                    pin_y = view_top + 2;
                }
                if (pin_y + 16) <= (view_bottom - 2) {
                    let pin_x = row_rc.left + 10 + 20 + 2;
                    draw_icon_tinted(memdc as _, pin_x, pin_y, state.icons.pin, 16, 16, dark);
                }
            }

            if i == state.hover_idx {
                if let Some(del_rc) = state.quick_delete_rect(i) {
                    let bg = inflate_rect(&del_rc, 2, 2);
                    draw_round_rect(memdc as _, &bg, th.surface, th.stroke, 10);
                    if state.icons.del != 0 {
                        draw_icon_tinted(memdc as _, del_rc.left, del_rc.top, state.icons.del, 16, 16, dark);
                    }
                }
            }

            row_rc.left += 40;
            row_rc.right -= 42;
            // 图片条目：显示截图时间（本地时间），让用户快速识别
            let display_preview: String;
            let preview_str = if item.kind == ClipKind::Image {
                display_preview = format_created_at_local(&item.created_at, &item.preview);
                &display_preview
            } else {
                &item.preview
            };
            draw_text(memdc as _, preview_str, &row_rc, th.text, 12, false, false);
        }

        if state.scroll_fade_alpha > 0 && state.total_content_height() > state.list_view_height() {
            if let Some(thumb) = state.scrollbar_thumb_rect() {
                let alpha = state.scroll_fade_alpha;
                // 根据 alpha 在 text_dim 和透明之间插值，hover 时更宽
                let thumb_w = if state.hover_scroll { 6 } else { 4 };
                let thumb_rc = RECT {
                    left: thumb.right - thumb_w,
                    top: thumb.top,
                    right: thumb.right,
                    bottom: thumb.bottom,
                };
                // 颜色根据 alpha 混合（简化：直接用 alpha 调整灰度）
                let c = ((alpha as u32 * 100 + 127) / 255) as u8 + 100; // 100~200 范围
                let thumb_color = rgb(c, c, c);
                draw_round_fill(memdc as _, &thumb_rc, thumb_color, 3);
            }
        }
    }
    RestoreDC(memdc, saved_clip);

    BitBlt(hdc, 0, 0, w, h, memdc, 0, 0, SRCCOPY);
    SelectObject(memdc, oldbmp);
    DeleteObject(membmp as _);
    DeleteDC(memdc);
    EndPaint(hwnd, &ps);
}


fn build_preview(text: &str) -> String {
    let one_line = text.replace('\r', " ").replace('\n', " ").trim().to_string();
    if one_line.chars().count() > 72 {
        let mut s = String::new();
        for (idx, ch) in one_line.chars().enumerate() {
            if idx >= 72 {
                break;
            }
            s.push(ch);
        }
        s.push_str(" ...");
        s
    } else {
        one_line
    }
}


fn build_files_preview(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    if paths.len() == 1 {
        let p = Path::new(&paths[0]);
        let name = p.file_name().and_then(|s| s.to_str()).unwrap_or(&paths[0]);
        name.to_string()
    } else {
        format!("{} 个项目", paths.len())
    }
}

fn file_paths_signature(paths: &[String]) -> String {
    let merged = paths.join("\n");
    format!("files:{}", hash_bytes(merged.as_bytes()))
}

fn output_image_path() -> PathBuf {
    let base = data_dir().join("images");
    let _ = fs::create_dir_all(&base);
    let ts = SystemTime::now().duration_since(UNIX_EPOCH).unwrap_or_default().as_millis();
    base.join(format!("zsclip_{}.png", ts))
}

unsafe fn set_file_paths_to_clipboard(paths: &[String]) -> bool {
    let cleaned: Vec<String> = paths
        .iter()
        .map(|p| p.trim())
        .filter(|p| !p.is_empty())
        .map(|p| p.to_string())
        .collect();
    if cleaned.is_empty() {
        return false;
    }

    let mut wide_paths: Vec<Vec<u16>> = cleaned
        .iter()
        .map(|p| {
            let mut w: Vec<u16> = p.encode_utf16().collect();
            w.push(0);
            w
        })
        .collect();
    let chars_len: usize = wide_paths.iter().map(|w| w.len()).sum::<usize>() + 1;
    let bytes_len = size_of::<DropFiles>() + chars_len * size_of::<u16>();
    let mem = GlobalAlloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes_len);
    if mem.is_null() {
        return false;
    }
    let locked = GlobalLock(mem);
    if locked.is_null() {
        GlobalFree(mem);
        return false;
    }

    let header = locked as *mut DropFiles;
    (*header).p_files = size_of::<DropFiles>() as u32;
    (*header).pt = POINT { x: 0, y: 0 };
    (*header).f_nc = 0;
    (*header).f_wide = 1;

    let mut cursor = (locked as *mut u8).add(size_of::<DropFiles>()) as *mut u16;
    for path in wide_paths.drain(..) {
        std::ptr::copy_nonoverlapping(path.as_ptr(), cursor, path.len());
        cursor = cursor.add(path.len());
    }
    *cursor = 0;
    GlobalUnlock(mem);

    if OpenClipboard(null_mut()) == 0 {
        GlobalFree(mem);
        return false;
    }
    let ok = if EmptyClipboard() == 0 {
        false
    } else {
        !SetClipboardData(CF_HDROP, mem).is_null()
    };
    CloseClipboard();
    if !ok {
        GlobalFree(mem);
    }
    ok
}

fn write_image_bytes_to_output_path(bytes: &[u8], width: u32, height: u32) -> Option<PathBuf> {
    use std::fs::File;
    use std::io::BufWriter;

    let out = output_image_path();
    let file = File::create(&out).ok()?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header().ok()?;
    png_writer.write_image_data(bytes).ok()?;
    Some(out)
}

fn load_image_bytes_from_path(path: &str) -> Option<(Vec<u8>, usize, usize)> {
    use std::fs::File;
    use std::io::BufReader;

    let file = File::open(path).ok()?;
    let decoder = png::Decoder::new(BufReader::new(file));
    let mut reader = decoder.read_info().ok()?;
    let out_size = reader.output_buffer_size();
    let mut buf = vec![0; out_size];
    let info = reader.next_frame(&mut buf).ok()?;
    let bytes = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => bytes.to_vec(),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
            for chunk in bytes.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
            for chunk in bytes.chunks_exact(2) {
                out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            out
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
            for v in bytes {
                out.extend_from_slice(&[*v, *v, *v, 255]);
            }
            out
        }
        _ => return None,
    };
    Some((rgba, info.width as usize, info.height as usize))
}

unsafe fn clipboard_file_paths() -> Option<Vec<String>> {
    if OpenClipboard(null_mut()) == 0 {
        return None;
    }
    let handle = GetClipboardData(CF_HDROP);
    if handle.is_null() {
        CloseClipboard();
        return None;
    }
    let count = DragQueryFileW(handle as _, 0xFFFF_FFFF, null_mut(), 0);
    let mut paths = Vec::new();
    for i in 0..count {
        let len = DragQueryFileW(handle as _, i, null_mut(), 0);
        if len == 0 {
            continue;
        }
        let mut buf = vec![0u16; len as usize + 1];
        let out = DragQueryFileW(handle as _, i, buf.as_mut_ptr(), len + 1);
        if out > 0 {
            paths.push(String::from_utf16_lossy(&buf[..out as usize]));
        }
    }
    CloseClipboard();
    if paths.is_empty() { None } else { Some(paths) }
}


fn save_image_item(item: &ClipItem) -> Option<PathBuf> {
    if let Some(path) = item.image_path.as_ref() {
        let src = PathBuf::from(path);
        if src.exists() {
            return Some(src);
        }
    }
    let (bytes, width, height) = ensure_item_image_bytes(item)?;
    write_image_bytes_to_output_path(&bytes, width as u32, height as u32)
}

pub(crate) fn ensure_item_image_bytes(item: &ClipItem) -> Option<(Vec<u8>, usize, usize)> {
    if let Some(bytes) = &item.image_bytes {
        return Some((bytes.clone(), item.image_width, item.image_height));
    }
    if let Some(path) = item.image_path.as_ref() {
        if let Some(loaded) = load_image_bytes_from_path(path) {
            return Some(loaded);
        }
    }
    if item.kind != ClipKind::Image || item.id <= 0 {
        return None;
    }
    let full = db_load_item_full(item.id)?;
    if let Some(bytes) = full.image_bytes {
        return Some((bytes, full.image_width, full.image_height));
    }
    full.image_path.as_deref().and_then(load_image_bytes_from_path)
}

fn hash_bytes(data: &[u8]) -> u64 {
    let mut hasher = DefaultHasher::new();
    data.hash(&mut hasher);
    hasher.finish()
}

fn trim_process_working_set() {
    unsafe {
        let process = GetCurrentProcess();
        if !process.is_null() {
            let _ = EmptyWorkingSet(process);
        }
    }
}

pub(crate) unsafe fn get_state_ptr(hwnd: HWND) -> *mut AppState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState
}

unsafe fn get_state_mut(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

fn loword(v: u32) -> u16 {
    (v & 0xffff) as u16
}

fn hiword(v: u32) -> u16 {
    ((v >> 16) & 0xffff) as u16
}

fn pt_in_rect(x: i32, y: i32, rc: &RECT) -> bool {
    x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom
}

fn hit_test_row(state: &AppState, x: i32, y: i32) -> i32 {
    let inner = RECT {
        left: LIST_X + LIST_PAD,
        top: LIST_Y + LIST_PAD,
        right: LIST_X + LIST_W - LIST_PAD,
        bottom: LIST_Y + LIST_H - LIST_PAD,
    };
    if !pt_in_rect(x, y, &inner) {
        return -1;
    }
    let yy = y - inner.top + state.scroll_y;
    let idx = yy / ROW_H;
    if idx < 0 || idx >= state.filtered_indices.len() as i32 {
        -1
    } else {
        idx
    }
}

fn inflate_rect(rc: &RECT, dx: i32, dy: i32) -> RECT {
    RECT {
        left: rc.left - dx,
        top: rc.top - dy,
        right: rc.right + dx,
        bottom: rc.bottom + dy,
    }
}
