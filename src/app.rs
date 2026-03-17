// BUILD_MARKER: v26b_hotkey_status_group_layout_registry_cleanup
// BUILD_MARKER: v24_hotkey_registry_autolayout
// BUILD_MARKER: v25_settings_ui_framework_registry_lazy_pages
// BUILD_MARKER: v21_tab_style_title_align_warning_cleanup
// BUILD_MARKER: v20_schemeA_bufferedpaint_deferwindowpos
// BUILD_MARKER: v19_hotkey_scroll_datadir
// BUILD_MARKER: v18c_compile_type_fixes
// BUILD_MARKER: v18b_compile_fixes
// BUILD_MARKER: v18_db_theme_sendinput_refactor
pub(crate) mod state;
pub(crate) mod runtime;
pub(crate) mod data;

pub(crate) use self::runtime::{db_file, save_settings};

use arboard::{Clipboard, ImageData};
use std::borrow::Cow;
use std::cmp::{max, min};
use std::collections::{HashMap, VecDeque};
use std::collections::hash_map::DefaultHasher;
use std::fs;
use std::hash::{Hash, Hasher};
use std::io;
use std::mem::{size_of, zeroed};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use rusqlite::{params, params_from_iter};
use rusqlite::types::Value as SqlValue;
use serde::{Deserialize, Serialize};
use self::data::*;
use std::ptr::{null, null_mut};
use self::runtime::*;
use self::state::*;
use crate::i18n::{app_title, tr, translate};
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
    fn GetCurrentProcessId() -> u32;
    fn GlobalAlloc(uflags: u32, dwbytes: usize) -> *mut core::ffi::c_void;
    fn GlobalLock(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn GlobalUnlock(hmem: *mut core::ffi::c_void) -> i32;
    fn GlobalFree(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
}

#[link(name = "imm32")]
unsafe extern "system" {
    fn ImmGetDefaultIMEWnd(hwnd: HWND) -> HWND;
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
use crate::ui::{draw_icon_tinted, draw_main_segment_bar, draw_round_fill, draw_round_rect, draw_text, draw_text_ex, is_dark_mode, parse_search_query, rgb, settings_nav_item_rect, ClipListState, MainUiLayout, SearchTimeFilter, Theme, SETTINGS_CONTENT_Y, SETTINGS_H, SETTINGS_NAV_W, SETTINGS_PAGES, SETTINGS_W, DT_LEFT, DT_CENTER, DT_VCENTER, DT_SINGLELINE};
use crate::shell::{
    is_directory_item, item_icon_handle, load_icons, open_parent_folder, open_path_with_shell,
    open_source_url, open_source_url_display, restart_explorer_shell, start_update_check,
    toggle_disabled_hotkey_char, update_check_available, update_check_latest_url_or_default,
    update_check_state_snapshot,
};
use crate::hover_preview::{hide_hover_preview, show_hover_preview};
use crate::sticker::show_image_sticker;
use crate::mail_merge_native::{launch_mail_merge_window, launch_mail_merge_window_with_excel};
use crate::tray::{add_tray_icon_localized, handle_tray, position_main_window, remember_window_pos, remove_tray_icon, toggle_window_visibility, toggle_window_visibility_hotkey};
use crate::cloud_sync::{cloud_sync_interval, perform_cloud_sync, CloudSyncAction, CloudSyncConfig, CloudSyncOutcome, CloudSyncPaths};
use crate::db_runtime::{close_db, ensure_db, with_db, with_db_mut};
use crate::time_utils::{days_to_sqlite_date, format_created_at_local, format_local_time_for_image_preview, gregorian_to_days, local_offset_secs, now_utc_sqlite, unix_secs_to_parts};
use crate::win_buffered_paint::{begin_buffered_paint, end_buffered_paint};
use crate::win_system_params::{settings_section_body_rect, CF_HDROP, DropFiles, GMEM_MOVEABLE, GMEM_ZEROINIT, IDC_SET_AUTOSTART, IDC_SET_AUTOHIDE_BLUR, IDC_SET_BTN_OPENCFG, IDC_SET_BTN_OPENDB, IDC_SET_BTN_OPENDATA, IDC_SET_CLICK_HIDE, IDC_SET_CLOSE, IDC_SET_CLOSETRAY, IDC_SET_CLOUD_APPLY_CFG, IDC_SET_CLOUD_DIR, IDC_SET_CLOUD_ENABLE, IDC_SET_CLOUD_INTERVAL, IDC_SET_CLOUD_PASS, IDC_SET_CLOUD_RESTORE_BACKUP, IDC_SET_CLOUD_SYNC_NOW, IDC_SET_CLOUD_UPLOAD_CFG, IDC_SET_CLOUD_URL, IDC_SET_CLOUD_USER, IDC_SET_DX, IDC_SET_DY, IDC_SET_EDGEHIDE, IDC_SET_FX, IDC_SET_FY, IDC_SET_GROUP_ADD, IDC_SET_GROUP_DELETE, IDC_SET_GROUP_DOWN, IDC_SET_GROUP_ENABLE, IDC_SET_GROUP_LIST, IDC_SET_GROUP_RENAME, IDC_SET_GROUP_UP, IDC_SET_GROUP_VIEW_PHRASES, IDC_SET_GROUP_VIEW_RECORDS, IDC_SET_HOVERPREVIEW, IDC_SET_IMAGE_PREVIEW, IDC_SET_MAX, IDC_SET_OPEN_SOURCE, IDC_SET_OPEN_UPDATE, IDC_SET_PLUGIN_MAILMERGE, IDC_SET_POSMODE, IDC_SET_QUICK_DELETE, IDC_SET_SAVE, IDC_SET_SILENTSTART, IDC_SET_TRAYICON, IDC_SET_VV_GROUP, IDC_SET_VV_MODE, IDC_SET_VV_SOURCE, IID_IDATAOBJECT_RAW, RPC_E_CHANGED_MODE_HR, SCROLL_BAR_MARGIN, SCROLL_BAR_W, SCROLL_BAR_W_ACTIVE, SETTINGS_CLASS, SETTINGS_CONTENT_TOTAL_H, SETTINGS_FORM_ROW_GAP, SETTINGS_FORM_ROW_H};
use crate::win_system_ui::{apply_dark_mode_to_window, apply_theme_to_menu, apply_window_corner_preference, caret_accessible_rect, create_drop_source, create_settings_component, create_settings_edit as host_create_settings_edit, create_settings_label as host_create_settings_label, create_settings_label_auto as host_create_settings_label_auto, create_settings_listbox as host_create_settings_listbox, create_settings_password_edit as host_create_settings_password_edit, cursor_over_window_tree, draw_settings_button_component, draw_settings_nav_item, draw_settings_page_cards, draw_settings_page_content, draw_settings_toggle_component, get_window_text, get_x_lparam, get_y_lparam, init_dark_mode_for_process, init_dpi_awareness_for_process, nav_divider_x, nearest_monitor_rect_for_window, nearest_monitor_work_rect_for_point, nearest_monitor_work_rect_for_window, release_raw_com, settings_child_visible, settings_dropdown_index_for_max_items, settings_dropdown_index_for_pos_mode, settings_dropdown_label_for_max_items, settings_dropdown_label_for_pos_mode, settings_dropdown_max_items_from_label, settings_dropdown_pos_mode_from_label, settings_safe_paint_rect, settings_title_rect_win as settings_title_rect, settings_viewport_mask_rect, settings_viewport_rect, show_settings_dropdown_popup, to_wide, window_rect_for_dock, SettingsComponentKind, SettingsCtrlReg, SettingsPage, SettingsUiRegistry, WM_SETTINGS_DROPDOWN_SELECTED};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW,
        CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
        FillRect, GetStockObject, InvalidateRect, IntersectClipRect, LineTo, MoveToEx, PAINTSTRUCT, RDW_ALLCHILDREN, RDW_ERASE, RDW_INVALIDATE, RDW_UPDATENOW, RedrawWindow, RestoreDC, RoundRect, SaveDC, ScreenToClient,
        SelectObject, SetBkColor, SetBkMode, SetTextColor, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
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
            VK_DOWN, VK_ESCAPE, VK_RETURN, VK_UP, VK_V, VK_SHIFT, VK_TAB, VK_LEFT, VK_RIGHT, VK_LWIN, VK_RWIN, VK_NUMPAD1, VK_NUMPAD9,
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

const CLASS_NAME: &str = "ZsClipMain";
const QUICK_CLASS_NAME: &str = "ZsClipQuick";

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
const ID_TIMER_EDGE_AUTO_HIDE: usize = 5;
const ID_TIMER_VV_SHOW: usize = 6;
const ID_TIMER_CLOUD_SYNC: usize = 7;
const STARTUP_RECOVERY_TICKS: u8 = 24;
const WM_VV_SHOW: u32 = WM_APP + 20;
const WM_VV_HIDE: u32 = WM_APP + 21;
const WM_VV_SELECT: u32 = WM_APP + 22;
const WM_ITEMS_PAGE_READY: u32 = WM_APP + 30;
const WM_UPDATE_CHECK_READY: u32 = WM_APP + 31;
const WM_OUTSIDE_CLICK_HIDE: u32 = WM_APP + 32;
const WM_CLOUD_SYNC_READY: u32 = WM_APP + 33;
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
const TME_HOVER: u32 = 0x00000001;
const WM_MOUSEHOVER: u32 = 0x02A1;
const WM_MOUSELEAVE: u32 = 0x02A3;
const SPI_GETMOUSEHOVERTIME_V: u32 = 0x0066;

type AppResult<T> = Result<T, io::Error>;

const EDGE_AUTO_HIDE_PEEK: i32 = 6;
const EDGE_AUTO_HIDE_MARGIN: i32 = 8;

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
#[repr(isize)]
pub(crate) enum WindowRole {
    Main = 1,
    Quick = 2,
}

impl WindowRole {
    fn from_create_param(value: isize) -> Self {
        match value {
            x if x == WindowRole::Quick as isize => WindowRole::Quick,
            _ => WindowRole::Main,
        }
    }

    fn class_name(self) -> &'static str {
        match self {
            WindowRole::Main => CLASS_NAME,
            WindowRole::Quick => QUICK_CLASS_NAME,
        }
    }
}

#[derive(Default)]
struct WindowHosts {
    main: isize,
    quick: isize,
}

static WINDOW_HOSTS: OnceLock<Mutex<WindowHosts>> = OnceLock::new();
static TASKBAR_CREATED_MESSAGE: OnceLock<u32> = OnceLock::new();

fn window_hosts() -> &'static Mutex<WindowHosts> {
    WINDOW_HOSTS.get_or_init(|| Mutex::new(WindowHosts::default()))
}

fn set_window_host(role: WindowRole, hwnd: HWND) {
    if let Ok(mut hosts) = window_hosts().lock() {
        match role {
            WindowRole::Main => hosts.main = hwnd as isize,
            WindowRole::Quick => hosts.quick = hwnd as isize,
        }
    }
}

fn clear_window_host(role: WindowRole, hwnd: HWND) {
    if let Ok(mut hosts) = window_hosts().lock() {
        let slot = match role {
            WindowRole::Main => &mut hosts.main,
            WindowRole::Quick => &mut hosts.quick,
        };
        if *slot == hwnd as isize {
            *slot = 0;
        }
    }
}

fn window_host_hwnds() -> [HWND; 2] {
    window_hosts()
        .lock()
        .ok()
        .map(|hosts| [hosts.main as HWND, hosts.quick as HWND])
        .unwrap_or([null_mut(), null_mut()])
}

unsafe fn set_ignore_clipboard_for_all_hosts(duration_ms: u64) {
    let until = Instant::now() + std::time::Duration::from_millis(duration_ms);
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() {
            continue;
        }
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            (*ptr).ignore_clipboard_until = Some(until);
        }
    }
}

fn is_app_window(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    window_host_hwnds()
        .into_iter()
        .any(|host| !host.is_null() && host == hwnd)
}

unsafe fn screen_point_hits_window_scope(hwnd: HWND, pt: POINT) -> bool {
    if hwnd.is_null() || IsWindow(hwnd) == 0 || IsWindowVisible(hwnd) == 0 {
        return false;
    }
    if pt_in_rect_screen(&pt, &window_rect_for_dock(hwnd)) {
        return true;
    }
    cursor_over_window_tree(hwnd, pt)
}

unsafe fn any_visible_window_requires_outside_hide() -> bool {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() || IsWindowVisible(hwnd) == 0 {
            continue;
        }
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() && (*ptr).settings.auto_hide_on_blur {
            return true;
        }
    }
    false
}

unsafe fn should_ignore_outside_click_for_point(pt: POINT) -> bool {
    for hwnd in window_host_hwnds() {
        if screen_point_hits_window_scope(hwnd, pt) {
            return true;
        }
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            if screen_point_hits_window_scope((*ptr).settings_hwnd, pt) {
                return true;
            }
            if !(*ptr).settings_hwnd.is_null() {
                let st_ptr = GetWindowLongPtrW((*ptr).settings_hwnd, GWLP_USERDATA) as *mut SettingsWndState;
                if !st_ptr.is_null() && screen_point_hits_window_scope((*st_ptr).dropdown_popup, pt) {
                    return true;
                }
            }
        }
    }
    let popup = current_vv_popup_hwnd();
    screen_point_hits_window_scope(popup, pt)
}

unsafe extern "system" fn outside_hide_mouse_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code >= 0
        && matches!(wparam as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN)
        && any_visible_window_requires_outside_hide()
    {
        let data = &*(lparam as *const MSLLHOOKSTRUCT);
        let pt = data.pt;
        if !should_ignore_outside_click_for_point(pt) {
            for hwnd in window_host_hwnds() {
                if hwnd.is_null() || IsWindowVisible(hwnd) == 0 {
                    continue;
                }
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() && (*ptr).settings.auto_hide_on_blur {
                    let _ = PostMessageW(hwnd, WM_OUTSIDE_CLICK_HIDE, 0, 0);
                }
            }
        }
    }
    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe fn ensure_outside_hide_mouse_hook() {
    let Ok(mut handle) = outside_hide_mouse_hook_handle().lock() else {
        return;
    };
    if *handle == 0 {
        *handle = SetWindowsHookExW(WH_MOUSE_LL, Some(outside_hide_mouse_hook_proc), GetModuleHandleW(null()), 0) as isize;
    }
}

unsafe extern "system" fn quick_escape_keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 || (wparam as u32 != WM_KEYDOWN && wparam as u32 != WM_SYSKEYDOWN) {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    let data = &*(lparam as *const KBDLLHOOKSTRUCT);
    if (data.flags & LLKHF_INJECTED_FLAG) != 0 || data.vkCode != VK_ESCAPE as u32 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let quick = quick_window_hwnd();
    if !quick.is_null() && IsWindowVisible(quick) != 0 {
        let _ = PostMessageW(quick, WM_KEYDOWN, VK_ESCAPE as usize, 0);
        return 1;
    }

    let main = main_window_hwnd();
    if !main.is_null() && IsWindowVisible(main) != 0 {
        let ptr = get_state_ptr(main);
        if !ptr.is_null() && (*ptr).main_window_noactivate {
            let _ = PostMessageW(main, WM_KEYDOWN, VK_ESCAPE as usize, 0);
            return 1;
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe fn ensure_quick_escape_keyboard_hook() {
    let Ok(mut handle) = quick_escape_keyboard_hook_handle().lock() else {
        return;
    };
    if *handle == 0 {
        *handle = SetWindowsHookExW(WH_KEYBOARD_LL, Some(quick_escape_keyboard_hook_proc), GetModuleHandleW(null()), 0) as isize;
    }
}

pub(crate) fn main_window_hwnd() -> HWND {
    window_hosts()
        .lock()
        .ok()
        .map(|hosts| hosts.main as HWND)
        .unwrap_or(null_mut())
}

pub(crate) fn quick_window_hwnd() -> HWND {
    window_hosts()
        .lock()
        .ok()
        .map(|hosts| hosts.quick as HWND)
        .unwrap_or(null_mut())
}

fn taskbar_created_message() -> u32 {
    *TASKBAR_CREATED_MESSAGE.get_or_init(|| unsafe {
        RegisterWindowMessageW(to_wide("TaskbarCreated").as_ptr())
    })
}

unsafe fn sync_main_tray_icon(hwnd: HWND, state: &mut AppState) {
    remove_tray_icon(hwnd);
    state.tray_icon_registered = false;
    if tray_mode_enabled(&state.settings) && state.icons.app != 0 {
        state.tray_icon_registered = add_tray_icon_localized(hwnd, state.icons.app);
    }
}

unsafe fn retry_startup_integrations(hwnd: HWND, state: &mut AppState) {
    if state.role != WindowRole::Main || state.startup_recovery_ticks == 0 {
        return;
    }

    if tray_mode_enabled(&state.settings) && state.icons.app != 0 && !state.tray_icon_registered {
        sync_main_tray_icon(hwnd, state);
    }

    if state.settings.hotkey_enabled && !state.hotkey_registered {
        register_hotkey_for(hwnd, state);
    }

    let tray_ready = !tray_mode_enabled(&state.settings) || state.icons.app == 0 || state.tray_icon_registered;
    let hotkey_ready = !state.settings.hotkey_enabled || state.hotkey_registered;
    if tray_ready && hotkey_ready {
        state.startup_recovery_ticks = 0;
    } else {
        state.startup_recovery_ticks = state.startup_recovery_ticks.saturating_sub(1);
    }
}

unsafe fn notify_update_state_changed() {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() {
            continue;
        }
        let _ = PostMessageW(hwnd, WM_UPDATE_CHECK_READY, 0, 0);
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() && !(*ptr).settings_hwnd.is_null() && IsWindow((*ptr).settings_hwnd) != 0 {
            InvalidateRect((*ptr).settings_hwnd, null(), 1);
        }
    }
}
const EDGE_AUTO_HIDE_NONE: i32 = -1;
const EDGE_AUTO_HIDE_LEFT: i32 = 0;
const EDGE_AUTO_HIDE_RIGHT: i32 = 1;
const EDGE_AUTO_HIDE_TOP: i32 = 2;
const ITEMS_PAGE_SIZE: usize = 200;
const ITEMS_LOAD_AHEAD_ROWS: i32 = 18;
const EDGE_AUTO_HIDE_BOTTOM: i32 = 3;
const VV_POPUP_CLASS: &str = "ZsClipVvPopup";
const VV_POPUP_MAX_ITEMS: usize = 9;
const VV_POPUP_W: i32 = 360;
const VV_POPUP_HEADER_H: i32 = 58;
const VV_POPUP_ROW_H: i32 = 30;
const LLKHF_INJECTED_FLAG: u32 = 0x00000010;
const VV_TRIGGER_TIMEOUT_MS: u128 = 300;
const VV_SHOW_RETRY_DELAY_MS: u32 = 30;
const VV_SHOW_RETRY_MAX: u8 = 10;
const VV_POPUP_MENU_GRACE_MS: u64 = 450;
const VV_IMM_POINT_MAX_X_DRIFT: i32 = 120;
const VV_IMM_POINT_MAX_Y_DRIFT: i32 = 180;
const IMC_GETCANDIDATEPOS: WPARAM = 0x0007;
const IMC_GETCOMPOSITIONWINDOW: WPARAM = 0x000B;
const CFS_RECT_V: u32 = 0x0001;
const CFS_POINT_V: u32 = 0x0002;
const CFS_FORCE_POSITION_V: u32 = 0x0020;
const CFS_CANDIDATEPOS_V: u32 = 0x0040;
const CFS_EXCLUDE_V: u32 = 0x0080;

#[repr(C)]
struct CandidateForm {
    dw_index: u32,
    dw_style: u32,
    pt_current_pos: POINT,
    rc_area: RECT,
}

#[repr(C)]
struct CompositionForm {
    dw_style: u32,
    pt_current_pos: POINT,
    rc_area: RECT,
}

#[derive(Copy, Clone)]
struct VvOverlayAnchor {
    left: i32,
    edge_y: i32,
    align_bottom: bool,
    exact_rect: bool,
}

fn vv_choose_overlay_edge(top: i32, bottom: i32, popup_height: i32, work_area: &RECT) -> (i32, bool) {
    let below_space = work_area.bottom - bottom;
    let above_space = top - work_area.top;
    let align_bottom = below_space < popup_height && above_space > below_space;
    let edge_y = if align_bottom { top } else { bottom };
    (edge_y, align_bottom)
}

fn vv_anchor_within(anchor: &VvOverlayAnchor, reference: &VvOverlayAnchor, max_dx: i32, max_dy: i32) -> bool {
    (anchor.left - reference.left).abs() <= max_dx && (anchor.edge_y - reference.edge_y).abs() <= max_dy
}

fn vv_imm_point_anchor_is_plausible(
    anchor: &VvOverlayAnchor,
    caret_anchor: Option<&VvOverlayAnchor>,
    focus_anchor: Option<&VvOverlayAnchor>,
) -> bool {
    if anchor.exact_rect {
        return true;
    }
    if let Some(caret) = caret_anchor {
        if vv_anchor_within(anchor, caret, VV_IMM_POINT_MAX_X_DRIFT, VV_IMM_POINT_MAX_Y_DRIFT) {
            return true;
        }
        if let Some(focus) = focus_anchor {
            return vv_anchor_within(anchor, focus, VV_IMM_POINT_MAX_X_DRIFT + 60, VV_IMM_POINT_MAX_Y_DRIFT + 40);
        }
        return false;
    }
    if let Some(focus) = focus_anchor {
        return vv_anchor_within(anchor, focus, VV_IMM_POINT_MAX_X_DRIFT + 60, VV_IMM_POINT_MAX_Y_DRIFT + 40);
    }
    true
}

unsafe fn vv_popup_row_rect(row: usize) -> RECT {
    let top = VV_POPUP_HEADER_H + 10 + row as i32 * VV_POPUP_ROW_H;
    RECT { left: 12, top, right: VV_POPUP_W - 12, bottom: top + VV_POPUP_ROW_H - 2 }
}

unsafe fn vv_popup_group_rect() -> RECT {
    RECT {
        left: VV_POPUP_W - 150,
        top: 10,
        right: VV_POPUP_W - 14,
        bottom: 34,
    }
}

fn vv_popup_resolved_group_id(state: &AppState, group_id: i64) -> i64 {
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    if group_id > 0 && state.groups_for_tab(source_tab).iter().any(|g| g.id == group_id) {
        group_id
    } else {
        0
    }
}

fn vv_popup_group_name(state: &AppState) -> String {
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    let all_label = if source_tab == 0 { "全部记录" } else { "全部短语" };
    group_name_for_display(state.groups_for_tab(source_tab), state.vv_popup_group_id, all_label)
}

fn vv_popup_rebuild_items(state: &mut AppState) {
    let group_id = vv_popup_resolved_group_id(state, state.vv_popup_group_id);
    state.vv_popup_group_id = group_id;
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    state.vv_popup_items = db_load_vv_popup_items(source_tab as i64, group_id, VV_POPUP_MAX_ITEMS)
        .into_iter()
        .enumerate()
        .map(|(i, item)| VvPopupEntry {
            index: i + 1,
            item,
        })
        .collect();
}

unsafe fn vv_popup_show_group_menu(hwnd: HWND, state: &AppState) -> Option<i64> {
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    let groups = state.groups_for_tab(source_tab);
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return None;
    }
    apply_theme_to_menu(menu as _);
    let current_group_id = vv_popup_resolved_group_id(state, state.vv_popup_group_id);
    let all_flags = if current_group_id == 0 { MF_STRING | MF_CHECKED } else { MF_STRING };
    AppendMenuW(
        menu,
        all_flags,
        IDM_GROUP_FILTER_ALL,
        to_wide(translate(if source_tab == 0 { "全部记录" } else { "全部短语" }).as_ref()).as_ptr(),
    );
    if !groups.is_empty() {
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        for (idx, g) in groups.iter().enumerate() {
            let flags = if current_group_id == g.id { MF_STRING | MF_CHECKED } else { MF_STRING };
            AppendMenuW(menu, flags, IDM_GROUP_FILTER_BASE + idx, to_wide(&g.name).as_ptr());
        }
    }
    let rect = vv_popup_group_rect();
    let mut pt = POINT {
        x: rect.left,
        y: rect.bottom + 4,
    };
    ClientToScreen(hwnd, &mut pt);
    vv_set_popup_menu_active(true);
    let cmd = TrackPopupMenu(
        menu,
        TPM_RIGHTBUTTON | TPM_TOPALIGN | TPM_LEFTALIGN | TPM_RETURNCMD,
        pt.x,
        pt.y,
        0,
        hwnd,
        null(),
    ) as usize;
    vv_set_popup_menu_active(false);
    DestroyMenu(menu);
    if cmd == IDM_GROUP_FILTER_ALL {
        Some(0)
    } else if (IDM_GROUP_FILTER_BASE..IDM_GROUP_FILTER_BASE + 2000).contains(&cmd) {
        groups.get(cmd - IDM_GROUP_FILTER_BASE).map(|g| g.id)
    } else {
        None
    }
}

unsafe fn ensure_vv_popup_class() {
    let hinstance = GetModuleHandleW(null());
    let cname = to_wide(VV_POPUP_CLASS);
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(vv_popup_wnd_proc);
    wc.hInstance = hinstance;
    wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
    wc.hbrBackground = null_mut();
    wc.lpszClassName = cname.as_ptr();
    RegisterClassExW(&wc);
}

unsafe fn vv_popup_hwnd(main_hwnd: HWND) -> HWND {
    let raw = *VV_POPUP_HWND.get_or_init(|| {
        ensure_vv_popup_class();
        CreateWindowExW(
            WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
            to_wide(VV_POPUP_CLASS).as_ptr(),
            to_wide("").as_ptr(),
            WS_POPUP,
            0,
            0,
            VV_POPUP_W,
            VV_POPUP_HEADER_H + 24,
            null_mut(),
            null_mut(),
            GetModuleHandleW(null()),
            main_hwnd as _,
        ) as isize
    });
    raw as HWND
}

fn current_vv_popup_hwnd() -> HWND {
    VV_POPUP_HWND.get().copied().unwrap_or(0) as HWND
}

unsafe fn vv_popup_height(rows: usize) -> i32 {
    VV_POPUP_HEADER_H + 20 + rows.max(1) as i32 * VV_POPUP_ROW_H
}

fn vv_rect_has_area(rc: &RECT) -> bool {
    rc.right > rc.left && rc.bottom > rc.top
}

fn vv_rect_has_point(rc: &RECT) -> bool {
    rc.left != 0 || rc.top != 0 || rc.right != 0 || rc.bottom != 0
}

unsafe fn vv_point_to_screen(hwnd: HWND, pt: &mut POINT) -> bool {
    if hwnd.is_null() {
        return false;
    }
    ClientToScreen(hwnd, pt) != 0
}

unsafe fn vv_client_rect_to_screen(hwnd: HWND, rc: &RECT) -> Option<RECT> {
    let mut tl = POINT { x: rc.left, y: rc.top };
    let mut br = POINT { x: rc.right, y: rc.bottom };
    if vv_point_to_screen(hwnd, &mut tl) && vv_point_to_screen(hwnd, &mut br) {
        return Some(RECT { left: tl.x, top: tl.y, right: br.x, bottom: br.y });
    }
    None
}

unsafe fn vv_thread_caret_anchor(target: HWND, popup_height: i32, work_area: &RECT) -> Option<VvOverlayAnchor> {
    if target.is_null() || IsWindow(target) == 0 {
        return None;
    }
    let mut pid = 0u32;
    let thread_id = GetWindowThreadProcessId(target, &mut pid);
    if thread_id == 0 {
        return None;
    }
    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = size_of::<GUITHREADINFO>() as u32;
    if GetGUIThreadInfo(thread_id, &mut info) == 0 || !vv_rect_has_point(&info.rcCaret) {
        return None;
    }
    let anchor_hwnd = if !info.hwndCaret.is_null() {
        info.hwndCaret
    } else if !info.hwndFocus.is_null() {
        info.hwndFocus
    } else {
        target
    };
    if anchor_hwnd.is_null() || IsWindow(anchor_hwnd) == 0 {
        return None;
    }

    let mut top_left = POINT { x: info.rcCaret.left, y: info.rcCaret.top };
    let mut bottom_left = POINT {
        x: info.rcCaret.left,
        y: if info.rcCaret.bottom > info.rcCaret.top {
            info.rcCaret.bottom
        } else {
            info.rcCaret.top + 24
        },
    };
    if !vv_point_to_screen(anchor_hwnd, &mut top_left) || !vv_point_to_screen(anchor_hwnd, &mut bottom_left) {
        return None;
    }

    let below_space = work_area.bottom - bottom_left.y;
    let above_space = top_left.y - work_area.top;
    let align_bottom = below_space < popup_height && above_space > below_space;
    let edge_y = if align_bottom { top_left.y } else { bottom_left.y };
    Some(VvOverlayAnchor {
        left: top_left.x,
        edge_y,
        align_bottom,
        exact_rect: true,
    })
}

unsafe fn vv_accessible_caret_anchor(
    focus_hwnd: HWND,
    popup_height: i32,
    work_area: &RECT,
) -> Option<VvOverlayAnchor> {
    let rc = caret_accessible_rect(focus_hwnd)?;
    if !vv_rect_has_area(&rc) {
        return None;
    }
    let (edge_y, align_bottom) =
        vv_choose_overlay_edge(rc.top, rc.bottom, popup_height, work_area);
    Some(VvOverlayAnchor {
        left: rc.left,
        edge_y,
        align_bottom,
        exact_rect: true,
    })
}

unsafe fn vv_imm_overlay_anchor(focus_hwnd: HWND, popup_height: i32, work_area: &RECT) -> Option<VvOverlayAnchor> {
    if focus_hwnd.is_null() || IsWindow(focus_hwnd) == 0 {
        return None;
    }
    let ime_hwnd = ImmGetDefaultIMEWnd(focus_hwnd);
    if ime_hwnd.is_null() || IsWindow(ime_hwnd) == 0 {
        return None;
    }

    for index in 0..=3 {
        let mut cand = CandidateForm {
            dw_index: index,
            dw_style: 0,
            pt_current_pos: POINT { x: 0, y: 0 },
            rc_area: RECT { left: 0, top: 0, right: 0, bottom: 0 },
        };
        if SendMessageW(ime_hwnd, WM_IME_CONTROL, IMC_GETCANDIDATEPOS, &mut cand as *mut _ as LPARAM) != 0 {
            continue;
        }
        let mut pt = cand.pt_current_pos;
        let pt_ok = vv_point_to_screen(focus_hwnd, &mut pt);
        if cand.dw_style == CFS_CANDIDATEPOS_V && pt_ok {
            let (edge_y, align_bottom) = vv_choose_overlay_edge(pt.y, pt.y, popup_height, work_area);
            return Some(VvOverlayAnchor { left: pt.x, edge_y, align_bottom, exact_rect: false });
        }
        if cand.dw_style == CFS_EXCLUDE_V && vv_rect_has_area(&cand.rc_area) {
            if let Some(exclude_rc) = vv_client_rect_to_screen(focus_hwnd, &cand.rc_area) {
                let (edge_y, align_bottom) =
                    vv_choose_overlay_edge(exclude_rc.top, exclude_rc.bottom, popup_height, work_area);
                return Some(VvOverlayAnchor { left: exclude_rc.left, edge_y, align_bottom, exact_rect: true });
            }
        }
    }

    let mut comp = CompositionForm {
        dw_style: 0,
        pt_current_pos: POINT { x: 0, y: 0 },
        rc_area: RECT { left: 0, top: 0, right: 0, bottom: 0 },
    };
    if SendMessageW(ime_hwnd, WM_IME_CONTROL, IMC_GETCOMPOSITIONWINDOW, &mut comp as *mut _ as LPARAM) == 0 {
        let mut pt = comp.pt_current_pos;
        let pt_ok = vv_point_to_screen(focus_hwnd, &mut pt);
        if (comp.dw_style == CFS_POINT_V || comp.dw_style == CFS_FORCE_POSITION_V) && pt_ok {
            let (edge_y, align_bottom) =
                vv_choose_overlay_edge(pt.y, pt.y, popup_height, work_area);
            return Some(VvOverlayAnchor { left: pt.x, edge_y, align_bottom, exact_rect: false });
        }
        if comp.dw_style == CFS_RECT_V && vv_rect_has_area(&comp.rc_area) {
            if let Some(rc) = vv_client_rect_to_screen(focus_hwnd, &comp.rc_area) {
                let (edge_y, align_bottom) =
                    vv_choose_overlay_edge(rc.top, rc.bottom, popup_height, work_area);
                return Some(VvOverlayAnchor { left: rc.left, edge_y, align_bottom, exact_rect: true });
            }
        }
    }

    None
}

unsafe fn vv_focus_rect_anchor(
    focus_hwnd: HWND,
    popup_height: i32,
    work_area: &RECT,
) -> Option<VvOverlayAnchor> {
    if focus_hwnd.is_null() || IsWindow(focus_hwnd) == 0 {
        return None;
    }
    let mut rc = zeroed();
    if GetWindowRect(focus_hwnd, &mut rc) == 0 || !vv_rect_has_area(&rc) {
        return None;
    }
    let width = rc.right - rc.left;
    let height = rc.bottom - rc.top;
    if width <= 0 || height <= 0 || height > 180 || width > (work_area.right - work_area.left) - 40 {
        return None;
    }
    let (edge_y, align_bottom) = vv_choose_overlay_edge(rc.top, rc.bottom, popup_height, work_area);
    Some(VvOverlayAnchor {
        left: rc.left,
        edge_y,
        align_bottom,
        exact_rect: true,
    })
}

unsafe fn vv_cursor_anchor(popup_height: i32, work_area: &RECT) -> Option<VvOverlayAnchor> {
    let mut pt = POINT { x: 0, y: 0 };
    if GetCursorPos(&mut pt) == 0 {
        return None;
    }
    let (edge_y, align_bottom) = vv_choose_overlay_edge(pt.y, pt.y, popup_height, work_area);
    Some(VvOverlayAnchor {
        left: pt.x,
        edge_y,
        align_bottom,
        exact_rect: false,
    })
}

unsafe fn vv_focus_hwnd_for_target(target: HWND) -> HWND {
    if target.is_null() {
        return null_mut();
    }
    let mut pid = 0u32;
    let thread_id = GetWindowThreadProcessId(target, &mut pid);
    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = size_of::<GUITHREADINFO>() as u32;
    if thread_id != 0 && GetGUIThreadInfo(thread_id, &mut info) != 0 {
        if !info.hwndFocus.is_null() {
            return info.hwndFocus;
        }
    }
    target
}

unsafe fn vv_popup_move_near_target(state: &AppState, popup: HWND) -> bool {
    if popup.is_null() || IsWindow(popup) == 0 {
        return false;
    }
    let focus_hwnd = vv_focus_hwnd_for_target(state.vv_popup_target);
    if focus_hwnd.is_null() {
        return false;
    }
    let mut wa = nearest_monitor_work_rect_for_window(focus_hwnd);
    let height = vv_popup_height(state.vv_popup_items.len());
    let caret_anchor = vv_accessible_caret_anchor(focus_hwnd, height, &wa)
        .or_else(|| vv_thread_caret_anchor(focus_hwnd, height, &wa));
    let focus_anchor = vv_focus_rect_anchor(focus_hwnd, height, &wa);
    let imm_anchor = vv_imm_overlay_anchor(focus_hwnd, height, &wa)
        .filter(|anchor| vv_imm_point_anchor_is_plausible(anchor, caret_anchor.as_ref(), focus_anchor.as_ref()));
    let anchor = imm_anchor
        .or(caret_anchor)
        .or(focus_anchor)
        .or_else(|| vv_cursor_anchor(height, &wa));
    let Some(anchor) = anchor else {
        return false;
    };
    wa = nearest_monitor_work_rect_for_point(POINT {
        x: anchor.left,
        y: anchor.edge_y,
    });
    let mut x = anchor.left;
    let mut y = if anchor.align_bottom {
        anchor.edge_y - height
    } else {
        anchor.edge_y
    };
    if x + VV_POPUP_W > wa.right {
        x = wa.right - VV_POPUP_W;
    }
    if x < wa.left {
        x = wa.left;
    }
    if y < wa.top {
        y = wa.top;
    }
    if y + height > wa.bottom {
        y = wa.bottom - height;
    }
    SetWindowPos(
        popup,
        HWND_TOPMOST,
        x,
        y,
        VV_POPUP_W,
        height,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
    true
}

unsafe fn vv_popup_sync_hook_state(visible: bool, target: HWND) {
    if let Ok(mut guard) = vv_hook_state().lock() {
        guard.popup_active = visible;
        guard.popup_target = if visible { target as isize } else { 0 };
        if !visible {
            guard.popup_menu_active = false;
            guard.popup_menu_grace_until = None;
            guard.last_was_v = false;
            guard.last_v_target = 0;
            guard.last_v_at = None;
        }
    }
}

unsafe fn send_escape_key() {
    keybd_event(VK_ESCAPE as u8, 0, 0, 0);
    keybd_event(VK_ESCAPE as u8, 0, KEYEVENTF_KEYUP, 0);
}

unsafe fn vv_popup_hide(_hwnd: HWND, state: &mut AppState) {
    state.vv_popup_visible = false;
    state.vv_popup_pending_target = null_mut();
    state.vv_popup_pending_retries = 0;
    state.vv_popup_target = null_mut();
    state.vv_popup_replaces_ime = false;
    state.vv_popup_group_id = 0;
    state.vv_popup_items.clear();
    vv_popup_sync_hook_state(false, null_mut());
    let popup = current_vv_popup_hwnd();
    if !popup.is_null() && IsWindow(popup) != 0 {
        ShowWindow(popup, SW_HIDE);
    }
}

unsafe fn vv_popup_show(hwnd: HWND, state: &mut AppState, target: HWND) -> bool {
    state.vv_popup_group_id = vv_popup_resolved_group_id(state, state.settings.vv_group_id);
    vv_popup_rebuild_items(state);
    state.vv_popup_target = target;
    state.vv_popup_pending_retries = 0;
    state.vv_popup_visible = true;
    state.vv_popup_replaces_ime = false;
    vv_popup_sync_hook_state(true, target);
    let popup = vv_popup_hwnd(hwnd);
    if !vv_popup_move_near_target(state, popup) {
        vv_popup_hide(hwnd, state);
        return false;
    }
    send_escape_key();
    state.vv_popup_replaces_ime = true;
    InvalidateRect(popup, null(), 1);
    ShowWindow(popup, SW_SHOWNOACTIVATE);
    true
}

unsafe fn vv_digit_index_from_vk(vk: u32) -> Option<usize> {
    match vk {
        0x31..=0x39 => Some((vk - 0x31) as usize),
        x if x >= VK_NUMPAD1 as u32 && x <= VK_NUMPAD9 as u32 => Some((x - VK_NUMPAD1 as u32) as usize),
        _ => None,
    }
}

unsafe fn vv_is_modifier_vk(vk: u32) -> bool {
    matches!(vk, x if x == VK_SHIFT as u32 || x == VK_CONTROL as u32 || x == VK_MENU as u32 || x == VK_LWIN as u32 || x == VK_RWIN as u32)
}

unsafe fn vv_target_is_ignored(hwnd: HWND, main_hwnd: HWND) -> bool {
    if hwnd.is_null() || hwnd == main_hwnd {
        return true;
    }
    let popup = current_vv_popup_hwnd();
    if hwnd == popup {
        return true;
    }
    let mut pid = 0u32;
    let _ = GetWindowThreadProcessId(hwnd, &mut pid);
    pid == GetCurrentProcessId()
}

unsafe extern "system" fn vv_keyboard_hook_proc(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code < 0 || (wparam as u32 != WM_KEYDOWN && wparam as u32 != WM_SYSKEYDOWN) {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    let data = &*(lparam as *const KBDLLHOOKSTRUCT);
    if (data.flags & LLKHF_INJECTED_FLAG) != 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let Ok(mut hook) = vv_hook_state().lock() else {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    };
    if !hook.enabled || hook.main_hwnd == 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    let main_hwnd = hook.main_hwnd as HWND;
    let menu_active = hook.popup_menu_active
        || hook
            .popup_menu_grace_until
            .map(|until| until > Instant::now())
            .unwrap_or(false);

    let fg = GetForegroundWindow();
    if vv_target_is_ignored(fg, main_hwnd) {
        if hook.popup_active && menu_active {
            return CallNextHookEx(null_mut(), code, wparam, lparam);
        }
        hook.last_was_v = false;
        hook.last_v_target = 0;
        hook.last_v_at = None;
        if hook.popup_active {
            hook.popup_active = false;
            hook.popup_target = 0;
            PostMessageW(main_hwnd, WM_VV_HIDE, 0, 0);
        }
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let has_mod = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0
        || (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) != 0
        || (GetAsyncKeyState(VK_LWIN as i32) as u16 & 0x8000) != 0
        || (GetAsyncKeyState(VK_RWIN as i32) as u16 & 0x8000) != 0;

    if hook.popup_active {
        if menu_active {
            return CallNextHookEx(null_mut(), code, wparam, lparam);
        }
        if fg as isize != hook.popup_target {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_v_at = None;
            PostMessageW(main_hwnd, WM_VV_HIDE, 0, 0);
            return CallNextHookEx(null_mut(), code, wparam, lparam);
        }
        if let Some(idx) = vv_digit_index_from_vk(data.vkCode) {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            PostMessageW(main_hwnd, WM_VV_SELECT, idx, 0);
            return 1;
        }
        if data.vkCode == VK_ESCAPE as u32 {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            PostMessageW(main_hwnd, WM_VV_HIDE, 0, 0);
            return 1;
        }
        if data.vkCode == VK_BACK as u32 {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_v_at = None;
            PostMessageW(main_hwnd, WM_VV_HIDE, 0, 0);
            return CallNextHookEx(null_mut(), code, wparam, lparam);
        }
        if !vv_is_modifier_vk(data.vkCode) {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            PostMessageW(main_hwnd, WM_VV_HIDE, 0, 0);
        }
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    if has_mod {
        hook.last_was_v = false;
        hook.last_v_target = 0;
        hook.last_v_at = None;
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    if data.vkCode == 0x56 {
        let within_timeout = hook
            .last_v_at
            .map(|t| t.elapsed().as_millis() <= VV_TRIGGER_TIMEOUT_MS)
            .unwrap_or(false);
        if hook.last_was_v && hook.last_v_target == fg as isize && within_timeout {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            PostMessageW(main_hwnd, WM_VV_SHOW, fg as usize, 0);
        } else {
            hook.last_was_v = true;
            hook.last_v_target = fg as isize;
            hook.last_v_at = Some(Instant::now());
        }
    } else if !vv_is_modifier_vk(data.vkCode) {
        hook.last_was_v = false;
        hook.last_v_target = 0;
        hook.last_v_at = None;
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}

unsafe fn update_vv_mode_hook(main_hwnd: HWND, enabled: bool) {
    if let Ok(mut hook_state) = vv_hook_state().lock() {
        hook_state.main_hwnd = main_hwnd as isize;
        hook_state.enabled = enabled;
        if !enabled {
            hook_state.last_was_v = false;
            hook_state.last_v_target = 0;
            hook_state.last_v_at = None;
            hook_state.popup_active = false;
            hook_state.popup_target = 0;
            hook_state.popup_menu_active = false;
            hook_state.popup_menu_grace_until = None;
        }
    }
    let Ok(mut handle) = vv_hook_handle().lock() else {
        return;
    };
    if enabled {
        if *handle == 0 {
            *handle = SetWindowsHookExW(WH_KEYBOARD_LL, Some(vv_keyboard_hook_proc), GetModuleHandleW(null()), 0) as isize;
        }
    } else if *handle != 0 {
        UnhookWindowsHookEx(*handle as _);
        *handle = 0;
    }
}

unsafe extern "system" fn vv_popup_wnd_proc(hwnd: HWND, msg: u32, _wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            apply_window_corner_preference(hwnd);
            1
        }
        WM_PAINT => {
            let main_hwnd = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
            let ptr = get_state_ptr(main_hwnd);
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !hdc.is_null() && !ptr.is_null() {
                let state = &*ptr;
                let th = Theme::default();
                let mut rc: RECT = zeroed();
                GetClientRect(hwnd, &mut rc);
                let bg = CreateSolidBrush(th.surface);
                FillRect(hdc, &rc, bg);
                DeleteObject(bg as _);
                draw_round_rect(hdc as _, &rc, th.surface, th.stroke, 12);

                let title_rc = RECT { left: 14, top: 10, right: 150, bottom: 30 };
                draw_text_ex(hdc as _, "VV 模式", &title_rc, th.text, 13, true, false, "Segoe UI Variable Display");
                let group_rc = vv_popup_group_rect();
                draw_round_fill(hdc as _, &group_rc, th.bg, 10);
                draw_round_rect(hdc as _, &group_rc, th.bg, th.stroke, 10);
                let mut group_text_rc = group_rc;
                group_text_rc.left += 10;
                group_text_rc.right -= 20;
                draw_text_ex(
                    hdc as _,
                    &vv_popup_group_name(state),
                    &group_text_rc,
                    th.text,
                    11,
                    false,
                    true,
                    "Segoe UI Variable Text",
                );
                let arrow_rc = RECT {
                    left: group_rc.right - 18,
                    top: group_rc.top,
                    right: group_rc.right - 4,
                    bottom: group_rc.bottom,
                };
                draw_text_ex(hdc as _, "v", &arrow_rc, th.text_muted, 11, true, true, "Segoe UI Variable Text");

                let sub_rc = RECT { left: 14, top: 34, right: rc.right - 14, bottom: 52 };
                draw_text_ex(hdc as _, "输入 1-9 直接粘贴，Esc 取消", &sub_rc, th.text_muted, 11, false, false, "Segoe UI Variable Text");

                if state.vv_popup_items.is_empty() {
                    let empty_rc = RECT {
                        left: 16,
                        top: VV_POPUP_HEADER_H + 16,
                        right: rc.right - 16,
                        bottom: VV_POPUP_HEADER_H + 48,
                    };
                    draw_text_ex(
                        hdc as _,
                        "当前分组暂无记录",
                        &empty_rc,
                        th.text_muted,
                        12,
                        true,
                        false,
                        "Segoe UI Variable Text",
                    );
                } else {
                    for (row, entry) in state.vv_popup_items.iter().enumerate() {
                        let row_rc = vv_popup_row_rect(row);
                        let bubble = RECT { left: row_rc.left, top: row_rc.top + 4, right: row_rc.left + 24, bottom: row_rc.top + 24 };
                        draw_round_fill(hdc as _, &bubble, th.accent, 8);
                        draw_text_ex(hdc as _, &entry.index.to_string(), &bubble, rgb(255, 255, 255), 11, true, true, "Segoe UI Variable Text");

                        let mut text_rc = row_rc;
                        text_rc.left += 34;
                        let label = if entry.item.kind == ClipKind::Image {
                            format_created_at_local(&entry.item.created_at, &entry.item.preview)
                        } else {
                            entry.item.preview.clone()
                        };
                        draw_text_ex(hdc as _, &label, &text_rc, th.text, 12, false, false, "Segoe UI Variable Text");
                    }
                }
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_LBUTTONUP => {
            let main_hwnd = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as HWND;
            let ptr = get_state_ptr(main_hwnd);
            if ptr.is_null() {
                return 0;
            }
            let state = &mut *ptr;
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            if pt_in_rect(x, y, &vv_popup_group_rect()) {
                if let Some(group_id) = vv_popup_show_group_menu(hwnd, state) {
                    state.vv_popup_group_id = vv_popup_resolved_group_id(state, group_id);
                    vv_popup_rebuild_items(state);
                    let _ = force_foreground_window(state.vv_popup_target);
                    vv_popup_sync_hook_state(true, state.vv_popup_target);
                    vv_popup_move_near_target(state, hwnd);
                    InvalidateRect(hwnd, null(), 1);
                    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
                }
                return 0;
            }
            for row in 0..state.vv_popup_items.len() {
                let row_rc = vv_popup_row_rect(row);
                if y >= row_rc.top && y < row_rc.bottom {
                    PostMessageW(main_hwnd, WM_VV_SELECT, row, 0);
                    break;
                }
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, 0, lparam),
    }
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
    pub(crate) role: WindowRole,
    pub(crate) hwnd: HWND,
    pub(crate) search_hwnd: HWND,
    pub(crate) theme: Theme,
    pub(crate) icons: Icons,
    pub(crate) records: Vec<ClipItem>,
    pub(crate) phrases: Vec<ClipItem>,
    pub(crate) record_groups: Vec<ClipGroup>,
    pub(crate) phrase_groups: Vec<ClipGroup>,
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
    pub(crate) tray_icon_registered: bool,
    pub(crate) hotkey_registered: bool,
    pub(crate) hotkey_conflict_notified: bool,
    pub(crate) startup_recovery_ticks: u8,
    pub(crate) settings_hwnd: HWND,
    pub(crate) hover_scroll: bool,   // 鼠标是否在滚动条区域
    pub(crate) scroll_fade_alpha: u8, // 滚动条透明度 0-255
    pub(crate) scroll_fade_timer: bool, // 渐隐 timer 是否运行中
    pub(crate) hover_to_top: bool,
    pub(crate) down_to_top: bool,
    tab_loads: [TabLoadState; 2],
    payload_cache: ItemPayloadCache,
    pub(crate) vv_popup_visible: bool,
    pub(crate) vv_popup_pending_target: HWND,
    pub(crate) vv_popup_pending_retries: u8,
    pub(crate) vv_popup_target: HWND,
    pub(crate) vv_popup_replaces_ime: bool,
    pub(crate) vv_popup_group_id: i64,
    vv_popup_items: Vec<VvPopupEntry>,
    pub(crate) paste_target_override: HWND,
    pub(crate) paste_backspace_count: u8,
    pub(crate) hotkey_passthrough_active: bool,
    pub(crate) hotkey_passthrough_target: HWND,
    pub(crate) hotkey_passthrough_edit: HWND,
    pub(crate) main_window_noactivate: bool,
    pub(crate) edge_hidden: bool,
    pub(crate) edge_hidden_side: i32,
    pub(crate) edge_restore_x: i32,
    pub(crate) edge_restore_y: i32,
    pub(crate) edge_docked_left: i32,
    pub(crate) edge_docked_top: i32,
    pub(crate) edge_docked_right: i32,
    pub(crate) edge_docked_bottom: i32,
    pub(crate) cloud_sync_in_progress: bool,
    pub(crate) cloud_sync_next_due: Option<Instant>,
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
    fn delete_selected_rows(&mut self) {
        let ids = self.selected_db_ids();
        if ids.is_empty() {
            self.delete_selected();
            return;
        }
        for id in ids {
            let _ = db_delete_item(id);
            self.remove_cached_item(id);
        }
        self.clear_selection();
        self.invalidate_tab_query(self.tab_index, true);
        self.refilter();
        unsafe { sync_peer_windows_from_db(self.hwnd); }
    }

    fn toggle_pin_rows(&mut self) {
        let src = self.selected_source_indices();
        if src.is_empty() {
            self.toggle_pin_selected();
            return;
        }
        let make_pinned = src.iter().filter_map(|&i| self.active_items().get(i)).any(|it| !it.pinned);
        let mut invalidate_ids = Vec::new();
        for src_idx in src {
            if let Some(it) = self.active_items().get(src_idx) {
                if it.id > 0 {
                    let _ = db_update_item_pinned(it.id, make_pinned);
                    invalidate_ids.push(it.id);
                }
            }
        }
        for id in invalidate_ids {
            self.remove_cached_item(id);
        }
        self.clear_selection();
        self.invalidate_tab_query(self.tab_index, true);
        self.refilter();
        unsafe { sync_peer_windows_from_db(self.hwnd); }
    }

    fn selected_items_owned(&self) -> Vec<ClipItem> {
        self.selected_source_indices().into_iter().filter_map(|i| self.active_items().get(i).cloned()).collect()
    }

    fn clear_payload_cache(&mut self) {
        self.payload_cache.clear();
    }

    fn cache_full_item(&mut self, item: ClipItem) {
        self.payload_cache.put(item);
    }

    fn remove_cached_item(&mut self, id: i64) {
        self.payload_cache.remove(id);
    }

    fn load_item_full_cached(&mut self, id: i64) -> Option<ClipItem> {
        if id <= 0 {
            return None;
        }
        if let Some(item) = self.payload_cache.get(id) {
            return Some(item);
        }
        let item = db_load_item_full(id)?;
        self.payload_cache.put(item.clone());
        Some(item)
    }

    fn resolve_item_for_use(&mut self, item: &ClipItem) -> Option<ClipItem> {
        if item.id <= 0 {
            return Some(item.clone());
        }
        let payload_missing = match item.kind {
            ClipKind::Text | ClipKind::Phrase => item.text.is_none(),
            ClipKind::Files => item.file_paths.is_none() && item.text.is_none(),
            ClipKind::Image => item.image_bytes.is_none() && item.image_path.is_none(),
        };
        if payload_missing {
            self.load_item_full_cached(item.id)
        } else {
            Some(item.clone())
        }
    }

    fn current_item_for_use(&mut self) -> Option<ClipItem> {
        let item = self.current_item_owned()?;
        self.resolve_item_for_use(&item)
    }

    fn selected_items_for_use(&mut self) -> Vec<ClipItem> {
        let items = self.selected_items_owned();
        items.into_iter()
            .filter_map(|item| self.resolve_item_for_use(&item))
            .collect()
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

    fn add_clip_item(&mut self, mut item: ClipItem, signature: String) {
        if !signature.is_empty() && self.last_signature == signature {
            return;
        }
        self.last_signature = signature;
        item.id = db_insert_item(0, &item).unwrap_or(0);
        // 回填内存中的 created_at（DB 由 CURRENT_TIMESTAMP 自动填写，内存补齐以便时间分组标头正常工作）
        if item.created_at.is_empty() {
            item.created_at = now_utc_sqlite();
        }
        if item.id > 0 {
            self.cache_full_item(item.clone());
        }
        let summary = clip_item_to_summary(&item);
        let visible_query = self.load_state_for_tab(0).query.clone();
        if matches!(visible_query, Some(ref query) if query.group_id == 0 && query.search_text.trim().is_empty()) {
            self.records.insert(0, summary);
            if self.tab_index == 0 {
                self.list.apply_visible_len(self.records.len());
            }
        } else {
            self.invalidate_tab_query(0, self.tab_index == 0);
        }
        let max_items = self.settings.max_items; // 0 = 无限制；仅限制非置顶条目
        if max_items > 0 {
            db_prune_items(max_items);
            self.invalidate_tab_query(0, self.tab_index == 0);
        }
        if self.tab_index == 0 {
            self.sel_idx = 0;
        }
        self.refilter();
        unsafe { sync_peer_windows_from_db(self.hwnd); }
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

    fn quick_action_rect_slot(&self, visible_idx: i32, slot: i32) -> Option<RECT> {
        MAIN_UI_LAYOUT
            .quick_action_rect(visible_idx, self.filtered_indices.len(), self.scroll_y, slot)
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

    fn scroll_to_top_rect(&self) -> RECT {
        MAIN_UI_LAYOUT.scroll_to_top_button_rect().into()
    }

    fn delete_selected(&mut self) {
        if self.sel_idx < 0 {
            return;
        }
        if let Some(item) = self.current_item_owned() {
            if item.id > 0 {
                let _ = db_delete_item(item.id);
                self.remove_cached_item(item.id);
            }
            self.clear_selection();
            self.invalidate_tab_query(self.tab_index, true);
            self.refilter();
            unsafe { sync_peer_windows_from_db(self.hwnd); }
        }
    }

    fn toggle_pin_selected(&mut self) {
        if let Some(it) = self.current_item_owned() {
            if it.id > 0 {
                let _ = db_update_item_pinned(it.id, !it.pinned);
                self.remove_cached_item(it.id);
            }
            self.invalidate_tab_query(self.tab_index, true);
            self.refilter();
            unsafe { sync_peer_windows_from_db(self.hwnd); }
        }
    }

    fn selected_db_ids(&self) -> Vec<i64> {
        self.selected_source_indices()
            .into_iter()
            .filter_map(|i| self.active_items().get(i).map(|it| it.id))
            .filter(|id| *id > 0)
            .collect()
    }
}

unsafe fn refresh_settings_window_from_app(app: &mut AppState) {
    if app.settings_hwnd.is_null() || IsWindow(app.settings_hwnd) == 0 {
        return;
    }
    let st_ptr = GetWindowLongPtrW(app.settings_hwnd, GWLP_USERDATA) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        settings_apply_from_app(&mut *st_ptr);
        InvalidateRect(app.settings_hwnd, null(), 1);
    }
}

unsafe fn apply_loaded_settings(hwnd: HWND, state: &mut AppState) {
    let old_edge_hide = state.settings.edge_auto_hide;
    let mut loaded = load_settings();
    loaded.auto_start = apply_autostart(loaded.auto_start);
    state.settings = loaded;
    save_settings(&state.settings);
    schedule_cloud_sync(state, false);
    if state.role == WindowRole::Main {
        sync_main_tray_icon(hwnd, state);
        register_hotkey_for(hwnd, state);
        update_vv_mode_hook(hwnd, state.settings.vv_mode_enabled);
        position_main_window(hwnd, &state.settings, false);
    }
    if old_edge_hide && !state.settings.edge_auto_hide {
        restore_edge_hidden_window(hwnd, state);
    }
    reload_state_from_db(state);
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
    refresh_settings_window_from_app(state);
}

unsafe fn queue_cloud_sync(hwnd: HWND, state: &mut AppState, action: CloudSyncAction, auto_triggered: bool) {
    if state.cloud_sync_in_progress {
        return;
    }
    if state.settings.cloud_webdav_url.trim().is_empty() {
        state.settings.cloud_last_sync_status = "未配置 WebDAV 地址".to_string();
        save_settings(&state.settings);
        refresh_settings_window_from_app(state);
        if !auto_triggered {
            MessageBoxW(
                hwnd,
                to_wide("请先填写 WebDAV 地址。").as_ptr(),
                to_wide(cloud_sync_action_label(action)).as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
        }
        return;
    }

    if matches!(action, CloudSyncAction::SyncNow | CloudSyncAction::RestoreBackup) {
        close_db();
    }

    state.cloud_sync_in_progress = true;
    state.settings.cloud_last_sync_status = cloud_sync_running_text(auto_triggered).to_string();
    save_settings(&state.settings);
    refresh_settings_window_from_app(state);
    spawn_cloud_sync_job(hwnd, action, auto_triggered, state.settings.clone());
}

unsafe fn apply_ready_cloud_syncs(hwnd: HWND, state: &mut AppState) {
    let mut ready = VecDeque::new();
    if let Ok(mut queue) = cloud_sync_results().lock() {
        let mut pending = VecDeque::new();
        while let Some(result) = queue.pop_front() {
            if result.hwnd == hwnd as isize {
                ready.push_back(result);
            } else {
                pending.push_back(result);
            }
        }
        *queue = pending;
    }

    while let Some(ready_item) = ready.pop_front() {
        state.cloud_sync_in_progress = false;
        schedule_cloud_sync(state, false);
        match ready_item.result {
            Ok(outcome) => {
                state.settings.cloud_last_sync_status = outcome.status_text;
                save_settings(&state.settings);
                if outcome.reload_settings {
                    apply_loaded_settings(hwnd, state);
                } else if outcome.reload_data {
                    reload_state_from_db(state);
                    layout_children(hwnd);
                    InvalidateRect(hwnd, null(), 1);
                } else {
                    refresh_settings_window_from_app(state);
                    InvalidateRect(hwnd, null(), 1);
                }
                sync_peer_windows_from_settings(hwnd);
            }
            Err(err) => {
                state.settings.cloud_last_sync_status = format!("失败：{err}");
                save_settings(&state.settings);
                refresh_settings_window_from_app(state);
                sync_peer_windows_from_settings(hwnd);
                if !ready_item.auto_triggered {
                    MessageBoxW(
                        hwnd,
                        to_wide(&err).as_ptr(),
                        to_wide(cloud_sync_action_label(ready_item.action)).as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                }
            }
        }
    }
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
            let hk = hotkey_preview_text(&state.settings.hotkey_mod, &state.settings.hotkey_key)
                .replace(tr("当前设置：", "Current setting: "), "");
        unsafe {
            MessageBoxW(
                hwnd,
                to_wide(&format!(
                    "{} {} {}",
                    tr("快捷键", "Hotkey"),
                    hk,
                    tr("已被其他程序或系统占用，当前不会注册全局热键。请在设置-快捷键中改用其他组合。", "is already used by another app or the system. The global hotkey will not be registered. Please choose another combination in Settings > Hotkeys.")
                )).as_ptr(),
                to_wide(translate("快捷键冲突").as_ref()).as_ptr(),
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
    chk_silent_start: HWND,
    chk_tray_icon: HWND,
    chk_close_tray: HWND,
    chk_click_hide: HWND,
    chk_auto_hide_on_blur: HWND,
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
    record_groups_cache: Vec<ClipGroup>,
    phrase_groups_cache: Vec<ClipGroup>,
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
    cb_vv_source: HWND,
    cb_vv_group: HWND,
    vv_source_selected: usize,
    vv_group_selected: i64,
    group_view_tab: usize,
    btn_group_view_records: HWND,
    btn_group_view_phrases: HWND,
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
    btn_open_update: HWND,
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
    let mut class_buf = [0u16; 32];
    let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
    let class_name = if class_len > 0 {
        String::from_utf16_lossy(&class_buf[..class_len as usize])
    } else {
        String::new()
    };
    let text = if matches!(class_name.as_str(), "BUTTON" | "STATIC") {
        translate(s).into_owned()
    } else {
        s.to_string()
    };
    SetWindowTextW(hwnd, to_wide(&text).as_ptr());
}

fn settings_groups_cache_for_tab(st: &SettingsWndState, tab: usize) -> &Vec<ClipGroup> {
    if normalize_source_tab(tab) == 0 {
        &st.record_groups_cache
    } else {
        &st.phrase_groups_cache
    }
}

fn settings_groups_cache_for_tab_mut(st: &mut SettingsWndState, tab: usize) -> &mut Vec<ClipGroup> {
    if normalize_source_tab(tab) == 0 {
        &mut st.record_groups_cache
    } else {
        &mut st.phrase_groups_cache
    }
}

unsafe fn settings_group_current_filter_text(st: &SettingsWndState) -> String {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() { return tr("全部记录", "All Records").to_string(); }
    let app = &*pst;
    let view_tab = normalize_source_tab(st.group_view_tab);
    let gid = app.tab_group_filters.get(view_tab).copied().unwrap_or(0);
    if gid == 0 {
        return if view_tab == 0 {
            tr("全部记录", "All Records").to_string()
        } else {
            tr("全部短语", "All Phrases").to_string()
        };
    }
    app.groups_for_tab(view_tab)
        .iter()
        .find(|g| g.id == gid)
        .map(|g| g.name.clone())
        .unwrap_or_else(|| format!("{} #{}", tr("分组", "Group"), gid))
}

unsafe fn settings_sync_vv_source_display(st: &mut SettingsWndState) {
    st.vv_source_selected = normalize_source_tab(st.vv_source_selected);
    if !st.cb_vv_source.is_null() {
        settings_set_text(st.cb_vv_source, source_tab_label(st.vv_source_selected));
    }
}

unsafe fn settings_sync_vv_group_display(st: &mut SettingsWndState) {
    let source_tab = settings_vv_source_current(st);
    let selected = st.vv_group_selected;
    let exists = if selected > 0 {
        settings_groups_cache_for_tab(st, source_tab)
            .iter()
            .any(|g| g.id == selected)
    } else {
        true
    };
    if selected > 0 && !exists {
        st.vv_group_selected = 0;
    }
    if !st.cb_vv_group.is_null() {
        let groups = settings_groups_cache_for_tab(st, source_tab);
        settings_set_text(
            st.cb_vv_group,
            &group_name_for_display(groups, st.vv_group_selected, source_tab_all_label(source_tab)),
        );
    }
}

unsafe fn settings_sync_group_view_tabs(st: &SettingsWndState) {
    if !st.btn_group_view_records.is_null() {
        InvalidateRect(st.btn_group_view_records, null(), 1);
    }
    if !st.btn_group_view_phrases.is_null() {
        InvalidateRect(st.btn_group_view_phrases, null(), 1);
    }
}

unsafe fn settings_sync_group_overview(st: &mut SettingsWndState) {
    st.group_view_tab = normalize_source_tab(st.group_view_tab);
    let text = format!(
        "{}（{}）：{}",
            tr("当前分组", "Current Group"),
            source_tab_label(st.group_view_tab),
            settings_group_current_filter_text(st)
        );
    if !st.lb_group_current.is_null() {
        settings_set_text(st.lb_group_current, &text);
    }
    let pst = get_state_ptr(st.parent_hwnd);
    let gid = if pst.is_null() {
        0
    } else {
        (&*pst)
            .tab_group_filters
            .get(st.group_view_tab)
            .copied()
            .unwrap_or(0)
    };
    settings_groups_refresh_list(st, gid);
    settings_sync_group_view_tabs(st);
}

fn settings_vv_source_current(st: &SettingsWndState) -> usize {
    normalize_source_tab(st.vv_source_selected)
}

fn settings_group_view_current(st: &SettingsWndState) -> usize {
    normalize_source_tab(st.group_view_tab)
}

unsafe fn settings_vv_source_from_app(st: &mut SettingsWndState) {
    st.vv_source_selected = normalize_source_tab(st.draft.vv_source_tab);
}

unsafe fn settings_group_view_from_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    st.group_view_tab = if pst.is_null() {
        0
    } else {
        normalize_source_tab((&*pst).tab_index)
    };
}

unsafe fn settings_sync_group_page(st: &mut SettingsWndState) {
    st.record_groups_cache = db_load_groups(0);
    st.phrase_groups_cache = db_load_groups(1);
    settings_vv_source_from_app(st);
    settings_sync_vv_source_display(st);
    st.vv_group_selected = st.draft.vv_group_id;
    settings_sync_vv_group_display(st);
    settings_group_view_from_app(st);
    settings_sync_group_overview(st);
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
            settings_set_text(st.cb_engine, &search_engine_display(&s.search_engine));
            settings_set_text(st.ed_tpl, &s.search_template);
        }
        SettingsPage::Group => {
            settings_sync_group_page(st);
        }
        SettingsPage::Cloud => {
            let s = &st.draft;
            settings_set_text(st.cb_cloud_interval, &s.cloud_sync_interval);
            settings_set_text(st.ed_cloud_url, &s.cloud_webdav_url);
            settings_set_text(st.ed_cloud_user, &s.cloud_webdav_user);
            settings_set_text(st.ed_cloud_pass, &s.cloud_webdav_pass);
            settings_set_text(st.ed_cloud_dir, &s.cloud_remote_dir);
            settings_set_text(
                st.lb_cloud_status,
                &format!(
                    "{}{}",
                    tr("上次同步：", "Last sync: "),
                    localized_cloud_status_text(&s.cloud_last_sync_status)
                ),
            );
        }
        SettingsPage::About => {}
    }
    settings_invalidate_page_ctrls(st.parent_hwnd, st, page);
}

fn localized_cloud_status_text(status: &str) -> String {
    let trimmed = status.trim();
    if trimmed.is_empty() {
        return tr("未同步", "Not synced").to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("失败：") {
        return format!("{}{}", tr("失败：", "Failed: "), rest);
    }
    translate(trimmed).into_owned()
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
    let label_text = translate(text);
    let label = CreateWindowExW(
        0,
        to_wide("STATIC").as_ptr(),
        to_wide(label_text.as_ref()).as_ptr(),
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

        let flags = SWP_NOZORDER | SWP_NOACTIVATE
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
    st.vv_source_selected = normalize_source_tab(st.draft.vv_source_tab);
    st.vv_group_selected = st.draft.vv_group_id;
    st.group_view_tab = normalize_source_tab(app.tab_index);
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
    st.draft.vv_source_tab = settings_vv_source_current(st);
    let vv_groups = settings_groups_cache_for_tab(st, st.draft.vv_source_tab);
    st.draft.vv_group_id = if st.vv_group_selected > 0 && vv_groups.iter().any(|g| g.id == st.vv_group_selected) {
        st.vv_group_selected
    } else {
        0
    };
    if !st.cb_cloud_interval.is_null() {
        st.draft.cloud_sync_interval = {
            let label = get_window_text(st.cb_cloud_interval);
            if label.trim().is_empty() { "1小时".to_string() } else { label }
        };
    }
    if !st.ed_cloud_url.is_null() {
        st.draft.cloud_webdav_url = get_window_text(st.ed_cloud_url);
    }
    if !st.ed_cloud_user.is_null() {
        st.draft.cloud_webdav_user = get_window_text(st.ed_cloud_user);
    }
    if !st.ed_cloud_pass.is_null() {
        st.draft.cloud_webdav_pass = get_window_text(st.ed_cloud_pass);
    }
    if !st.ed_cloud_dir.is_null() {
        st.draft.cloud_remote_dir = {
            let dir = get_window_text(st.ed_cloud_dir);
            if dir.trim().is_empty() { "ZSClip".to_string() } else { dir }
        };
    }
    let app = &mut *pst;
    let grouping_old = app.settings.grouping_enabled;
    let autostart_old = app.settings.auto_start;
    let tray_icon_old = app.settings.tray_icon_enabled;
    let hotkey_old = format!("{}+{}+{}", app.settings.hotkey_enabled, app.settings.hotkey_mod, app.settings.hotkey_key);
    let edge_hide_old = app.settings.edge_auto_hide;
    let vv_mode_old = app.settings.vv_mode_enabled;
    app.settings = st.draft.clone();
    if !app.settings.grouping_enabled {
        app.current_group_filter = 0;
        app.tab_group_filters = [0, 0];
    }
    save_settings(&app.settings);
    // 开机自启：同步写注册表
    if autostart_old != app.settings.auto_start {
        app.settings.auto_start = apply_autostart(app.settings.auto_start);
        st.draft.auto_start = app.settings.auto_start;
        save_settings(&app.settings);
    }
    if tray_icon_old != app.settings.tray_icon_enabled {
        let main_hwnd = main_window_hwnd();
        if !main_hwnd.is_null() {
            sync_main_tray_icon(main_hwnd, app);
        }
    }
    if grouping_old != app.settings.grouping_enabled {
        app.clear_selection();
    }
    let hotkey_new = format!("{}+{}+{}", app.settings.hotkey_enabled, app.settings.hotkey_mod, app.settings.hotkey_key);
    if hotkey_old != hotkey_new {
        register_hotkey_for(st.parent_hwnd, app);
    }
    if vv_mode_old != app.settings.vv_mode_enabled {
        update_vv_mode_hook(st.parent_hwnd, app.settings.vv_mode_enabled);
        if !app.settings.vv_mode_enabled {
            vv_popup_hide(st.parent_hwnd, app);
        }
    }
    schedule_cloud_sync(app, false);
    // 保存后按新的上限清理 DB 中多余条目（0=无限制不清理）
    let new_max = app.settings.max_items;
    if new_max > 0 {
        db_prune_items(new_max);
        // 同步刷新内存列表
        reload_state_from_db(app);
    }
    if edge_hide_old && !app.settings.edge_auto_hide {
        restore_edge_hidden_window(st.parent_hwnd, app);
    }
    app.refilter();
    sync_peer_windows_from_settings(st.parent_hwnd);
    InvalidateRect(st.parent_hwnd, null(), 1);
}

unsafe fn settings_toggle_get(st: &SettingsWndState, cid: isize) -> bool {
    match cid {
        IDC_SET_AUTOSTART    => st.draft.auto_start,
        IDC_SET_SILENTSTART  => st.draft.silent_start,
        IDC_SET_TRAYICON     => st.draft.tray_icon_enabled,
        IDC_SET_CLOSETRAY    => st.draft.close_without_exit,
        IDC_SET_CLICK_HIDE   => st.draft.click_hide,
        IDC_SET_AUTOHIDE_BLUR => st.draft.auto_hide_on_blur,
        IDC_SET_EDGEHIDE     => st.draft.edge_auto_hide,
        IDC_SET_HOVERPREVIEW => st.draft.hover_preview,
        IDC_SET_VV_MODE => st.draft.vv_mode_enabled,
        IDC_SET_IMAGE_PREVIEW => st.draft.image_preview_enabled,
        IDC_SET_QUICK_DELETE => st.draft.quick_delete_button,
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
        IDC_SET_SILENTSTART  => st.draft.silent_start = !st.draft.silent_start,
        IDC_SET_TRAYICON     => st.draft.tray_icon_enabled = !st.draft.tray_icon_enabled,
        IDC_SET_CLOSETRAY    => st.draft.close_without_exit = !st.draft.close_without_exit,
        IDC_SET_CLICK_HIDE   => st.draft.click_hide = !st.draft.click_hide,
        IDC_SET_AUTOHIDE_BLUR => st.draft.auto_hide_on_blur = !st.draft.auto_hide_on_blur,
        IDC_SET_EDGEHIDE     => st.draft.edge_auto_hide = !st.draft.edge_auto_hide,
        IDC_SET_HOVERPREVIEW => st.draft.hover_preview = !st.draft.hover_preview,
        IDC_SET_VV_MODE => st.draft.vv_mode_enabled = !st.draft.vv_mode_enabled,
        IDC_SET_IMAGE_PREVIEW => st.draft.image_preview_enabled = !st.draft.image_preview_enabled,
        IDC_SET_QUICK_DELETE => st.draft.quick_delete_button = !st.draft.quick_delete_button,
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
    st.chk_silent_start = settings_create_toggle(hwnd, st, "静默启动（打开默认不显示）", IDC_SET_SILENTSTART, sec0.left(), sec0.row_y(1), sec0.full_w(), ui_font);
    st.chk_tray_icon = settings_create_toggle(hwnd, st, "右下角图标开启/关闭", IDC_SET_TRAYICON, sec0.left(), sec0.row_y(2), sec0.full_w(), ui_font);
    st.chk_close_tray = settings_create_toggle(hwnd, st, "关闭不退出（托盘驻留）", IDC_SET_CLOSETRAY, sec0.left(), sec0.row_y(3), sec0.full_w(), ui_font);
    st.chk_click_hide = settings_create_toggle(hwnd, st, "单击后隐藏主窗口", IDC_SET_CLICK_HIDE, sec0.left(), sec0.row_y(4), sec0.full_w(), ui_font);
    st.chk_auto_hide_on_blur = settings_create_toggle(hwnd, st, "呼出后点击外部自动隐藏", IDC_SET_AUTOHIDE_BLUR, sec0.left(), sec0.row_y(5), sec0.full_w(), ui_font);
    st.chk_edge_hide = settings_create_toggle(hwnd, st, "贴边自动隐藏", IDC_SET_EDGEHIDE, sec0.left(), sec0.row_y(6), sec0.full_w(), ui_font);
    st.chk_hover_preview = settings_create_toggle(hwnd, st, "悬停预览", IDC_SET_HOVERPREVIEW, sec0.left(), sec0.row_y(7), sec0.full_w(), ui_font);
    let _ = settings_create_toggle(hwnd, st, "VV 模式", IDC_SET_VV_MODE, sec0.left(), sec0.row_y(8), sec0.full_w(), ui_font);
    let _ = settings_create_toggle(hwnd, st, "显示图片记录", IDC_SET_IMAGE_PREVIEW, sec0.left(), sec0.row_y(9), sec0.full_w(), ui_font);
    let _ = settings_create_toggle(hwnd, st, "快速删除按钮", IDC_SET_QUICK_DELETE, sec0.left(), sec0.row_y(10), sec0.full_w(), ui_font);

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
    let category = source_tab_category(settings_group_view_current(st));
    SendMessageW(st.lb_groups, LB_RESETCONTENT, 0, 0);
    *settings_groups_cache_for_tab_mut(st, settings_group_view_current(st)) = db_load_groups(category);
    let groups = settings_groups_cache_for_tab(st, settings_group_view_current(st));
    let mut sel_idx: i32 = -1;
    for (i, g) in groups.iter().enumerate() {
        SendMessageW(st.lb_groups, LB_ADDSTRING, 0, to_wide(&g.name).as_ptr() as LPARAM);
        if g.id == select_gid {
            sel_idx = i as i32;
        }
    }
    if sel_idx < 0 && !groups.is_empty() {
        sel_idx = 0;
    }
    if sel_idx >= 0 {
        SendMessageW(st.lb_groups, LB_SETCURSEL, sel_idx as WPARAM, 0);
    }
    settings_sync_vv_group_display(st);
}

unsafe fn settings_groups_selected(st: &SettingsWndState) -> Option<(usize, ClipGroup)> {
    if st.lb_groups.is_null() { return None; }
    let row = SendMessageW(st.lb_groups, LB_GETCURSEL, 0, 0) as i32;
    if row < 0 { return None; }
    settings_groups_cache_for_tab(st, settings_group_view_current(st))
        .get(row as usize)
        .cloned()
        .map(|g| (row as usize, g))
}

unsafe fn settings_groups_sync_name(_st: &mut SettingsWndState) {
}

unsafe fn settings_groups_move(st: &mut SettingsWndState, step: i32) {
    let Some((idx, _)) = settings_groups_selected(st) else { return; };
    let tab = settings_group_view_current(st);
    let category = source_tab_category(tab);
    let groups = settings_groups_cache_for_tab(st, tab);
    let new_idx = idx as i32 + step;
    if new_idx < 0 || new_idx >= groups.len() as i32 {
        return;
    }
    let mut ids: Vec<i64> = groups.iter().map(|g| g.id).collect();
    let item = ids.remove(idx);
    ids.insert(new_idx as usize, item);
    if db_set_groups_order(category, &ids).is_ok() {
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
            let btn_cancel = CreateWindowExW(0, to_wide("BUTTON").as_ptr(), to_wide(translate("取消").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                148, 132, 88, 30, hwnd, IDC_INPUT_CANCEL as _, hmod, null());
            SendMessageW(btn_cancel, WM_SETFONT, d.ui_font as usize, 1);

            // 保存按钮（右）
            let btn_ok = CreateWindowExW(0, to_wide("BUTTON").as_ptr(), to_wide(translate("保存").as_ref()).as_ptr(),
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
        title_w: translate(title).encode_utf16().chain(std::iter::once(0)).collect(),
        label_w: translate(label).encode_utf16().chain(std::iter::once(0)).collect(),
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

    let title_w = to_wide(translate(title).as_ref());
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
            let btn_cancel = CreateWindowExW(0, to_wide("BUTTON").as_ptr(), to_wide(translate("取消").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                w - 210, h - 44, 90, 30, hwnd, IDC_EDIT_CANCEL as _, hmod, null());
            SendMessageW(btn_cancel, WM_SETFONT, d.btn_font as usize, 1);

            // 保存按钮
            let btn_save = CreateWindowExW(0, to_wide("BUTTON").as_ptr(), to_wide(translate("保存").as_ref()).as_ptr(),
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
    let (_ai_lbl, ai_btn) = b.toggle_row(st, "AI 文本清洗", 7101, sec1.left(), sec1.row_y(0), sec1.full_w());
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
    let sec0 = SettingsFormSectionLayout::new(page, 0, 104);
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

    let lbl_vv_source = settings_create_label(hwnd, "VV 来源：", sec0.left(), sec0.label_y(1, 24), sec0.label_w, 24, ui_font);
    push(st, lbl_vv_source);
    st.cb_vv_source = settings_create_dropdown_btn(hwnd, "复制记录", IDC_SET_VV_SOURCE, sec0.field_x(), sec0.row_y(1), 180, ui_font);
    if !st.cb_vv_source.is_null() {
        settings_page_push_ctrl(st, page, st.cb_vv_source);
        st.ownerdraw_ctrls.push(st.cb_vv_source);
    }

    let lbl_vv_group = settings_create_label(hwnd, "VV 默认分组：", sec0.left(), sec0.label_y(2, 24), sec0.label_w, 24, ui_font);
    push(st, lbl_vv_group);
    st.cb_vv_group = settings_create_dropdown_btn(hwnd, "全部记录", IDC_SET_VV_GROUP, sec0.field_x(), sec0.row_y(2), 220, ui_font);
    if !st.cb_vv_group.is_null() {
        settings_page_push_ctrl(st, page, st.cb_vv_group);
        st.ownerdraw_ctrls.push(st.cb_vv_group);
    }

    let tab_w = 118;
    st.btn_group_view_records = settings_create_small_btn(hwnd, "复制记录", IDC_SET_GROUP_VIEW_RECORDS, sec1.left(), sec1.row_y(0), tab_w, ui_font);
    st.btn_group_view_phrases = settings_create_small_btn(hwnd, "常用短语", IDC_SET_GROUP_VIEW_PHRASES, sec1.left() + tab_w + 10, sec1.row_y(0), tab_w, ui_font);
    for &hh in &[st.btn_group_view_records, st.btn_group_view_phrases] {
        if !hh.is_null() {
            settings_page_push_ctrl(st, page, hh);
            st.ownerdraw_ctrls.push(hh);
        }
    }

    st.lb_group_current = settings_create_label(hwnd, "当前分组：全部记录", sec1.left(), sec1.row_y(1), sec1.full_w(), 24, ui_font);
    push(st, st.lb_group_current);

    let lbl3 = settings_create_label(hwnd, "分组列表：", sec1.left(), sec1.row_y(2), 220, 22, ui_font);
    push(st, lbl3);

    st.lb_groups = settings_create_listbox(hwnd, IDC_SET_GROUP_LIST, sec1.left(), sec1.row_y(3), sec1.full_w(), 170, ui_font);
    if !st.lb_groups.is_null() { settings_page_push_ctrl(st, page, st.lb_groups); }

    let btn_y = sec1.row_y(3) + 186;
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
    let update_state = update_check_state_snapshot();
    let lines = [
        format!("{}{}", tr("版本：", "Version: "), env!("CARGO_PKG_VERSION")),
        "设置界面现在统一使用同一套 section/form 布局。".to_string(),
        "新增设置项时可以直接复用卡片、字段列、按钮行和统一间距。".to_string(),
    ];
    let mut y = sec.row_y(0);
    for line in lines.iter() {
        let (_, h) = b.label_auto(st, line, sec.left(), y, sec.full_w(), 24);
        y += h + 10;
    }

    let (_, label_h) = b.label_auto(st, "开源地址：", sec.left(), y, 72, 24);
    let link = b.button(
        st,
        open_source_url_display(),
        IDC_SET_OPEN_SOURCE,
        sec.left() + 64,
        y - 4,
        sec.full_w() - 64,
    );
    if !link.is_null() {
        st.ownerdraw_ctrls.push(link);
    }
    y += label_h.max(32) + 10;

    let update_text = if update_state.checking {
        tr("更新检查中…", "Checking for updates...").to_string()
    } else if !update_state.started {
        tr("点击下方按钮后再检查更新。", "Click the button below to check for updates.").to_string()
    } else if update_state.available {
        format!(
            "{} {}",
            tr("发现新版本：", "New version available: "),
            if update_state.latest_tag.trim().is_empty() {
                "latest".to_string()
            } else {
                update_state.latest_tag.clone()
            }
        )
    } else if !update_state.error.trim().is_empty() {
        format!("{} {}", tr("更新检查失败：", "Update check failed: "), update_state.error)
    } else {
        tr("当前已经是最新版本。", "You are already on the latest version.").to_string()
    };
    let (_, update_h) = b.label_auto(st, &update_text, sec.left(), y, sec.full_w(), 24);
    y += update_h + 8;
    st.btn_open_update = b.button(
        st,
        if update_state.checking {
            tr("检测中…", "Checking...")
        } else if update_state.available {
            tr("打开新版本", "Open new version")
        } else if update_state.started {
            tr("再次检查", "Check again")
        } else {
            tr("检查更新", "Check for updates")
        },
        IDC_SET_OPEN_UPDATE,
        sec.left(),
        y,
        180,
    );
    if !st.btn_open_update.is_null() {
        st.ownerdraw_ctrls.push(st.btn_open_update);
    }
    y += 42;

    for line in [
        format!("{}{}", tr("数据目录：", "Data directory: "), data_dir().to_string_lossy()),
        format!("{}{}", tr("数据库：", "Database: "), db_file().to_string_lossy()),
    ] {
        let (_, h) = b.label_auto(st, &line, sec.left(), y, sec.full_w(), 24);
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

    if cid == IDC_SET_AUTOSTART || cid == IDC_SET_SILENTSTART || cid == IDC_SET_TRAYICON || cid == IDC_SET_CLOSETRAY
        || cid == IDC_SET_CLICK_HIDE || cid == IDC_SET_AUTOHIDE_BLUR || cid == IDC_SET_EDGEHIDE
        || cid == IDC_SET_HOVERPREVIEW || cid == IDC_SET_VV_MODE || cid == IDC_SET_IMAGE_PREVIEW
        || cid == IDC_SET_QUICK_DELETE || cid == IDC_SET_GROUP_ENABLE
        || cid == IDC_SET_CLOUD_ENABLE
        || cid == 6101 || cid == 7102 || cid == 7101 || cid == 7103
    {
        let checked = settings_toggle_get(st, cid);
        draw_settings_toggle_component(hdc as _, &rc, hover, checked, th);
        return;
    }

    if cid == IDC_SET_OPEN_SOURCE {
        let text_color = if open_source_url().trim().is_empty() {
            th.text_muted
        } else if pressed {
            rgb(22, 78, 180)
        } else if hover {
            rgb(14, 111, 214)
        } else {
            rgb(24, 92, 189)
        };
        let font = CreateFontW(
            -14,
            0,
            0,
            0,
            400,
            0,
            1,
            0,
            1,
            0,
            0,
            5,
            0,
            to_wide("Segoe UI").as_ptr(),
        ) as *mut core::ffi::c_void;
        let old_font = if !font.is_null() {
            SelectObject(hdc, font)
        } else {
            null_mut()
        };
        SetBkMode(hdc, 1);
        SetTextColor(hdc, text_color);
        let mut text_rc = rc;
        text_rc.left += if pressed { 5 } else { 4 };
        text_rc.top += if pressed { 1 } else { 0 };
        let text_w = to_wide(&text);
        DrawTextW(
            hdc,
            text_w.as_ptr(),
            -1,
            &mut text_rc,
            DT_LEFT | DT_VCENTER | DT_SINGLELINE,
        );
        if !font.is_null() {
            SelectObject(hdc, old_font);
            DeleteObject(font as _);
        }
        return;
    }

    let kind = if cid == IDC_SET_MAX || cid == IDC_SET_POSMODE || cid == IDC_SET_CLOUD_INTERVAL || cid == IDC_SET_VV_GROUP || cid == IDC_SET_VV_SOURCE || cid == 6102 || cid == 6103 || cid == 7201 {
        SettingsComponentKind::Dropdown
    } else if (cid == IDC_SET_GROUP_VIEW_RECORDS && settings_group_view_current(st) == 0)
        || (cid == IDC_SET_GROUP_VIEW_PHRASES && settings_group_view_current(st) == 1)
    {
        SettingsComponentKind::AccentButton
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
                chk_silent_start: null_mut(),
                chk_tray_icon: null_mut(),
                chk_close_tray: null_mut(),
                chk_click_hide: null_mut(),
                chk_auto_hide_on_blur: null_mut(),
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
                record_groups_cache: Vec::new(),
                phrase_groups_cache: Vec::new(),
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
                cb_vv_source: null_mut(),
                cb_vv_group: null_mut(),
                vv_source_selected: 0,
                vv_group_selected: 0,
                group_view_tab: 0,
                btn_group_view_records: null_mut(),
                btn_group_view_phrases: null_mut(),
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
                btn_open_update: null_mut(),
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
                        let old_engine = get_window_text(st.cb_engine);
                        let old_key = search_engine_key_from_display(&old_engine);
                        let old_tpl = search_engine_template(old_key).to_string();
                        let current_tpl = get_window_text(st.ed_tpl);
                        settings_set_text(st.cb_engine, label);
                        if current_tpl.trim().is_empty() || current_tpl == old_tpl {
                            settings_set_text(st.ed_tpl, tpl);
                        }
                        InvalidateRect(st.cb_engine, null(), 1);
                    }
                }
                IDC_SET_VV_SOURCE => {
                    st.vv_source_selected = normalize_source_tab(idx);
                    settings_sync_vv_source_display(st);
                    settings_sync_vv_group_display(st);
                    InvalidateRect(st.cb_vv_source, null(), 1);
                    InvalidateRect(st.cb_vv_group, null(), 1);
                }
                IDC_SET_VV_GROUP => {
                    let vv_source = settings_vv_source_current(st);
                    let groups = settings_groups_cache_for_tab(st, vv_source);
                    if idx == 0 {
                        st.vv_group_selected = 0;
                    } else if let Some(group) = groups.get(idx - 1) {
                        st.vv_group_selected = group.id;
                    }
                    settings_sync_vv_group_display(st);
                    InvalidateRect(st.cb_vv_group, null(), 1);
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
            let bg_fill = if dis.CtlID as isize == IDC_SET_AUTOSTART || dis.CtlID as isize == IDC_SET_SILENTSTART || dis.CtlID as isize == IDC_SET_TRAYICON || dis.CtlID as isize == IDC_SET_CLOSETRAY || dis.CtlID as isize == IDC_SET_CLICK_HIDE || dis.CtlID as isize == IDC_SET_AUTOHIDE_BLUR || dis.CtlID as isize == IDC_SET_EDGEHIDE || dis.CtlID as isize == IDC_SET_HOVERPREVIEW || dis.CtlID as isize == IDC_SET_VV_MODE || dis.CtlID as isize == IDC_SET_IMAGE_PREVIEW || dis.CtlID as isize == IDC_SET_QUICK_DELETE || dis.CtlID as isize == IDC_SET_GROUP_ENABLE || dis.CtlID as isize == IDC_SET_CLOUD_ENABLE || dis.CtlID as isize == IDC_SET_OPEN_SOURCE || dis.CtlID as isize == IDC_SET_OPEN_UPDATE || dis.CtlID as isize == IDC_SET_MAX || dis.CtlID as isize == IDC_SET_POSMODE || dis.CtlID as isize == IDC_SET_CLOUD_INTERVAL || dis.CtlID as isize == IDC_SET_VV_SOURCE || dis.CtlID as isize == IDC_SET_VV_GROUP || dis.CtlID as isize == 6101 || dis.CtlID as isize == 6102 || dis.CtlID as isize == 6103 || dis.CtlID as isize == 7102 || dis.CtlID as isize == 7101 || dis.CtlID as isize == 7103 || dis.CtlID as isize == 7201 { th.surface } else { th.bg };
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
                    IDC_SET_AUTOSTART | IDC_SET_SILENTSTART | IDC_SET_TRAYICON | IDC_SET_CLOSETRAY | IDC_SET_CLICK_HIDE | IDC_SET_AUTOHIDE_BLUR | IDC_SET_EDGEHIDE | IDC_SET_HOVERPREVIEW | IDC_SET_VV_MODE | IDC_SET_IMAGE_PREVIEW | IDC_SET_QUICK_DELETE | IDC_SET_GROUP_ENABLE | IDC_SET_CLOUD_ENABLE | 6101 | 7102 | 7101 | 7103 => {
                    settings_toggle_flip(st, cmd);
                    let sender = lparam as HWND;
                    if !sender.is_null() { InvalidateRect(sender, null(), 1); }
                }
                IDC_SET_GROUP_ADD => {
                    if let Some(name) = input_name_dialog(hwnd, "新建分组", "请输入分组名称：", "新分组") {
                        let category = source_tab_category(settings_group_view_current(st));
                        match db_create_named_group(category, &name) {
                            Ok(group) => {
                                settings_groups_refresh_list(st, group.id);
                                let pst = get_state_ptr(st.parent_hwnd);
                                if !pst.is_null() { reload_state_from_db(&mut *pst); InvalidateRect(st.parent_hwnd, null(), 1); }
                            }
                            Err(e) => {
            MessageBoxW(
                hwnd,
                to_wide(&format!("{}: {}", tr("新建分组失败", "Failed to create group"), e)).as_ptr(),
                to_wide(translate("分组").as_ref()).as_ptr(),
                MB_OK | MB_ICONERROR,
            );
                            }
                        }
                    }
                }
                IDC_SET_GROUP_RENAME => {
                    if let Some((_, g)) = settings_groups_selected(st) {
                        if let Some(new_name) = input_name_dialog(hwnd, "重命名分组", "请输入新名称：", &g.name) {
                            if let Err(e) = db_rename_group(g.category, g.id, &new_name) {
                MessageBoxW(
                    hwnd,
                    to_wide(&format!("{}: {}", tr("重命名失败", "Rename failed"), e)).as_ptr(),
                    to_wide(translate("分组").as_ref()).as_ptr(),
                    MB_OK | MB_ICONERROR,
                );
                            } else {
                                settings_groups_refresh_list(st, g.id);
                                let pst = get_state_ptr(st.parent_hwnd);
                                if !pst.is_null() { reload_state_from_db(&mut *pst); InvalidateRect(st.parent_hwnd, null(), 1); }
                            }
                        }
                    } else {
            MessageBoxW(
                hwnd,
                to_wide(translate("请先选择一个分组。").as_ref()).as_ptr(),
                to_wide(translate("分组").as_ref()).as_ptr(),
                MB_OK | MB_ICONINFORMATION,
            );
                    }
                }
                IDC_SET_GROUP_DELETE => {
                    if let Some((_, g)) = settings_groups_selected(st) {
            let ask = format!(
                "{} \"{}\"?\n{}",
                tr("确认删除分组", "Delete group"),
                g.name,
                tr("不会删除记录，只会清空这些记录的分组。", "Records will be kept. Only their group assignment will be cleared.")
            );
            if MessageBoxW(hwnd, to_wide(&ask).as_ptr(), to_wide(translate("分组").as_ref()).as_ptr(), MB_YESNO | MB_ICONQUESTION) == IDYES {
                            if let Err(e) = db_delete_group(g.id) {
                    MessageBoxW(
                        hwnd,
                        to_wide(&format!("{}: {}", tr("删除分组失败", "Delete group failed"), e)).as_ptr(),
                        to_wide(translate("分组").as_ref()).as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
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
                IDC_SET_GROUP_VIEW_RECORDS => {
                    st.group_view_tab = 0;
                    settings_sync_group_overview(st);
                }
                IDC_SET_GROUP_VIEW_PHRASES => {
                    st.group_view_tab = 1;
                    settings_sync_group_overview(st);
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
                IDC_SET_VV_SOURCE => {
                    if !st.dropdown_popup.is_null() { if IsWindow(st.dropdown_popup) != 0 { DestroyWindow(st.dropdown_popup); } st.dropdown_popup = null_mut(); }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_vv_source, &mut rc);
                    let current = settings_vv_source_current(st);
                    st.dropdown_popup = show_settings_dropdown_popup(hwnd, IDC_SET_VV_SOURCE, &rc, &["复制记录", "常用短语"], current, 200);
                }
                IDC_SET_VV_GROUP => {
                    if !st.dropdown_popup.is_null() { if IsWindow(st.dropdown_popup) != 0 { DestroyWindow(st.dropdown_popup); } st.dropdown_popup = null_mut(); }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_vv_group, &mut rc);
                    let vv_source = settings_vv_source_current(st);
                    let groups = settings_groups_cache_for_tab(st, vv_source);
                    let labels_owned: Vec<String> = std::iter::once(source_tab_all_label(vv_source).to_string())
                        .chain(groups.iter().map(|g| g.name.clone()))
                        .collect();
                    let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
                    let current = if st.vv_group_selected == 0 {
                        0
                    } else {
                        groups
                            .iter()
                            .position(|g| g.id == st.vv_group_selected)
                            .map(|idx| idx + 1)
                            .unwrap_or(0)
                    };
                    st.dropdown_popup = show_settings_dropdown_popup(hwnd, IDC_SET_VV_GROUP, &rc, &labels, current, 260);
                }
                7203 => {
                    let key = search_engine_key_from_display(&get_window_text(st.cb_engine));
                    settings_set_text(st.ed_tpl, search_engine_template(key));
                }
                IDC_SET_PLUGIN_MAILMERGE => {
                    launch_mail_merge_window(hwnd);
                }
                IDC_SET_OPEN_SOURCE => {
                    if open_source_url().trim().is_empty() {
                        MessageBoxW(
                            hwnd,
                            to_wide(translate("当前还没有配置开源地址，请先在 Cargo.toml 的 package.repository 中填写。").as_ref()).as_ptr(),
                            to_wide(translate("开源地址").as_ref()).as_ptr(),
                            MB_OK | MB_ICONINFORMATION,
                        );
                    } else {
                        open_path_with_shell(open_source_url());
                    }
                }
                IDC_SET_OPEN_UPDATE => {
                    let update_state = update_check_state_snapshot();
                    if update_state.checking {
                        return 0;
                    }
                    if update_state.available {
                        let url = update_check_latest_url_or_default();
                        open_path_with_shell(&url);
                    } else {
                        start_update_check(|| unsafe {
                            notify_update_state_changed();
                        });
                        InvalidateRect(hwnd, null(), 1);
                    }
                }
                6111 => {
                    if let Err(e) = toggle_disabled_hotkey_char('V', true) {
                    MessageBoxW(
                        hwnd,
                        to_wide(&format!("{}: {}", tr("屏蔽 Win+V 失败", "Disable Win+V failed"), e)).as_ptr(),
                        to_wide(translate("系统剪贴板历史").as_ref()).as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                    }
                }
                6112 => {
                    if let Err(e) = toggle_disabled_hotkey_char('V', false) {
                    MessageBoxW(
                        hwnd,
                        to_wide(&format!("{}: {}", tr("恢复 Win+V 失败", "Restore Win+V failed"), e)).as_ptr(),
                        to_wide(translate("系统剪贴板历史").as_ref()).as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                    }
                }
                6113 => {
                    if let Err(e) = restart_explorer_shell() {
                    MessageBoxW(
                        hwnd,
                        to_wide(&format!("{}: {}", tr("重启资源管理器失败", "Restart Explorer failed"), e)).as_ptr(),
                        to_wide(translate("系统剪贴板历史").as_ref()).as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                    }
                }
                IDC_SET_CLOUD_SYNC_NOW | IDC_SET_CLOUD_UPLOAD_CFG | IDC_SET_CLOUD_APPLY_CFG | IDC_SET_CLOUD_RESTORE_BACKUP => {
                    settings_collect_to_app(st);
                    let action = match cmd {
                        IDC_SET_CLOUD_SYNC_NOW => CloudSyncAction::SyncNow,
                        IDC_SET_CLOUD_UPLOAD_CFG => CloudSyncAction::UploadConfig,
                        IDC_SET_CLOUD_APPLY_CFG => CloudSyncAction::ApplyRemoteConfig,
                        _ => CloudSyncAction::RestoreBackup,
                    };
                    let pst = get_state_ptr(st.parent_hwnd);
                    if !pst.is_null() {
                        queue_cloud_sync(st.parent_hwnd, &mut *pst, action, false);
                        settings_apply_from_app(st);
                    }
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
                    if i == SettingsPage::About.index()
                        && update_check_available()
                    {
                        let item_rc = settings_nav_item_rect(i);
                        let dot = RECT {
                            left: item_rc.right - 22,
                            top: item_rc.top + 14,
                            right: item_rc.right - 12,
                            bottom: item_rc.top + 24,
                        };
                        draw_round_fill(memdc as _, &dot, rgb(228, 60, 60), 5);
                    }
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
    let owner_hwnd = {
        let main = main_window_hwnd();
        if !main.is_null() { main } else { hwnd }
    };
    let pst = get_state_ptr(owner_hwnd);
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
        owner_hwnd,
        null_mut(),
        hinstance,
        owner_hwnd as _,
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

unsafe fn refresh_window_state(hwnd: HWND, reload_settings: bool) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    if reload_settings {
        state.settings = load_settings();
        state.settings.auto_start = is_autostart_enabled();
        schedule_cloud_sync(state, false);
        if state.role == WindowRole::Main {
            sync_main_tray_icon(hwnd, state);
        }
    }
    reload_state_from_db(state);
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
}

unsafe fn sync_peer_windows_from_db(source_hwnd: HWND) {
    for target in window_host_hwnds() {
        if target.is_null() || target == source_hwnd || IsWindow(target) == 0 {
            continue;
        }
        let ptr = get_state_ptr(target);
        if !ptr.is_null() && (*ptr).role == WindowRole::Quick && IsWindowVisible(target) == 0 {
            continue;
        }
        refresh_window_state(target, false);
    }
}

unsafe fn sync_peer_windows_from_settings(source_hwnd: HWND) {
    for target in window_host_hwnds() {
        if target.is_null() || target == source_hwnd || IsWindow(target) == 0 {
            continue;
        }
        let ptr = get_state_ptr(target);
        if !ptr.is_null() && (*ptr).role == WindowRole::Quick && IsWindowVisible(target) == 0 {
            continue;
        }
        refresh_window_state(target, true);
    }
}

pub(crate) unsafe fn refresh_window_for_show(hwnd: HWND) {
    refresh_window_state(hwnd, true);
}

pub fn run() -> AppResult<()> {
    let boot_settings = load_settings();
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
            use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, PostMessageW, ShowWindow, SetForegroundWindow, SW_RESTORE};
            let cls = to_wide(WindowRole::Main.class_name());
            let hwnd = FindWindowW(cls.as_ptr(), core::ptr::null());
            if !hwnd.is_null() {
                if !boot_settings.tray_icon_enabled {
                    PostMessageW(hwnd, WM_CLOSE, 0, 0);
                } else {
                    ShowWindow(hwnd, SW_RESTORE);
                    SetForegroundWindow(hwnd);
                }
            }
            return Ok(());
        }
    }
    unsafe {
        init_dpi_awareness_for_process();
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

        for role in [WindowRole::Main, WindowRole::Quick] {
            let class_name = to_wide(role.class_name());
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
        }

    let title = to_wide(app_title());
        let main_hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            to_wide(WindowRole::Main.class_name()).as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WIN_W,
            WIN_H,
            null_mut(),
            null_mut(),
            hinstance,
            WindowRole::Main as usize as _,
        );
        if main_hwnd.is_null() {
            return Err(io::Error::last_os_error());
        }

        let quick_hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
            to_wide(WindowRole::Quick.class_name()).as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            WIN_W,
            WIN_H,
            null_mut(),
            null_mut(),
            hinstance,
            WindowRole::Quick as usize as _,
        );
        if quick_hwnd.is_null() {
            DestroyWindow(main_hwnd);
            return Err(io::Error::last_os_error());
        }

        ShowWindow(main_hwnd, if startup_can_hide(&boot_settings) { SW_HIDE } else { SW_SHOW });
        ShowWindow(quick_hwnd, SW_HIDE);

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
    if msg == taskbar_created_message() {
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() && (*ptr).role == WindowRole::Main {
            sync_main_tray_icon(hwnd, &mut *ptr);
            retry_startup_integrations(hwnd, &mut *ptr);
        }
        return 0;
    }
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let role = WindowRole::from_create_param(cs.lpCreateParams as isize);
            match on_create(hwnd, role) {
                Ok(_) => 0,
                Err(_) => -1,
            }
        }
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
        WM_SHOWWINDOW => {
            if wparam == 0 {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    (*ptr).release_list_memory();
                    trim_process_working_set();
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_TIMER => {
            if wparam as usize == ID_TIMER_CARET {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    retry_startup_integrations(hwnd, state);
                    if state.vv_popup_visible {
                        if !vv_popup_menu_active()
                            && (GetForegroundWindow() != state.vv_popup_target || IsWindow(state.vv_popup_target) == 0)
                        {
                            vv_popup_hide(hwnd, state);
                        }
                    }
                }
                return 0;
            }
            if wparam as usize == ID_TIMER_VV_SHOW {
                KillTimer(hwnd, ID_TIMER_VV_SHOW);
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    let target = state.vv_popup_pending_target;
                    if !target.is_null() && GetForegroundWindow() == target && IsWindow(target) != 0 {
                        state.vv_popup_pending_target = null_mut();
                        if !vv_popup_show(hwnd, state, target) && state.vv_popup_pending_retries > 0 {
                            state.vv_popup_pending_target = target;
                            state.vv_popup_pending_retries -= 1;
                            SetTimer(hwnd, ID_TIMER_VV_SHOW, VV_SHOW_RETRY_DELAY_MS, None);
                        }
                    } else {
                        state.vv_popup_pending_target = null_mut();
                        state.vv_popup_pending_retries = 0;
                    }
                }
                return 0;
            }
            if wparam as usize == ID_TIMER_PASTE {
                KillTimer(hwnd, ID_TIMER_PASTE);
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    let target = state.paste_target_override;
                    if !target.is_null() {
                        let _ = force_foreground_window(target);
                    }
                    send_backspace_times(state.paste_backspace_count);
                    state.paste_backspace_count = 0;
                    state.paste_target_override = null_mut();
                    clear_hotkey_passthrough_state(state);
                }
                send_ctrl_v();
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
            if wparam as usize == ID_TIMER_EDGE_AUTO_HIDE {
                handle_edge_auto_hide_tick(hwnd);
                return 0;
            }
            if wparam as usize == ID_TIMER_CLOUD_SYNC {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    if !state.cloud_sync_in_progress
                        && state.role == WindowRole::Main
                        && state
                            .cloud_sync_next_due
                            .map(|due| due <= Instant::now())
                            .unwrap_or(false)
                    {
                        queue_cloud_sync(hwnd, state, CloudSyncAction::SyncNow, true);
                    }
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
        WM_MOUSEHOVER => {
            handle_mouse_hover_main(hwnd, lparam);
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
        WM_MOUSEACTIVATE => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let mut keep_noactivate = false;
                let state = &*ptr;
                if state.main_window_noactivate {
                    let mut pt = POINT { x: 0, y: 0 };
                    GetCursorPos(&mut pt);
                    ScreenToClient(hwnd, &mut pt);
                    keep_noactivate = main_window_should_stay_noactivate(state, pt.x, pt.y);
                }
                if keep_noactivate {
                    return MA_NOACTIVATE as LRESULT;
                }
                if state.main_window_noactivate {
                    set_main_window_noactivate_mode(hwnd, false);
                    let ptr = get_state_ptr(hwnd);
                    if !ptr.is_null() {
                        clear_hotkey_passthrough_state(&mut *ptr);
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
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
        WM_VV_SHOW => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                state.vv_popup_pending_target = wparam as HWND;
                state.vv_popup_pending_retries = VV_SHOW_RETRY_MAX;
                KillTimer(hwnd, ID_TIMER_VV_SHOW);
                SetTimer(hwnd, ID_TIMER_VV_SHOW, VV_SHOW_RETRY_DELAY_MS, None);
            }
            0
        }
        WM_VV_HIDE => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                vv_popup_hide(hwnd, &mut *ptr);
            }
            0
        }
        WM_VV_SELECT => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                handle_vv_select(hwnd, &mut *ptr, wparam as usize);
            }
            0
        }
        WM_ITEMS_PAGE_READY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                apply_ready_page_loads(hwnd, &mut *ptr);
            }
            0
        }
        WM_CLOUD_SYNC_READY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                apply_ready_cloud_syncs(hwnd, &mut *ptr);
            }
            0
        }
        WM_UPDATE_CHECK_READY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() && !(*ptr).settings_hwnd.is_null() && IsWindow((*ptr).settings_hwnd) != 0 {
                InvalidateRect((*ptr).settings_hwnd, null(), 1);
            }
            0
        }
        WM_OUTSIDE_CLICK_HIDE => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() && (*ptr).settings.auto_hide_on_blur {
                hide_hover_preview();
                ShowWindow(hwnd, SW_HIDE);
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
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() && (*ptr).role == WindowRole::Main {
                remember_window_pos(hwnd);
            }
            0
        }
        WM_CLOSE => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                if close_to_tray_enabled(&state.settings) {
                    ShowWindow(hwnd, SW_HIDE);
                    return 0;
                }
            }
            DestroyWindow(hwnd);
            0
        }
        WM_NCHITTEST => handle_nchittest(hwnd, lparam),
        WM_DESTROY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                match (*ptr).role {
                    WindowRole::Main => {
                        KillTimer(hwnd, ID_TIMER_CARET);
                        KillTimer(hwnd, ID_TIMER_VV_SHOW);
                        KillTimer(hwnd, ID_TIMER_PASTE);
                        KillTimer(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
                        KillTimer(hwnd, ID_TIMER_CLOUD_SYNC);
                        let popup = current_vv_popup_hwnd();
                        if !popup.is_null() && IsWindow(popup) != 0 {
                            DestroyWindow(popup);
                        }
                        update_vv_mode_hook(hwnd, false);
                        if let Ok(mut handle) = outside_hide_mouse_hook_handle().lock() {
                            if *handle != 0 {
                                UnhookWindowsHookEx(*handle as _);
                                *handle = 0;
                            }
                        }
                        if let Ok(mut handle) = quick_escape_keyboard_hook_handle().lock() {
                            if *handle != 0 {
                                UnhookWindowsHookEx(*handle as _);
                                *handle = 0;
                            }
                        }
                        RemoveClipboardFormatListener(hwnd);
                        unregister_hotkey_for(hwnd, &mut *ptr);
                        remove_tray_icon(hwnd);
                        let quick = quick_window_hwnd();
                        if !quick.is_null() && quick != hwnd && IsWindow(quick) != 0 {
                            DestroyWindow(quick);
                        }
                        PostQuitMessage(0);
                    }
                    WindowRole::Quick => {
                        KillTimer(hwnd, ID_TIMER_PASTE);
                        KillTimer(hwnd, ID_TIMER_SCROLL_FADE);
                    }
                }
            }
            0
        }
        WM_NCDESTROY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                clear_window_host((*ptr).role, hwnd);
                (*ptr).icons.destroy();
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn on_create(hwnd: HWND, role: WindowRole) -> AppResult<()> {
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

    let state = Box::new(AppState::new(role, hwnd, search_hwnd, icons));
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
    if let Some(state) = unsafe { get_state_mut(hwnd) } {
        ensure_db();
        if role == WindowRole::Main {
            reload_state_from_db(state);
            register_hotkey_for(hwnd, state);
            update_vv_mode_hook(hwnd, state.settings.vv_mode_enabled);
            position_main_window(hwnd, &state.settings, false);
            ensure_outside_hide_mouse_hook();
            ensure_quick_escape_keyboard_hook();
        }
    }

    if role == WindowRole::Main {
        AddClipboardFormatListener(hwnd);
    }
    apply_main_window_region(hwnd);
    apply_dark_mode_to_window(hwnd);
    if role == WindowRole::Main {
        if let Some(state) = unsafe { get_state_mut(hwnd) } {
            sync_main_tray_icon(hwnd, state);
        } else if tray_icon != 0 {
            add_tray_icon_localized(hwnd, tray_icon);
        }
    } else {
        set_main_window_noactivate_mode(hwnd, true);
    }
    set_window_host(role, hwnd);
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
    if role == WindowRole::Main {
        SetTimer(hwnd, ID_TIMER_CARET, 500, None);
        SetTimer(hwnd, ID_TIMER_EDGE_AUTO_HIDE, 120, None);
        SetTimer(hwnd, ID_TIMER_CLOUD_SYNC, 5000, None);
    }
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
                let items = state.selected_items_for_use();
                if items.is_empty() {
                    if let Some(item) = state.current_item_for_use() {
                        let _ = db_add_phrase_from_item(&item);
                    }
                } else {
                    for item in &items {
                        let _ = db_add_phrase_from_item(item);
                    }
                }
                state.invalidate_tab_query(1, state.tab_index == 1);
                if state.tab_index == 1 {
                    state.refilter();
                }
                sync_peer_windows_from_db(hwnd);
                InvalidateRect(hwnd, null(), 1);
            }
        }
        IDM_ROW_STICKER => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item_for_use() {
                    if item.kind == ClipKind::Image {
                        show_image_sticker(&item);
                    }
                }
            }
        }
        IDM_ROW_SAVE_IMAGE => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item_for_use() {
                    if item.kind == ClipKind::Image {
                        if let Some(path) = save_image_item(&item) {
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
                if let Some(item) = state.current_item_for_use() {
                    if let Some(paths) = &item.file_paths {
                        for p in paths { open_path_with_shell(p); }
                    }
                }
            }
        }
        IDM_ROW_OPEN_FOLDER => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                if let Some(item) = state.current_item_for_use() {
                    if let Some(paths) = &item.file_paths {
                        for p in paths { open_parent_folder(p); }
                    }
                }
            }
        }
        IDM_ROW_COPY_PATH => {
            if state.context_row >= 0 {
                state.sel_idx = state.context_row;
                let items = state.selected_items_for_use();
                let mut lines = Vec::new();
                if items.is_empty() {
                    if let Some(item) = state.current_item_for_use() {
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
                if let Some(item) = state.current_item_for_use() {
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
                if let Some(item) = state.current_item_for_use() {
                    if let Some(path) = materialize_item_as_file(&item) {
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
                if let Some(item) = state.current_item_for_use() {
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
                if let Some(item) = state.current_item_owned() {
                    let item_id = item.id;
                    let title = format!("编辑 — {}", item.preview.chars().take(40).collect::<String>());
                    let saved = show_edit_item_dialog(hwnd, item_id, &title);
                    if saved {
                        reload_state_from_db(state);
                        state.refilter();
                        sync_peer_windows_from_db(hwnd);
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
                    sync_peer_windows_from_db(hwnd);
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
                if let Some(group_id) = state.groups_for_tab(state.tab_index).get(idx).map(|g| g.id) {
                    let ids = state.selected_db_ids();
                    if !ids.is_empty() {
                        let _ = db_assign_group(&ids, group_id);
                        reload_state_from_db(state);
                        state.refilter();
                        sync_peer_windows_from_db(hwnd);
                        InvalidateRect(hwnd, null(), 1);
                    }
                }
            }
        }
        _ if (IDM_GROUP_FILTER_BASE..IDM_GROUP_FILTER_BASE + 2000).contains(&cmd) => {
            let idx = cmd - IDM_GROUP_FILTER_BASE;
            if let Some(group_id) = state.groups_for_tab(state.tab_index).get(idx).map(|g| g.id) {
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
    state.maybe_request_more_for_active_tab();
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
        dw_flags: TME_LEAVE | TME_HOVER,
        hwnd_track: hwnd,
        dw_hover_time: system_mouse_hover_time_ms(),
    };
    TrackMouseEvent(&mut tme);
}

unsafe fn system_mouse_hover_time_ms() -> u32 {
    let mut hover_ms = 0u32;
    if SystemParametersInfoW(
        SPI_GETMOUSEHOVERTIME_V,
        0,
        &mut hover_ms as *mut _ as _,
        0,
    ) != 0
        && hover_ms > 0
    {
        hover_ms
    } else {
        400
    }
}

unsafe fn hover_preview_blocked_at_point(state: &AppState, x: i32, y: i32) -> bool {
    if scroll_to_top_visible(state) && pt_in_rect(x, y, &state.scroll_to_top_rect()) {
        return true;
    }
    let Some(item) = hovered_item_clone(state) else {
        return false;
    };
    row_quick_delete_rect(state, state.hover_idx, &item)
        .map(|rc| pt_in_rect(x, y, &rc))
        .unwrap_or(false)
}

unsafe fn refresh_hover_preview(hwnd: HWND, state: &AppState, x: i32, y: i32) {
    if !state.settings.hover_preview || state.edge_hidden {
        hide_hover_preview();
        return;
    }
    let Some(item) = hovered_item_clone(state) else {
        hide_hover_preview();
        return;
    };
    if hover_preview_blocked_at_point(state, x, y) {
        hide_hover_preview();
        return;
    }
    let mut win_rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut win_rc) == 0 {
        hide_hover_preview();
        return;
    }
    show_hover_preview(&item, win_rc.left + x, win_rc.top + y);
}

unsafe fn handle_mouse_hover_main(hwnd: HWND, lparam: LPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &*ptr;
    refresh_hover_preview(hwnd, state, get_x_lparam(lparam), get_y_lparam(lparam));
}

fn clear_edge_dock_state(state: &mut AppState) {
    state.edge_hidden = false;
    state.edge_hidden_side = EDGE_AUTO_HIDE_NONE;
    state.edge_docked_left = 0;
    state.edge_docked_top = 0;
    state.edge_docked_right = 0;
    state.edge_docked_bottom = 0;
}

unsafe fn update_edge_dock_state(hwnd: HWND, state: &mut AppState, rc: &RECT) -> bool {
    if let Some((side, base)) = edge_choose_dock_side(hwnd, rc) {
        state.edge_hidden_side = side;
        set_edge_docked_rect(state, &base);
        if !state.edge_hidden {
            state.edge_restore_x = rc.left;
            state.edge_restore_y = rc.top;
        }
        true
    } else {
        clear_edge_dock_state(state);
        false
    }
}

fn set_edge_docked_rect(state: &mut AppState, rc: &RECT) {
    state.edge_docked_left = rc.left;
    state.edge_docked_top = rc.top;
    state.edge_docked_right = rc.right;
    state.edge_docked_bottom = rc.bottom;
}

fn edge_docked_rect(state: &AppState) -> RECT {
    RECT {
        left: state.edge_docked_left,
        top: state.edge_docked_top,
        right: state.edge_docked_right,
        bottom: state.edge_docked_bottom,
    }
}

fn edge_detect_margin_v() -> i32 {
    EDGE_AUTO_HIDE_MARGIN.max(12)
}

fn edge_detect_margin_h() -> i32 {
    edge_detect_margin_v().max(24)
}

unsafe fn edge_choose_dock_side(hwnd: HWND, rc: &RECT) -> Option<(i32, RECT)> {
    let work = nearest_monitor_work_rect_for_window(hwnd);
    let monitor = nearest_monitor_rect_for_window(hwnd);
    let margin_v = edge_detect_margin_v();
    let margin_h = edge_detect_margin_h();

    let candidates = [
        ((rc.left - work.left).abs(), EDGE_AUTO_HIDE_LEFT, work, margin_h),
        ((rc.left - monitor.left).abs(), EDGE_AUTO_HIDE_LEFT, monitor, margin_h),
        ((work.right - rc.right).abs(), EDGE_AUTO_HIDE_RIGHT, work, margin_h),
        ((monitor.right - rc.right).abs(), EDGE_AUTO_HIDE_RIGHT, monitor, margin_h),
        ((rc.top - work.top).abs(), EDGE_AUTO_HIDE_TOP, work, margin_v),
        ((rc.top - monitor.top).abs(), EDGE_AUTO_HIDE_TOP, monitor, margin_v),
        ((work.bottom - rc.bottom).abs(), EDGE_AUTO_HIDE_BOTTOM, work, margin_v),
        ((monitor.bottom - rc.bottom).abs(), EDGE_AUTO_HIDE_BOTTOM, monitor, margin_v),
    ];

    let mut best: Option<(i32, i32, RECT)> = None;
    for (dist, side, base, limit) in candidates {
        if dist > limit {
            continue;
        }
        match best {
            Some((best_dist, _, _)) if best_dist <= dist => {}
            _ => best = Some((dist, side, base)),
        }
    }
    best.map(|(_, side, base)| (side, base))
}

unsafe fn restore_edge_hidden_window(hwnd: HWND, state: &mut AppState) {
    if !state.edge_hidden {
        return;
    }
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        state.edge_restore_x,
        state.edge_restore_y,
        0,
        0,
        SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
    state.edge_hidden = false;
}

unsafe fn hide_edge_docked_window(hwnd: HWND, state: &mut AppState) {
    if state.role != WindowRole::Main || !state.settings.edge_auto_hide || state.edge_hidden {
        return;
    }

    let rc = window_rect_for_dock(hwnd);
    if !update_edge_dock_state(hwnd, state, &rc) {
        return;
    }

    let mut cursor: POINT = zeroed();
    GetCursorPos(&mut cursor);
    if cursor_over_window_tree(hwnd, cursor) {
        return;
    }

    state.edge_restore_x = rc.left;
    state.edge_restore_y = rc.top;
    let docked = edge_docked_rect(state);
    let width = (rc.right - rc.left).max(1);
    let height = (rc.bottom - rc.top).max(1);
    let (hide_x, hide_y) = match state.edge_hidden_side {
        EDGE_AUTO_HIDE_LEFT => (docked.left + EDGE_AUTO_HIDE_PEEK - width, rc.top),
        EDGE_AUTO_HIDE_RIGHT => (docked.right - EDGE_AUTO_HIDE_PEEK, rc.top),
        EDGE_AUTO_HIDE_TOP => (rc.left, docked.top + EDGE_AUTO_HIDE_PEEK - height),
        EDGE_AUTO_HIDE_BOTTOM => (rc.left, docked.bottom - EDGE_AUTO_HIDE_PEEK),
        _ => (rc.left, rc.top),
    };
    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        hide_x,
        hide_y,
        0,
        0,
        SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
    state.edge_hidden = true;
    hide_hover_preview();
    InvalidateRect(hwnd, null(), 0);
}

unsafe fn handle_edge_auto_hide_tick(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() || IsWindowVisible(hwnd) == 0 {
        return;
    }
    let state = &mut *ptr;
    if !state.settings.edge_auto_hide {
        restore_edge_hidden_window(hwnd, state);
        clear_edge_dock_state(state);
        return;
    }

    let rc = window_rect_for_dock(hwnd);
    let mut cursor: POINT = zeroed();
    GetCursorPos(&mut cursor);
    let width = (rc.right - rc.left).max(1);
    let height = (rc.bottom - rc.top).max(1);
    let monitor = nearest_monitor_rect_for_window(hwnd);
    let docked = edge_docked_rect(state);

    if !pt_in_rect_screen(&cursor, &RECT {
        left: monitor.left - 2,
        top: monitor.top - 2,
        right: monitor.right + 2,
        bottom: monitor.bottom + 2,
    }) {
        return;
    }

    if state.edge_hidden {
        let hot = match state.edge_hidden_side {
            EDGE_AUTO_HIDE_LEFT => RECT {
                left: docked.left,
                top: state.edge_restore_y,
                right: docked.left + EDGE_AUTO_HIDE_MARGIN,
                bottom: state.edge_restore_y + height,
            },
            EDGE_AUTO_HIDE_RIGHT => RECT {
                left: docked.right - EDGE_AUTO_HIDE_MARGIN,
                top: state.edge_restore_y,
                right: docked.right,
                bottom: state.edge_restore_y + height,
            },
            EDGE_AUTO_HIDE_TOP => RECT {
                left: state.edge_restore_x,
                top: docked.top,
                right: state.edge_restore_x + width,
                bottom: docked.top + EDGE_AUTO_HIDE_MARGIN,
            },
            EDGE_AUTO_HIDE_BOTTOM => RECT {
                left: state.edge_restore_x,
                top: docked.bottom - EDGE_AUTO_HIDE_MARGIN,
                right: state.edge_restore_x + width,
                bottom: docked.bottom,
            },
            _ => rc,
        };
        if pt_in_rect_screen(&cursor, &hot) || GetForegroundWindow() == hwnd {
            restore_edge_hidden_window(hwnd, state);
            InvalidateRect(hwnd, null(), 0);
        }
        return;
    }

    update_edge_dock_state(hwnd, state, &rc);
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
    let old_to_top = state.hover_to_top;
    state.hover_btn = "";
    for key in ["search", "setting", "min", "close"] {
        if !title_button_visible(&state.settings, key) {
            continue;
        }
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

    state.hover_to_top = scroll_to_top_visible(state) && pt_in_rect(x, y, &state.scroll_to_top_rect());

    let old_hover = state.hover_idx;
    state.hover_idx = if state.hover_to_top { -1 } else { hit_test_row(state, x, y) };
    let preview_target_changed = old_btn != state.hover_btn
        || old_hover != state.hover_idx
        || old_tab != state.hover_tab
        || old_scroll != state.hover_scroll
        || old_to_top != state.hover_to_top;
    if old_hover != state.hover_idx || hover_preview_blocked_at_point(state, x, y) {
        hide_hover_preview();
    }

    if preview_target_changed {
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
    if state.hover_to_top { state.hover_to_top = false; dirty = true; }
    hide_hover_preview();
    // 保留当前选择集，避免右键菜单弹出时因为 WM_MOUSELEAVE 把多选清空。
    if dirty {
        InvalidateRect(hwnd, null(), 0);
    }
    hide_edge_docked_window(hwnd, state);
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
    if state.hover_to_top { state.hover_to_top = false; dirty = true; }
    if state.down_to_top { state.down_to_top = false; dirty = true; }
    if state.down_row != -1 { state.down_row = -1; state.down_x = 0; state.down_y = 0; dirty = true; }
    hide_hover_preview();
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
            if !title_button_visible(&state.settings, key) {
                continue;
            }
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
        if !title_button_visible(&state.settings, key) {
            continue;
        }
        if pt_in_rect(x, y, &state.title_button_rect(key)) {
            state.down_btn = key;
            InvalidateRect(hwnd, null(), 0);
            return;
        }
    }

    state.down_to_top = scroll_to_top_visible(state) && pt_in_rect(x, y, &state.scroll_to_top_rect());
    if state.down_to_top {
        InvalidateRect(hwnd, null(), 0);
        return;
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
        let ctrl = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;
        let shift = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
        if !ctrl && !shift && state.hotkey_passthrough_active && !state.hotkey_passthrough_edit.is_null() {
            let mut handled = false;
            if let Some(src_idx) = state.filtered_indices.get(idx as usize).copied() {
                if let Some(item) = state.active_items().get(src_idx).cloned() {
                    if let Some(del_rc) = row_quick_delete_rect(state, idx, &item) {
                        if !pt_in_rect(x, y, &del_rc) {
                            handled = try_apply_to_explorer_rename(state, &item);
                        }
                    } else {
                        handled = try_apply_to_explorer_rename(state, &item);
                    }
                }
            }
            if handled {
                ShowWindow(hwnd, SW_HIDE);
                state.clear_selection();
                clear_main_hover_state(hwnd);
                return;
            }
        }
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
                if close_to_tray_enabled(&state.settings) {
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

    if state.down_to_top {
        let activate = pt_in_rect(x, y, &state.scroll_to_top_rect());
        state.down_to_top = false;
        if activate {
            state.scroll_y = 0;
            state.scroll_fade_alpha = 255;
            if !state.scroll_fade_timer {
                state.scroll_fade_timer = true;
                SetTimer(hwnd, ID_TIMER_SCROLL_FADE, 50, None);
            }
        }
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
    if let Some(src_idx) = state.filtered_indices.get(idx as usize).copied() {
        if let Some(item) = state.active_items().get(src_idx).cloned() {
            if let Some(del_rc) = row_quick_delete_rect(state, idx, &item) {
                if pt_in_rect(x, y, &del_rc) {
                    state.delete_selected();
                    InvalidateRect(hwnd, null(), 1);
                    return;
                }
            }
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

    // 单击逻辑统一走粘贴入口，资源管理器重命名会在这里走直接写 Edit 的专用路径。
    paste_selected(hwnd, state);
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
    hide_hover_preview();
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
            if let Some(group_id) = state.groups_for_tab(target_tab).get(idx).map(|g| g.id) {
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
    let current_item = state.current_item_for_use();
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
        if state.search_on && pt_in_rect(pt.x, pt.y, &state.search_rect()) {
            return HTCLIENT as LRESULT;
        }
        for key in ["search", "setting", "min", "close"] {
            if !title_button_visible(&state.settings, key) {
                continue;
            }
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
    let items = state.selected_items_for_use();
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
        set_ignore_clipboard_for_all_hosts(450);
    }
    ok
}

unsafe fn apply_item_to_clipboard(state: &mut AppState, item_ref: &ClipItem) -> bool {
    let full_item;
    let item: &ClipItem = if let Some(resolved) = state.resolve_item_for_use(item_ref) {
        full_item = resolved;
        &full_item
    } else {
        return false;
    };

    let ok = match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let mut clipboard = match Clipboard::new() {
                Ok(c) => c,
                Err(_) => return false,
            };
            if let Some(text) = &item.text {
                clipboard.set_text(maybe_ai_clean_text(state, text)).is_ok()
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
                clipboard.set_text(maybe_ai_clean_text(state, text)).is_ok()
            } else {
                false
            }
        }
    };
    if ok {
        set_ignore_clipboard_for_all_hosts(450);
    }
    ok
}

unsafe fn apply_selected_to_clipboard(state: &mut AppState) -> bool {
    let Some(item_ref) = state.current_item().cloned() else {
        return false;
    };
    apply_item_to_clipboard(state, &item_ref)
}

unsafe fn try_apply_to_explorer_rename(state: &mut AppState, item_ref: &ClipItem) -> bool {
    if !state.hotkey_passthrough_active || state.hotkey_passthrough_edit.is_null() {
        return false;
    }
    if IsWindow(state.hotkey_passthrough_edit) == 0 {
        clear_hotkey_passthrough_state(state);
        return false;
    }

    let full_item = match state.resolve_item_for_use(item_ref) {
        Some(item) => item,
        None => return false,
    };

    let text = match full_item.kind {
        ClipKind::Text | ClipKind::Phrase => full_item
            .text
            .as_ref()
            .map(|text| maybe_ai_clean_text(state, text)),
        ClipKind::Files => full_item
            .text
            .as_ref()
            .map(|text| maybe_ai_clean_text(state, text)),
        ClipKind::Image => None,
    };

    let Some(text) = text else {
        return false;
    };

    let wide = to_wide(&text);
    let ok = SendMessageW(
        state.hotkey_passthrough_edit,
        WM_SETTEXT,
        0,
        wide.as_ptr() as LPARAM,
    ) != 0;
    if ok {
        let caret = text.encode_utf16().count() as isize;
        SendMessageW(state.hotkey_passthrough_edit, EM_SETSEL, caret as usize, caret);
        set_ignore_clipboard_for_all_hosts(250);
        clear_hotkey_passthrough_state(state);
    }
    ok
}

unsafe fn paste_selected(hwnd: HWND, state: &mut AppState) {
    let Some(item_ref) = state.current_item().cloned() else {
        return;
    };
    if try_apply_to_explorer_rename(state, &item_ref) {
        ShowWindow(hwnd, SW_HIDE);
        state.clear_selection();
        clear_main_hover_state(hwnd);
        return;
    }
    if !apply_item_to_clipboard(state, &item_ref) {
        return;
    }
    state.clear_selection();
    clear_main_hover_state(hwnd);
    paste_after_clipboard_ready(hwnd, state, state.settings.click_hide);
}

unsafe fn handle_vv_select(hwnd: HWND, state: &mut AppState, index: usize) {
    if !state.vv_popup_visible {
        return;
    }
    let target = state.vv_popup_target;
    let backspaces = if state.vv_popup_replaces_ime { 0 } else { 2 };
    let item = state.vv_popup_items.get(index).map(|entry| entry.item.clone());
    vv_popup_hide(hwnd, state);
    let Some(item) = item else {
        return;
    };
    if !apply_item_to_clipboard(state, &item) {
        return;
    }
    paste_after_clipboard_ready_to_target(hwnd, state, target, false, backspaces);
}

fn ai_clean_text(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut output = Vec::new();
    let mut blank = 0usize;
    for line in normalized.lines() {
        let trimmed = line.trim_end().to_string();
        if trimmed.trim().is_empty() {
            blank += 1;
            if blank <= 2 {
                output.push(String::new());
            }
        } else {
            blank = 0;
            output.push(trimmed);
        }
    }
    output.join("\n").trim().to_string()
}

unsafe fn maybe_ai_clean_text(state: &AppState, input: &str) -> String {
    if !state.settings.ai_clean_enabled {
        return input.to_string();
    }
    let shift_down = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
    if shift_down {
        return input.to_string();
    }
    if input.matches('\n').count() >= 8 || input.contains("```") || input.chars().count() >= 600 {
        ai_clean_text(input)
    } else {
        input.to_string()
    }
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
    let Some(item) = state.resolve_item_for_use(&item) else {
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
    let groups = state.groups_for_tab(state.tab_index);
    let group_menu = if state.settings.grouping_enabled { CreatePopupMenu() } else { null_mut() };
    if !group_menu.is_null() {
        apply_theme_to_menu(group_menu as _);
        if groups.is_empty() {
            AppendMenuW(group_menu, MF_GRAYED | MF_STRING, 0xFFFFusize, to_wide(translate("（暂无分组）").as_ref()).as_ptr());
        } else {
            for (idx, g) in groups.iter().enumerate() {
                AppendMenuW(group_menu, MF_STRING, IDM_ROW_GROUP_BASE + idx, to_wide(&g.name).as_ptr());
            }
        }
    }
    if selected_count > 1 {
        AppendMenuW(menu, MF_STRING, IDM_ROW_COPY, to_wide(translate("合并复制").as_ref()).as_ptr());
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let pin_text = if has_unpinned { "置顶所选" } else { "取消置顶" };
        AppendMenuW(menu, MF_STRING, IDM_ROW_PIN, to_wide(translate(pin_text).as_ref()).as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_ROW_TO_PHRASE, to_wide(translate("添加到短语").as_ref()).as_ptr());
        if !group_menu.is_null() {
            AppendMenuW(menu, MF_POPUP, group_menu as usize, to_wide(translate("添加到分组").as_ref()).as_ptr());
        }
        AppendMenuW(menu, MF_STRING, IDM_ROW_GROUP_REMOVE, to_wide(translate("移出分组").as_ref()).as_ptr());
        AppendMenuW(menu, MF_STRING, IDM_ROW_DELETE, to_wide(translate("删除所选").as_ref()).as_ptr());
    } else {
        match current_kind {
            ClipKind::Image => {
                AppendMenuW(menu, MF_STRING, IDM_ROW_STICKER, to_wide(translate("贴图").as_ref()).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_SAVE_IMAGE, to_wide(translate("另存为 PNG").as_ref()).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_EXPORT_FILE, to_wide(translate("导出为文件").as_ref()).as_ptr());
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                let pin_text = if has_unpinned { "置顶" } else { "取消置顶" };
                AppendMenuW(menu, MF_STRING, IDM_ROW_PIN, to_wide(translate(pin_text).as_ref()).as_ptr());
                if !group_menu.is_null() {
                    AppendMenuW(menu, MF_POPUP, group_menu as usize, to_wide(translate("添加到分组").as_ref()).as_ptr());
                }
                AppendMenuW(menu, MF_STRING, IDM_ROW_GROUP_REMOVE, to_wide(translate("移出分组").as_ref()).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_DELETE, to_wide(translate("删除").as_ref()).as_ptr());
            }
            ClipKind::Files => {
                let open_text = if current_is_dir { "打开文件夹" } else { "打开文件" };
                AppendMenuW(menu, MF_STRING, IDM_ROW_OPEN_PATH, to_wide(translate(open_text).as_ref()).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_OPEN_FOLDER, to_wide(translate("打开所在文件夹").as_ref()).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_COPY_PATH, to_wide(translate("复制路径").as_ref()).as_ptr());
                if current_is_excel && state.settings.super_mail_merge_enabled {
                    AppendMenuW(menu, MF_STRING, IDM_ROW_MAIL_MERGE, to_wide(translate("超级邮件合并").as_ref()).as_ptr());
                }
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                let pin_text = if has_unpinned { "置顶" } else { "取消置顶" };
                AppendMenuW(menu, MF_STRING, IDM_ROW_PIN, to_wide(translate(pin_text).as_ref()).as_ptr());
                if !group_menu.is_null() {
                    AppendMenuW(menu, MF_POPUP, group_menu as usize, to_wide(translate("添加到分组").as_ref()).as_ptr());
                }
                AppendMenuW(menu, MF_STRING, IDM_ROW_GROUP_REMOVE, to_wide(translate("移出分组").as_ref()).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_DELETE, to_wide(translate("删除").as_ref()).as_ptr());
            }
            _ => {
                let pin_text = if has_unpinned { "置顶" } else { "取消置顶" };
                AppendMenuW(menu, MF_STRING, IDM_ROW_EDIT, to_wide(translate("编辑").as_ref()).as_ptr());
                if state.settings.quick_search_enabled {
                    AppendMenuW(menu, MF_STRING, IDM_ROW_QUICK_SEARCH, to_wide(translate("快速搜索").as_ref()).as_ptr());
                }
                AppendMenuW(menu, MF_STRING, IDM_ROW_EXPORT_FILE, to_wide(translate("导出为文件").as_ref()).as_ptr());
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                AppendMenuW(menu, MF_STRING, IDM_ROW_PIN, to_wide(translate(pin_text).as_ref()).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_TO_PHRASE, to_wide(translate("添加到短语").as_ref()).as_ptr());
                if !group_menu.is_null() {
                    AppendMenuW(menu, MF_POPUP, group_menu as usize, to_wide(translate("添加到分组").as_ref()).as_ptr());
                }
                AppendMenuW(menu, MF_STRING, IDM_ROW_GROUP_REMOVE, to_wide(translate("移出分组").as_ref()).as_ptr());
                AppendMenuW(menu, MF_STRING, IDM_ROW_DELETE, to_wide(translate("删除").as_ref()).as_ptr());
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
    let groups = state.groups_for_tab(tab_index);
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
    AppendMenuW(menu, all_flags, IDM_GROUP_FILTER_ALL, to_wide(translate("全部").as_ref()).as_ptr());
    if !groups.is_empty() {
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        for (idx, g) in groups.iter().enumerate() {
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

unsafe fn send_backspace_times(count: u8) {
    for _ in 0..count {
        keybd_event(VK_BACK as u8, 0, 0, 0);
        keybd_event(VK_BACK as u8, 0, KEYEVENTF_KEYUP, 0);
    }
}

unsafe fn send_alt_tap() {
    let inputs = [
        INPUT { r#type: INPUT_KEYBOARD, anonymous: INPUT_UNION { ki: KEYBDINPUT { w_vk: VK_MENU as u16, w_scan: 0, dw_flags: 0, time: 0, dw_extra_info: 0 } } },
        INPUT { r#type: INPUT_KEYBOARD, anonymous: INPUT_UNION { ki: KEYBDINPUT { w_vk: VK_MENU as u16, w_scan: 0, dw_flags: KEYEVENTF_KEYUP, time: 0, dw_extra_info: 0 } } },
    ];
    let _ = SendInput(inputs.len() as u32, inputs.as_ptr(), size_of::<INPUT>() as i32);
}

fn clear_hotkey_passthrough_state(state: &mut AppState) {
    state.hotkey_passthrough_active = false;
    state.hotkey_passthrough_target = null_mut();
    state.hotkey_passthrough_edit = null_mut();
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

unsafe fn effective_paste_target(state: &AppState, hwnd: HWND) -> HWND {
    if !state.paste_target_override.is_null() {
        return state.paste_target_override;
    }
    if state.hotkey_passthrough_active && !state.hotkey_passthrough_target.is_null() {
        return state.hotkey_passthrough_target;
    }
    if state.role == WindowRole::Quick {
        let fg = GetForegroundWindow();
        if is_viable_paste_window(fg, hwnd) {
            return fg;
        }
    }
    find_next_paste_target(hwnd)
}

unsafe fn paste_after_clipboard_ready(hwnd: HWND, state: &mut AppState, hide_main: bool) {
    let target = effective_paste_target(state, hwnd);
    paste_after_clipboard_ready_to_target(hwnd, state, target, hide_main, 0);
}

unsafe fn paste_after_clipboard_ready_to_target(hwnd: HWND, state: &mut AppState, target: HWND, hide_main: bool, backspaces: u8) {
    state.paste_target_override = target;
    state.paste_backspace_count = backspaces;
    if !target.is_null() {
        if hide_main || state.hotkey_passthrough_active {
            ShowWindow(hwnd, SW_HIDE);
        }
        let _ = force_foreground_window(target);
        KillTimer(hwnd, ID_TIMER_PASTE);
        SetTimer(hwnd, ID_TIMER_PASTE, 150, None);
    } else {
        clear_hotkey_passthrough_state(state);
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
    if hwnd.is_null() || hwnd == app_hwnd || is_app_window(hwnd) {
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
    state.maybe_request_more_for_active_tab();
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
        if !title_button_visible(&state.settings, key) {
            continue;
        }
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
        let msg = if state.active_load_state().loading {
            "正在加载..."
        } else if state.active_load_state().error.is_some() {
            "加载失败，请稍后重试"
        } else if state.settings.grouping_enabled && state.current_group_filter != 0 {
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

            if let Some(del_rc) = row_quick_delete_rect(state, i, &item) {
                let bg = inflate_rect(&del_rc, 2, 2);
                draw_round_rect(memdc as _, &bg, th.surface, th.stroke, 10);
                if state.icons.del != 0 {
                    draw_icon_tinted(memdc as _, del_rc.left, del_rc.top, state.icons.del, 16, 16, dark);
                }
            }

            row_rc.left += 40;
            row_rc.right -= row_text_right_padding(state, i);
            if let Some(preview_rc) = row_inline_preview_rect(&row_rc, &item, &state.settings) {
                let bg = inflate_rect(&preview_rc, 2, 2);
                draw_round_rect(memdc as _, &bg, th.surface2, th.stroke, 8);
                if let Some((bytes, width, height)) = ensure_item_image_bytes(&item) {
                    draw_rgba_image_fit(memdc as _, &bytes, width, height, &preview_rc);
                }
                row_rc.left = preview_rc.right + 10;
            }
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

        if state.active_load_state().loading {
            let loading_rc = RECT {
                left: LIST_X + 18,
                top: LIST_Y + LIST_H - 36,
                right: LIST_X + LIST_W - 18,
                bottom: LIST_Y + LIST_H - 12,
            };
            draw_text(memdc as _, "继续加载中...", &loading_rc, th.text_muted, 11, false, true);
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

        if scroll_to_top_visible(state) {
            let top_rc = state.scroll_to_top_rect();
            let fill = if state.down_to_top {
                th.button_pressed
            } else if state.hover_to_top {
                th.button_hover
            } else {
                th.surface
            };
            draw_round_rect(memdc as _, &top_rc, fill, th.stroke, 10);
            draw_text_ex(memdc as _, "↑", &top_rc, th.text, 18, true, true, "Segoe UI Variable Display");
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

pub(crate) unsafe fn set_main_window_noactivate_mode(hwnd: HWND, enable: bool) {
    if hwnd.is_null() {
        return;
    }
    let ex_style = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32;
    let desired = if enable {
        ex_style | WS_EX_NOACTIVATE
    } else {
        ex_style & !WS_EX_NOACTIVATE
    };
    if desired == ex_style {
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            (*ptr).main_window_noactivate = enable;
        }
        return;
    }
    SetWindowLongW(hwnd, GWL_EXSTYLE, desired as i32);
    let flags = SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | if enable { SWP_NOACTIVATE } else { 0 };
    SetWindowPos(hwnd, null_mut(), 0, 0, 0, 0, flags);
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        (*ptr).main_window_noactivate = enable;
    }
}

unsafe fn get_state_mut(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

unsafe fn main_window_should_stay_noactivate(state: &AppState, x: i32, y: i32) -> bool {
    hit_test_row(state, x, y) >= 0
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

fn pt_in_rect_screen(pt: &POINT, rc: &RECT) -> bool {
    pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom
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

fn row_supports_image_preview(item: &ClipItem, settings: &AppSettings) -> bool {
    settings.image_preview_enabled && item.kind == ClipKind::Image
}

fn row_shows_delete_button(state: &AppState, visible_idx: i32) -> bool {
    state.settings.quick_delete_button && state.hover_idx == visible_idx
}

fn row_text_right_padding(state: &AppState, visible_idx: i32) -> i32 {
    if row_shows_delete_button(state, visible_idx) { 42 } else { 18 }
}

fn row_quick_delete_rect(state: &AppState, visible_idx: i32, _item: &ClipItem) -> Option<RECT> {
    if !row_shows_delete_button(state, visible_idx) {
        return None;
    }
    state.quick_action_rect_slot(visible_idx, 0)
}

fn row_inline_preview_rect(row_rc: &RECT, item: &ClipItem, settings: &AppSettings) -> Option<RECT> {
    if !row_supports_image_preview(item, settings) {
        return None;
    }
    let size = 30;
    let left = row_rc.left + 2;
    let top = row_rc.top + ((row_rc.bottom - row_rc.top - size) / 2);
    Some(RECT { left, top, right: left + size, bottom: top + size })
}

fn scroll_to_top_visible(state: &AppState) -> bool {
    state.scroll_y > ROW_H
}

unsafe fn hovered_item_clone(state: &AppState) -> Option<ClipItem> {
    if state.hover_idx < 0 {
        return None;
    }
    let src_idx = *state.filtered_indices.get(state.hover_idx as usize)?;
    state.active_items().get(src_idx).cloned()
}

fn inflate_rect(rc: &RECT, dx: i32, dy: i32) -> RECT {
    RECT {
        left: rc.left - dx,
        top: rc.top - dy,
        right: rc.right + dx,
        bottom: rc.bottom + dy,
    }
}

unsafe fn draw_rgba_image_fit(
    hdc: *mut core::ffi::c_void,
    bytes: &[u8],
    width: usize,
    height: usize,
    dest: &RECT,
) {
    if bytes.is_empty() || width == 0 || height == 0 {
        return;
    }
    let avail_w = (dest.right - dest.left).max(1);
    let avail_h = (dest.bottom - dest.top).max(1);
    let scale = (avail_w as f32 / width as f32)
        .min(avail_h as f32 / height as f32)
        .max(0.01);
    let draw_w = ((width as f32) * scale).round().max(1.0) as i32;
    let draw_h = ((height as f32) * scale).round().max(1.0) as i32;
    let draw_x = dest.left + (avail_w - draw_w) / 2;
    let draw_y = dest.top + (avail_h - draw_h) / 2;

    let mut bmi: BITMAPINFO = zeroed();
    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
    bmi.bmiHeader.biWidth = width as i32;
    bmi.bmiHeader.biHeight = -(height as i32);
    bmi.bmiHeader.biPlanes = 1;
    bmi.bmiHeader.biBitCount = 32;
    bmi.bmiHeader.biCompression = BI_RGB;

    StretchDIBits(
        hdc as _,
        draw_x,
        draw_y,
        draw_w,
        draw_h,
        0,
        0,
        width as i32,
        height as i32,
        bytes.as_ptr() as _,
        &bmi,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}
