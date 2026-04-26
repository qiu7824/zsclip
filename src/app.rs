pub(crate) mod data;
pub(crate) mod hosts;
pub(crate) mod runtime;
pub(crate) mod state;

pub(crate) use self::hosts::{
    get_state_ptr, main_window_hwnd, quick_window_hwnd, refresh_low_level_input_hooks,
    refresh_window_for_show, set_main_window_noactivate_mode, shutdown_low_level_input_hooks,
};
pub(crate) use self::runtime::{db_file, save_settings};

use self::data::*;
use self::hosts::*;
use self::runtime::*;
use self::state::*;
use crate::i18n::{app_title, tr, translate};
use crate::settings_model::settings_page_content_total_h;
use crate::settings_model::SETTINGS_PAGE_COUNT;
use arboard::{Clipboard, ImageData};
use image::ImageFormat;
use rusqlite::types::Value as SqlValue;
use rusqlite::{params, params_from_iter};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::cmp::{max, min};
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io;
use std::mem::{size_of, zeroed};
use std::ops::{Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::{Mutex, OnceLock};
use std::time::{Instant, SystemTime, UNIX_EPOCH};
#[link(name = "user32")]
unsafe extern "system" {
    fn RegisterHotKey(hwnd: HWND, id: i32, fsmodifiers: u32, vk: u32) -> i32;
    fn UnregisterHotKey(hwnd: HWND, id: i32) -> i32;
    fn EnableWindow(hwnd: HWND, benable: i32) -> i32;
    fn IsWindow(hwnd: HWND) -> i32;
    fn GetClipboardSequenceNumber() -> u32;
    fn ShowScrollBar(hwnd: HWND, wbar: i32, bshow: i32) -> i32;
    fn AttachThreadInput(id_attach: u32, id_attach_to: u32, attach: i32) -> i32;
    fn TrackMouseEvent(lpeventtrack: *mut TRACKMOUSEEVENT) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetLastError() -> u32;
    fn GetCurrentProcess() -> *mut core::ffi::c_void;
    fn GetCurrentProcessId() -> u32;
    fn GetCurrentThreadId() -> u32;
    fn OpenProcess(
        dwdesiredaccess: u32,
        binherithandle: i32,
        dwprocessid: u32,
    ) -> *mut core::ffi::c_void;
    fn QueryFullProcessImageNameW(
        hprocess: *mut core::ffi::c_void,
        dwflags: u32,
        lpexename: *mut u16,
        lpdwsize: *mut u32,
    ) -> i32;
    fn CloseHandle(hobject: *mut core::ffi::c_void) -> i32;
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

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: HWND,
        dwattribute: u32,
        pvattribute: *const core::ffi::c_void,
        cbattribute: u32,
    ) -> i32;
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

const ERROR_HOTKEY_ALREADY_REGISTERED: u32 = 1409;

use crate::cloud_sync::{
    cloud_sync_interval, perform_cloud_sync, CloudSyncAction, CloudSyncConfig, CloudSyncOutcome,
    CloudSyncPaths,
};
use crate::db_runtime::{close_db, ensure_db, with_db, with_db_mut};
use crate::hover_preview::{hide_hover_preview, show_hover_preview};
use crate::mail_merge_native::{launch_mail_merge_window, launch_mail_merge_window_with_excel};
use crate::shell::{
    detect_wechat_runtime_dir, icon_handle_for, is_directory_item, item_icon_handle, load_icons,
    open_parent_folder, open_path_with_shell, open_source_url, open_source_url_display,
    pick_paste_sound_file, play_paste_success_sound, plugin_downloads_url, restart_explorer_shell,
    run_baidu_ocr_api, run_baidu_translate_api, run_winocr_dll_ocr,
    set_system_clipboard_history_enabled, start_update_check, update_check_available,
    update_check_latest_url_or_default, update_check_state_snapshot, IconAssetKind,
};
use crate::sticker::show_image_sticker;
use crate::time_utils::{
    days_to_sqlite_date, format_created_at_local, format_local_time_for_image_preview,
    gregorian_to_days, now_utc_sqlite, utc_secs_to_local_parts,
};
use crate::tray::{
    add_tray_icon_localized, handle_tray, position_main_window, remember_window_pos,
    remove_tray_icon, toggle_window_visibility, toggle_window_visibility_hotkey,
};
use crate::ui::{
    draw_icon_tinted, draw_icon_tinted_soft, draw_main_segment_bar, draw_round_fill,
    draw_round_rect, draw_text, draw_text_ex, parse_search_query, rgb, set_settings_ui_dpi,
    settings_content_y_scaled, settings_h_scaled, settings_nav_item_rect, settings_nav_w_scaled,
    settings_scale, settings_w_scaled, ui_display_font_family, ui_text_font_family, ClipListState,
    MainUiLayout, SearchTimeFilter, Theme, DT_LEFT, DT_SINGLELINE, DT_VCENTER, SETTINGS_PAGES,
};
pub(crate) use crate::ui::{ClipGroup, ClipItem, ClipKind};
use crate::win_buffered_paint::{begin_buffered_paint, end_buffered_paint};
use crate::win_system_params::{
    DropFiles, SettingsFormSectionLayout, CF_HDROP, GMEM_MOVEABLE, GMEM_ZEROINIT,
    IDC_SET_AUTOHIDE_BLUR, IDC_SET_AUTOSTART, IDC_SET_BTN_OPENCFG, IDC_SET_BTN_OPENDATA,
    IDC_SET_BTN_OPENDB, IDC_SET_CLICK_HIDE, IDC_SET_CLOSE, IDC_SET_CLOSETRAY,
    IDC_SET_CLOUD_APPLY_CFG, IDC_SET_CLOUD_DIR, IDC_SET_CLOUD_ENABLE, IDC_SET_CLOUD_INTERVAL,
    IDC_SET_CLOUD_PASS, IDC_SET_CLOUD_RESTORE_BACKUP, IDC_SET_CLOUD_SYNC_NOW,
    IDC_SET_CLOUD_UPLOAD_CFG, IDC_SET_CLOUD_URL, IDC_SET_CLOUD_USER, IDC_SET_DEDUPE_FILTER,
    IDC_SET_DX, IDC_SET_DY, IDC_SET_EDGEHIDE, IDC_SET_FX, IDC_SET_FY, IDC_SET_GROUP_ADD,
    IDC_SET_GROUP_DELETE, IDC_SET_GROUP_DOWN, IDC_SET_GROUP_ENABLE, IDC_SET_GROUP_LIST,
    IDC_SET_GROUP_RENAME, IDC_SET_GROUP_UP, IDC_SET_GROUP_VIEW_PHRASES, IDC_SET_GROUP_VIEW_RECORDS,
    IDC_SET_HK_RECORD, IDC_SET_HOVERPREVIEW, IDC_SET_IMAGE_PREVIEW, IDC_SET_MAX,
    IDC_SET_OCR_CLOUD_TOKEN, IDC_SET_OCR_CLOUD_URL, IDC_SET_OCR_PROVIDER,
    IDC_SET_OCR_WECHAT_DETECT, IDC_SET_OPEN_SOURCE, IDC_SET_OPEN_UPDATE, IDC_SET_PASTE_MOVE_TOP,
    IDC_SET_PASTE_SOUND_ENABLE, IDC_SET_PASTE_SOUND_KIND, IDC_SET_PASTE_SOUND_PICK,
    IDC_SET_PERSIST_SEARCH, IDC_SET_PLAIN_HK_ENABLE, IDC_SET_PLAIN_HK_KEY, IDC_SET_PLAIN_HK_MOD,
    IDC_SET_PLUGIN_DOWNLOADS, IDC_SET_PLUGIN_MAILMERGE, IDC_SET_POSMODE, IDC_SET_QUICK_DELETE,
    IDC_SET_SAVE, IDC_SET_SILENTSTART, IDC_SET_TRANSLATE_PROVIDER, IDC_SET_TRANSLATE_TARGET,
    IDC_SET_TRAYICON, IDC_SET_VV_GROUP, IDC_SET_VV_MODE, IDC_SET_VV_SOURCE, IID_IDATAOBJECT_RAW,
    RPC_E_CHANGED_MODE_HR, SCROLL_BAR_MARGIN, SCROLL_BAR_W, SCROLL_BAR_W_ACTIVE, SETTINGS_CLASS,
};
use crate::win_system_ui::{
    apply_dark_mode_to_window, apply_theme_to_menu, apply_window_corner_preference,
    caret_accessible_rect, create_drop_source, create_settings_button as settings_create_btn,
    create_settings_fonts, cursor_over_window_tree, draw_settings_nav_item,
    draw_settings_page_cards, draw_text_wide_centered, force_foreground_window, get_ctrl_text_wide,
    get_window_text, get_x_lparam, get_y_lparam, init_dark_mode_for_process,
    init_dpi_awareness_for_process, is_dark_mode, is_per_monitor_dpi_aware, layout_dpi_for_point,
    layout_dpi_for_window, monitor_dpi_for_window, nav_divider_x, nearest_monitor_rect_for_window,
    nearest_monitor_work_rect_for_point, nearest_monitor_work_rect_for_window,
    point_in_rect_screen, release_raw_com, scale_for_window, send_backspace_times, send_ctrl_v,
    set_settings_font as settings_set_font, settings_child_visible,
    settings_dropdown_index_for_max_items, settings_dropdown_index_for_pos_mode,
    settings_dropdown_label_for_max_items, settings_dropdown_label_for_pos_mode,
    settings_dropdown_max_items_from_label, settings_dropdown_pos_mode_from_label,
    settings_safe_paint_rect, settings_title_rect_win as settings_title_rect,
    settings_viewport_mask_rect, settings_viewport_rect, show_settings_dropdown_popup,
    system_mouse_hover_time_ms, to_wide, window_rect_for_dock, SettingsCtrlReg, SettingsPage,
    SettingsUiRegistry, WM_SETTINGS_DROPDOWN_SELECTED,
};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, BitBlt, ClientToScreen, CreateCompatibleBitmap, CreateCompatibleDC,
        CreateFontW, CreatePen, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
        FillRect, GetStockObject, IntersectClipRect, InvalidateRect, LineTo, MoveToEx,
        RedrawWindow, RestoreDC, RoundRect, SaveDC, ScreenToClient, SelectObject, SetBkColor,
        SetBkMode, SetTextColor, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
        DEFAULT_GUI_FONT, DIB_RGB_COLORS, NULL_PEN, PAINTSTRUCT, RDW_ALLCHILDREN, RDW_ERASE,
        RDW_INVALIDATE, RDW_UPDATENOW, SRCCOPY,
    },
    System::{
        DataExchange::{
            AddClipboardFormatListener, CloseClipboard, EmptyClipboard, EnumClipboardFormats,
            GetClipboardData, GetClipboardFormatNameW, GetClipboardOwner, OpenClipboard,
            RemoveClipboardFormatListener, SetClipboardData,
        },
        LibraryLoader::GetModuleHandleW,
        Ole::{DoDragDrop, OleInitialize, OleUninitialize, DROPEFFECT, DROPEFFECT_COPY},
    },
    UI::{
        Controls::{DRAWITEMSTRUCT, ODS_SELECTED},
        Input::KeyboardAndMouse::{
            keybd_event, GetAsyncKeyState, ReleaseCapture, SetCapture, SetFocus, KEYEVENTF_KEYUP,
            VK_BACK, VK_CONTROL, VK_DELETE, VK_DOWN, VK_END, VK_ESCAPE, VK_HOME, VK_INSERT,
            VK_LBUTTON, VK_LEFT, VK_LWIN, VK_MBUTTON, VK_MENU, VK_NEXT, VK_NUMPAD1, VK_NUMPAD9,
            VK_PRIOR, VK_RBUTTON, VK_RETURN, VK_RIGHT, VK_RWIN, VK_SHIFT, VK_SPACE, VK_TAB, VK_UP,
        },
        Shell::{
            DragQueryFileW, ILClone, ILCreateFromPathW, ILFindLastID, ILFree, SHCreateDataObject,
        },
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

const IDC_SEARCH: isize = 1001;
const ID_TIMER_STARTUP_RECOVERY: usize = 1;
const ID_TIMER_PASTE: usize = 2;
const ID_TIMER_SCROLL_FADE: usize = 3;
const ID_TIMER_SETTINGS_SCROLLBAR: usize = 4; // settings 滚动条自动隐藏
const ID_TIMER_EDGE_AUTO_HIDE: usize = 5;
const ID_TIMER_VV_SHOW: usize = 6;
const ID_TIMER_CLOUD_SYNC: usize = 7;
const ID_TIMER_SETTINGS_SAVE_HINT: usize = 8;
const ID_TIMER_OUTSIDE_HIDE: usize = 9;
const ID_TIMER_VV_WATCH: usize = 10;
const ID_TIMER_SEARCH_DEBOUNCE: usize = 11;
const ID_TIMER_HIDDEN_RECLAIM: usize = 12;
const ID_TIMER_CLIPBOARD_RETRY: usize = 13;
const ID_TIMER_DPI_FIT: usize = 14;
const ID_TIMER_SETTINGS_DPI_FIT: usize = 15;
const STARTUP_RECOVERY_TICKS: u8 = 24;
const LLKHF_LOWER_IL_INJECTED_FLAG: u32 = 0x0000_0002;
const WM_VV_SHOW: u32 = WM_APP + 20;
const WM_VV_HIDE: u32 = WM_APP + 21;
const WM_VV_SELECT: u32 = WM_APP + 22;
const WM_ITEMS_PAGE_READY: u32 = WM_APP + 30;
const WM_UPDATE_CHECK_READY: u32 = WM_APP + 31;
const WM_CLOUD_SYNC_READY: u32 = WM_APP + 33;
const WM_IMAGE_PASTE_READY: u32 = WM_APP + 34;
const WM_IMAGE_OCR_READY: u32 = WM_APP + 35;
const WM_TEXT_TRANSLATE_READY: u32 = WM_APP + 36;
const WM_IMAGE_THUMB_READY: u32 = WM_APP + 37;
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
const IDM_ROW_DELETE_UNPINNED: usize = 41016;
const IDM_ROW_IMAGE_OCR: usize = 41017;
const IDM_ROW_QR_IMAGE: usize = 41018;
const IDM_ROW_TEXT_TRANSLATE: usize = 41019;
const IDM_ROW_GROUP_BASE: usize = 41100;
const IDM_GROUP_FILTER_ALL: usize = 41200;
const IDM_GROUP_FILTER_BASE: usize = 41210;
const HOTKEY_ID: i32 = 1;
const HOTKEY_ID_PLAIN: i32 = 3;
const MAIN_UI_LAYOUT: MainUiLayout = MainUiLayout::zsclip();
const CLIPBOARD_IGNORE_MS_PASTE: u64 = 1800;
const CLIPBOARD_IGNORE_MS_DIRECT_EDIT: u64 = 600;
const TRANSIENT_DUPLICATE_CAPTURE_MS: u64 = 1200;
const TRANSIENT_TOP_DUPLICATE_MS: u64 = 2500;
const TRANSIENT_DUPLICATE_QUEUE_MS: u64 = 2500;
const EDGE_AUTO_HIDE_TIMER_MS: u32 = 120;
const EDGE_AUTO_HIDE_DELAY_MS: u64 = 650;
const EDGE_AUTO_HIDE_RESTORE_GRACE_MS: u64 = 450;
const EDGE_AUTO_HIDE_ANIM_MS: u64 = 180;
const EDGE_AUTO_HIDE_ANIM_TIMER_MS: u32 = 16;
const CLIPBOARD_RETRY_DELAY_MS: u32 = 140;
const CLIPBOARD_RETRY_MAX_ATTEMPTS: u8 = 5;
const PIXPIN_CLIPBOARD_RETRY_MAX_ATTEMPTS: u8 = 18;
const MAX_CAPTURE_PIXELS: usize = 16_000_000;
const MAX_CAPTURE_SIDE: usize = 8192;
const MAX_CLIPBOARD_BITMAP_DECODE_PIXELS: usize = MAX_CAPTURE_PIXELS * 2;

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

type AppResult<T> = Result<T, io::Error>;

struct ImagePasteReadyResult {
    image: Option<(Vec<u8>, usize, usize)>,
    target: isize,
    hide_main: bool,
    backspaces: u8,
}

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;

struct ImageOcrReadyResult {
    text: Option<String>,
    error: Option<String>,
}

struct ImageThumbReadyResult {
    item_id: i64,
    image: Option<ImageThumbnail>,
}

struct TextTranslateReadyResult {
    text: Option<String>,
    error: Option<String>,
}

unsafe fn start_flagged_timer(hwnd: HWND, timer_id: usize, period_ms: u32, flag: &mut bool) {
    *flag = true;
    SetTimer(hwnd, timer_id, period_ms, None);
}

unsafe fn stop_flagged_timer(hwnd: HWND, timer_id: usize, flag: &mut bool) {
    KillTimer(hwnd, timer_id);
    *flag = false;
}

pub(crate) unsafe fn post_boxed_message<T>(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    payload: Box<T>,
) -> bool {
    let raw = Box::into_raw(payload);
    if hwnd.is_null() || IsWindow(hwnd) == 0 || PostMessageW(hwnd, msg, wparam, raw as LPARAM) == 0
    {
        drop(Box::from_raw(raw));
        return false;
    }
    true
}

unsafe fn trim_process_working_set() {
    let process = GetCurrentProcess();
    if !process.is_null() {
        let _ = EmptyWorkingSet(process);
    }
}

unsafe fn schedule_hidden_memory_reclaim(hwnd: HWND, state: &mut AppState) {
    // 回归保护：不要再改回“窗口一隐藏就立刻清缓存”，那会让快速显示/隐藏反复抖动。
    start_flagged_timer(
        hwnd,
        ID_TIMER_HIDDEN_RECLAIM,
        1800,
        &mut state.hidden_reclaim_timer,
    );
}

unsafe fn cancel_hidden_memory_reclaim(hwnd: HWND, state: &mut AppState) {
    stop_flagged_timer(
        hwnd,
        ID_TIMER_HIDDEN_RECLAIM,
        &mut state.hidden_reclaim_timer,
    );
}

unsafe fn reset_clipboard_retry(hwnd: HWND, state: &mut AppState) {
    if state.clipboard_retry_timer {
        stop_flagged_timer(
            hwnd,
            ID_TIMER_CLIPBOARD_RETRY,
            &mut state.clipboard_retry_timer,
        );
    }
    state.clipboard_retry_sequence = 0;
    state.clipboard_retry_attempts = 0;
}

unsafe fn clipboard_has_named_format(target: &str) -> bool {
    if OpenClipboard(null_mut()) == 0 {
        return false;
    }
    let mut format = 0u32;
    let mut found = false;
    loop {
        format = EnumClipboardFormats(format);
        if format == 0 {
            break;
        }
        let mut buf = [0u16; 128];
        let len = GetClipboardFormatNameW(format, buf.as_mut_ptr(), buf.len() as i32);
        if len > 0 {
            let name = String::from_utf16_lossy(&buf[..len as usize]);
            if name.eq_ignore_ascii_case(target) {
                found = true;
                break;
            }
        }
    }
    let _ = CloseClipboard();
    found
}

unsafe fn schedule_clipboard_retry_with_limit(
    hwnd: HWND,
    state: &mut AppState,
    sequence: u32,
    max_attempts: u8,
) -> bool {
    if sequence == 0 {
        return false;
    }
    if state.clipboard_retry_sequence != sequence {
        reset_clipboard_retry(hwnd, state);
        state.clipboard_retry_sequence = sequence;
    }
    let limit = max_attempts.max(CLIPBOARD_RETRY_MAX_ATTEMPTS);
    if state.clipboard_retry_attempts >= limit {
        return false;
    }
    state.clipboard_retry_attempts += 1;
    if state.clipboard_retry_timer {
        SetTimer(
            hwnd,
            ID_TIMER_CLIPBOARD_RETRY,
            CLIPBOARD_RETRY_DELAY_MS,
            None,
        );
    } else {
        start_flagged_timer(
            hwnd,
            ID_TIMER_CLIPBOARD_RETRY,
            CLIPBOARD_RETRY_DELAY_MS,
            &mut state.clipboard_retry_timer,
        );
    }
    true
}

fn source_app_prefers_long_clipboard_retry(source_app: &str) -> bool {
    let source = source_app.trim().to_ascii_lowercase();
    source.contains("pixpin")
}

unsafe fn show_main_scrollbar_feedback(hwnd: HWND, state: &mut AppState, erase: bool) {
    state.scroll_fade_alpha = 255;
    start_flagged_timer(hwnd, ID_TIMER_SCROLL_FADE, 50, &mut state.scroll_fade_timer);
    InvalidateRect(hwnd, null(), erase as i32);
}

fn main_layout_for_dpi(dpi: u32) -> MainUiLayout {
    MAIN_UI_LAYOUT.scaled(dpi.max(96))
}

fn main_window_size_for_dpi(dpi: u32) -> (i32, i32) {
    let layout = main_layout_for_dpi(dpi);
    (layout.win_w, layout.list_y + layout.list_h + 7)
}

unsafe fn main_layout_for_window(hwnd: HWND) -> MainUiLayout {
    main_layout_for_dpi(layout_dpi_for_window(hwnd))
}

fn clamp_window_pos_to_rect(x: i32, y: i32, rc: &RECT, win_w: i32, win_h: i32) -> (i32, i32) {
    let max_x = max(rc.left, rc.right - win_w);
    let max_y = max(rc.top, rc.bottom - win_h);
    (max(rc.left, min(x, max_x)), max(rc.top, min(y, max_y)))
}

fn dpi_compensated_size(
    base_w: i32,
    base_h: i32,
    base_monitor_dpi: u32,
    monitor_dpi: u32,
) -> (i32, i32) {
    let base_monitor_dpi = base_monitor_dpi.max(96) as i64;
    let monitor_dpi = monitor_dpi.max(96) as i64;
    let w = (((base_w.max(1) as i64) * base_monitor_dpi) + (monitor_dpi / 2)) / monitor_dpi;
    let h = (((base_h.max(1) as i64) * base_monitor_dpi) + (monitor_dpi / 2)) / monitor_dpi;
    (w.max(1) as i32, h.max(1) as i32)
}

const EDGE_AUTO_HIDE_PEEK: i32 = 2;
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
const VV_POPUP_MENU_GRACE_MS: u64 = 900;
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

fn vv_choose_overlay_edge(
    top: i32,
    bottom: i32,
    popup_height: i32,
    work_area: &RECT,
) -> (i32, bool) {
    let below_space = work_area.bottom - bottom;
    let above_space = top - work_area.top;
    let align_bottom = below_space < popup_height && above_space > below_space;
    let edge_y = if align_bottom { top } else { bottom };
    (edge_y, align_bottom)
}

fn vv_anchor_within(
    anchor: &VvOverlayAnchor,
    reference: &VvOverlayAnchor,
    max_dx: i32,
    max_dy: i32,
) -> bool {
    (anchor.left - reference.left).abs() <= max_dx
        && (anchor.edge_y - reference.edge_y).abs() <= max_dy
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
        if vv_anchor_within(
            anchor,
            caret,
            VV_IMM_POINT_MAX_X_DRIFT,
            VV_IMM_POINT_MAX_Y_DRIFT,
        ) {
            return true;
        }
        if let Some(focus) = focus_anchor {
            return vv_anchor_within(
                anchor,
                focus,
                VV_IMM_POINT_MAX_X_DRIFT + 60,
                VV_IMM_POINT_MAX_Y_DRIFT + 40,
            );
        }
        return false;
    }
    if let Some(focus) = focus_anchor {
        return vv_anchor_within(
            anchor,
            focus,
            VV_IMM_POINT_MAX_X_DRIFT + 60,
            VV_IMM_POINT_MAX_Y_DRIFT + 40,
        );
    }
    true
}

unsafe fn vv_popup_row_rect(row: usize) -> RECT {
    let top = VV_POPUP_HEADER_H + 10 + row as i32 * VV_POPUP_ROW_H;
    RECT {
        left: 12,
        top,
        right: VV_POPUP_W - 12,
        bottom: top + VV_POPUP_ROW_H - 2,
    }
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
    if group_id > 0
        && state
            .groups_for_tab(source_tab)
            .iter()
            .any(|g| g.id == group_id)
    {
        group_id
    } else {
        0
    }
}

fn vv_popup_group_name(state: &AppState) -> String {
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    let all_label = if source_tab == 0 {
        "全部记录"
    } else {
        "全部短语"
    };
    group_name_for_display(
        state.groups_for_tab(source_tab),
        state.vv_popup_group_id,
        all_label,
    )
}

fn vv_popup_rebuild_items(state: &mut AppState) {
    let group_id = vv_popup_resolved_group_id(state, state.vv_popup_group_id);
    state.vv_popup_group_id = group_id;
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    state.vv_popup_items = db_load_vv_popup_items(source_tab as i64, group_id, VV_POPUP_MAX_ITEMS)
        .into_iter()
        .enumerate()
        .map(|(i, item)| VvPopupEntry { index: i + 1, item })
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
    let all_flags = if current_group_id == 0 {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    AppendMenuW(
        menu,
        all_flags,
        IDM_GROUP_FILTER_ALL,
        to_wide(
            translate(if source_tab == 0 {
                "全部记录"
            } else {
                "全部短语"
            })
            .as_ref(),
        )
        .as_ptr(),
    );
    if !groups.is_empty() {
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        for (idx, g) in groups.iter().enumerate() {
            let flags = if current_group_id == g.id {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING
            };
            AppendMenuW(
                menu,
                flags,
                IDM_GROUP_FILTER_BASE + idx,
                to_wide(&g.name).as_ptr(),
            );
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
    let mut tl = POINT {
        x: rc.left,
        y: rc.top,
    };
    let mut br = POINT {
        x: rc.right,
        y: rc.bottom,
    };
    if vv_point_to_screen(hwnd, &mut tl) && vv_point_to_screen(hwnd, &mut br) {
        return Some(RECT {
            left: tl.x,
            top: tl.y,
            right: br.x,
            bottom: br.y,
        });
    }
    None
}

unsafe fn vv_thread_caret_anchor(
    target: HWND,
    popup_height: i32,
    work_area: &RECT,
) -> Option<VvOverlayAnchor> {
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

    let mut top_left = POINT {
        x: info.rcCaret.left,
        y: info.rcCaret.top,
    };
    let mut bottom_left = POINT {
        x: info.rcCaret.left,
        y: if info.rcCaret.bottom > info.rcCaret.top {
            info.rcCaret.bottom
        } else {
            info.rcCaret.top + 24
        },
    };
    if !vv_point_to_screen(anchor_hwnd, &mut top_left)
        || !vv_point_to_screen(anchor_hwnd, &mut bottom_left)
    {
        return None;
    }

    let below_space = work_area.bottom - bottom_left.y;
    let above_space = top_left.y - work_area.top;
    let align_bottom = below_space < popup_height && above_space > below_space;
    let edge_y = if align_bottom {
        top_left.y
    } else {
        bottom_left.y
    };
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
    let (edge_y, align_bottom) = vv_choose_overlay_edge(rc.top, rc.bottom, popup_height, work_area);
    Some(VvOverlayAnchor {
        left: rc.left,
        edge_y,
        align_bottom,
        exact_rect: true,
    })
}

unsafe fn vv_imm_overlay_anchor(
    focus_hwnd: HWND,
    popup_height: i32,
    work_area: &RECT,
) -> Option<VvOverlayAnchor> {
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
            rc_area: RECT {
                left: 0,
                top: 0,
                right: 0,
                bottom: 0,
            },
        };
        if SendMessageW(
            ime_hwnd,
            WM_IME_CONTROL,
            IMC_GETCANDIDATEPOS,
            &mut cand as *mut _ as LPARAM,
        ) != 0
        {
            continue;
        }
        let mut pt = cand.pt_current_pos;
        let pt_ok = vv_point_to_screen(focus_hwnd, &mut pt);
        if cand.dw_style == CFS_CANDIDATEPOS_V && pt_ok {
            let (edge_y, align_bottom) =
                vv_choose_overlay_edge(pt.y, pt.y, popup_height, work_area);
            return Some(VvOverlayAnchor {
                left: pt.x,
                edge_y,
                align_bottom,
                exact_rect: false,
            });
        }
        if cand.dw_style == CFS_EXCLUDE_V && vv_rect_has_area(&cand.rc_area) {
            if let Some(exclude_rc) = vv_client_rect_to_screen(focus_hwnd, &cand.rc_area) {
                let (edge_y, align_bottom) = vv_choose_overlay_edge(
                    exclude_rc.top,
                    exclude_rc.bottom,
                    popup_height,
                    work_area,
                );
                return Some(VvOverlayAnchor {
                    left: exclude_rc.left,
                    edge_y,
                    align_bottom,
                    exact_rect: true,
                });
            }
        }
    }

    let mut comp = CompositionForm {
        dw_style: 0,
        pt_current_pos: POINT { x: 0, y: 0 },
        rc_area: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
    };
    if SendMessageW(
        ime_hwnd,
        WM_IME_CONTROL,
        IMC_GETCOMPOSITIONWINDOW,
        &mut comp as *mut _ as LPARAM,
    ) == 0
    {
        let mut pt = comp.pt_current_pos;
        let pt_ok = vv_point_to_screen(focus_hwnd, &mut pt);
        if (comp.dw_style == CFS_POINT_V || comp.dw_style == CFS_FORCE_POSITION_V) && pt_ok {
            let (edge_y, align_bottom) =
                vv_choose_overlay_edge(pt.y, pt.y, popup_height, work_area);
            return Some(VvOverlayAnchor {
                left: pt.x,
                edge_y,
                align_bottom,
                exact_rect: false,
            });
        }
        if comp.dw_style == CFS_RECT_V && vv_rect_has_area(&comp.rc_area) {
            if let Some(rc) = vv_client_rect_to_screen(focus_hwnd, &comp.rc_area) {
                let (edge_y, align_bottom) =
                    vv_choose_overlay_edge(rc.top, rc.bottom, popup_height, work_area);
                return Some(VvOverlayAnchor {
                    left: rc.left,
                    edge_y,
                    align_bottom,
                    exact_rect: true,
                });
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
    if width <= 0 || height <= 0 || height > 180 || width > (work_area.right - work_area.left) - 40
    {
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
    if thread_id != 0 && GetGUIThreadInfo(thread_id, &mut info) != 0 && !info.hwndFocus.is_null() {
        return info.hwndFocus;
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
    let imm_anchor = vv_imm_overlay_anchor(focus_hwnd, height, &wa).filter(|anchor| {
        vv_imm_point_anchor_is_plausible(anchor, caret_anchor.as_ref(), focus_anchor.as_ref())
    });
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
    let focus_hwnd = vv_focus_hwnd_for_target(target);
    let ime_replaced_trigger = if focus_hwnd.is_null() {
        false
    } else {
        let work_area = nearest_monitor_work_rect_for_window(focus_hwnd);
        vv_imm_overlay_anchor(
            focus_hwnd,
            vv_popup_height(state.vv_popup_items.len()),
            &work_area,
        )
        .is_some()
    };
    send_escape_key();
    state.vv_popup_replaces_ime = ime_replaced_trigger;
    InvalidateRect(popup, null(), 1);
    ShowWindow(popup, SW_SHOWNOACTIVATE);
    true
}

unsafe fn vv_digit_index_from_vk(vk: u32) -> Option<usize> {
    match vk {
        0x31..=0x39 => Some((vk - 0x31) as usize),
        x if x >= VK_NUMPAD1 as u32 && x <= VK_NUMPAD9 as u32 => {
            Some((x - VK_NUMPAD1 as u32) as usize)
        }
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

unsafe fn process_image_name(pid: u32) -> String {
    if pid == 0 {
        return String::new();
    }
    let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
    if handle.is_null() {
        return String::new();
    }
    let mut size: u32 = 512;
    let mut buf = vec![0u16; size as usize + 1];
    let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
    let _ = CloseHandle(handle);
    if ok == 0 || size == 0 {
        return String::new();
    }
    let path = String::from_utf16_lossy(&buf[..size as usize]);
    Path::new(&path)
        .file_name()
        .map(|name| name.to_string_lossy().to_lowercase())
        .unwrap_or_default()
}

unsafe fn clipboard_source_app_name() -> String {
    let owner = GetClipboardOwner();
    let owner_root = if owner.is_null() {
        owner
    } else {
        GetAncestor(owner, GA_ROOT)
    };
    if !owner_root.is_null() && !is_app_window(owner_root) {
        let name = window_process_name(owner_root);
        if !name.is_empty() {
            return name;
        }
    }
    let fg = GetForegroundWindow();
    if fg.is_null() || is_app_window(fg) {
        return String::new();
    }
    let mut pid = 0;
    let _ = GetWindowThreadProcessId(fg, &mut pid);
    process_image_name(pid)
}

unsafe fn window_process_name(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut pid = 0u32;
    let _ = GetWindowThreadProcessId(hwnd, &mut pid);
    process_image_name(pid)
}

fn is_self_clipboard_source_app(source_app: &str) -> bool {
    let source = source_app.trim().to_ascii_lowercase();
    if source.is_empty() {
        return false;
    }
    if matches!(source.as_str(), "zsclip.exe" | "剪贴板.exe") {
        return true;
    }
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .map(|name| name.trim().to_ascii_lowercase() == source)
        .unwrap_or(false)
}

unsafe fn vv_window_class_name(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut buf = [0u16; 128];
    let len = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

fn vv_is_word_process(process_name: &str) -> bool {
    let process = process_name.trim().to_ascii_lowercase();
    process == "winword.exe" || process == "winword" || process.contains("winword")
}

fn vv_is_word_document_class(class_name: &str) -> bool {
    let class_name = class_name.trim().to_ascii_lowercase();
    class_name.starts_with("_ww")
}

fn vv_word_target_is_text_input_ready(
    process_name: &str,
    target_cls: &str,
    focus_cls: &str,
) -> bool {
    if !vv_is_word_process(process_name) {
        return false;
    }
    if vv_is_word_document_class(target_cls) || vv_is_word_document_class(focus_cls) {
        return true;
    }
    target_cls.eq_ignore_ascii_case("opusapp")
        && (focus_cls.is_empty() || focus_cls.eq_ignore_ascii_case("opusapp"))
}

unsafe fn vv_target_is_text_input_ready(target: HWND) -> bool {
    if target.is_null() || IsWindow(target) == 0 {
        return false;
    }
    let target_cls = vv_window_class_name(target).to_ascii_lowercase();
    let process_name = window_process_name(target);
    let thread_id = GetWindowThreadProcessId(target, null_mut());
    if thread_id == 0 {
        return vv_word_target_is_text_input_ready(&process_name, &target_cls, "");
    }
    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = size_of::<GUITHREADINFO>() as u32;
    if GetGUIThreadInfo(thread_id, &mut info) == 0 {
        return vv_word_target_is_text_input_ready(&process_name, &target_cls, "");
    }
    let focus = if !info.hwndFocus.is_null() {
        info.hwndFocus
    } else {
        target
    };
    let cls = vv_window_class_name(focus).to_ascii_lowercase();
    let dlg_code = SendMessageW(focus, WM_GETDLGCODE, 0, 0) as u32;
    if matches!(
        cls.as_str(),
        "edit"
            | "richedit20w"
            | "richedit50w"
            | "scintilla"
            | "chrome_renderwidgethosthwnd"
            | "chrome_widgetwin_0"
            | "chrome_widgetwin_1"
            | "mozillawindowclass"
            | "windows.ui.composition.desktopwindowcontentbridge"
    ) {
        return true;
    }
    if matches!(
        target_cls.as_str(),
        "chrome_widgetwin_0"
            | "chrome_widgetwin_1"
            | "windows.ui.composition.desktopwindowcontentbridge"
    ) && (process_name.contains("codex") || process_name.contains("chatgpt"))
    {
        return true;
    }
    if vv_word_target_is_text_input_ready(&process_name, &target_cls, &cls) {
        return true;
    }
    if (dlg_code & (DLGC_HASSETSEL | DLGC_WANTCHARS | DLGC_WANTTAB | DLGC_WANTARROWS)) != 0 {
        return true;
    }
    let ime_hwnd = ImmGetDefaultIMEWnd(focus);
    if !ime_hwnd.is_null()
        && IsWindow(ime_hwnd) != 0
        && (process_name.contains("codex")
            || process_name.contains("chatgpt")
            || process_name.contains("cursor")
            || process_name.contains("code"))
    {
        return true;
    }
    if caret_accessible_rect(focus).is_some() {
        return true;
    }
    !info.hwndCaret.is_null()
}

unsafe extern "system" fn vv_keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code < 0 || (wparam as u32 != WM_KEYDOWN && wparam as u32 != WM_SYSKEYDOWN) {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    let data = &*(lparam as *const KBDLLHOOKSTRUCT);
    if (data.flags & (LLKHF_INJECTED_FLAG | LLKHF_LOWER_IL_INJECTED_FLAG)) != 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let Ok(mut hook) = vv_hook_state().try_lock() else {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    };
    if !hook.enabled || hook.main_hwnd == 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    let main_hwnd = hook.main_hwnd as HWND;
    let trigger_vk = hook.trigger_vk;
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

    if data.vkCode == trigger_vk {
        let within_timeout = hook
            .last_v_at
            .map(|t| t.elapsed().as_millis() <= VV_TRIGGER_TIMEOUT_MS)
            .unwrap_or(false);
        if hook.last_was_v && hook.last_v_target == fg as isize && within_timeout {
            let target = if !fg.is_null() && IsWindow(fg) != 0 {
                fg
            } else {
                hook.last_v_target as HWND
            };
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            if !target.is_null() && IsWindow(target) != 0 && vv_target_is_text_input_ready(target) {
                PostMessageW(main_hwnd, WM_VV_SHOW, target as usize, 0);
            }
        } else {
            if vv_target_is_text_input_ready(fg) {
                hook.last_was_v = true;
                hook.last_v_target = fg as isize;
                hook.last_v_at = Some(Instant::now());
            } else {
                hook.last_was_v = false;
                hook.last_v_target = 0;
                hook.last_v_at = None;
            }
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
        hook_state.trigger_vk = b'V' as u32;
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
            *handle = SetWindowsHookExW(
                WH_KEYBOARD_LL,
                Some(vv_keyboard_hook_proc),
                GetModuleHandleW(null()),
                0,
            ) as isize;
        }
    } else if *handle != 0 {
        UnhookWindowsHookEx(*handle as _);
        *handle = 0;
    }
}

unsafe extern "system" fn vv_popup_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
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

                let title_rc = RECT {
                    left: 14,
                    top: 10,
                    right: 150,
                    bottom: 30,
                };
                draw_text_ex(
                    hdc as _,
                    tr("VV 模式", "VV Mode"),
                    &title_rc,
                    th.text,
                    13,
                    true,
                    false,
                    "Segoe UI Variable Display",
                );
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
                draw_text_ex(
                    hdc as _,
                    "v",
                    &arrow_rc,
                    th.text_muted,
                    11,
                    true,
                    true,
                    "Segoe UI Variable Text",
                );

                let sub_rc = RECT {
                    left: 14,
                    top: 34,
                    right: rc.right - 14,
                    bottom: 52,
                };
                draw_text_ex(
                    hdc as _,
                    tr(
                        "输入 1-9 直接粘贴，Esc 取消",
                        "Press 1-9 to paste, Esc to cancel",
                    ),
                    &sub_rc,
                    th.text_muted,
                    11,
                    false,
                    false,
                    "Segoe UI Variable Text",
                );

                if state.vv_popup_items.is_empty() {
                    let empty_rc = RECT {
                        left: 16,
                        top: VV_POPUP_HEADER_H + 16,
                        right: rc.right - 16,
                        bottom: VV_POPUP_HEADER_H + 48,
                    };
                    draw_text_ex(
                        hdc as _,
                        tr("当前分组暂无记录", "No records in this group"),
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
                        let bubble = RECT {
                            left: row_rc.left,
                            top: row_rc.top + 4,
                            right: row_rc.left + 24,
                            bottom: row_rc.top + 24,
                        };
                        draw_round_fill(hdc as _, &bubble, th.accent, 8);
                        draw_text_ex(
                            hdc as _,
                            &entry.index.to_string(),
                            &bubble,
                            rgb(255, 255, 255),
                            11,
                            true,
                            true,
                            "Segoe UI Variable Text",
                        );

                        let mut text_rc = row_rc;
                        text_rc.left += 34;
                        let label = if entry.item.kind == ClipKind::Image {
                            format_created_at_local(&entry.item.created_at, &entry.item.preview)
                        } else {
                            entry.item.preview.clone()
                        };
                        draw_text_ex(
                            hdc as _,
                            &label,
                            &text_rc,
                            th.text,
                            12,
                            false,
                            false,
                            "Segoe UI Variable Text",
                        );
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
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
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
        if ch.is_ascii_alphabetic() {
            return ch.to_ascii_uppercase() as u32;
        }
        if ch.is_ascii_digit() {
            return ch as u32;
        }
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
    if !settings.quick_search_enabled {
        return;
    }
    let mut raw = text.trim().to_string();
    if raw.is_empty() {
        return;
    }
    if raw.chars().count() > 200 {
        raw = raw.chars().take(200).collect();
    }
    let q = raw.replace(['\r', '\n'], " ");
    let enc = url_encode_component(&q);
    let raw_enc = url_encode_component(&raw);
    let tpl = if settings.search_template.trim().is_empty() {
        search_engine_template(&settings.search_engine).to_string()
    } else {
        settings.search_template.clone()
    };
    let url = tpl
        .replace("{key}", &enc)
        .replace("{q}", &enc)
        .replace("{raw}", &raw_enc);
    open_path_with_shell(&url);
}

fn url_encode_component(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 3 / 2);
    for b in s.as_bytes() {
        match *b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*b as char)
            }
            b' ' => out.push('+'),
            _ => out.push_str(&format!("%{:02X}", *b)),
        }
    }
    out
}

#[derive(Clone, Copy, Default)]
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
    pub(crate) ui_dpi: u32,
    pub(crate) dpi_comp_base_w: i32,
    pub(crate) dpi_comp_base_h: i32,
    pub(crate) dpi_comp_base_monitor_dpi: u32,
    pub(crate) dpi_comp_last_monitor_dpi: u32,
    pub(crate) dpi_comp_applying: bool,
    pub(crate) search_font: *mut core::ffi::c_void,
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
    pub(crate) last_capture_signature: String,
    pub(crate) last_capture_source_app: String,
    pub(crate) recent_capture_signatures: VecDeque<(String, String, Instant)>,
    pub(crate) last_capture_at: Option<Instant>,
    pub(crate) last_clipboard_seq: u32,
    pub(crate) ignore_clipboard_until: Option<Instant>,
    pub(crate) skip_next_clipboard_update_once: bool,
    pub(crate) recent_programmatic_clipboard_signature: String,
    pub(crate) recent_programmatic_clipboard_until: Option<Instant>,
    pub(crate) settings: AppSettings,
    pub(crate) tray_icon_registered: bool,
    pub(crate) hotkey_registered: bool,
    pub(crate) plain_paste_hotkey_registered: bool,
    pub(crate) hotkey_conflict_notified: bool,
    pub(crate) startup_recovery_ticks: u8,
    pub(crate) settings_hwnd: HWND,
    pub(crate) hover_scroll: bool,      // 鼠标是否在滚动条区域
    pub(crate) scroll_fade_alpha: u8,   // 滚动条透明度 0-255
    pub(crate) scroll_fade_timer: bool, // 渐隐 timer 是否运行中
    pub(crate) search_debounce_timer: bool,
    pub(crate) hidden_reclaim_timer: bool,
    pub(crate) clipboard_retry_timer: bool,
    pub(crate) clipboard_retry_sequence: u32,
    pub(crate) clipboard_retry_attempts: u8,
    pub(crate) scroll_dragging: bool,
    pub(crate) scroll_drag_start_y: i32,
    pub(crate) scroll_drag_start_scroll: i32,
    pub(crate) hover_to_top: bool,
    pub(crate) down_to_top: bool,
    tab_loads: [TabLoadState; 2],
    payload_cache: ItemPayloadCache,
    image_thumb_cache: ImageThumbnailCache,
    image_thumb_loading: HashSet<i64>,
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
    pub(crate) hotkey_passthrough_focus: HWND,
    pub(crate) hotkey_passthrough_edit: HWND,
    pub(crate) plain_text_paste_mode: bool,
    pub(crate) main_window_noactivate: bool,
    pub(crate) edge_hidden: bool,
    pub(crate) edge_hidden_side: i32,
    pub(crate) edge_restore_x: i32,
    pub(crate) edge_restore_y: i32,
    pub(crate) edge_docked_left: i32,
    pub(crate) edge_docked_top: i32,
    pub(crate) edge_docked_right: i32,
    pub(crate) edge_docked_bottom: i32,
    pub(crate) edge_hide_armed: bool,
    pub(crate) edge_hide_pending_until: Option<Instant>,
    pub(crate) edge_hide_grace_until: Option<Instant>,
    pub(crate) edge_restore_wait_leave: bool,
    pub(crate) edge_anim_from_x: i32,
    pub(crate) edge_anim_from_y: i32,
    pub(crate) edge_anim_to_x: i32,
    pub(crate) edge_anim_to_y: i32,
    pub(crate) edge_anim_until: Option<Instant>,
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
    fn is_recent_top_duplicate_item(&self, item: &ClipItem) -> bool {
        let Some(at) = self.last_capture_at else {
            return false;
        };
        if Instant::now().saturating_duration_since(at).as_millis()
            > TRANSIENT_TOP_DUPLICATE_MS as u128
        {
            return false;
        }
        let Some(top) = self.records.first() else {
            return false;
        };
        if top.kind != item.kind {
            return false;
        }
        match item.kind {
            ClipKind::Text | ClipKind::Phrase => top.text == item.text,
            ClipKind::Files => top.file_paths == item.file_paths,
            ClipKind::Image => false,
        }
    }

    fn should_skip_transient_duplicate_capture(
        &mut self,
        signature: &str,
        source_app: &str,
        sequence: u32,
    ) -> bool {
        if signature.is_empty() {
            return false;
        }
        let now = Instant::now();
        while self
            .recent_capture_signatures
            .front()
            .map(|(_, _, at)| {
                now.saturating_duration_since(*at).as_millis()
                    > TRANSIENT_DUPLICATE_QUEUE_MS as u128
            })
            .unwrap_or(false)
        {
            self.recent_capture_signatures.pop_front();
        }
        let same_seq = sequence != 0 && self.last_clipboard_seq == sequence;
        let same_source_app = self.last_capture_source_app.is_empty()
            || source_app.is_empty()
            || self.last_capture_source_app == source_app;
        let same_sig = same_source_app
            && self.last_capture_signature == signature
            && self
                .last_capture_at
                .map(|at| {
                    now.saturating_duration_since(at).as_millis()
                        <= TRANSIENT_DUPLICATE_CAPTURE_MS as u128
                })
                .unwrap_or(false);
        let recent_sig = self
            .recent_capture_signatures
            .iter()
            .rev()
            .any(|(sig, app, at)| {
                sig == signature
                    && (app.is_empty() || source_app.is_empty() || app == source_app)
                    && now.saturating_duration_since(*at).as_millis()
                        <= TRANSIENT_DUPLICATE_CAPTURE_MS as u128
            });
        if sequence != 0 {
            self.last_clipboard_seq = sequence;
        }
        self.last_capture_signature.clear();
        self.last_capture_signature.push_str(signature);
        self.last_capture_source_app.clear();
        self.last_capture_source_app.push_str(source_app);
        self.last_capture_at = Some(now);
        self.recent_capture_signatures.push_back((
            signature.to_string(),
            source_app.to_string(),
            now,
        ));
        while self.recent_capture_signatures.len() > 8 {
            self.recent_capture_signatures.pop_front();
        }
        same_seq || same_sig || recent_sig
    }

    fn note_programmatic_clipboard_signature(&mut self, signature: String, ms: u64) {
        self.recent_programmatic_clipboard_signature = signature;
        self.recent_programmatic_clipboard_until =
            Some(Instant::now() + std::time::Duration::from_millis(ms.max(1)));
    }

    fn consume_skip_next_clipboard_update_once(&mut self, sequence: u32) -> bool {
        if !self.skip_next_clipboard_update_once {
            return false;
        }
        self.skip_next_clipboard_update_once = false;
        if sequence != 0 {
            self.last_clipboard_seq = sequence;
        }
        true
    }

    fn consume_recent_programmatic_clipboard_signature(&mut self, signature: &str) -> bool {
        if signature.is_empty() || self.recent_programmatic_clipboard_signature.is_empty() {
            return false;
        }
        let now = Instant::now();
        let still_active = self
            .recent_programmatic_clipboard_until
            .map(|until| now < until)
            .unwrap_or(false);
        if !still_active {
            self.recent_programmatic_clipboard_signature.clear();
            self.recent_programmatic_clipboard_until = None;
            return false;
        }
        if self.recent_programmatic_clipboard_signature == signature {
            self.recent_programmatic_clipboard_signature.clear();
            self.recent_programmatic_clipboard_until = None;
            return true;
        }
        false
    }

    fn delete_selected_rows(&mut self) {
        let ids = self.selected_db_ids();
        if ids.is_empty() {
            self.delete_selected();
            return;
        }
        let anchor = self.current_scroll_anchor();
        for &id in &ids {
            let _ = db_delete_item(id);
            self.remove_cached_item(id);
        }
        self.remove_items_from_active_tab(&ids);
        self.clear_selection();
        self.refilter();
        self.restore_scroll_anchor(anchor);
        unsafe {
            sync_peer_windows_from_db(self.hwnd);
        }
    }

    fn toggle_pin_rows(&mut self) {
        let src = self.selected_source_indices();
        if src.is_empty() {
            self.toggle_pin_selected();
            return;
        }
        let make_pinned = src
            .iter()
            .filter_map(|&i| self.active_items().get(i))
            .any(|it| !it.pinned);
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
        unsafe {
            sync_peer_windows_from_db(self.hwnd);
        }
    }

    fn selected_items_owned(&self) -> Vec<ClipItem> {
        self.selected_source_indices()
            .into_iter()
            .filter_map(|i| self.active_items().get(i).cloned())
            .collect()
    }

    fn clear_payload_cache(&mut self) {
        self.payload_cache.clear();
        self.image_thumb_cache.clear();
        self.image_thumb_loading.clear();
    }

    fn cache_full_item(&mut self, item: ClipItem) {
        self.payload_cache.put(item);
    }

    fn remove_cached_item(&mut self, id: i64) {
        self.payload_cache.remove(id);
        self.image_thumb_cache.remove(id);
        self.image_thumb_loading.remove(&id);
    }

    fn current_scroll_anchor(&self) -> Option<(i64, i32)> {
        let row_h = self.layout().row_h.max(1);
        let top_visible = (self.scroll_y / row_h).max(0) as usize;
        let offset = self.scroll_y - (top_visible as i32 * row_h);
        let item = self.active_items().get(top_visible)?;
        if item.id > 0 {
            Some((item.id, offset))
        } else {
            None
        }
    }

    fn restore_scroll_anchor(&mut self, anchor: Option<(i64, i32)>) {
        if let Some((id, offset)) = anchor {
            let row_h = self.layout().row_h.max(1);
            if let Some(visible_idx) = self.active_items().iter().position(|item| item.id == id) {
                self.scroll_y = visible_idx as i32 * row_h + offset;
            }
        }
        self.clamp_scroll();
        self.maybe_request_more_for_active_tab();
    }

    fn reload_state_from_db_preserve_scroll(&mut self, anchor: Option<(i64, i32)>) {
        reload_state_from_db_persisting(self);
        self.restore_scroll_anchor(anchor);
    }

    fn remove_items_from_active_tab(&mut self, ids: &[i64]) {
        if ids.is_empty() {
            return;
        }
        let id_set: HashSet<i64> = ids.iter().copied().collect();
        self.items_for_tab_mut(self.tab_index)
            .retain(|item| !id_set.contains(&item.id));
    }

    fn remove_duplicate_history_items(&mut self, ids: &[i64]) {
        if ids.is_empty() {
            return;
        }
        let id_set: HashSet<i64> = ids.iter().copied().collect();
        self.records.retain(|item| !id_set.contains(&item.id));
        self.phrases.retain(|item| !id_set.contains(&item.id));
    }

    fn promote_loaded_item_to_top(&mut self, old_id: i64, new_id: i64) -> bool {
        if old_id <= 0 || new_id <= 0 {
            return false;
        }
        let items = self.items_for_tab_mut(self.tab_index);
        let Some(pos) = items.iter().position(|item| item.id == old_id) else {
            return false;
        };
        let mut item = items.remove(pos);
        item.id = new_id;
        items.insert(0, clip_item_to_summary(&item));
        true
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
        items
            .into_iter()
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
        if self.is_recent_top_duplicate_item(&item) {
            return;
        }
        if self.settings.dedupe_filter_enabled && !signature.is_empty() {
            let duplicate_ids = db_find_duplicate_item_ids(0, &item, &signature);
            if let Some(existing_id) = duplicate_ids.first().copied() {
                let anchor = self.current_scroll_anchor();
                let removed_ids: Vec<i64> = duplicate_ids.into_iter().skip(1).collect();
                if !removed_ids.is_empty() {
                    for id in &removed_ids {
                        let _ = db_delete_item(*id);
                        self.remove_cached_item(*id);
                    }
                    self.remove_duplicate_history_items(&removed_ids);
                }
                if let Ok(new_id) = db_promote_item_to_top(existing_id) {
                    self.remove_cached_item(existing_id);
                    self.remove_cached_item(new_id);
                    if !self.promote_loaded_item_to_top(existing_id, new_id) {
                        self.reload_state_from_db_preserve_scroll(anchor);
                    } else {
                        self.refilter();
                        self.restore_scroll_anchor(anchor);
                    }
                    unsafe {
                        sync_peer_windows_from_db(self.hwnd);
                    }
                    return;
                }
                return;
            }
        }
        item.id = db_insert_item(0, &item, Some(signature.as_str())).unwrap_or(0);
        // 回填内存中的 created_at（DB 由 CURRENT_TIMESTAMP 自动填写，内存补齐以便时间分组标头正常工作）
        if item.created_at.is_empty() {
            item.created_at = now_utc_sqlite();
        }
        if item.id > 0 {
            self.cache_full_item(item.clone());
        }
        let summary = clip_item_to_summary(&item);
        let visible_query = self.load_state_for_tab(0).query.clone();
        if matches!(visible_query, Some(ref query) if query.group_id == 0 && query.search_text.trim().is_empty())
        {
            self.records.insert(0, summary);
            if self.tab_index == 0 {
                self.list.apply_visible_len(self.records.len());
            }
        } else {
            self.invalidate_tab_query(0, self.tab_index == 0);
        }
        let max_items = self.settings.max_items; // 0 = 无限制；仅限制非置顶条目
        if max_items > 0 {
            db_prune_items(0, max_items);
            self.invalidate_tab_query(0, self.tab_index == 0);
        }
        if self.tab_index == 0 {
            self.sel_idx = 0;
        }
        self.refilter();
        unsafe {
            sync_peer_windows_from_db(self.hwnd);
        }
    }

    fn list_view_height(&self) -> i32 {
        self.layout().list_view_height()
    }

    fn total_content_height(&self) -> i32 {
        self.layout().total_content_height(self.visible_count())
    }

    fn clamp_scroll(&mut self) {
        self.scroll_y = self
            .layout()
            .clamp_scroll(self.scroll_y, self.visible_count());
    }

    fn ensure_visible(&mut self, idx: i32) {
        self.scroll_y = self
            .layout()
            .ensure_visible(self.scroll_y, idx, self.visible_count());
    }

    fn layout(&self) -> MainUiLayout {
        main_layout_for_dpi(self.ui_dpi)
    }

    fn row_rect(&self, visible_idx: i32) -> Option<RECT> {
        self.layout()
            .row_rect(visible_idx, self.visible_count(), self.scroll_y)
            .map(Into::into)
    }

    fn quick_action_rect_slot(&self, visible_idx: i32, slot: i32) -> Option<RECT> {
        self.layout()
            .quick_action_rect(visible_idx, self.visible_count(), self.scroll_y, slot)
            .map(Into::into)
    }

    fn search_rect(&self) -> RECT {
        self.layout().search_rect().into()
    }

    fn title_button_rect(&self, key: &str) -> RECT {
        self.layout().title_button_rect(key).into()
    }

    fn segment_rects(&self) -> (RECT, RECT) {
        let (left, right) = self.layout().segment_rects();
        (left.into(), right.into())
    }

    fn scrollbar_track_rect(&self) -> Option<RECT> {
        self.layout()
            .scrollbar_track_rect(self.visible_count())
            .map(Into::into)
    }

    fn scrollbar_thumb_rect(&self) -> Option<RECT> {
        self.layout()
            .scrollbar_thumb_rect(self.visible_count(), self.scroll_y)
            .map(Into::into)
    }

    fn scroll_to_top_rect(&self) -> RECT {
        self.layout().scroll_to_top_button_rect().into()
    }

    fn delete_selected(&mut self) {
        if self.sel_idx < 0 {
            return;
        }
        if let Some(item) = self.current_item_owned() {
            let anchor = self.current_scroll_anchor();
            if item.id > 0 {
                let _ = db_delete_item(item.id);
                self.remove_cached_item(item.id);
            }
            self.remove_items_from_active_tab(&[item.id]);
            self.clear_selection();
            self.refilter();
            self.restore_scroll_anchor(anchor);
            unsafe {
                sync_peer_windows_from_db(self.hwnd);
            }
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
            unsafe {
                sync_peer_windows_from_db(self.hwnd);
            }
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

unsafe fn queue_cloud_sync(
    hwnd: HWND,
    state: &mut AppState,
    action: CloudSyncAction,
    auto_triggered: bool,
) {
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

    if matches!(
        action,
        CloudSyncAction::SyncNow | CloudSyncAction::RestoreBackup
    ) {
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
                    reload_state_from_db_persisting(state);
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
        unsafe {
            UnregisterHotKey(hwnd, HOTKEY_ID);
        }
        state.hotkey_registered = false;
    }
}

fn register_plain_paste_hotkey_for(hwnd: HWND, state: &mut AppState) {
    unregister_plain_paste_hotkey_for(hwnd, state);
    if !state.settings.plain_paste_hotkey_enabled {
        return;
    }
    let mods = hotkey_mods_from_label(&state.settings.plain_paste_hotkey_mod);
    let vk = hotkey_vk_from_label(&state.settings.plain_paste_hotkey_key);
    let ok = unsafe { RegisterHotKey(hwnd, HOTKEY_ID_PLAIN, mods | MOD_NOREPEAT, vk) };
    state.plain_paste_hotkey_registered = ok != 0;
}

fn unregister_plain_paste_hotkey_for(hwnd: HWND, state: &mut AppState) {
    if state.plain_paste_hotkey_registered {
        unsafe {
            UnregisterHotKey(hwnd, HOTKEY_ID_PLAIN);
        }
        state.plain_paste_hotkey_registered = false;
    }
}

struct SettingsWndState {
    parent_hwnd: HWND,
    cur_page: usize,
    ui_dpi: u32,
    dpi_comp_base_w: i32,
    dpi_comp_base_h: i32,
    dpi_comp_base_monitor_dpi: u32,
    dpi_comp_last_monitor_dpi: u32,
    dpi_comp_applying: bool,
    nav_hot: i32,
    content_scroll_y: i32, // 内容区当前滚动偏移（像素）
    page_scroll_y: [i32; SETTINGS_PAGE_COUNT],
    scroll_dragging: bool,         // 正在拖拽滚动条拇指
    scroll_drag_start_y: i32,      // 拖拽起始鼠标Y
    scroll_drag_start_scroll: i32, // 拖拽起始scroll_y
    scroll_bar_visible: bool,      // 滚动条当前是否可见（auto-hide）
    scroll_hide_timer: bool,       // 渐隐 timer 是否运行中
    save_hint_timer: bool,
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
    chk_move_pasted_to_top: HWND,
    chk_persistent_search: HWND,
    chk_paste_sound: HWND,
    cb_paste_sound: HWND,
    btn_paste_sound_pick: HWND,
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
    btn_hk_record: HWND,
    chk_plain_hk_enable: HWND,
    cb_plain_hk_mod: HWND,
    cb_plain_hk_key: HWND,
    lb_plain_hk_preview: HWND,
    hotkey_recording: bool,
    btn_clip_hist_block: HWND,
    btn_clip_hist_restore: HWND,
    btn_restart_explorer: HWND,
    chk_qs: HWND,
    cb_engine: HWND,
    ed_tpl: HWND,
    cb_ocr_provider: HWND,
    lb_ocr_primary: HWND,
    ed_ocr_cloud_url: HWND,
    lb_ocr_secondary: HWND,
    ed_ocr_cloud_token: HWND,
    lb_ocr_status: HWND,
    btn_ocr_detect: HWND,
    cb_translate_provider: HWND,
    lb_translate_status: HWND,
    lb_translate_primary: HWND,
    ed_translate_app_id: HWND,
    lb_translate_secondary: HWND,
    ed_translate_secret: HWND,
    lb_translate_target: HWND,
    cb_translate_target: HWND,
    btn_plugin_downloads: HWND,
    cb_vv_source: HWND,
    cb_vv_group: HWND,
    vv_source_selected: usize,
    vv_group_selected: i64,
    group_view_tab: usize,
    btn_group_view_records: HWND,
    btn_group_view_phrases: HWND,
    chk_ai: HWND,
    chk_mm: HWND,
    chk_qr: HWND,
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

fn settings_page_content_total_h_for_state(st: &SettingsWndState, page: usize) -> i32 {
    settings_page_content_total_h(page).max(st.ui.measured_content_total_h(page))
}

fn settings_page_max_scroll_for_state(st: &SettingsWndState, page: usize, view_h: i32) -> i32 {
    (settings_page_content_total_h_for_state(st, page) - view_h).max(0)
}

/// 计算自绘滚动条拇指矩形（宽度可变：正常=SCROLL_BAR_W，拖拽=SCROLL_BAR_W_ACTIVE）
fn settings_scrollbar_thumb_w(
    st: &SettingsWndState,
    crc: &RECT,
    scroll_y: i32,
    bar_w: i32,
) -> Option<RECT> {
    let content_y = settings_content_y_scaled();
    let view_h = (crc.bottom - crc.top) - content_y;
    let max_s = settings_page_max_scroll_for_state(st, st.cur_page, view_h);
    if max_s <= 0 {
        return None;
    }
    let track_top = content_y + settings_scale(8);
    let track_bottom = crc.bottom - settings_scale(8);
    let track_h = (track_bottom - track_top).max(1);
    let content_h = settings_page_content_total_h_for_state(st, st.cur_page).max(view_h + 1);
    let thumb_h = ((view_h as f32 / content_h as f32) * track_h as f32) as i32;
    let thumb_h = thumb_h.max(settings_scale(24));
    let thumb_top =
        track_top + ((scroll_y as f32 / max_s as f32) * (track_h - thumb_h) as f32) as i32;
    let right = crc.right - SCROLL_BAR_MARGIN;
    Some(RECT {
        left: right - bar_w,
        top: thumb_top,
        right,
        bottom: thumb_top + thumb_h,
    })
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
    if !data.surface_brush.is_null() {
        DeleteObject(data.surface_brush as _);
    }
    if !data.control_brush.is_null() {
        DeleteObject(data.control_brush as _);
    }
    let th = Theme::default();
    data.surface_brush = CreateSolidBrush(th.surface) as _;
    data.control_brush = CreateSolidBrush(th.control_bg) as _;
}

const IDC_INPUT_EDIT: usize = 9001;
const IDC_INPUT_OK: usize = 9002;
const IDC_INPUT_CANCEL: usize = 9003;
const INPUT_DLG_CLASS: &str = "ZsClipInputDlg";

unsafe extern "system" fn input_dlg_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let data = cs.lpCreateParams as *mut InputDlgData;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, data as isize);

            let d = &mut *data;
            let hmod = GetModuleHandleW(null());
            d.ui_font = CreateFontW(
                -scale_for_window(hwnd, 14),
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                to_wide(ui_text_font_family()).as_ptr(),
            ) as _;
            input_dialog_refresh_theme(d);

            // 标签
            let lbl = CreateWindowExW(
                0,
                to_wide("STATIC").as_ptr(),
                d.label_w.as_ptr(),
                WS_CHILD | WS_VISIBLE,
                20,
                58,
                320,
                22,
                hwnd,
                null_mut(),
                hmod,
                null(),
            );
            SendMessageW(lbl, WM_SETFONT, d.ui_font as usize, 1);

            // 输入框
            let ed = CreateWindowExW(
                WS_EX_CLIENTEDGE,
                to_wide("EDIT").as_ptr(),
                d.initial.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32),
                20,
                84,
                320,
                32,
                hwnd,
                IDC_INPUT_EDIT as _,
                hmod,
                null(),
            );
            SendMessageW(ed, WM_SETFONT, d.ui_font as usize, 1);
            SetWindowTheme(ed, to_wide("Explorer").as_ptr(), null());
            // 全选
            SendMessageW(ed, EM_SETSEL, 0, -1isize);
            SetFocus(ed);

            // 取消按钮（左）
            let btn_cancel = CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(translate("取消").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                148,
                132,
                88,
                30,
                hwnd,
                IDC_INPUT_CANCEL as _,
                hmod,
                null(),
            );
            SendMessageW(btn_cancel, WM_SETFONT, d.ui_font as usize, 1);

            // 保存按钮（右）
            let btn_ok = CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(translate("保存").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                248,
                132,
                88,
                30,
                hwnd,
                IDC_INPUT_OK as _,
                hmod,
                null(),
            );
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
            let top_rc = RECT {
                left: 0,
                top: 0,
                right: rc.right,
                bottom: 48,
            };
            let top_br = CreateSolidBrush(th.surface);
            FillRect(hdc, &top_rc, top_br);
            DeleteObject(top_br as _);
            // 标题
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            if !data_ptr.is_null() {
                let d = &*data_ptr;
                let title_rc = RECT {
                    left: 20,
                    top: 12,
                    right: rc.right - 20,
                    bottom: 46,
                };
                let title_font: *mut core::ffi::c_void = CreateFontW(
                    -scale_for_window(hwnd, 16),
                    0,
                    0,
                    0,
                    600,
                    0,
                    0,
                    0,
                    1,
                    0,
                    0,
                    5,
                    0,
                    to_wide(ui_display_font_family()).as_ptr(),
                ) as _;
                let old = SelectObject(hdc, title_font as _);
                SetBkMode(hdc, 1);
                SetTextColor(hdc, th.text);
                DrawTextW(
                    hdc,
                    d.title_w.as_ptr(),
                    -1,
                    &title_rc as *const _ as *mut _,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                );
                SelectObject(hdc, old);
                DeleteObject(title_font as _);
            }
            // 分隔线
            let sep_br = CreateSolidBrush(th.stroke);
            let sep_rc = RECT {
                left: 0,
                top: 47,
                right: rc.right,
                bottom: 48,
            };
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
            let pressed = (dis.itemState & ODS_SELECTED) != 0;
            let cid = dis.CtlID as usize;
            let text_w = get_ctrl_text_wide(dis.hwndItem);
            let rr = RECT {
                left: rc.left + 1,
                top: rc.top + 1,
                right: rc.right - 1,
                bottom: rc.bottom - 1,
            };
            if cid == IDC_INPUT_OK {
                let fill = if pressed {
                    let r = (th.accent & 0xFF) as i32;
                    let g = ((th.accent >> 8) & 0xFF) as i32;
                    let b = ((th.accent >> 16) & 0xFF) as i32;
                    rgb(
                        (r - 18).max(0) as u8,
                        (g - 18).max(0) as u8,
                        (b - 18).max(0) as u8,
                    )
                } else {
                    th.accent
                };
                draw_round_rect(hdc as _, &rr, fill, fill, 4);
                draw_text_wide_centered(
                    hdc as _,
                    &text_w,
                    &rr,
                    rgb(255, 255, 255),
                    14,
                    "Segoe UI Variable Text",
                );
            } else {
                let fill = if pressed {
                    th.button_pressed
                } else {
                    th.button_bg
                };
                let border = if pressed {
                    rgb(180, 180, 180)
                } else {
                    rgb(196, 196, 196)
                };
                draw_round_rect(hdc as _, &rr, fill, border, 4);
                draw_text_wide_centered(
                    hdc as _,
                    &text_w,
                    &rr,
                    th.text,
                    14,
                    "Segoe UI Variable Text",
                );
            }
            1
        }
        WM_COMMAND => {
            let cid = wparam & 0xffff;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            if data_ptr.is_null() {
                return 0;
            }
            let d = &mut *data_ptr;
            if cid == IDC_INPUT_OK {
                let ed = GetDlgItem(hwnd, IDC_INPUT_EDIT as i32);
                if !ed.is_null() {
                    let len = GetWindowTextLengthW(ed);
                    let mut buf = vec![0u16; (len as usize) + 2];
                    GetWindowTextW(ed, buf.as_mut_ptr(), buf.len() as i32);
                    let s = String::from_utf16_lossy(&buf[..len as usize])
                        .trim()
                        .to_string();
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
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_NCDESTROY => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut InputDlgData;
            if !data_ptr.is_null() {
                if !(*data_ptr).ui_font.is_null()
                    && (*data_ptr).ui_font != GetStockObject(DEFAULT_GUI_FONT)
                {
                    DeleteObject((*data_ptr).ui_font as _);
                }
                if !(*data_ptr).surface_brush.is_null() {
                    DeleteObject((*data_ptr).surface_brush as _);
                }
                if !(*data_ptr).control_brush.is_null() {
                    DeleteObject((*data_ptr).control_brush as _);
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

// 注册并显示输入对话框，返回用户输入文本
unsafe fn input_name_dialog(
    parent: HWND,
    title: &str,
    label: &str,
    initial: &str,
) -> Option<String> {
    let hmod = GetModuleHandleW(null());
    // 注册窗口类
    let cls_w = to_wide(INPUT_DLG_CLASS);
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(input_dlg_proc);
    wc.hInstance = hmod;
    wc.lpszClassName = cls_w.as_ptr();
    wc.hbrBackground = null_mut();
    let _ = RegisterClassExW(&wc);

    let mut init_arr = [0u16; 256];
    let iw: Vec<u16> = initial.encode_utf16().collect();
    let copy_len = iw.len().min(255);
    init_arr[..copy_len].copy_from_slice(&iw[..copy_len]);

    let data = Box::new(InputDlgData {
        result: None,
        initial: init_arr,
        title_w: translate(title)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect(),
        label_w: translate(label)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect(),
        ui_font: null_mut(),
        surface_brush: null_mut(),
        control_brush: null_mut(),
    });
    let data_ptr = Box::into_raw(data);

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
        cx,
        cy,
        dw,
        dh,
        parent,
        null_mut(),
        hmod,
        data_ptr as _,
    );
    if hwnd.is_null() {
        drop(Box::from_raw(data_ptr));
        return None;
    }
    EnableWindow(parent, 0);

    let mut msg: MSG = zeroed();
    loop {
        if GetMessageW(&mut msg, null_mut(), 0, 0) == 0 {
            break;
        }
        if msg.message == WM_KEYDOWN
            && (msg.wParam == VK_RETURN as usize || msg.wParam == VK_ESCAPE as usize)
        {
            SendMessageW(hwnd, WM_KEYDOWN, msg.wParam, msg.lParam);
            continue;
        }
        if IsDialogMessageW(hwnd, &msg) == 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if IsWindow(hwnd) == 0 {
            break;
        }
    }
    if IsWindow(hwnd) != 0 {
        DestroyWindow(hwnd);
    }
    EnableWindow(parent, 1);
    SetForegroundWindow(parent);
    Box::from_raw(data_ptr).result
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
    dirty: bool,
    loading_text: bool,
    original_text: String,
    last_w: i32,
    last_h: i32,
    ui_font: *mut core::ffi::c_void,
    btn_font: *mut core::ffi::c_void,
    surface_brush: *mut core::ffi::c_void,
    control_brush: *mut core::ffi::c_void,
    gutter_brush: *mut core::ffi::c_void,
}

unsafe fn edit_dialog_refresh_theme(data: &mut EditDlgData) {
    if !data.surface_brush.is_null() {
        DeleteObject(data.surface_brush as _);
    }
    if !data.control_brush.is_null() {
        DeleteObject(data.control_brush as _);
    }
    if !data.gutter_brush.is_null() {
        DeleteObject(data.gutter_brush as _);
    }
    let th = Theme::default();
    data.surface_brush = CreateSolidBrush(th.surface) as _;
    data.control_brush = CreateSolidBrush(th.control_bg) as _;
    data.gutter_brush = CreateSolidBrush(if is_dark_mode() {
        rgb(38, 42, 48)
    } else {
        rgb(246, 248, 250)
    }) as _;
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

unsafe fn edit_dialog_text(hwnd: HWND) -> String {
    let ed = GetDlgItem(hwnd, IDC_EDIT_TEXTAREA as i32);
    if ed.is_null() {
        return String::new();
    }
    let len = GetWindowTextLengthW(ed);
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; (len as usize) + 2];
    GetWindowTextW(ed, buf.as_mut_ptr(), buf.len() as i32);
    String::from_utf16_lossy(&buf[..len as usize])
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

unsafe fn edit_dialog_save(hwnd: HWND, data: &mut EditDlgData) -> bool {
    let normalized = edit_dialog_text(hwnd);
    if db_update_item_text(data.item_id, &normalized).is_ok() {
        data.saved = true;
        data.dirty = false;
        data.original_text = normalized;
        true
    } else {
        MessageBoxW(
            hwnd,
            to_wide(tr(
                "保存失败，请稍后重试。",
                "Save failed. Please try again.",
            ))
            .as_ptr(),
            to_wide(tr("编辑记录", "Edit Record")).as_ptr(),
            MB_OK | MB_ICONERROR,
        );
        false
    }
}

unsafe fn edit_dialog_save_size(hwnd: HWND, data: &EditDlgData) {
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) == 0 {
        return;
    }
    let width = rc.right - rc.left;
    let height = rc.bottom - rc.top;
    if width <= 0 || height <= 0 {
        return;
    }
    let mut settings = load_settings();
    settings.edit_dialog_w = if data.last_w > 0 { data.last_w } else { width };
    settings.edit_dialog_h = if data.last_h > 0 { data.last_h } else { height };
    save_settings(&settings);
}

unsafe fn edit_dialog_confirm_close(hwnd: HWND, data: &mut EditDlgData) -> bool {
    if !data.dirty {
        return true;
    }
    let result = MessageBoxW(
        hwnd,
        to_wide(tr(
            "当前修改尚未保存，是否先保存？",
            "You have unsaved changes. Save before closing?",
        ))
        .as_ptr(),
        to_wide(tr("编辑记录", "Edit Record")).as_ptr(),
        MB_YESNOCANCEL | MB_ICONWARNING,
    );
    match result {
        IDYES => edit_dialog_save(hwnd, data),
        IDNO => true,
        _ => false,
    }
}

unsafe fn edit_dialog_mark_dirty(hwnd: HWND, data: &mut EditDlgData) {
    if data.loading_text {
        return;
    }
    let current = edit_dialog_text(hwnd);
    data.dirty = current != data.original_text;
}

unsafe extern "system" fn edit_dlg_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let data = cs.lpCreateParams as *mut EditDlgData;
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, data as isize);
            let d = &mut *data;
            let hmod = GetModuleHandleW(null());

            d.ui_font = CreateFontW(
                -scale_for_window(hwnd, 14),
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                to_wide(ui_text_font_family()).as_ptr(),
            ) as _;
            edit_dialog_refresh_theme(d);
            d.btn_font = CreateFontW(
                -scale_for_window(hwnd, 14),
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                to_wide(ui_text_font_family()).as_ptr(),
            ) as _;

            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            let w = rc.right;
            let h = rc.bottom;
            let lineno_w = 44i32;
            let edit_x = lineno_w;
            let edit_w = w - lineno_w;
            let edit_h = h - 56;

            // 行号面板（只读编辑框，右对齐）
            let lineno = CreateWindowExW(
                0,
                to_wide("EDIT").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD
                    | WS_VISIBLE
                    | (ES_MULTILINE as u32)
                    | (ES_READONLY as u32)
                    | (ES_RIGHT as u32),
                0,
                0,
                lineno_w,
                edit_h,
                hwnd,
                IDC_EDIT_LINENO as _,
                hmod,
                null(),
            );
            SendMessageW(lineno, WM_SETFONT, d.ui_font as usize, 1);

            // 主编辑区
            let ed = CreateWindowExW(
                0,
                to_wide("EDIT").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD
                    | WS_VISIBLE
                    | WS_VSCROLL
                    | WS_TABSTOP
                    | (ES_MULTILINE as u32)
                    | (ES_AUTOVSCROLL as u32)
                    | (ES_WANTRETURN as u32)
                    | (ES_NOHIDESEL as u32),
                edit_x,
                0,
                edit_w,
                edit_h,
                hwnd,
                IDC_EDIT_TEXTAREA as _,
                hmod,
                null(),
            );
            SendMessageW(ed, WM_SETFONT, d.ui_font as usize, 1);
            SetWindowTheme(ed, to_wide("Explorer").as_ptr(), null());

            // 加载文本
            if let Ok(t) = with_db(|conn| {
                conn.query_row(
                    "SELECT COALESCE(text_data,'') FROM items WHERE id=?",
                    params![d.item_id],
                    |r| r.get::<_, String>(0),
                )
            }) {
                d.original_text = t.replace("\r\n", "\n").replace('\r', "\n");
                let crlf = t.replace('\n', "\r\n");
                d.loading_text = true;
                SetWindowTextW(ed, to_wide(&crlf).as_ptr());
                d.loading_text = false;
            }
            d.dirty = false;
            let mut wr: RECT = zeroed();
            if GetWindowRect(hwnd, &mut wr) != 0 {
                d.last_w = wr.right - wr.left;
                d.last_h = wr.bottom - wr.top;
            }
            SendMessageW(ed, EM_SETSEL, 0, 0);
            SetFocus(ed);

            // 取消按钮
            let btn_cancel = CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(translate("取消").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                w - 210,
                h - 44,
                90,
                30,
                hwnd,
                IDC_EDIT_CANCEL as _,
                hmod,
                null(),
            );
            SendMessageW(btn_cancel, WM_SETFONT, d.btn_font as usize, 1);

            // 保存按钮
            let btn_save = CreateWindowExW(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(translate("保存").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | (BS_OWNERDRAW as u32),
                w - 110,
                h - 44,
                90,
                30,
                hwnd,
                IDC_EDIT_SAVE as _,
                hmod,
                null(),
            );
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
            let bar_rc = RECT {
                left: 0,
                top: rc.bottom - 56,
                right: rc.right,
                bottom: rc.bottom,
            };
            let bar_br = CreateSolidBrush(th.surface);
            FillRect(hdc, &bar_rc, bar_br);
            DeleteObject(bar_br as _);
            // 分隔线
            let sep_br = CreateSolidBrush(th.stroke);
            let sep_rc = RECT {
                left: 0,
                top: rc.bottom - 56,
                right: rc.right,
                bottom: rc.bottom - 55,
            };
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
                    SetBkColor(
                        hdc,
                        if is_dark_mode() {
                            rgb(38, 42, 48)
                        } else {
                            rgb(246, 248, 250)
                        },
                    );
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
            let pressed = (dis.itemState & ODS_SELECTED) != 0;
            let cid = dis.CtlID as usize;
            let text_w = get_ctrl_text_wide(dis.hwndItem);
            let rr = RECT {
                left: rc.left + 1,
                top: rc.top + 1,
                right: rc.right - 1,
                bottom: rc.bottom - 1,
            };
            if cid == IDC_EDIT_SAVE {
                let fill = if pressed {
                    let r = (th.accent & 0xFF) as i32;
                    let g = ((th.accent >> 8) & 0xFF) as i32;
                    let b = ((th.accent >> 16) & 0xFF) as i32;
                    rgb(
                        (r - 18).max(0) as u8,
                        (g - 18).max(0) as u8,
                        (b - 18).max(0) as u8,
                    )
                } else {
                    th.accent
                };
                draw_round_rect(hdc as _, &rr, fill, fill, 4);
                draw_text_wide_centered(
                    hdc as _,
                    &text_w,
                    &rr,
                    rgb(255, 255, 255),
                    14,
                    "Segoe UI Variable Text",
                );
            } else {
                let fill = if pressed {
                    th.button_pressed
                } else {
                    th.button_bg
                };
                let border = if pressed {
                    rgb(180, 180, 180)
                } else {
                    rgb(196, 196, 196)
                };
                draw_round_rect(hdc as _, &rr, fill, border, 4);
                draw_text_wide_centered(
                    hdc as _,
                    &text_w,
                    &rr,
                    th.text,
                    14,
                    "Segoe UI Variable Text",
                );
            }
            1
        }
        WM_COMMAND => {
            let cid = wparam & 0xffff;
            let notify = ((wparam >> 16) & 0xffff) as u32;
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditDlgData;
            if data_ptr.is_null() {
                return 0;
            }
            let d = &mut *data_ptr;

            // 文本区滚动时同步行号
            if cid == IDC_EDIT_TEXTAREA && notify == EN_VSCROLL {
                let ed = GetDlgItem(hwnd, IDC_EDIT_TEXTAREA as i32);
                let ln = GetDlgItem(hwnd, IDC_EDIT_LINENO as i32);
                sync_line_numbers(ln, ed);
            }
            if cid == IDC_EDIT_TEXTAREA && notify == EN_CHANGE_CODE as u32 {
                let ed = GetDlgItem(hwnd, IDC_EDIT_TEXTAREA as i32);
                let ln = GetDlgItem(hwnd, IDC_EDIT_LINENO as i32);
                sync_line_numbers(ln, ed);
                edit_dialog_mark_dirty(hwnd, d);
            }

            if cid == IDC_EDIT_SAVE {
                if edit_dialog_save(hwnd, d) {
                    PostMessageW(hwnd, WM_CLOSE, 0, 0);
                }
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
                SetWindowPos(
                    lineno,
                    null_mut(),
                    0,
                    0,
                    lineno_w,
                    edit_h,
                    SWP_NOMOVE | SWP_NOZORDER,
                );
            }
            if !ed.is_null() {
                SetWindowPos(
                    ed,
                    null_mut(),
                    lineno_w,
                    0,
                    w - lineno_w,
                    edit_h,
                    SWP_NOZORDER,
                );
            }
            if !btn_cancel.is_null() {
                SetWindowPos(
                    btn_cancel,
                    null_mut(),
                    w - 210,
                    h - 44,
                    90,
                    30,
                    SWP_NOZORDER,
                );
            }
            if !btn_save.is_null() {
                SetWindowPos(btn_save, null_mut(), w - 110, h - 44, 90, 30, SWP_NOZORDER);
            }
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditDlgData;
            if !data_ptr.is_null() {
                let mut wr: RECT = zeroed();
                if GetWindowRect(hwnd, &mut wr) != 0 {
                    (*data_ptr).last_w = wr.right - wr.left;
                    (*data_ptr).last_h = wr.bottom - wr.top;
                }
            }
            0
        }
        WM_KEYDOWN => {
            if wparam == VK_ESCAPE as usize {
                PostMessageW(hwnd, WM_CLOSE, 0, 0);
            } else if wparam == ('S' as usize)
                && (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0
            {
                SendMessageW(hwnd, WM_COMMAND, IDC_EDIT_SAVE, 0);
            }
            0
        }
        WM_CLOSE => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditDlgData;
            if !data_ptr.is_null() {
                let data = &mut *data_ptr;
                if !edit_dialog_confirm_close(hwnd, data) {
                    return 0;
                }
                edit_dialog_save_size(hwnd, data);
            }
            DestroyWindow(hwnd);
            0
        }
        WM_NCDESTROY => {
            let data_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut EditDlgData;
            if !data_ptr.is_null() {
                if !(*data_ptr).ui_font.is_null()
                    && (*data_ptr).ui_font != GetStockObject(DEFAULT_GUI_FONT)
                {
                    DeleteObject((*data_ptr).ui_font as _);
                }
                if !(*data_ptr).btn_font.is_null()
                    && (*data_ptr).btn_font != GetStockObject(DEFAULT_GUI_FONT)
                {
                    DeleteObject((*data_ptr).btn_font as _);
                }
                if !(*data_ptr).surface_brush.is_null() {
                    DeleteObject((*data_ptr).surface_brush as _);
                }
                if !(*data_ptr).control_brush.is_null() {
                    DeleteObject((*data_ptr).control_brush as _);
                }
                if !(*data_ptr).gutter_brush.is_null() {
                    DeleteObject((*data_ptr).gutter_brush as _);
                }
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
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
    wc.hbrBackground = null_mut();
    wc.style = CS_HREDRAW | CS_VREDRAW;
    let _ = RegisterClassExW(&wc);

    let mut parent_rc: RECT = zeroed();
    GetWindowRect(parent, &mut parent_rc);
    let pw = parent_rc.right - parent_rc.left;
    let ph = parent_rc.bottom - parent_rc.top;
    let settings = load_settings();
    let work = nearest_monitor_work_rect_for_window(parent);
    let max_w = (work.right - work.left - settings_scale(32)).max(640);
    let max_h = (work.bottom - work.top - settings_scale(32)).max(500);
    let dw = if settings.edit_dialog_w > 0 {
        settings.edit_dialog_w.min(max_w)
    } else {
        (pw * 3).max(640).min(max_w)
    };
    let dh = if settings.edit_dialog_h > 0 {
        settings.edit_dialog_h.min(max_h)
    } else {
        (ph * 4 / 3).max(500).min(max_h)
    };
    let cx = max(work.left, work.left + ((work.right - work.left - dw) / 2));
    let cy = max(work.top, work.top + ((work.bottom - work.top - dh) / 2));

    let data = Box::new(EditDlgData {
        item_id,
        saved: false,
        dirty: false,
        loading_text: false,
        original_text: String::new(),
        last_w: dw,
        last_h: dh,
        ui_font: null_mut(),
        btn_font: null_mut(),
        surface_brush: null_mut(),
        control_brush: null_mut(),
        gutter_brush: null_mut(),
    });
    let data_ptr = Box::into_raw(data);
    let title_w = to_wide(title);
    let hwnd = CreateWindowExW(
        WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
        cls_w.as_ptr(),
        title_w.as_ptr(),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE | WS_CLIPCHILDREN,
        cx,
        cy,
        dw,
        dh,
        parent,
        null_mut(),
        hmod,
        data_ptr as _,
    );
    if hwnd.is_null() {
        drop(Box::from_raw(data_ptr));
        return false;
    }
    EnableWindow(parent, 0);

    let mut msg: MSG = zeroed();
    loop {
        let r = GetMessageW(&mut msg, null_mut(), 0, 0);
        if r == 0 || r == -1 {
            break;
        }
        let root = if !msg.hwnd.is_null() {
            GetAncestor(msg.hwnd, GA_ROOT)
        } else {
            null_mut()
        };
        if root == hwnd
            && msg.message == WM_KEYDOWN
            && msg.wParam == ('S' as usize)
            && (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0
        {
            SendMessageW(hwnd, WM_COMMAND, IDC_EDIT_SAVE, 0);
            continue;
        }
        if root == hwnd && msg.message == WM_KEYDOWN && msg.wParam == VK_ESCAPE as usize {
            PostMessageW(hwnd, WM_CLOSE, 0, 0);
            continue;
        }
        if IsDialogMessageW(hwnd, &msg) == 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }
        if IsWindow(hwnd) == 0 {
            break;
        }
    }
    if IsWindow(hwnd) != 0 {
        DestroyWindow(hwnd);
    }
    EnableWindow(parent, 1);
    SetForegroundWindow(parent);
    Box::from_raw(data_ptr).saved
}

unsafe fn refresh_settings_window_metrics(hwnd: HWND, st: &mut SettingsWndState) {
    let dpi = layout_dpi_for_window(hwnd).max(96);
    st.ui_dpi = dpi;
    set_settings_ui_dpi(dpi);
    if !st.dropdown_popup.is_null() && IsWindow(st.dropdown_popup) != 0 {
        DestroyWindow(st.dropdown_popup);
        st.dropdown_popup = null_mut();
    }
    if !st.nav_font.is_null() {
        DeleteObject(st.nav_font as _);
    }
    if !st.ui_font.is_null() && st.ui_font != GetStockObject(DEFAULT_GUI_FONT) {
        DeleteObject(st.ui_font as _);
    }
    if !st.title_font.is_null() && st.title_font != GetStockObject(DEFAULT_GUI_FONT) {
        DeleteObject(st.title_font as _);
    }
    let (nav_font, ui_font, title_font) = create_settings_fonts(hwnd);
    st.nav_font = nav_font;
    st.ui_font = ui_font;
    st.title_font = title_font;

    let mut crc: RECT = zeroed();
    GetClientRect(hwnd, &mut crc);
    let content_y = settings_content_y_scaled();
    let view_h = (crc.bottom - crc.top - content_y).max(0);
    for page in 0..SETTINGS_PAGES.len() {
        st.page_scroll_y[page] =
            st.page_scroll_y[page].clamp(0, settings_page_max_scroll_for_state(st, page, view_h));
    }
    let top_margin = settings_scale(24);
    let btn_h = settings_scale(32);
    let save_w = settings_scale(72);
    let close_w = settings_scale(64);
    let gap = settings_scale(20);
    let right = crc.right - top_margin;
    if !st.btn_save.is_null() {
        MoveWindow(st.btn_save, right - save_w, top_margin, save_w, btn_h, 1);
        settings_set_font(st.btn_save, st.ui_font);
    }
    if !st.btn_close.is_null() {
        MoveWindow(
            st.btn_close,
            right - save_w - gap - close_w,
            top_margin,
            close_w,
            btn_h,
            1,
        );
        settings_set_font(st.btn_close, st.ui_font);
    }
    for page in 0..SETTINGS_PAGES.len() {
        for reg in st.ui.page_regs(page) {
            settings_set_font(reg.hwnd, st.ui_font);
        }
    }
    let built_pages: Vec<usize> = (0..SETTINGS_PAGES.len())
        .filter(|&page| st.ui.is_built(page))
        .collect();
    for &page in built_pages.iter() {
        st.ui.clear_page(page);
    }
    st.ownerdraw_ctrls
        .retain(|&ctrl| !ctrl.is_null() && IsWindow(ctrl) != 0);
    if !st.hot_ownerdraw.is_null() && IsWindow(st.hot_ownerdraw) == 0 {
        st.hot_ownerdraw = null_mut();
    }
    let current_page = st.cur_page.min(SETTINGS_PAGES.len().saturating_sub(1));
    for &page in built_pages.iter() {
        settings_ensure_page(hwnd, st, page);
        settings_sync_page_state(st, page);
    }
    if !st.ui.is_built(current_page) {
        settings_ensure_page(hwnd, st, current_page);
        settings_sync_page_state(st, current_page);
    }
    st.content_scroll_y = if crate::settings_model::settings_page_scrollable(current_page) {
        st.page_scroll_y[current_page]
    } else {
        0
    };
    for page in 0..SETTINGS_PAGES.len() {
        for reg in st.ui.page_regs(page) {
            if reg.hwnd.is_null() {
                continue;
            }
            ShowWindow(
                reg.hwnd,
                if page == current_page {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
        }
    }
    if crate::settings_model::settings_page_scrollable(current_page) {
        settings_repos_controls(hwnd, st, true);
    }
    InvalidateRect(hwnd, null(), 1);
}

unsafe fn resize_settings_window_for_dpi_transition(hwnd: HWND, old_dpi: u32, new_dpi: u32) {
    let old_dpi = old_dpi.max(96);
    let new_dpi = new_dpi.max(96);
    if old_dpi == new_dpi {
        return;
    }
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) == 0 || rc.right <= rc.left || rc.bottom <= rc.top {
        return;
    }

    set_settings_ui_dpi(new_dpi);
    let work = nearest_monitor_work_rect_for_window(hwnd);
    let pad = settings_scale(12);
    let work_w = (work.right - work.left).max(1);
    let work_h = (work.bottom - work.top).max(1);
    let max_w = (work_w - pad * 2).max(1);
    let max_h = (work_h - pad * 2).max(1);
    let min_w = min(settings_scale(720), max_w);
    let min_h = min(settings_scale(480), max_h);
    let cur_w = rc.right - rc.left;
    let cur_h = rc.bottom - rc.top;
    let target_w = (((cur_w as i64 * new_dpi as i64) + (old_dpi as i64 / 2)) / old_dpi as i64)
        .clamp(min_w as i64, max_w as i64) as i32;
    let target_h = (((cur_h as i64 * new_dpi as i64) + (old_dpi as i64 / 2)) / old_dpi as i64)
        .clamp(min_h as i64, max_h as i64) as i32;
    let center_x = rc.left + (cur_w / 2);
    let center_y = rc.top + (cur_h / 2);
    let (x, y) = clamp_window_pos_to_rect(
        center_x - (target_w / 2),
        center_y - (target_h / 2),
        &RECT {
            left: work.left + pad,
            top: work.top + pad,
            right: work.right - pad,
            bottom: work.bottom - pad,
        },
        target_w,
        target_h,
    );
    SetWindowPos(
        hwnd,
        null_mut(),
        x,
        y,
        target_w,
        target_h,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

unsafe fn ensure_settings_window_in_work_area(hwnd: HWND) {
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) == 0 || rc.right <= rc.left || rc.bottom <= rc.top {
        return;
    }
    let work = nearest_monitor_work_rect_for_window(hwnd);
    let pad = settings_scale(12);
    let work_w = (work.right - work.left).max(1);
    let work_h = (work.bottom - work.top).max(1);
    let max_w = (work_w - pad * 2).max(1);
    let max_h = (work_h - pad * 2).max(1);
    let min_w = min(settings_scale(720), max_w);
    let min_h = min(settings_scale(480), max_h);
    let target_w = (rc.right - rc.left).clamp(min_w, max_w);
    let target_h = (rc.bottom - rc.top).clamp(min_h, max_h);
    let (x, y) = clamp_window_pos_to_rect(
        rc.left,
        rc.top,
        &RECT {
            left: work.left + pad,
            top: work.top + pad,
            right: work.right - pad,
            bottom: work.bottom - pad,
        },
        target_w,
        target_h,
    );
    if x != rc.left
        || y != rc.top
        || target_w != rc.right - rc.left
        || target_h != rc.bottom - rc.top
    {
        SetWindowPos(
            hwnd,
            null_mut(),
            x,
            y,
            target_w,
            target_h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
}

unsafe fn reset_settings_dpi_compensation(st: &mut SettingsWndState) {
    st.dpi_comp_base_w = 0;
    st.dpi_comp_base_h = 0;
    st.dpi_comp_base_monitor_dpi = 0;
    st.dpi_comp_last_monitor_dpi = 0;
    st.dpi_comp_applying = false;
}

unsafe fn update_settings_dpi_compensation_base(hwnd: HWND, st: &mut SettingsWndState) {
    if is_per_monitor_dpi_aware() || st.dpi_comp_applying {
        return;
    }
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) == 0 || rc.right <= rc.left || rc.bottom <= rc.top {
        return;
    }
    st.dpi_comp_base_w = rc.right - rc.left;
    st.dpi_comp_base_h = rc.bottom - rc.top;
    st.dpi_comp_base_monitor_dpi = monitor_dpi_for_window(hwnd).max(96);
    st.dpi_comp_last_monitor_dpi = st.dpi_comp_base_monitor_dpi;
}

unsafe fn apply_settings_system_dpi_compensation(hwnd: HWND, st: &mut SettingsWndState) -> bool {
    if is_per_monitor_dpi_aware() {
        reset_settings_dpi_compensation(st);
        return false;
    }
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) == 0 || rc.right <= rc.left || rc.bottom <= rc.top {
        return false;
    }
    let monitor_dpi = monitor_dpi_for_window(hwnd).max(96);
    if st.dpi_comp_base_monitor_dpi == 0 || st.dpi_comp_base_w <= 0 || st.dpi_comp_base_h <= 0 {
        st.dpi_comp_base_w = rc.right - rc.left;
        st.dpi_comp_base_h = rc.bottom - rc.top;
        st.dpi_comp_base_monitor_dpi = monitor_dpi;
        st.dpi_comp_last_monitor_dpi = monitor_dpi;
        return false;
    }
    let (mut target_w, mut target_h) = dpi_compensated_size(
        st.dpi_comp_base_w,
        st.dpi_comp_base_h,
        st.dpi_comp_base_monitor_dpi,
        monitor_dpi,
    );
    let work = nearest_monitor_work_rect_for_window(hwnd);
    let pad = settings_scale(12);
    let bounds = RECT {
        left: work.left + pad,
        top: work.top + pad,
        right: work.right - pad,
        bottom: work.bottom - pad,
    };
    target_w = target_w.min((bounds.right - bounds.left).max(1)).max(1);
    target_h = target_h.min((bounds.bottom - bounds.top).max(1)).max(1);
    let cur_w = rc.right - rc.left;
    let cur_h = rc.bottom - rc.top;
    if st.dpi_comp_last_monitor_dpi == monitor_dpi
        && (cur_w - target_w).abs() <= 2
        && (cur_h - target_h).abs() <= 2
    {
        return false;
    }
    let center_x = rc.left + cur_w / 2;
    let center_y = rc.top + cur_h / 2;
    let (x, y) = clamp_window_pos_to_rect(
        center_x - target_w / 2,
        center_y - target_h / 2,
        &bounds,
        target_w,
        target_h,
    );
    st.dpi_comp_applying = true;
    SetWindowPos(
        hwnd,
        null_mut(),
        x,
        y,
        target_w,
        target_h,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    st.dpi_comp_applying = false;
    st.dpi_comp_last_monitor_dpi = monitor_dpi;
    true
}

fn reload_state_from_db_persisting(state: &mut AppState) {
    if reload_state_from_db(state) {
        save_settings(&state.settings);
    }
}

unsafe fn show_settings_saved_feedback(hwnd: HWND, st: &mut SettingsWndState) {
    settings_set_text(st.btn_save, tr("已保存", "Saved"));
    InvalidateRect(st.btn_save, null(), 1);
    start_flagged_timer(
        hwnd,
        ID_TIMER_SETTINGS_SAVE_HINT,
        1200,
        &mut st.save_hint_timer,
    );
}

unsafe fn cancel_settings_scroll_drag(hwnd: HWND, st: &mut SettingsWndState) {
    if st.scroll_dragging {
        st.scroll_dragging = false;
        ReleaseCapture();
        InvalidateRect(hwnd, null(), 0);
    }
}

// 辅助函数：绘制宽字节文字居中
unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let parent_hwnd = cs.lpCreateParams as HWND;
            set_settings_ui_dpi(layout_dpi_for_window(hwnd));
            let (nav_font, ui_font, title_font) = create_settings_fonts(hwnd);
            let mut st = Box::new(SettingsWndState {
                parent_hwnd,
                cur_page: 0,
                ui_dpi: layout_dpi_for_window(hwnd).max(96),
                dpi_comp_base_w: 0,
                dpi_comp_base_h: 0,
                dpi_comp_base_monitor_dpi: 0,
                dpi_comp_last_monitor_dpi: 0,
                dpi_comp_applying: false,
                content_scroll_y: 0,
                page_scroll_y: [0; SETTINGS_PAGE_COUNT],
                scroll_dragging: false,
                scroll_drag_start_y: 0,
                scroll_drag_start_scroll: 0,
                scroll_bar_visible: false,
                scroll_hide_timer: false,
                save_hint_timer: false,
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
                chk_move_pasted_to_top: null_mut(),
                chk_persistent_search: null_mut(),
                chk_paste_sound: null_mut(),
                cb_paste_sound: null_mut(),
                btn_paste_sound_pick: null_mut(),
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
                btn_hk_record: null_mut(),
                chk_plain_hk_enable: null_mut(),
                cb_plain_hk_mod: null_mut(),
                cb_plain_hk_key: null_mut(),
                lb_plain_hk_preview: null_mut(),
                hotkey_recording: false,
                btn_clip_hist_block: null_mut(),
                btn_clip_hist_restore: null_mut(),
                btn_restart_explorer: null_mut(),
                chk_qs: null_mut(),
                cb_engine: null_mut(),
                ed_tpl: null_mut(),
                cb_ocr_provider: null_mut(),
                lb_ocr_primary: null_mut(),
                ed_ocr_cloud_url: null_mut(),
                lb_ocr_secondary: null_mut(),
                ed_ocr_cloud_token: null_mut(),
                lb_ocr_status: null_mut(),
                btn_ocr_detect: null_mut(),
                cb_translate_provider: null_mut(),
                lb_translate_status: null_mut(),
                lb_translate_primary: null_mut(),
                ed_translate_app_id: null_mut(),
                lb_translate_secondary: null_mut(),
                ed_translate_secret: null_mut(),
                lb_translate_target: null_mut(),
                cb_translate_target: null_mut(),
                btn_plugin_downloads: null_mut(),
                cb_vv_source: null_mut(),
                cb_vv_group: null_mut(),
                vv_source_selected: 0,
                vv_group_selected: 0,
                group_view_tab: 0,
                btn_group_view_records: null_mut(),
                btn_group_view_phrases: null_mut(),
                chk_ai: null_mut(),
                chk_mm: null_mut(),
                chk_qr: null_mut(),
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
            st.btn_close =
                settings_create_btn(hwnd, "关闭", IDC_SET_CLOSE, 900, 24, 64, st.ui_font);
            for &hh in &[st.btn_save, st.btn_close] {
                if !hh.is_null() {
                    st.ownerdraw_ctrls.push(hh);
                }
            }
            refresh_settings_window_metrics(hwnd, &mut st);
            settings_ensure_page(hwnd, &mut st, SettingsPage::General.index());
            settings_apply_from_app(&mut st);
            settings_show_page(hwnd, &mut st, 0);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(st) as isize);
            apply_window_corner_preference(hwnd);
            apply_dark_mode_to_window(hwnd);
            0
        }
        WM_SIZE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() && wparam != SIZE_MINIMIZED as usize {
                let st = &mut *st_ptr;
                if !is_per_monitor_dpi_aware() && !st.dpi_comp_applying {
                    update_settings_dpi_compensation_base(hwnd, st);
                }
                refresh_settings_window_metrics(hwnd, st);
            }
            0
        }
        WM_MOUSEMOVE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            ensure_mouse_leave_tracking(hwnd);
            let st = &mut *st_ptr;
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            // 滚动条拖拽
            if st.scroll_dragging {
                let my = y;
                let mut crc: RECT = core::mem::zeroed();
                GetClientRect(hwnd, &mut crc);
                let content_y = settings_content_y_scaled();
                let view_h = (crc.bottom - crc.top) - content_y;
                let max_s = settings_page_max_scroll_for_state(st, st.cur_page, view_h);
                // 与 settings_scrollbar_thumb_w 保持一致的轨道范围
                let track_top = content_y + settings_scale(8);
                let track_bottom = crc.bottom - settings_scale(8);
                let track_h = (track_bottom - track_top).max(1);
                let content_h =
                    settings_page_content_total_h_for_state(st, st.cur_page).max(view_h + 1);
                let thumb_h = ((view_h as f32 / content_h as f32) * track_h as f32) as i32;
                let thumb_h = thumb_h.max(settings_scale(24));
                let drag_range = (track_h - thumb_h).max(1);
                let dy = my - st.scroll_drag_start_y;
                let new_y = st.scroll_drag_start_scroll
                    + (dy as f32 / drag_range as f32 * max_s as f32) as i32;
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
            let mut hot_ctrl =
                ChildWindowFromPointEx(hwnd, pt, CWP_SKIPDISABLED | CWP_SKIPINVISIBLE);
            if !st.ownerdraw_ctrls.contains(&hot_ctrl) {
                hot_ctrl = null_mut();
            }
            if hot_ctrl != st.hot_ownerdraw {
                if !st.hot_ownerdraw.is_null() {
                    InvalidateRect(st.hot_ownerdraw, null(), 1);
                }
                st.hot_ownerdraw = hot_ctrl;
                if !st.hot_ownerdraw.is_null() {
                    InvalidateRect(st.hot_ownerdraw, null(), 1);
                }
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
            if st_ptr.is_null() {
                return 0;
            }
            let st = &mut *st_ptr;
            let mx = get_x_lparam(lparam);
            let my = get_y_lparam(lparam);
            if !st.dropdown_popup.is_null() && IsWindow(st.dropdown_popup) != 0 {
                let mut prc: RECT = core::mem::zeroed();
                GetWindowRect(st.dropdown_popup, &mut prc);
                let mut pt = POINT { x: mx, y: my };
                ClientToScreen(hwnd, &mut pt);
                if !(pt.x >= prc.left && pt.x <= prc.right && pt.y >= prc.top && pt.y <= prc.bottom)
                {
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
            if let Some(thumb) =
                settings_scrollbar_thumb_w(st, &crc, st.content_scroll_y, SCROLL_BAR_W_ACTIVE)
            {
                if mx >= thumb.left - 4
                    && mx <= thumb.right + 4
                    && my >= thumb.top
                    && my <= thumb.bottom
                {
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
            let left_edge = right_edge - SCROLL_BAR_W_ACTIVE - 4;
            let content_y = settings_content_y_scaled();
            if mx >= left_edge
                && mx <= right_edge + 2
                && my >= content_y + settings_scale(4)
                && my < crc.bottom - settings_scale(4)
            {
                let view_h = (crc.bottom - crc.top) - content_y;
                let max_s = settings_page_max_scroll_for_state(st, st.cur_page, view_h);
                let track_h =
                    (crc.bottom - settings_scale(8) - (content_y + settings_scale(8))).max(1);
                let new_y = ((my - content_y - settings_scale(8)) as f32 / track_h as f32
                    * max_s as f32) as i32;
                settings_scroll_to(hwnd, st, new_y);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEWHEEL => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() {
                return 0;
            }
            let st = &mut *st_ptr;
            let delta = ((wparam >> 16) & 0xffff) as u16 as i16 as i32;
            settings_scroll(hwnd, st, if delta > 0 { -60 } else { 60 });
            0
        }
        WM_KEYDOWN | WM_SYSKEYDOWN => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() {
                return 0;
            }
            let st = &mut *st_ptr;
            if !st.hotkey_recording {
                return DefWindowProcW(hwnd, msg, wparam, lparam);
            }
            let vk = wparam as u32;
            if vk == VK_ESCAPE as u32 {
                settings_set_hotkey_recording(st, false);
                return 0;
            }
            if vv_is_modifier_vk(vk) {
                return 0;
            }
            if let Some(key_label) = hotkey_key_label_from_vk(vk) {
                if let Some(mod_label) = hotkey_mod_label_from_pressed_state() {
                    settings_set_text(st.cb_hk_mod, &mod_label);
                    settings_set_text(st.cb_hk_key, key_label);
                    settings_set_hotkey_recording(st, false);
                } else {
                    settings_set_text(
                        st.lb_hk_preview,
                        tr("请按修饰键 + 按键", "Press modifier + key"),
                    );
                    InvalidateRect(st.lb_hk_preview, null(), 1);
                }
                return 0;
            }
            0
        }
        WM_LBUTTONUP => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() && (*st_ptr).scroll_dragging {
                cancel_settings_scroll_drag(hwnd, &mut *st_ptr);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                cancel_settings_scroll_drag(hwnd, &mut *st_ptr);
            }
            0
        }
        WM_SETTINGS_DROPDOWN_SELECTED => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if st_ptr.is_null() {
                return 0;
            }
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
                        settings_set_text(
                            st.lb_hk_preview,
                            &hotkey_preview_text(label, &get_window_text(st.cb_hk_key)),
                        );
                        InvalidateRect(st.cb_hk_mod, null(), 1);
                    }
                }
                6103 => {
                    if let Some(label) = HOTKEY_KEY_OPTIONS.get(idx) {
                        settings_set_text(st.cb_hk_key, label);
                        settings_set_text(
                            st.lb_hk_preview,
                            &hotkey_preview_text(&get_window_text(st.cb_hk_mod), label),
                        );
                        InvalidateRect(st.cb_hk_key, null(), 1);
                    }
                }
                IDC_SET_PASTE_SOUND_KIND => {
                    if let Some((key, _)) = PASTE_SOUND_OPTIONS.get(idx) {
                        st.draft.paste_success_sound_kind = (*key).to_string();
                        settings_set_text(st.cb_paste_sound, &paste_sound_display(key));
                        InvalidateRect(st.cb_paste_sound, null(), 1);
                        settings_sync_page_state(st, SettingsPage::General.index());
                    }
                }
                IDC_SET_PLAIN_HK_MOD => {
                    if let Some(label) = HOTKEY_MOD_OPTIONS.get(idx) {
                        settings_set_text(st.cb_plain_hk_mod, label);
                        settings_set_text(
                            st.lb_plain_hk_preview,
                            &hotkey_preview_text(label, &get_window_text(st.cb_plain_hk_key)),
                        );
                        InvalidateRect(st.cb_plain_hk_mod, null(), 1);
                    }
                }
                IDC_SET_PLAIN_HK_KEY => {
                    if let Some(label) = HOTKEY_KEY_OPTIONS.get(idx) {
                        settings_set_text(st.cb_plain_hk_key, label);
                        settings_set_text(
                            st.lb_plain_hk_preview,
                            &hotkey_preview_text(&get_window_text(st.cb_plain_hk_mod), label),
                        );
                        InvalidateRect(st.cb_plain_hk_key, null(), 1);
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
                IDC_SET_OCR_PROVIDER => {
                    if let Some((key, _label)) = IMAGE_OCR_PROVIDER_OPTIONS.get(idx) {
                        st.draft.image_ocr_provider = (*key).to_string();
                        settings_set_text(st.cb_ocr_provider, &image_ocr_provider_display(key));
                        settings_sync_page_state(st, SettingsPage::Plugin.index());
                        InvalidateRect(st.cb_ocr_provider, null(), 1);
                    }
                }
                IDC_SET_TRANSLATE_PROVIDER => {
                    if let Some((key, _)) = TEXT_TRANSLATE_PROVIDER_OPTIONS.get(idx) {
                        st.draft.text_translate_provider = (*key).to_string();
                        settings_set_text(
                            st.cb_translate_provider,
                            &text_translate_provider_display(key),
                        );
                        settings_sync_page_state(st, SettingsPage::Plugin.index());
                        InvalidateRect(st.cb_translate_provider, null(), 1);
                    }
                }
                IDC_SET_TRANSLATE_TARGET => {
                    if let Some((key, _)) = TEXT_TRANSLATE_TARGET_OPTIONS.get(idx) {
                        st.draft.text_translate_target_lang = (*key).to_string();
                        settings_set_text(
                            st.cb_translate_target,
                            &text_translate_target_display(key),
                        );
                        InvalidateRect(st.cb_translate_target, null(), 1);
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
            if st_ptr.is_null() {
                return 0;
            }
            let st = &mut *st_ptr;
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            let rc0 = dis.rcItem;
            let w = (rc0.right - rc0.left).max(1);
            let h = (rc0.bottom - rc0.top).max(1);
            let memdc = CreateCompatibleDC(dis.hDC);
            let bmp = CreateCompatibleBitmap(dis.hDC, w, h);
            let oldbmp = SelectObject(memdc, bmp as _);
            let th = Theme::default();
            let bg_fill = if dis.CtlID as isize == IDC_SET_AUTOSTART
                || dis.CtlID as isize == IDC_SET_SILENTSTART
                || dis.CtlID as isize == IDC_SET_TRAYICON
                || dis.CtlID as isize == IDC_SET_CLOSETRAY
                || dis.CtlID as isize == IDC_SET_CLICK_HIDE
                || dis.CtlID as isize == IDC_SET_PASTE_MOVE_TOP
                || dis.CtlID as isize == IDC_SET_DEDUPE_FILTER
                || dis.CtlID as isize == IDC_SET_PERSIST_SEARCH
                || dis.CtlID as isize == IDC_SET_PASTE_SOUND_ENABLE
                || dis.CtlID as isize == IDC_SET_AUTOHIDE_BLUR
                || dis.CtlID as isize == IDC_SET_EDGEHIDE
                || dis.CtlID as isize == IDC_SET_HOVERPREVIEW
                || dis.CtlID as isize == IDC_SET_VV_MODE
                || dis.CtlID as isize == IDC_SET_IMAGE_PREVIEW
                || dis.CtlID as isize == IDC_SET_QUICK_DELETE
                || dis.CtlID as isize == IDC_SET_GROUP_ENABLE
                || dis.CtlID as isize == IDC_SET_CLOUD_ENABLE
                || dis.CtlID as isize == IDC_SET_OPEN_SOURCE
                || dis.CtlID as isize == IDC_SET_OPEN_UPDATE
                || dis.CtlID as isize == IDC_SET_MAX
                || dis.CtlID as isize == IDC_SET_POSMODE
                || dis.CtlID as isize == IDC_SET_CLOUD_INTERVAL
                || dis.CtlID as isize == IDC_SET_PASTE_SOUND_KIND
                || dis.CtlID as isize == IDC_SET_PASTE_SOUND_PICK
                || dis.CtlID as isize == IDC_SET_VV_SOURCE
                || dis.CtlID as isize == IDC_SET_VV_GROUP
                || dis.CtlID as isize == 6101
                || dis.CtlID as isize == 6102
                || dis.CtlID as isize == 6103
                || dis.CtlID as isize == IDC_SET_HK_RECORD
                || dis.CtlID as isize == IDC_SET_PLAIN_HK_ENABLE
                || dis.CtlID as isize == IDC_SET_PLAIN_HK_MOD
                || dis.CtlID as isize == IDC_SET_PLAIN_HK_KEY
                || dis.CtlID as isize == 7102
                || dis.CtlID as isize == 7101
                || dis.CtlID as isize == 7103
                || dis.CtlID as isize == 7104
                || dis.CtlID as isize == 7201
            {
                th.surface
            } else {
                th.bg
            };
            let bg = CreateSolidBrush(bg_fill);
            let local = RECT {
                left: 0,
                top: 0,
                right: w,
                bottom: h,
            };
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
            if st_ptr.is_null() {
                return 0;
            }
            let st = &mut *st_ptr;
            match cmd {
                IDC_SET_AUTOSTART
                | IDC_SET_SILENTSTART
                | IDC_SET_TRAYICON
                | IDC_SET_CLOSETRAY
                | IDC_SET_CLICK_HIDE
                | IDC_SET_PASTE_MOVE_TOP
                | IDC_SET_DEDUPE_FILTER
                | IDC_SET_PERSIST_SEARCH
                | IDC_SET_PASTE_SOUND_ENABLE
                | IDC_SET_AUTOHIDE_BLUR
                | IDC_SET_EDGEHIDE
                | IDC_SET_HOVERPREVIEW
                | IDC_SET_VV_MODE
                | IDC_SET_IMAGE_PREVIEW
                | IDC_SET_QUICK_DELETE
                | IDC_SET_GROUP_ENABLE
                | IDC_SET_CLOUD_ENABLE
                | 6101
                | 7102
                | 7101
                | 7103
                | 7104 => {
                    settings_toggle_flip(st, cmd);
                    if cmd == IDC_SET_EDGEHIDE {
                        settings_sync_pos_fields_enabled(st);
                    } else if cmd == IDC_SET_PASTE_SOUND_ENABLE {
                        settings_sync_page_state(st, SettingsPage::General.index());
                    }
                    let sender = lparam as HWND;
                    if !sender.is_null() {
                        InvalidateRect(sender, null(), 1);
                    }
                }
                IDC_SET_HK_RECORD => {
                    let next = !st.hotkey_recording;
                    settings_set_hotkey_recording(st, next);
                    if next {
                        SetFocus(hwnd);
                    }
                }
                IDC_SET_GROUP_ADD => {
                    if let Some(name) =
                        input_name_dialog(hwnd, "新建分组", "请输入分组名称：", "新分组")
                    {
                        let category = source_tab_category(settings_group_view_current(st));
                        match db_create_named_group(category, &name) {
                            Ok(group) => {
                                settings_groups_refresh_list(st, group.id);
                                let pst = get_state_ptr(st.parent_hwnd);
                                if !pst.is_null() {
                                    reload_state_from_db_persisting(&mut *pst);
                                    InvalidateRect(st.parent_hwnd, null(), 1);
                                }
                            }
                            Err(e) => {
                                MessageBoxW(
                                    hwnd,
                                    to_wide(&format!(
                                        "{}: {}",
                                        tr("新建分组失败", "Failed to create group"),
                                        e
                                    ))
                                    .as_ptr(),
                                    to_wide(translate("分组").as_ref()).as_ptr(),
                                    MB_OK | MB_ICONERROR,
                                );
                            }
                        }
                    }
                }
                IDC_SET_GROUP_RENAME => {
                    if let Some((_, g)) = settings_groups_selected(st) {
                        if let Some(new_name) =
                            input_name_dialog(hwnd, "重命名分组", "请输入新名称：", &g.name)
                        {
                            if let Err(e) = db_rename_group(g.category, g.id, &new_name) {
                                MessageBoxW(
                                    hwnd,
                                    to_wide(&format!(
                                        "{}: {}",
                                        tr("重命名失败", "Rename failed"),
                                        e
                                    ))
                                    .as_ptr(),
                                    to_wide(translate("分组").as_ref()).as_ptr(),
                                    MB_OK | MB_ICONERROR,
                                );
                            } else {
                                settings_groups_refresh_list(st, g.id);
                                let pst = get_state_ptr(st.parent_hwnd);
                                if !pst.is_null() {
                                    reload_state_from_db_persisting(&mut *pst);
                                    InvalidateRect(st.parent_hwnd, null(), 1);
                                }
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
                        if MessageBoxW(
                            hwnd,
                            to_wide(&ask).as_ptr(),
                            to_wide(translate("分组").as_ref()).as_ptr(),
                            MB_YESNO | MB_ICONQUESTION,
                        ) == IDYES
                        {
                            if let Err(e) = db_delete_group(g.id) {
                                MessageBoxW(
                                    hwnd,
                                    to_wide(&format!(
                                        "{}: {}",
                                        tr("删除分组失败", "Delete group failed"),
                                        e
                                    ))
                                    .as_ptr(),
                                    to_wide(translate("分组").as_ref()).as_ptr(),
                                    MB_OK | MB_ICONERROR,
                                );
                            } else {
                                settings_groups_refresh_list(st, 0);
                                let pst = get_state_ptr(st.parent_hwnd);
                                if !pst.is_null() {
                                    reload_state_from_db_persisting(&mut *pst);
                                    InvalidateRect(st.parent_hwnd, null(), 1);
                                }
                            }
                        }
                    }
                }
                IDC_SET_GROUP_UP => {
                    settings_groups_move(st, -1);
                }
                IDC_SET_GROUP_DOWN => {
                    settings_groups_move(st, 1);
                }
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
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_max, &mut rc);
                    let current = settings_dropdown_index_for_max_items(
                        settings_dropdown_max_items_from_label(&get_window_text(st.cb_max)),
                    );
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_MAX,
                        &rc,
                        &["100", "200", "500", "1000", "3000", "无限制"],
                        current,
                        180,
                    );
                }
                IDC_SET_POSMODE => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_pos, &mut rc);
                    let current = settings_dropdown_index_for_pos_mode(
                        &settings_dropdown_pos_mode_from_label(&get_window_text(st.cb_pos)),
                    );
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_POSMODE,
                        &rc,
                        &["跟随鼠标", "固定位置", "上次位置"],
                        current,
                        180,
                    );
                }
                IDC_SET_CLOUD_INTERVAL => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_cloud_interval, &mut rc);
                    let items = ["15分钟", "30分钟", "1小时", "6小时", "12小时", "24小时"];
                    let current = items
                        .iter()
                        .position(|x| *x == get_window_text(st.cb_cloud_interval))
                        .unwrap_or(2);
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_CLOUD_INTERVAL,
                        &rc,
                        &items,
                        current,
                        180,
                    );
                }
                6102 => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_hk_mod, &mut rc);
                    let current = HOTKEY_MOD_OPTIONS
                        .iter()
                        .position(|x| *x == get_window_text(st.cb_hk_mod))
                        .unwrap_or(0);
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        6102,
                        &rc,
                        &HOTKEY_MOD_OPTIONS,
                        current,
                        200,
                    );
                }
                6103 => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_hk_key, &mut rc);
                    let current = HOTKEY_KEY_OPTIONS
                        .iter()
                        .position(|x| *x == get_window_text(st.cb_hk_key))
                        .unwrap_or(21);
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        6103,
                        &rc,
                        &HOTKEY_KEY_OPTIONS,
                        current,
                        220,
                    );
                }
                IDC_SET_PASTE_SOUND_KIND => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_paste_sound, &mut rc);
                    let current = PASTE_SOUND_OPTIONS
                        .iter()
                        .position(|(key, _)| {
                            paste_sound_display(key) == get_window_text(st.cb_paste_sound)
                        })
                        .unwrap_or(0);
                    let labels_owned: Vec<String> = PASTE_SOUND_OPTIONS
                        .iter()
                        .map(|(key, _)| paste_sound_display(key))
                        .collect();
                    let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_PASTE_SOUND_KIND,
                        &rc,
                        &labels,
                        current,
                        220,
                    );
                }
                IDC_SET_PASTE_SOUND_PICK => {
                    match pick_paste_sound_file(&st.draft.paste_success_sound_path) {
                        Ok(Some(path)) => {
                            st.draft.paste_success_sound_kind = "custom".to_string();
                            st.draft.paste_success_sound_path = path;
                            settings_set_text(
                                st.cb_paste_sound,
                                &paste_sound_display(&st.draft.paste_success_sound_kind),
                            );
                            settings_sync_page_state(st, SettingsPage::General.index());
                            InvalidateRect(hwnd, null(), 1);
                        }
                        Ok(None) => {}
                        Err(err) => {
                            let message = format!(
                                "{}: {}",
                                tr("选择提示音文件失败", "Failed to choose sound file"),
                                err
                            );
                            MessageBoxW(
                                hwnd,
                                to_wide(&message).as_ptr(),
                                to_wide(tr("粘贴成功声音", "Paste success sound")).as_ptr(),
                                MB_OK | MB_ICONERROR,
                            );
                        }
                    }
                }
                IDC_SET_PLAIN_HK_MOD => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_plain_hk_mod, &mut rc);
                    let current = HOTKEY_MOD_OPTIONS
                        .iter()
                        .position(|x| *x == get_window_text(st.cb_plain_hk_mod))
                        .unwrap_or(5);
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_PLAIN_HK_MOD,
                        &rc,
                        &HOTKEY_MOD_OPTIONS,
                        current,
                        220,
                    );
                }
                IDC_SET_PLAIN_HK_KEY => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_plain_hk_key, &mut rc);
                    let current = HOTKEY_KEY_OPTIONS
                        .iter()
                        .position(|x| *x == get_window_text(st.cb_plain_hk_key))
                        .unwrap_or(21);
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_PLAIN_HK_KEY,
                        &rc,
                        &HOTKEY_KEY_OPTIONS,
                        current,
                        220,
                    );
                }
                7201 => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_engine, &mut rc);
                    let current = SEARCH_ENGINE_PRESETS
                        .iter()
                        .position(|(_, name, _)| *name == get_window_text(st.cb_engine))
                        .unwrap_or(0);
                    let labels: Vec<&str> = SEARCH_ENGINE_PRESETS
                        .iter()
                        .map(|(_, name, _)| *name)
                        .collect();
                    st.dropdown_popup =
                        show_settings_dropdown_popup(hwnd, 7201, &rc, &labels, current, 260);
                }
                IDC_SET_OCR_PROVIDER => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_ocr_provider, &mut rc);
                    let current = IMAGE_OCR_PROVIDER_OPTIONS
                        .iter()
                        .position(|(key, _)| {
                            image_ocr_provider_display(key) == get_window_text(st.cb_ocr_provider)
                        })
                        .unwrap_or(0);
                    let labels_owned: Vec<String> = IMAGE_OCR_PROVIDER_OPTIONS
                        .iter()
                        .map(|(key, _)| image_ocr_provider_display(key))
                        .collect();
                    let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_OCR_PROVIDER,
                        &rc,
                        &labels,
                        current,
                        240,
                    );
                }
                IDC_SET_TRANSLATE_PROVIDER => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_translate_provider, &mut rc);
                    let current = TEXT_TRANSLATE_PROVIDER_OPTIONS
                        .iter()
                        .position(|(key, _)| {
                            text_translate_provider_display(key)
                                == get_window_text(st.cb_translate_provider)
                        })
                        .unwrap_or(0);
                    let labels_owned: Vec<String> = TEXT_TRANSLATE_PROVIDER_OPTIONS
                        .iter()
                        .map(|(key, _)| text_translate_provider_display(key))
                        .collect();
                    let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_TRANSLATE_PROVIDER,
                        &rc,
                        &labels,
                        current,
                        240,
                    );
                }
                IDC_SET_TRANSLATE_TARGET => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_translate_target, &mut rc);
                    let current = TEXT_TRANSLATE_TARGET_OPTIONS
                        .iter()
                        .position(|(key, _)| {
                            text_translate_target_display(key)
                                == get_window_text(st.cb_translate_target)
                        })
                        .unwrap_or(0);
                    let labels_owned: Vec<String> = TEXT_TRANSLATE_TARGET_OPTIONS
                        .iter()
                        .map(|(key, _)| text_translate_target_display(key))
                        .collect();
                    let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_TRANSLATE_TARGET,
                        &rc,
                        &labels,
                        current,
                        200,
                    );
                }
                IDC_SET_VV_SOURCE => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_vv_source, &mut rc);
                    let current = settings_vv_source_current(st);
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_VV_SOURCE,
                        &rc,
                        &["复制记录", "常用短语"],
                        current,
                        200,
                    );
                }
                IDC_SET_VV_GROUP => {
                    if !st.dropdown_popup.is_null() {
                        if IsWindow(st.dropdown_popup) != 0 {
                            DestroyWindow(st.dropdown_popup);
                        }
                        st.dropdown_popup = null_mut();
                    }
                    let mut rc: RECT = zeroed();
                    GetWindowRect(st.cb_vv_group, &mut rc);
                    let vv_source = settings_vv_source_current(st);
                    let groups = settings_groups_cache_for_tab(st, vv_source);
                    let labels_owned: Vec<String> =
                        std::iter::once(source_tab_all_label(vv_source).to_string())
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
                    st.dropdown_popup = show_settings_dropdown_popup(
                        hwnd,
                        IDC_SET_VV_GROUP,
                        &rc,
                        &labels,
                        current,
                        260,
                    );
                }
                7203 => {
                    let key = search_engine_key_from_display(&get_window_text(st.cb_engine));
                    settings_set_text(st.ed_tpl, search_engine_template(key));
                }
                IDC_SET_PLUGIN_DOWNLOADS => {
                    open_path_with_shell(&plugin_downloads_url());
                }
                IDC_SET_OCR_WECHAT_DETECT => {
                    if let Some(path) =
                        detect_wechat_runtime_dir(&get_window_text(st.ed_ocr_cloud_url))
                    {
                        st.draft.image_ocr_wechat_dir = path;
                        settings_sync_page_state(st, SettingsPage::Plugin.index());
                        InvalidateRect(hwnd, null(), 1);
                    } else {
                        MessageBoxW(
                            hwnd,
                            to_wide(tr("未能自动检测到微信目录，请先启动微信或手动填写安装目录。", "Could not auto-detect the WeChat directory. Please start WeChat or enter the install directory manually.")).as_ptr(),
                            to_wide(tr("WinOCR（微信 OCR）", "WinOCR (WeChat OCR)")).as_ptr(),
                            MB_OK | MB_ICONINFORMATION,
                        );
                    }
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
                    if let Err(e) = set_system_clipboard_history_enabled(false) {
                        MessageBoxW(
                            hwnd,
                            to_wide(&format!(
                                "{}: {}",
                                tr("屏蔽 Win+V 失败", "Disable Win+V failed"),
                                e
                            ))
                            .as_ptr(),
                            to_wide(translate("系统剪贴板历史").as_ref()).as_ptr(),
                            MB_OK | MB_ICONERROR,
                        );
                    }
                }
                6112 => {
                    if let Err(e) = set_system_clipboard_history_enabled(true) {
                        MessageBoxW(
                            hwnd,
                            to_wide(&format!(
                                "{}: {}",
                                tr("恢复 Win+V 失败", "Restore Win+V failed"),
                                e
                            ))
                            .as_ptr(),
                            to_wide(translate("系统剪贴板历史").as_ref()).as_ptr(),
                            MB_OK | MB_ICONERROR,
                        );
                    }
                }
                6113 => {
                    if let Err(e) = restart_explorer_shell() {
                        MessageBoxW(
                            hwnd,
                            to_wide(&format!(
                                "{}: {}",
                                tr("重启资源管理器失败", "Restart Explorer failed"),
                                e
                            ))
                            .as_ptr(),
                            to_wide(translate("系统剪贴板历史").as_ref()).as_ptr(),
                            MB_OK | MB_ICONERROR,
                        );
                    }
                }
                IDC_SET_CLOUD_SYNC_NOW
                | IDC_SET_CLOUD_UPLOAD_CFG
                | IDC_SET_CLOUD_APPLY_CFG
                | IDC_SET_CLOUD_RESTORE_BACKUP => {
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
                    settings_apply_from_app(st);
                    settings_sync_page_state(st, st.cur_page);
                    show_settings_saved_feedback(hwnd, st);
                    InvalidateRect(hwnd, null(), 1);
                }
                IDC_SET_CLOSE => {
                    DestroyWindow(hwnd);
                }
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
        WM_THEMECHANGED => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                settings_refresh_theme_resources(&mut *st_ptr);
                apply_dark_mode_to_window(hwnd);
                InvalidateRect(hwnd, null(), 1);
            }
            0
        }
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                set_settings_ui_dpi(layout_dpi_for_window(hwnd).max(96));
                let _ = apply_settings_system_dpi_compensation(hwnd, &mut *st_ptr);
                ensure_settings_window_in_work_area(hwnd);
                refresh_settings_window_metrics(hwnd, &mut *st_ptr);
            }
            0
        }
        WM_MOVE => {
            if !is_per_monitor_dpi_aware() {
                SetTimer(hwnd, ID_TIMER_SETTINGS_DPI_FIT, 60, None);
            }
            0
        }
        WM_EXITSIZEMOVE => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                let next_dpi = layout_dpi_for_window(hwnd).max(96);
                let old_dpi = (*st_ptr).ui_dpi.max(96);
                if !is_per_monitor_dpi_aware() {
                    let _ = apply_settings_system_dpi_compensation(hwnd, &mut *st_ptr);
                } else if old_dpi != next_dpi {
                    (*st_ptr).ui_dpi = next_dpi;
                    resize_settings_window_for_dpi_transition(hwnd, old_dpi, next_dpi);
                } else {
                    set_settings_ui_dpi(next_dpi);
                    ensure_settings_window_in_work_area(hwnd);
                }
                refresh_settings_window_metrics(hwnd, &mut *st_ptr);
            }
            0
        }
        WM_DPICHANGED => {
            let dpi = ((wparam & 0xffff) as u32).max(96);
            if lparam != 0 {
                let suggested = &*(lparam as *const RECT);
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            set_settings_ui_dpi(dpi);
            ensure_settings_window_in_work_area(hwnd);
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                reset_settings_dpi_compensation(&mut *st_ptr);
                (*st_ptr).ui_dpi = dpi;
                refresh_settings_window_metrics(hwnd, &mut *st_ptr);
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
                let paint_dpi = layout_dpi_for_window(hwnd);
                set_settings_ui_dpi(paint_dpi);
                crate::win_system_ui::set_paint_dpi_override(paint_dpi);
                let th = Theme::default();
                let mut rc: RECT = zeroed();
                GetClientRect(hwnd, &mut rc);
                let paint_target = begin_buffered_paint(hdc, &rc);
                let memdc = if let Some((_, pdc)) = paint_target {
                    pdc
                } else {
                    hdc
                };

                let bg = CreateSolidBrush(th.bg);
                FillRect(memdc, &rc, bg);
                DeleteObject(bg as _);

                let nav_rc = RECT {
                    left: 0,
                    top: 0,
                    right: settings_nav_w_scaled(),
                    bottom: rc.bottom,
                };
                draw_round_rect(memdc as _, &nav_rc, th.nav_bg, 0, 0);
                let line_pen = CreatePen(0, 1, th.stroke);
                let old_pen = SelectObject(memdc, line_pen as _);
                MoveToEx(memdc, nav_divider_x(), 0, null_mut());
                LineTo(memdc, nav_divider_x(), rc.bottom);
                SelectObject(memdc, old_pen);
                DeleteObject(line_pen as _);

                let menu_rc = RECT {
                    left: settings_scale(22),
                    top: settings_scale(18),
                    right: settings_scale(50),
                    bottom: settings_scale(46),
                };
                draw_text_ex(
                    memdc as _,
                    "",
                    &menu_rc,
                    th.text_muted,
                    16,
                    false,
                    false,
                    "Segoe Fluent Icons",
                );
                let title_rc = RECT {
                    left: settings_scale(56),
                    top: settings_scale(18),
                    right: settings_scale(220),
                    bottom: settings_scale(50),
                };
                draw_text_ex(
                    memdc as _,
                    "设置",
                    &title_rc,
                    th.text,
                    15,
                    true,
                    false,
                    "Segoe UI Variable Text",
                );
                let cur_page = if st_ptr.is_null() {
                    0
                } else {
                    (*st_ptr).cur_page.min(SETTINGS_PAGES.len() - 1)
                };
                let scroll_y = if st_ptr.is_null() {
                    0
                } else {
                    (*st_ptr).content_scroll_y
                };
                let sub_rc = settings_title_rect();
                draw_text_ex(
                    memdc as _,
                    SETTINGS_PAGES[cur_page],
                    &sub_rc,
                    th.text,
                    24,
                    true,
                    false,
                    "Segoe UI Variable Display",
                );

                for i in 0..SETTINGS_PAGES.len() {
                    let selected = !st_ptr.is_null() && (*st_ptr).cur_page == i;
                    let hover = !st_ptr.is_null() && (*st_ptr).nav_hot == i as i32;
                    draw_settings_nav_item(memdc as _, i, selected, hover, th);
                    if i == SettingsPage::About.index() && update_check_available() {
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
                IntersectClipRect(
                    memdc,
                    content_clip.left,
                    content_clip.top,
                    content_clip.right,
                    content_clip.bottom,
                );
                draw_settings_page_cards(memdc as _, cur_page, scroll_y, th);
                RestoreDC(memdc, -1);

                let mask_rc = settings_viewport_mask_rect(&rc);
                let mask_br = CreateSolidBrush(th.bg);
                FillRect(memdc, &mask_rc, mask_br);
                DeleteObject(mask_br as _);
                let mask_line = CreateSolidBrush(th.stroke);
                let mask_sep = RECT {
                    left: mask_rc.left + 12,
                    top: mask_rc.bottom - 1,
                    right: mask_rc.right - 12,
                    bottom: mask_rc.bottom,
                };
                FillRect(memdc, &mask_sep, mask_line);
                DeleteObject(mask_line as _);

                // 自绘 WinUI 细条滚动条（仅在滚动时短暂显示，1.5秒后自动隐藏）
                let content_y = settings_content_y_scaled();
                let view_h = (rc.bottom - rc.top) - content_y;
                let show_bar = !st_ptr.is_null()
                    && (*st_ptr).scroll_bar_visible
                    && settings_page_max_scroll_for_state(&*st_ptr, cur_page, view_h) > 0;
                if show_bar {
                    let dragging = !st_ptr.is_null() && (*st_ptr).scroll_dragging;
                    let bar_w = if dragging {
                        SCROLL_BAR_W_ACTIVE
                    } else {
                        SCROLL_BAR_W
                    };
                    let track_rc = RECT {
                        left: rc.right - bar_w - SCROLL_BAR_MARGIN,
                        top: content_y + settings_scale(8),
                        right: rc.right - SCROLL_BAR_MARGIN,
                        bottom: rc.bottom - settings_scale(8),
                    };
                    if dragging {
                        let track_color = if th.bg == rgb(32, 32, 32) {
                            rgb(70, 70, 70)
                        } else {
                            rgb(200, 200, 200)
                        };
                        let track_br = CreateSolidBrush(track_color);
                        let old_br = SelectObject(memdc, track_br as _);
                        let old_pn = SelectObject(memdc, GetStockObject(NULL_PEN));
                        RoundRect(
                            memdc,
                            track_rc.left,
                            track_rc.top,
                            track_rc.right + 1,
                            track_rc.bottom + 1,
                            bar_w,
                            bar_w,
                        );
                        SelectObject(memdc, old_pn);
                        SelectObject(memdc, old_br);
                        DeleteObject(track_br as _);
                    }
                    if let Some(thumb) = settings_scrollbar_thumb_w(&*st_ptr, &rc, scroll_y, bar_w)
                    {
                        let thumb_color = if dragging {
                            th.accent
                        } else if th.bg == rgb(32, 32, 32) {
                            rgb(120, 120, 120)
                        } else {
                            rgb(160, 160, 160)
                        };
                        let thumb_br = CreateSolidBrush(thumb_color);
                        let old_b2 = SelectObject(memdc, thumb_br as _);
                        let old_p2 = SelectObject(memdc, GetStockObject(NULL_PEN));
                        RoundRect(
                            memdc,
                            thumb.left,
                            thumb.top,
                            thumb.right + 1,
                            thumb.bottom + 1,
                            bar_w,
                            bar_w,
                        );
                        SelectObject(memdc, old_p2);
                        SelectObject(memdc, old_b2);
                        DeleteObject(thumb_br as _);
                    }
                }

                if let Some((paint_buf, _)) = paint_target {
                    end_buffered_paint(paint_buf, true);
                }
                crate::win_system_ui::clear_paint_dpi_override();
                EndPaint(hwnd, &ps);
            }
            0
        }
        WM_TIMER => {
            if wparam == ID_TIMER_SETTINGS_SCROLLBAR {
                let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
                if !st_ptr.is_null() {
                    let st = &mut *st_ptr;
                    stop_flagged_timer(
                        hwnd,
                        ID_TIMER_SETTINGS_SCROLLBAR,
                        &mut st.scroll_hide_timer,
                    );
                    st.scroll_bar_visible = false;
                    InvalidateRect(hwnd, null(), 0);
                }
                return 0;
            }
            if wparam == ID_TIMER_SETTINGS_SAVE_HINT {
                let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
                if !st_ptr.is_null() {
                    let st = &mut *st_ptr;
                    stop_flagged_timer(hwnd, ID_TIMER_SETTINGS_SAVE_HINT, &mut st.save_hint_timer);
                    settings_set_text(st.btn_save, tr("保存", "Save"));
                    InvalidateRect(st.btn_save, null(), 1);
                }
                return 0;
            }
            if wparam == ID_TIMER_SETTINGS_DPI_FIT {
                KillTimer(hwnd, ID_TIMER_SETTINGS_DPI_FIT);
                let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
                if !st_ptr.is_null() {
                    let st = &mut *st_ptr;
                    let _ = apply_settings_system_dpi_compensation(hwnd, st);
                    refresh_settings_window_metrics(hwnd, st);
                }
                return 0;
            }
            0
        }
        WM_CLOSE => {
            DestroyWindow(hwnd);
            0
        }
        WM_DESTROY => {
            let st_ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut SettingsWndState;
            if !st_ptr.is_null() {
                cancel_settings_scroll_drag(hwnd, &mut *st_ptr);
                KillTimer(hwnd, ID_TIMER_SETTINGS_SCROLLBAR);
                KillTimer(hwnd, ID_TIMER_SETTINGS_SAVE_HINT);
                KillTimer(hwnd, ID_TIMER_SETTINGS_DPI_FIT);
                let parent = (*st_ptr).parent_hwnd;
                if !(*st_ptr).dropdown_popup.is_null() {
                    if IsWindow((*st_ptr).dropdown_popup) != 0 {
                        DestroyWindow((*st_ptr).dropdown_popup);
                    }
                    (*st_ptr).dropdown_popup = null_mut();
                }
                if !(*st_ptr).nav_font.is_null() {
                    DeleteObject((*st_ptr).nav_font as _);
                }
                if !(*st_ptr).ui_font.is_null()
                    && (*st_ptr).ui_font != GetStockObject(DEFAULT_GUI_FONT)
                {
                    DeleteObject((*st_ptr).ui_font as _);
                }
                if !(*st_ptr).title_font.is_null()
                    && (*st_ptr).title_font != GetStockObject(DEFAULT_GUI_FONT)
                {
                    DeleteObject((*st_ptr).title_font as _);
                }
                if !(*st_ptr).bg_brush.is_null() {
                    DeleteObject((*st_ptr).bg_brush as _);
                }
                if !(*st_ptr).surface_brush.is_null() {
                    DeleteObject((*st_ptr).surface_brush as _);
                }
                if !(*st_ptr).control_brush.is_null() {
                    DeleteObject((*st_ptr).control_brush as _);
                }
                if !(*st_ptr).nav_brush.is_null() {
                    DeleteObject((*st_ptr).nav_brush as _);
                }
                drop(Box::from_raw(st_ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                let pst = get_state_ptr(parent);
                if !pst.is_null() {
                    (*pst).settings_hwnd = null_mut();
                }
                refresh_low_level_input_hooks();
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
        if !main.is_null() {
            main
        } else {
            hwnd
        }
    };
    let pst = get_state_ptr(owner_hwnd);
    if pst.is_null() {
        return;
    }
    let app = &mut *pst;
    if !app.settings_hwnd.is_null() {
        app.edge_hide_pending_until = None;
        app.edge_hide_grace_until = None;
        let st_ptr = GetWindowLongPtrW(app.settings_hwnd, GWLP_USERDATA) as *mut SettingsWndState;
        if !st_ptr.is_null() {
            let _ = apply_settings_system_dpi_compensation(app.settings_hwnd, &mut *st_ptr);
            refresh_settings_window_metrics(app.settings_hwnd, &mut *st_ptr);
        }
        ShowWindow(app.settings_hwnd, SW_SHOW);
        SetForegroundWindow(app.settings_hwnd);
        refresh_low_level_input_hooks();
        return;
    }
    ensure_settings_class();
    let hinstance = GetModuleHandleW(null());
    let mut anchor: POINT = zeroed();
    if GetCursorPos(&mut anchor) == 0 {
        anchor.x = GetSystemMetrics(SM_CXSCREEN) / 2;
        anchor.y = GetSystemMetrics(SM_CYSCREEN) / 2;
    }
    let mut owner_rc: RECT = zeroed();
    if !owner_hwnd.is_null() && GetWindowRect(owner_hwnd, &mut owner_rc) != 0 {
        anchor.x = owner_rc.left + ((owner_rc.right - owner_rc.left) / 2);
        anchor.y = owner_rc.top + ((owner_rc.bottom - owner_rc.top) / 2);
    }
    let work = nearest_monitor_work_rect_for_point(anchor);
    set_settings_ui_dpi(layout_dpi_for_point(anchor));
    let settings_w = settings_w_scaled();
    let settings_h = settings_h_scaled();
    let x = max(
        work.left,
        work.left + ((work.right - work.left - settings_w) / 2),
    );
    let y = max(
        work.top,
        work.top + ((work.bottom - work.top - settings_h) / 2),
    );
    let whd = CreateWindowExW(
        WS_EX_APPWINDOW | WS_EX_DLGMODALFRAME,
        to_wide(SETTINGS_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_OVERLAPPED
            | WS_CAPTION
            | WS_SYSMENU
            | WS_MINIMIZEBOX
            | WS_MAXIMIZEBOX
            | WS_THICKFRAME
            | WS_VISIBLE
            | WS_CLIPCHILDREN,
        x,
        y,
        settings_w,
        settings_h,
        owner_hwnd,
        null_mut(),
        hinstance,
        owner_hwnd as _,
    );
    if !whd.is_null() {
        apply_window_corner_preference(whd);
        apply_dark_mode_to_window(whd);
        app.settings_hwnd = whd;
        app.edge_hide_pending_until = None;
        app.edge_hide_grace_until = None;
        refresh_low_level_input_hooks();
    }
}

pub fn run() -> AppResult<()> {
    let boot_settings = load_settings();
    // ── 单实例保护：若已有实例运行则激活它并退出 ──
    unsafe {
        #[link(name = "kernel32")]
        unsafe extern "system" {
            fn CreateMutexW(
                lp_attributes: *const core::ffi::c_void,
                b_initial_owner: i32,
                lp_name: *const u16,
            ) -> *mut core::ffi::c_void;
            fn GetLastError() -> u32;
        }
        const ERROR_ALREADY_EXISTS: u32 = 183;
        let name: Vec<u16> = "Global\\ZsClipSingleInstance"
            .encode_utf16()
            .chain(std::iter::once(0u16))
            .collect();
        let _mutex = CreateMutexW(core::ptr::null(), 0, name.as_ptr());
        if GetLastError() == ERROR_ALREADY_EXISTS {
            // 已有实例：找到主窗口并激活
            use windows_sys::Win32::UI::WindowsAndMessaging::{
                FindWindowW, PostMessageW, SetForegroundWindow, ShowWindow, SW_RESTORE,
            };
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
        let startup_layout = main_layout_for_window(null_mut());
        let startup_h = startup_layout.list_y + startup_layout.list_h + 7;
        let main_hwnd = CreateWindowExW(
            WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            to_wide(WindowRole::Main.class_name()).as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            startup_layout.win_w,
            startup_h,
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
            startup_layout.win_w,
            startup_h,
            null_mut(),
            null_mut(),
            hinstance,
            WindowRole::Quick as usize as _,
        );
        if quick_hwnd.is_null() {
            DestroyWindow(main_hwnd);
            return Err(io::Error::last_os_error());
        }

        ShowWindow(
            main_hwnd,
            if startup_can_hide(&boot_settings) {
                SW_HIDE
            } else {
                SW_SHOW
            },
        );
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

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
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
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                if wparam == SIZE_MINIMIZED as usize {
                    hide_hover_preview();
                    schedule_hidden_memory_reclaim(hwnd, state);
                } else {
                    cancel_hidden_memory_reclaim(hwnd, state);
                }
            }
            apply_main_window_region(hwnd);
            layout_children(hwnd);
            InvalidateRect(hwnd, null(), 1);
            0
        }
        WM_SHOWWINDOW => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                if wparam == 0 {
                    hide_hover_preview();
                    schedule_hidden_memory_reclaim(hwnd, state);
                } else {
                    cancel_hidden_memory_reclaim(hwnd, state);
                    SetTimer(hwnd, ID_TIMER_DPI_FIT, 60, None);
                }
            }
            refresh_low_level_input_hooks();
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_TIMER => {
            if wparam == ID_TIMER_STARTUP_RECOVERY {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    retry_startup_integrations(hwnd, &mut *ptr);
                }
                return 0;
            }
            if wparam == ID_TIMER_VV_WATCH {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    if state.vv_popup_visible
                        && !vv_popup_menu_active()
                        && (GetForegroundWindow() != state.vv_popup_target
                            || IsWindow(state.vv_popup_target) == 0)
                    {
                        vv_popup_hide(hwnd, state);
                    }
                }
                return 0;
            }
            if wparam == ID_TIMER_VV_SHOW {
                KillTimer(hwnd, ID_TIMER_VV_SHOW);
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    let target = state.vv_popup_pending_target;
                    if !target.is_null() && GetForegroundWindow() == target && IsWindow(target) != 0
                    {
                        state.vv_popup_pending_target = null_mut();
                        if !vv_popup_show(hwnd, state, target) && state.vv_popup_pending_retries > 0
                        {
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
            if wparam == ID_TIMER_PASTE {
                KillTimer(hwnd, ID_TIMER_PASTE);
                let mut should_send_paste = true;
                let mut should_play_sound = false;
                let mut paste_target = null_mut();
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    let target = state.paste_target_override;
                    paste_target = target;
                    if !target.is_null() {
                        should_send_paste = force_foreground_window(target);
                        if should_send_paste {
                            restore_hotkey_focus_target(state, target);
                            should_send_paste = can_send_ctrl_v_to_target(state, target);
                        }
                    }
                    if should_send_paste {
                        send_backspace_times(state.paste_backspace_count);
                    }
                    state.paste_backspace_count = 0;
                    state.paste_target_override = null_mut();
                    clear_hotkey_passthrough_state(state);
                    should_play_sound = state.settings.paste_success_sound_enabled;
                }
                if should_send_paste {
                    send_ctrl_v();
                    if should_play_sound {
                        let ptr = get_state_ptr(hwnd);
                        if !ptr.is_null() {
                            play_paste_success_sound(
                                &(*ptr).settings.paste_success_sound_kind,
                                &(*ptr).settings.paste_success_sound_path,
                            );
                        }
                    }
                } else if !ptr.is_null() {
                    show_paste_failure_message(hwnd, &*ptr, paste_target);
                }
                return 0;
            }
            if wparam == ID_TIMER_SEARCH_DEBOUNCE {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    stop_search_debounce_timer(hwnd, state);
                    apply_search_filter(hwnd, state);
                }
                return 0;
            }
            if wparam == ID_TIMER_HIDDEN_RECLAIM {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    cancel_hidden_memory_reclaim(hwnd, state);
                    if IsWindowVisible(hwnd) == 0 || IsIconic(hwnd) != 0 {
                        hide_hover_preview();
                        state.release_list_memory();
                        trim_process_working_set();
                    }
                }
                return 0;
            }
            if wparam == ID_TIMER_CLIPBOARD_RETRY {
                capture_clipboard(hwnd);
                return 0;
            }
            if wparam == ID_TIMER_DPI_FIT {
                KillTimer(hwnd, ID_TIMER_DPI_FIT);
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    ensure_main_window_size_for_monitor(hwnd, &mut *ptr);
                }
                return 0;
            }
            if wparam == ID_TIMER_SCROLL_FADE {
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
                            stop_flagged_timer(
                                hwnd,
                                ID_TIMER_SCROLL_FADE,
                                &mut state.scroll_fade_timer,
                            );
                        }
                    }
                    InvalidateRect(hwnd, null(), 0);
                }
                return 0;
            }
            if wparam == ID_TIMER_EDGE_AUTO_HIDE {
                handle_edge_auto_hide_tick(hwnd);
                return 0;
            }
            if wparam == ID_TIMER_OUTSIDE_HIDE {
                hosts::handle_outside_hide_tick(hwnd);
                return 0;
            }
            if wparam == ID_TIMER_CLOUD_SYNC {
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
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                cancel_main_scroll_drag(hwnd, &mut *ptr);
            }
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
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    (*ptr).plain_text_paste_mode = false;
                }
                toggle_window_visibility_hotkey(hwnd);
                return 0;
            }
            if wparam as i32 == HOTKEY_ID_PLAIN {
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    (*ptr).plain_text_paste_mode = true;
                }
                toggle_window_visibility_hotkey(hwnd);
                return 0;
            }
            0
        }
        WM_VV_SHOW => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                let mut target = wparam as HWND;
                if target.is_null() || IsWindow(target) == 0 {
                    let fg = GetForegroundWindow();
                    if !fg.is_null() && IsWindow(fg) != 0 {
                        target = fg;
                    }
                }
                if !target.is_null() && IsWindow(target) != 0 {
                    state.vv_popup_pending_target = target;
                    state.vv_popup_pending_retries = VV_SHOW_RETRY_MAX;
                    KillTimer(hwnd, ID_TIMER_VV_SHOW);
                    SetTimer(hwnd, ID_TIMER_VV_SHOW, VV_SHOW_RETRY_DELAY_MS, None);
                } else {
                    state.vv_popup_pending_target = null_mut();
                    state.vv_popup_pending_retries = 0;
                }
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
                handle_vv_select(hwnd, &mut *ptr, wparam);
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
        WM_IMAGE_PASTE_READY => {
            if lparam == 0 {
                return 0;
            }
            let payload = Box::from_raw(lparam as *mut ImagePasteReadyResult);
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                if let Some((bytes, width, height)) = payload.image {
                    if let Ok(mut clipboard) = Clipboard::new() {
                        if clipboard
                            .set_image(ImageData {
                                width,
                                height,
                                bytes: Cow::Owned(bytes.clone()),
                            })
                            .is_ok()
                        {
                            state.note_programmatic_clipboard_signature(
                                format!(
                                    "img:{}:{}:{}",
                                    width,
                                    height,
                                    hash_bytes(bytes.as_slice())
                                ),
                                1200,
                            );
                            set_ignore_clipboard_for_all_hosts(1200);
                            paste_after_clipboard_ready_to_target(
                                hwnd,
                                state,
                                payload.target as HWND,
                                payload.hide_main,
                                payload.backspaces,
                            );
                        }
                    }
                }
            }
            0
        }
        WM_IMAGE_OCR_READY => {
            if lparam == 0 {
                return 0;
            }
            let payload = Box::from_raw(lparam as *mut ImageOcrReadyResult);
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                if let Some(text) = payload.text {
                    let normalized = text.replace("\r\n", "\n");
                    let preview = build_preview(&normalized);
                    let sig = format!("txt:{}", hash_bytes(normalized.as_bytes()));
                    skip_next_clipboard_update_for_all_hosts();
                    if let Ok(mut clipboard) = Clipboard::new() {
                        let _ = clipboard.set_text(normalized.clone());
                    }
                    state.add_clip_item(
                        ClipItem {
                            id: 0,
                            kind: ClipKind::Text,
                            preview,
                            text: Some(normalized),
                            source_app: String::new(),
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
                } else if let Some(err) = payload.error {
                    MessageBoxW(
                        hwnd,
                        to_wide(&format!(
                            "{}: {}",
                            tr("图片转文字失败", "Image OCR failed"),
                            err
                        ))
                        .as_ptr(),
                        to_wide(tr("图片转文字", "Image OCR")).as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                }
            }
            0
        }
        WM_TEXT_TRANSLATE_READY => {
            if lparam == 0 {
                return 0;
            }
            let payload = Box::from_raw(lparam as *mut TextTranslateReadyResult);
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                if let Some(text) = payload.text {
                    let normalized = text.replace("\r\n", "\n");
                    let preview = build_preview(&normalized);
                    let sig = format!("txt:{}", hash_bytes(normalized.as_bytes()));
                    skip_next_clipboard_update_for_all_hosts();
                    if let Ok(mut clipboard) = Clipboard::new() {
                        let _ = clipboard.set_text(normalized.clone());
                    }
                    state.add_clip_item(
                        ClipItem {
                            id: 0,
                            kind: ClipKind::Text,
                            preview,
                            text: Some(normalized),
                            source_app: String::new(),
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
                } else if let Some(err) = payload.error {
                    MessageBoxW(
                        hwnd,
                        to_wide(&format!(
                            "{}: {}",
                            tr("文本翻译失败", "Text translation failed"),
                            err
                        ))
                        .as_ptr(),
                        to_wide(tr("文本翻译", "Text Translate")).as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                }
            }
            0
        }
        WM_IMAGE_THUMB_READY => {
            if lparam == 0 {
                return 0;
            }
            let payload = Box::from_raw(lparam as *mut ImageThumbReadyResult);
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                state.image_thumb_loading.remove(&payload.item_id);
                if let Some(image) = payload.image {
                    state.image_thumb_cache.put(payload.item_id, image);
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_UPDATE_CHECK_READY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null()
                && !(*ptr).settings_hwnd.is_null()
                && IsWindow((*ptr).settings_hwnd) != 0
            {
                InvalidateRect((*ptr).settings_hwnd, null(), 1);
            }
            0
        }
        WM_ACTIVATEAPP => {
            if wparam == 0 {
                clear_main_hover_state(hwnd);
                let ptr = get_state_ptr(hwnd);
                if !ptr.is_null() {
                    let state = &mut *ptr;
                    if state.settings.auto_hide_on_blur
                        && !state.main_window_noactivate
                        && state.role != WindowRole::Quick
                        && !vv_popup_menu_active()
                    {
                        let mut pt: POINT = zeroed();
                        if GetCursorPos(&mut pt) == 0
                            || !hosts::edge_window_scope_contains_point(hwnd, pt)
                        {
                            hide_hover_preview();
                            ShowWindow(hwnd, SW_HIDE);
                            refresh_low_level_input_hooks();
                        }
                    }
                }
            }
            0
        }
        WM_SETTINGCHANGE | WM_DISPLAYCHANGE => {
            refresh_main_window_metrics(hwnd);
            SetTimer(hwnd, ID_TIMER_DPI_FIT, 60, None);
            0
        }
        WM_TRAYICON => {
            handle_tray(hwnd, lparam as u32);
            0
        }
        WM_MOVE => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                if (*ptr).role == WindowRole::Main && !hosts::edge_animation_active(&*ptr) {
                    remember_window_pos(hwnd);
                }
                note_window_moved_for_edge_hide(hwnd, &mut *ptr);
                let dpi = layout_dpi_for_window(hwnd);
                if (*ptr).ui_dpi != dpi.max(96) {
                    refresh_main_window_layout_for_monitor(hwnd, &mut *ptr, Some(dpi));
                }
                if !is_per_monitor_dpi_aware() {
                    SetTimer(hwnd, ID_TIMER_DPI_FIT, 60, None);
                }
            }
            0
        }
        WM_EXITSIZEMOVE => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                ensure_main_window_size_for_monitor(hwnd, &mut *ptr);
                if (*ptr).role == WindowRole::Main && !hosts::edge_animation_active(&*ptr) {
                    remember_window_pos(hwnd);
                }
            }
            0
        }
        WM_DPICHANGED => {
            if lparam != 0 {
                let suggested = &*(lparam as *const RECT);
                SetWindowPos(
                    hwnd,
                    null_mut(),
                    suggested.left,
                    suggested.top,
                    suggested.right - suggested.left,
                    suggested.bottom - suggested.top,
                    SWP_NOZORDER | SWP_NOACTIVATE,
                );
            }
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let dpi = (wparam & 0xffff) as u32;
                reset_main_dpi_compensation(&mut *ptr);
                refresh_main_window_layout_for_monitor(hwnd, &mut *ptr, Some(dpi));
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
                cancel_main_scroll_drag(hwnd, &mut *ptr);
                clear_page_load_results_for_hwnd(hwnd);
                clear_cloud_sync_results_for_hwnd(hwnd);
                match (*ptr).role {
                    WindowRole::Main => {
                        save_settings(&(*ptr).settings);
                        KillTimer(hwnd, ID_TIMER_STARTUP_RECOVERY);
                        KillTimer(hwnd, ID_TIMER_VV_WATCH);
                        KillTimer(hwnd, ID_TIMER_VV_SHOW);
                        KillTimer(hwnd, ID_TIMER_PASTE);
                        KillTimer(hwnd, ID_TIMER_SEARCH_DEBOUNCE);
                        KillTimer(hwnd, ID_TIMER_HIDDEN_RECLAIM);
                        KillTimer(hwnd, ID_TIMER_CLIPBOARD_RETRY);
                        KillTimer(hwnd, ID_TIMER_DPI_FIT);
                        KillTimer(hwnd, ID_TIMER_SCROLL_FADE);
                        KillTimer(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
                        KillTimer(hwnd, ID_TIMER_OUTSIDE_HIDE);
                        KillTimer(hwnd, ID_TIMER_CLOUD_SYNC);
                        let popup = current_vv_popup_hwnd();
                        if !popup.is_null() && IsWindow(popup) != 0 {
                            DestroyWindow(popup);
                        }
                        update_vv_mode_hook(hwnd, false);
                        shutdown_low_level_input_hooks();
                        RemoveClipboardFormatListener(hwnd);
                        unregister_hotkey_for(hwnd, &mut *ptr);
                        unregister_plain_paste_hotkey_for(hwnd, &mut *ptr);
                        remove_tray_icon(hwnd);
                        let quick = quick_window_hwnd();
                        if !quick.is_null() && quick != hwnd && IsWindow(quick) != 0 {
                            DestroyWindow(quick);
                        }
                        PostQuitMessage(0);
                    }
                    WindowRole::Quick => {
                        KillTimer(hwnd, ID_TIMER_PASTE);
                        KillTimer(hwnd, ID_TIMER_SEARCH_DEBOUNCE);
                        KillTimer(hwnd, ID_TIMER_HIDDEN_RECLAIM);
                        KillTimer(hwnd, ID_TIMER_CLIPBOARD_RETRY);
                        KillTimer(hwnd, ID_TIMER_DPI_FIT);
                        KillTimer(hwnd, ID_TIMER_SCROLL_FADE);
                        KillTimer(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
                        KillTimer(hwnd, ID_TIMER_OUTSIDE_HIDE);
                        refresh_low_level_input_hooks();
                    }
                }
            }
            0
        }
        WM_NCDESTROY => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                clear_window_host((*ptr).role, hwnd);
                if !(*ptr).search_font.is_null() {
                    DeleteObject((*ptr).search_font as _);
                    (*ptr).search_font = null_mut();
                }
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

    let layout = main_layout_for_window(hwnd);
    let search_hwnd = CreateWindowExW(
        0,
        to_wide("EDIT").as_ptr(),
        to_wide("").as_ptr(),
        WS_CHILD | WS_VISIBLE | WS_TABSTOP | (ES_AUTOHSCROLL as u32),
        layout.search_left + 10,
        layout.search_top + 3,
        layout.search_w - 20,
        layout.search_h - 6,
        hwnd,
        IDC_SEARCH as usize as _,
        hinstance,
        null(),
    );
    if search_hwnd.is_null() {
        return Err(io::Error::last_os_error());
    }
    SendMessageW(
        search_hwnd,
        EM_SETMARGINS,
        (EC_LEFTMARGIN | EC_RIGHTMARGIN) as WPARAM,
        0,
    );

    let icons = load_icons();
    let tray_icon = icons.app;
    if icons.app != 0 {
        SendMessageW(hwnd, WM_SETICON, ICON_SMALL as WPARAM, icons.app as LPARAM);
        SendMessageW(hwnd, WM_SETICON, ICON_BIG as WPARAM, icons.app as LPARAM);
    }

    let state = Box::new(AppState::new(role, hwnd, search_hwnd, icons));
    SetWindowLongPtrW(hwnd, GWLP_USERDATA, Box::into_raw(state) as isize);
    set_window_host(role, hwnd);
    if let Some(state) = unsafe { get_state_mut(hwnd) } {
        refresh_search_font(state);
        ensure_db();
        if role == WindowRole::Main {
            reload_state_from_db_persisting(state);
            register_hotkey_for(hwnd, state);
            register_plain_paste_hotkey_for(hwnd, state);
            update_vv_mode_hook(hwnd, state.settings.vv_mode_enabled);
            position_main_window(hwnd, &state.settings, false);
            refresh_low_level_input_hooks();
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
        refresh_low_level_input_hooks();
    }
    refresh_low_level_input_hooks();
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
    if role == WindowRole::Main {
        SetTimer(hwnd, ID_TIMER_STARTUP_RECOVERY, 500, None);
        SetTimer(hwnd, ID_TIMER_VV_WATCH, 500, None);
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
    ShowWindow(
        state.search_hwnd,
        if state.search_on { SW_SHOW } else { SW_HIDE },
    );
}

unsafe fn refresh_search_font(state: &mut AppState) {
    let created = CreateFontW(
        -scale_for_window(state.hwnd, 14),
        0,
        0,
        0,
        400,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        to_wide(ui_text_font_family()).as_ptr(),
    ) as *mut core::ffi::c_void;
    let old_font = state.search_font;
    state.search_font = created;
    let font: *mut core::ffi::c_void = if created.is_null() {
        GetStockObject(DEFAULT_GUI_FONT) as _
    } else {
        created
    };
    SendMessageW(state.search_hwnd, WM_SETFONT, font as WPARAM, 1 as LPARAM);
    if !old_font.is_null() {
        DeleteObject(old_font as _);
    }
}

unsafe fn refresh_main_window_metrics(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) != 0 {
        let layout = main_layout_for_window(hwnd);
        let win_h = layout.list_y + layout.list_h + 7;
        SetWindowPos(
            hwnd,
            null_mut(),
            rc.left,
            rc.top,
            layout.win_w,
            win_h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }
    refresh_main_window_layout_only(hwnd, state);
}

unsafe fn ensure_main_window_size_for_monitor(hwnd: HWND, state: &mut AppState) {
    if hosts::edge_animation_active(state) || state.edge_hidden {
        return;
    }
    if !is_per_monitor_dpi_aware() {
        if apply_main_system_dpi_compensation(hwnd, state) {
            return;
        }
        refresh_main_window_layout_for_monitor(hwnd, state, Some(layout_dpi_for_window(hwnd)));
        return;
    }
    let dpi = layout_dpi_for_window(hwnd).max(96);
    let (win_w, win_h) = main_window_size_for_dpi(dpi);
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) == 0 || rc.right <= rc.left || rc.bottom <= rc.top {
        refresh_main_window_layout_for_monitor(hwnd, state, Some(dpi));
        return;
    }
    let cur_w = rc.right - rc.left;
    let cur_h = rc.bottom - rc.top;
    if state.ui_dpi != dpi || (cur_w - win_w).abs() > 2 || (cur_h - win_h).abs() > 2 {
        let work = nearest_monitor_work_rect_for_window(hwnd);
        let (x, y) = clamp_window_pos_to_rect(rc.left, rc.top, &work, win_w, win_h);
        SetWindowPos(
            hwnd,
            null_mut(),
            x,
            y,
            win_w,
            win_h,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
        refresh_main_window_layout_for_monitor(hwnd, state, Some(dpi));
    }
}

unsafe fn reset_main_dpi_compensation(state: &mut AppState) {
    state.dpi_comp_base_w = 0;
    state.dpi_comp_base_h = 0;
    state.dpi_comp_base_monitor_dpi = 0;
    state.dpi_comp_last_monitor_dpi = 0;
    state.dpi_comp_applying = false;
}

unsafe fn apply_main_system_dpi_compensation(hwnd: HWND, state: &mut AppState) -> bool {
    if is_per_monitor_dpi_aware() {
        reset_main_dpi_compensation(state);
        return false;
    }
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) == 0 || rc.right <= rc.left || rc.bottom <= rc.top {
        return false;
    }
    let monitor_dpi = monitor_dpi_for_window(hwnd).max(96);
    if state.dpi_comp_base_monitor_dpi == 0
        || state.dpi_comp_base_w <= 0
        || state.dpi_comp_base_h <= 0
    {
        state.dpi_comp_base_w = rc.right - rc.left;
        state.dpi_comp_base_h = rc.bottom - rc.top;
        state.dpi_comp_base_monitor_dpi = monitor_dpi;
        state.dpi_comp_last_monitor_dpi = monitor_dpi;
        return false;
    }
    let (mut target_w, mut target_h) = dpi_compensated_size(
        state.dpi_comp_base_w,
        state.dpi_comp_base_h,
        state.dpi_comp_base_monitor_dpi,
        monitor_dpi,
    );
    let cur_w = rc.right - rc.left;
    let cur_h = rc.bottom - rc.top;
    if state.dpi_comp_last_monitor_dpi == monitor_dpi
        && (cur_w - target_w).abs() <= 2
        && (cur_h - target_h).abs() <= 2
    {
        return false;
    }
    let work = nearest_monitor_work_rect_for_window(hwnd);
    target_w = target_w.min((work.right - work.left).max(1)).max(1);
    target_h = target_h.min((work.bottom - work.top).max(1)).max(1);
    let center_x = rc.left + cur_w / 2;
    let center_y = rc.top + cur_h / 2;
    let (x, y) = clamp_window_pos_to_rect(
        center_x - target_w / 2,
        center_y - target_h / 2,
        &work,
        target_w,
        target_h,
    );
    state.dpi_comp_applying = true;
    SetWindowPos(
        hwnd,
        null_mut(),
        x,
        y,
        target_w,
        target_h,
        SWP_NOZORDER | SWP_NOACTIVATE,
    );
    state.dpi_comp_applying = false;
    state.dpi_comp_last_monitor_dpi = monitor_dpi;
    refresh_main_window_layout_for_monitor(hwnd, state, Some(layout_dpi_for_window(hwnd)));
    true
}

unsafe fn sync_main_window_dpi(state: &mut AppState, dpi: u32) -> bool {
    let next = dpi.max(96);
    if state.ui_dpi == next {
        return false;
    }
    state.ui_dpi = next;
    true
}

pub(crate) unsafe fn refresh_main_window_layout_for_monitor(
    hwnd: HWND,
    state: &mut AppState,
    forced_dpi: Option<u32>,
) {
    let dpi = forced_dpi.unwrap_or_else(|| layout_dpi_for_window(hwnd));
    let _ = sync_main_window_dpi(state, dpi);
    refresh_search_font(state);
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
}

unsafe fn refresh_main_window_layout_only(hwnd: HWND, state: &mut AppState) {
    refresh_main_window_layout_for_monitor(hwnd, state, None);
}

pub(crate) unsafe fn reset_search_ui_state(state: &mut AppState) {
    if !state.search_on && state.search_text.is_empty() {
        return;
    }
    if !state.hwnd.is_null() && state.search_debounce_timer {
        stop_search_debounce_timer(state.hwnd, state);
    }
    state.search_on = false;
    state.search_text.clear();
    SetWindowTextW(state.search_hwnd, to_wide("").as_ptr());
    state.clear_selection();
    state.refilter();
    if !state.hwnd.is_null() {
        layout_children(state.hwnd);
        InvalidateRect(state.hwnd, null(), 1);
    }
}

pub(crate) unsafe fn prepare_search_ui_for_show(hwnd: HWND, state: &mut AppState) {
    if state.settings.persistent_search_box {
        state.search_on = true;
        layout_children(hwnd);
        InvalidateRect(hwnd, null(), 0);
    } else {
        reset_search_ui_state(state);
    }
}

unsafe fn activate_window_for_search_input(hwnd: HWND, state: &mut AppState) {
    if state.main_window_noactivate || state.role == WindowRole::Quick {
        set_main_window_noactivate_mode(hwnd, false);
        ShowWindow(hwnd, SW_SHOW);
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        let _ = force_foreground_window(hwnd);
    }
    SetFocus(state.search_hwnd);
}

unsafe fn open_search_ui(hwnd: HWND, state: &mut AppState) {
    if !state.search_on {
        state.search_on = true;
    }
    layout_children(hwnd);
    activate_window_for_search_input(hwnd, state);
    InvalidateRect(hwnd, null(), 1);
}

unsafe fn close_search_ui(hwnd: HWND, state: &mut AppState) {
    if state.settings.persistent_search_box {
        state.search_on = true;
        layout_children(hwnd);
        InvalidateRect(hwnd, null(), 1);
        return;
    }
    if !state.search_on && state.search_text.is_empty() {
        return;
    }
    if state.search_debounce_timer {
        stop_search_debounce_timer(hwnd, state);
    }
    reset_search_ui_state(state);
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
}

fn select_context_row(state: &mut AppState) -> bool {
    if state.context_row < 0 {
        return false;
    }
    state.sel_idx = state.context_row;
    true
}

unsafe fn execute_row_command(hwnd: HWND, state: &mut AppState, cmd: usize) {
    match cmd {
        IDM_ROW_PASTE => {
            if select_context_row(state) {
                paste_selected(hwnd, state);
                InvalidateRect(hwnd, null(), 0);
            }
        }
        IDM_ROW_COPY => {
            if select_context_row(state) {
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
            if select_context_row(state) {
                state.toggle_pin_rows();
                InvalidateRect(hwnd, null(), 1);
            }
        }
        IDM_ROW_DELETE => {
            if select_context_row(state) {
                state.delete_selected_rows();
                InvalidateRect(hwnd, null(), 1);
            }
        }
        IDM_ROW_DELETE_UNPINNED => {
            let anchor = state.current_scroll_anchor();
            if db_delete_unpinned_items(source_tab_category(state.tab_index)).is_ok() {
                let active_tab = state.tab_index;
                state
                    .items_for_tab_mut(active_tab)
                    .retain(|item| item.pinned);
                state.clear_selection();
                state.refilter();
                state.restore_scroll_anchor(anchor);
                sync_peer_windows_from_db(hwnd);
                InvalidateRect(hwnd, null(), 1);
            }
        }
        IDM_ROW_TO_PHRASE => {
            if select_context_row(state) {
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
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    if item.kind == ClipKind::Image {
                        show_image_sticker(&item, &state.settings);
                    }
                }
            }
        }
        IDM_ROW_SAVE_IMAGE => {
            if select_context_row(state) {
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
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    if let Some(paths) = &item.file_paths {
                        for p in paths {
                            open_path_with_shell(p);
                        }
                    }
                }
            }
        }
        IDM_ROW_OPEN_FOLDER => {
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    if let Some(paths) = &item.file_paths {
                        for p in paths {
                            open_parent_folder(p);
                        }
                    }
                }
            }
        }
        IDM_ROW_COPY_PATH => {
            if select_context_row(state) {
                let items = state.selected_items_for_use();
                let mut lines = Vec::new();
                if items.is_empty() {
                    if let Some(item) = state.current_item_for_use() {
                        if let Some(paths) = &item.file_paths {
                            lines.extend(paths.clone());
                        }
                    }
                } else {
                    for item in &items {
                        if let Some(paths) = &item.file_paths {
                            lines.extend(paths.clone());
                        }
                    }
                }
                if !lines.is_empty() {
                    if let Ok(mut cb) = Clipboard::new() {
                        let _ = cb.set_text(lines.join("\n"));
                    }
                }
            }
        }
        IDM_ROW_QUICK_SEARCH => {
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    let text = match item.kind {
                        ClipKind::Text | ClipKind::Phrase => {
                            item.text.clone().unwrap_or_else(|| item.preview.clone())
                        }
                        ClipKind::Files => item
                            .file_paths
                            .as_ref()
                            .map(|v| v.join(" "))
                            .unwrap_or_else(|| item.preview.clone()),
                        ClipKind::Image => item.preview.clone(),
                    };
                    quick_search_open(&state.settings, &text);
                }
            }
        }
        IDM_ROW_EXPORT_FILE => {
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    if let Some(path) = materialize_item_as_file(&item) {
                        if let Some(parent) = path.parent().and_then(|p| p.to_str()) {
                            open_path_with_shell(parent);
                        }
                    }
                }
            }
        }
        IDM_ROW_IMAGE_OCR => {
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    spawn_image_ocr_job(hwnd, state.settings.clone(), item);
                }
            }
        }
        IDM_ROW_TEXT_TRANSLATE => {
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    spawn_text_translate_job(hwnd, state.settings.clone(), item);
                }
            }
        }
        IDM_ROW_QR_IMAGE => {
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    if let Some(text) = text_payload_for_qr(&item) {
                        if let Some((qr_item, sig)) = build_qr_clip_item(&text) {
                            let _ = apply_item_to_clipboard(state, &qr_item);
                            state.add_clip_item(qr_item, sig);
                            InvalidateRect(hwnd, null(), 1);
                        } else {
                            MessageBoxW(
                                hwnd,
                                to_wide(tr("二维码生成失败", "QR generation failed")).as_ptr(),
                                to_wide(tr("快捷转换二维码", "Quick QR Convert")).as_ptr(),
                                MB_OK | MB_ICONERROR,
                            );
                        }
                    }
                }
            }
        }
        IDM_ROW_MAIL_MERGE => {
            if !state.settings.super_mail_merge_enabled {
                return;
            }
            if select_context_row(state) {
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
            if select_context_row(state) {
                if let Some(item) = state.current_item_owned() {
                    let item_id = item.id;
                    let title = format!(
                        "编辑 — {}",
                        item.preview.chars().take(40).collect::<String>()
                    );
                    let saved = show_edit_item_dialog(hwnd, item_id, &title);
                    if saved {
                        reload_state_from_db_persisting(state);
                        state.refilter();
                        sync_peer_windows_from_db(hwnd);
                        InvalidateRect(hwnd, null(), 1);
                    }
                }
            }
        }
        IDM_ROW_GROUP_REMOVE => {
            if select_context_row(state) {
                let ids = state.selected_db_ids();
                if !ids.is_empty() {
                    let _ = db_assign_group(&ids, 0);
                    reload_state_from_db_persisting(state);
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
            if select_context_row(state) {
                let idx = cmd - IDM_ROW_GROUP_BASE;
                if let Some(group_id) = state.groups_for_tab(state.tab_index).get(idx).map(|g| g.id)
                {
                    let ids = state.selected_db_ids();
                    if !ids.is_empty() {
                        let _ = db_assign_group(&ids, group_id);
                        reload_state_from_db_persisting(state);
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
    let code = hiword(wparam as u32);

    if id == IDC_SEARCH as usize && code == EN_CHANGE_CODE {
        state.search_text = get_window_text(state.search_hwnd);
        start_flagged_timer(
            hwnd,
            ID_TIMER_SEARCH_DEBOUNCE,
            180,
            &mut state.search_debounce_timer,
        );
        return;
    }

    match id {
        IDM_TRAY_TOGGLE => {
            toggle_window_visibility(hwnd);
        }
        IDM_TRAY_EXIT => {
            DestroyWindow(hwnd);
        }
        IDM_ROW_PASTE
        | IDM_ROW_COPY
        | IDM_ROW_PIN
        | IDM_ROW_DELETE
        | IDM_ROW_DELETE_UNPINNED
        | IDM_ROW_TO_PHRASE
        | IDM_ROW_STICKER
        | IDM_ROW_SAVE_IMAGE
        | IDM_ROW_IMAGE_OCR
        | IDM_ROW_TEXT_TRANSLATE
        | IDM_ROW_OPEN_PATH
        | IDM_ROW_OPEN_FOLDER
        | IDM_ROW_COPY_PATH
        | IDM_ROW_GROUP_REMOVE
        | IDM_ROW_EDIT
        | IDM_ROW_QUICK_SEARCH
        | IDM_ROW_EXPORT_FILE
        | IDM_ROW_MAIL_MERGE
        | IDM_GROUP_FILTER_ALL => {
            execute_row_command(hwnd, state, id);
        }
        _ if (IDM_ROW_GROUP_BASE..IDM_ROW_GROUP_BASE + 2000).contains(&id)
            || (IDM_GROUP_FILTER_BASE..IDM_GROUP_FILTER_BASE + 2000).contains(&id) =>
        {
            execute_row_command(hwnd, state, id);
        }
        _ => {}
    }

    state.context_row = -1;
}

fn main_scrollbar_drag_target(state: &AppState, y: i32) -> Option<i32> {
    let track = state.scrollbar_track_rect()?;
    let thumb = state.scrollbar_thumb_rect()?;
    let track_h = (track.bottom - track.top).max(1);
    let thumb_h = (thumb.bottom - thumb.top).max(1);
    let drag_range = (track_h - thumb_h).max(1);
    let max_scroll = state.layout().max_scroll(state.visible_count()).max(0);
    if max_scroll <= 0 {
        return Some(0);
    }
    let pos = (y - track.top - (thumb_h / 2)).clamp(0, track_h - thumb_h);
    Some(((pos as f32 / drag_range as f32) * max_scroll as f32) as i32)
}

unsafe fn handle_mouse_wheel(hwnd: HWND, wparam: WPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let delta = ((wparam >> 16) & 0xffff) as u16 as i16 as i32;
    let scroll_step = (state.layout().row_h * 2).max(32);
    if delta > 0 {
        state.scroll_y -= scroll_step;
    } else {
        state.scroll_y += scroll_step;
    }
    state.clamp_scroll();
    state.maybe_request_more_for_active_tab();
    show_main_scrollbar_feedback(hwnd, state, true);
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

    if state.scroll_dragging {
        let track = state.scrollbar_track_rect();
        let thumb = state.scrollbar_thumb_rect();
        if let (Some(track), Some(thumb)) = (track, thumb) {
            let track_h = (track.bottom - track.top).max(1);
            let thumb_h = (thumb.bottom - thumb.top).max(1);
            let drag_range = (track_h - thumb_h).max(1);
            let max_scroll = state.layout().max_scroll(state.visible_count()).max(0);
            if max_scroll <= 0 {
                state.scroll_y = 0;
            } else {
                let dy = y - state.scroll_drag_start_y;
                let new_y = state.scroll_drag_start_scroll
                    + ((dy as f32 / drag_range as f32) * max_scroll as f32) as i32;
                state.scroll_y = new_y.clamp(0, max_scroll);
            }
            state.maybe_request_more_for_active_tab();
            show_main_scrollbar_feedback(hwnd, state, false);
            return;
        }
    }

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
    state.hover_scroll = state
        .scrollbar_track_rect()
        .map(|tr| {
            let hot = RECT {
                left: tr.left - 8,
                top: tr.top,
                right: tr.right + 2,
                bottom: tr.bottom,
            };
            pt_in_rect(x, y, &hot)
        })
        .unwrap_or(false);
    // 悬停时立即显示滚动条（满透明）
    if state.hover_scroll && !was_hover_scroll {
        show_main_scrollbar_feedback(hwnd, state, false);
    }

    state.hover_to_top =
        scroll_to_top_visible(state) && pt_in_rect(x, y, &state.scroll_to_top_rect());

    let old_hover = state.hover_idx;
    state.hover_idx = if state.hover_to_top {
        -1
    } else {
        hit_test_row(state, x, y)
    };
    let preview_target_changed = old_btn != state.hover_btn
        || old_hover != state.hover_idx
        || old_tab != state.hover_tab
        || old_scroll != state.hover_scroll
        || old_to_top != state.hover_to_top;
    if old_hover != state.hover_idx {
        hide_hover_preview();
    }

    if preview_target_changed {
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

    if (0..state.layout().title_h).contains(&y) {
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
            if state.role == WindowRole::Quick || state.main_window_noactivate {
                set_main_window_noactivate_mode(hwnd, false);
                let _ = force_foreground_window(hwnd);
            }
            ReleaseCapture();
            SendMessageW(
                hwnd,
                WM_SYSCOMMAND,
                (SC_MOVE as usize | HTCAPTION as usize) as WPARAM,
                0,
            );
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

    state.down_to_top =
        scroll_to_top_visible(state) && pt_in_rect(x, y, &state.scroll_to_top_rect());
    if state.down_to_top {
        InvalidateRect(hwnd, null(), 0);
        return;
    }

    if let Some(thumb) = state.scrollbar_thumb_rect() {
        let hit = RECT {
            left: thumb.left - 8,
            top: thumb.top,
            right: thumb.right + 8,
            bottom: thumb.bottom,
        };
        if pt_in_rect(x, y, &hit) {
            state.scroll_dragging = true;
            state.scroll_drag_start_y = y;
            state.scroll_drag_start_scroll = state.scroll_y;
            show_main_scrollbar_feedback(hwnd, state, false);
            SetCapture(hwnd);
            return;
        }
    }
    if let Some(track) = state.scrollbar_track_rect() {
        let hit = RECT {
            left: track.left - 8,
            top: track.top,
            right: track.right + 8,
            bottom: track.bottom,
        };
        if pt_in_rect(x, y, &hit) {
            if let Some(target_scroll) = main_scrollbar_drag_target(state, y) {
                state.scroll_y = target_scroll;
                state.clamp_scroll();
                state.maybe_request_more_for_active_tab();
                show_main_scrollbar_feedback(hwnd, state, false);
            }
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
        remember_shared_tab_view_state(state);
        InvalidateRect(hwnd, null(), 1);
        return;
    }
    if pt_in_rect(x, y, &tab1) {
        state.tab_index = 1;
        state.current_group_filter = state.tab_group_filters[1];
        state.clear_selection();
        state.scroll_y = 0;
        state.refilter();
        remember_shared_tab_view_state(state);
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
        if !ctrl
            && !shift
            && state.hotkey_passthrough_active
            && !state.hotkey_passthrough_edit.is_null()
        {
            let mut handled = false;
            if let Some(src_idx) = state.visible_src_idx(idx as usize) {
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
                maybe_hide_after_paste(hwnd, state);
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
    if state.scroll_dragging {
        cancel_main_scroll_drag(hwnd, state);
        return;
    }
    if !key.is_empty() {
        if !pt_in_rect(x, y, &state.title_button_rect(key)) {
            InvalidateRect(hwnd, null(), 0);
            return;
        }

        match key {
            "search" => {
                if state.search_on {
                    close_search_ui(hwnd, state);
                } else {
                    open_search_ui(hwnd, state);
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
            show_main_scrollbar_feedback(hwnd, state, false);
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
    if let Some(src_idx) = state.visible_src_idx(idx as usize) {
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
        let target_tab = if pt_in_rect(x, y, &tab1) {
            1usize
        } else {
            0usize
        };
        let mut pt: POINT = zeroed();
        GetCursorPos(&mut pt);
        let cmd = show_group_filter_menu(hwnd, pt.x, pt.y, target_tab, state);
        if cmd == IDM_GROUP_FILTER_ALL {
            state.tab_index = target_tab;
            state.tab_group_filters[target_tab] = 0;
            state.current_group_filter = 0;
            state.scroll_y = 0;
            state.clear_selection();
            state.refilter();
            remember_shared_tab_view_state(state);
        } else if (IDM_GROUP_FILTER_BASE..IDM_GROUP_FILTER_BASE + 2000).contains(&cmd) {
            let idx = cmd - IDM_GROUP_FILTER_BASE;
            if let Some(group_id) = state.groups_for_tab(target_tab).get(idx).map(|g| g.id) {
                state.tab_index = target_tab;
                state.tab_group_filters[target_tab] = group_id;
                state.current_group_filter = group_id;
                state.scroll_y = 0;
                state.clear_selection();
                state.refilter();
                remember_shared_tab_view_state(state);
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
        let already_multi_selected =
            state.selected_rows.len() > 1 && state.selected_rows.contains(&idx);
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
    let current_kind = current_item
        .as_ref()
        .map(|it| it.kind)
        .unwrap_or(ClipKind::Text);
    let current_is_dir = current_item
        .as_ref()
        .map(is_directory_item)
        .unwrap_or(false);
    let current_is_excel = current_item
        .as_ref()
        .and_then(|it| it.file_paths.as_ref())
        .and_then(|paths| paths.first())
        .map(|path| {
            let lower = path.to_ascii_lowercase();
            lower.ends_with(".xls")
                || lower.ends_with(".xlsx")
                || lower.ends_with(".xlsm")
                || lower.ends_with(".csv")
        })
        .unwrap_or(false);
    let current_can_ocr = state.settings.image_ocr_provider != "off"
        && current_item
            .as_ref()
            .and_then(image_input_for_ocr)
            .is_some();
    let current_can_translate = state.settings.text_translate_provider != "off"
        && current_item
            .as_ref()
            .and_then(text_input_for_translate)
            .is_some();
    let cmd = show_row_menu(
        hwnd,
        x,
        y,
        state.tab_index,
        state,
        state.context_selection_count(),
        state.context_selection_has_unpinned(),
        current_kind,
        current_is_dir,
        current_is_excel,
        current_can_ocr,
        current_can_translate,
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
            if state.visible_count() == 0 {
                return;
            }
            let new_idx = if state.sel_idx <= 0 {
                0
            } else {
                state.sel_idx - 1
            };
            if shift {
                // Shift+Up: 扩展选择
                if state.selection_anchor < 0 {
                    state.selection_anchor = state.sel_idx;
                }
                state.sel_idx = new_idx;
                state.selected_rows.clear();
                let a = min(state.selection_anchor, state.sel_idx);
                let b = max(state.selection_anchor, state.sel_idx);
                for i in a..=b {
                    state.selected_rows.insert(i);
                }
            } else {
                state.sel_idx = new_idx;
                state.selected_rows.clear();
                state.selection_anchor = -1;
            }
            state.ensure_visible(state.sel_idx);
            InvalidateRect(hwnd, null(), 0);
        }
        x if x == VK_DOWN as u32 => {
            if state.visible_count() == 0 {
                return;
            }
            let new_idx = if state.sel_idx < 0 {
                0
            } else {
                min(state.visible_count() as i32 - 1, state.sel_idx + 1)
            };
            if shift {
                if state.selection_anchor < 0 {
                    state.selection_anchor = state.sel_idx;
                }
                state.sel_idx = new_idx;
                state.selected_rows.clear();
                let a = min(state.selection_anchor, state.sel_idx);
                let b = max(state.selection_anchor, state.sel_idx);
                for i in a..=b {
                    state.selected_rows.insert(i);
                }
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
            for i in 0..state.visible_count() as i32 {
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
                close_search_ui(hwnd, state);
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
            if state.search_on {
                close_search_ui(hwnd, state);
            } else {
                open_search_ui(hwnd, state);
            }
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

    if pt.y >= 0 && pt.y < state.layout().title_h {
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
        if state.role == WindowRole::Quick || state.main_window_noactivate {
            return HTCLIENT as LRESULT;
        }
        return HTCAPTION as LRESULT;
    }

    HTCLIENT as LRESULT
}

fn normalize_captured_text(raw: &str) -> String {
    let mut normalized = raw
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| !matches!(*ch, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}'))
        .collect::<String>();
    normalized = normalized
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if normalized.contains('\n') {
        return normalized;
    }
    if normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("www.")
    {
        return normalized
            .split_whitespace()
            .next()
            .unwrap_or(&normalized)
            .to_string();
    }
    normalized
}

fn normalize_plain_text_for_paste(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

unsafe fn capture_clipboard(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let sequence = GetClipboardSequenceNumber();
    let pixpin_format = clipboard_has_named_format("PixPinData");
    if sequence != 0
        && state.clipboard_retry_sequence != 0
        && state.clipboard_retry_sequence != sequence
    {
        reset_clipboard_retry(hwnd, state);
    }
    if state.consume_skip_next_clipboard_update_once(sequence) {
        reset_clipboard_retry(hwnd, state);
        return;
    }
    if let Some(until) = state.ignore_clipboard_until {
        if Instant::now() < until {
            if sequence != 0 {
                state.last_clipboard_seq = sequence;
            }
            reset_clipboard_retry(hwnd, state);
            return;
        }
        state.ignore_clipboard_until = None;
    }
    let source_app = clipboard_source_app_name();
    let prefer_long_retry = source_app_prefers_long_clipboard_retry(&source_app);
    if is_self_clipboard_source_app(&source_app) {
        if sequence != 0 {
            state.last_clipboard_seq = sequence;
        }
        reset_clipboard_retry(hwnd, state);
        return;
    }

    // Some apps (PixPin, Office, browsers) publish multiple clipboard formats at
    // once. Prefer explicit files/text before bitmap formats so a preview image
    // cannot swallow the actual user payload.
    if let Some(paths) = clipboard_file_paths() {
        let preview = build_files_preview(&paths);
        let sig = file_paths_signature(&paths);
        if state.consume_recent_programmatic_clipboard_signature(&sig) {
            reset_clipboard_retry(hwnd, state);
            return;
        }
        if state.should_skip_transient_duplicate_capture(&sig, source_app.as_str(), sequence) {
            reset_clipboard_retry(hwnd, state);
            return;
        }
        let candidate = ClipItem {
            id: 0,
            kind: ClipKind::Files,
            preview,
            text: Some(paths.join("\n")),
            source_app: source_app.clone(),
            file_paths: Some(paths),
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned: false,
            group_id: 0,
            created_at: String::new(),
        };
        if state.is_recent_top_duplicate_item(&candidate) {
            reset_clipboard_retry(hwnd, state);
            return;
        }
        state.add_clip_item(candidate, sig);
        reset_clipboard_retry(hwnd, state);
        InvalidateRect(hwnd, null(), 1);
        return;
    }

    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => {
            let retry_limit = if prefer_long_retry || pixpin_format {
                PIXPIN_CLIPBOARD_RETRY_MAX_ATTEMPTS
            } else {
                CLIPBOARD_RETRY_MAX_ATTEMPTS
            };
            let _ = schedule_clipboard_retry_with_limit(hwnd, state, sequence, retry_limit);
            return;
        }
    };
    if let Ok(text) = clipboard.get_text() {
        let normalized = normalize_captured_text(&text);
        if !normalized.is_empty() {
            let preview = build_preview(&normalized);
            let sig = format!("txt:{}", hash_bytes(normalized.as_bytes()));
            if state.consume_recent_programmatic_clipboard_signature(&sig) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            if state.should_skip_transient_duplicate_capture(&sig, source_app.as_str(), sequence) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            let candidate = ClipItem {
                id: 0,
                kind: ClipKind::Text,
                preview,
                text: Some(normalized),
                source_app: source_app.clone(),
                file_paths: None,
                image_bytes: None,
                image_path: None,
                image_width: 0,
                image_height: 0,
                pinned: false,
                group_id: 0,
                created_at: String::new(),
            };
            if state.is_recent_top_duplicate_item(&candidate) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            state.add_clip_item(candidate, sig);
            reset_clipboard_retry(hwnd, state);
            InvalidateRect(hwnd, null(), 1);
            return;
        }
    }

    if let Ok(img) = clipboard.get_image() {
        let bytes = img.bytes.into_owned();
        if let Some((bytes, norm_w, norm_h)) =
            normalize_captured_image_rgba(bytes, img.width, img.height)
        {
            let image_path = write_image_bytes_to_output_path(&bytes, norm_w as u32, norm_h as u32);
            let image_bytes = if image_path.is_none() {
                Some(bytes.clone())
            } else {
                None
            };
            let sig = format!("img:{}:{}:{}", norm_w, norm_h, hash_bytes(bytes.as_slice()));
            if state.consume_recent_programmatic_clipboard_signature(&sig) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            if state.should_skip_transient_duplicate_capture(&sig, source_app.as_str(), sequence) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            // 图片预览：用当前本地时间作为标识（HH:MM:SS），便于用户识别截图时间
            let preview = format_local_time_for_image_preview();
            let candidate = ClipItem {
                id: 0,
                kind: ClipKind::Image,
                preview,
                text: None,
                source_app: source_app.clone(),
                file_paths: None,
                image_bytes,
                image_path: image_path.map(|p| p.to_string_lossy().to_string()),
                image_width: norm_w,
                image_height: norm_h,
                pinned: false,
                group_id: 0,
                created_at: String::new(),
            };
            if state.is_recent_top_duplicate_item(&candidate) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            state.add_clip_item(candidate, sig);
            reset_clipboard_retry(hwnd, state);
            InvalidateRect(hwnd, null(), 1);
            return;
        } else if schedule_clipboard_retry_with_limit(
            hwnd,
            state,
            sequence,
            if prefer_long_retry || pixpin_format {
                PIXPIN_CLIPBOARD_RETRY_MAX_ATTEMPTS
            } else {
                CLIPBOARD_RETRY_MAX_ATTEMPTS
            },
        ) {
            return;
        } else {
            reset_clipboard_retry(hwnd, state);
            // 图片尚未就绪时允许继续回退到文件/文本分支，避免 PixPin 等来源丢记录。
        }
    }

    drop(clipboard);

    if let Some((bytes, width, height)) = read_windows_clipboard_bitmap_rgba() {
        if let Some((bytes, norm_w, norm_h)) = normalize_captured_image_rgba(bytes, width, height) {
            let image_path = write_image_bytes_to_output_path(&bytes, norm_w as u32, norm_h as u32);
            let image_bytes = if image_path.is_none() {
                Some(bytes.clone())
            } else {
                None
            };
            let sig = format!("img:{}:{}:{}", norm_w, norm_h, hash_bytes(bytes.as_slice()));
            if state.consume_recent_programmatic_clipboard_signature(&sig) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            if state.should_skip_transient_duplicate_capture(&sig, source_app.as_str(), sequence) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            let preview = format_local_time_for_image_preview();
            let candidate = ClipItem {
                id: 0,
                kind: ClipKind::Image,
                preview,
                text: None,
                source_app: source_app.clone(),
                file_paths: None,
                image_bytes,
                image_path: image_path.map(|p| p.to_string_lossy().to_string()),
                image_width: norm_w,
                image_height: norm_h,
                pinned: false,
                group_id: 0,
                created_at: String::new(),
            };
            if state.is_recent_top_duplicate_item(&candidate) {
                reset_clipboard_retry(hwnd, state);
                return;
            }
            state.add_clip_item(candidate, sig);
            reset_clipboard_retry(hwnd, state);
            InvalidateRect(hwnd, null(), 1);
            return;
        }
    }

    if sequence != 0 {
        let retry_limit = if prefer_long_retry || pixpin_format {
            PIXPIN_CLIPBOARD_RETRY_MAX_ATTEMPTS
        } else {
            CLIPBOARD_RETRY_MAX_ATTEMPTS
        };
        let _ = schedule_clipboard_retry_with_limit(hwnd, state, sequence, retry_limit);
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
    let ok = clipboard.set_text(&merged).is_ok();
    if ok {
        state.note_programmatic_clipboard_signature(
            format!("txt:{}", hash_bytes(merged.as_bytes())),
            CLIPBOARD_IGNORE_MS_PASTE,
        );
        set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_PASTE);
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
                let prepared = maybe_ai_clean_text(state, text);
                let ok = clipboard.set_text(prepared.clone()).is_ok();
                if ok {
                    state.note_programmatic_clipboard_signature(
                        format!("txt:{}", hash_bytes(prepared.as_bytes())),
                        CLIPBOARD_IGNORE_MS_PASTE,
                    );
                }
                ok
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
                let ok = clipboard
                    .set_image(ImageData {
                        width,
                        height,
                        bytes: Cow::Owned(bytes.clone()),
                    })
                    .is_ok();
                if ok {
                    state.note_programmatic_clipboard_signature(
                        format!("img:{}:{}:{}", width, height, hash_bytes(bytes.as_slice())),
                        CLIPBOARD_IGNORE_MS_PASTE,
                    );
                }
                ok
            } else {
                false
            }
        }
        ClipKind::Files => {
            if let Some(paths) = &item.file_paths {
                let ok = set_file_paths_to_clipboard(paths);
                if ok {
                    state.note_programmatic_clipboard_signature(
                        file_paths_signature(paths),
                        CLIPBOARD_IGNORE_MS_PASTE,
                    );
                }
                ok
            } else if let Some(text) = &item.text {
                let mut clipboard = match Clipboard::new() {
                    Ok(c) => c,
                    Err(_) => return false,
                };
                let prepared = maybe_ai_clean_text(state, text);
                let ok = clipboard.set_text(prepared.clone()).is_ok();
                if ok {
                    state.note_programmatic_clipboard_signature(
                        format!("txt:{}", hash_bytes(prepared.as_bytes())),
                        CLIPBOARD_IGNORE_MS_PASTE,
                    );
                }
                ok
            } else {
                false
            }
        }
    };
    if ok {
        set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_PASTE);
    }
    ok
}

unsafe fn apply_item_to_clipboard_plain_text(state: &mut AppState, item_ref: &ClipItem) -> bool {
    let full_item;
    let item: &ClipItem = if let Some(resolved) = state.resolve_item_for_use(item_ref) {
        full_item = resolved;
        &full_item
    } else {
        return false;
    };
    let text = match item.kind {
        ClipKind::Text | ClipKind::Phrase => item
            .text
            .as_ref()
            .map(|t| normalize_plain_text_for_paste(t)),
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .map(|paths| paths.join("\n"))
            .or_else(|| {
                item.text
                    .as_ref()
                    .map(|t| normalize_plain_text_for_paste(t))
            }),
        ClipKind::Image => None,
    };
    let Some(text) = text else {
        return false;
    };
    let mut clipboard = match Clipboard::new() {
        Ok(c) => c,
        Err(_) => return false,
    };
    let ok = clipboard.set_text(&text).is_ok();
    if ok {
        state.note_programmatic_clipboard_signature(
            format!("txt:{}", hash_bytes(text.as_bytes())),
            CLIPBOARD_IGNORE_MS_PASTE,
        );
        set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_PASTE);
    }
    ok
}

fn spawn_async_image_paste_load(
    hwnd: HWND,
    item_id: i64,
    target: HWND,
    hide_main: bool,
    backspaces: u8,
) {
    let hwnd_raw = hwnd as isize;
    let target_raw = target as isize;
    std::thread::spawn(move || {
        let image = db_load_item_full(item_id).and_then(|full| {
            if let Some(bytes) = full.image_bytes {
                Some((bytes, full.image_width, full.image_height))
            } else {
                full.image_path
                    .as_deref()
                    .and_then(load_image_bytes_from_path)
            }
        });
        let payload = Box::new(ImagePasteReadyResult {
            image,
            target: target_raw,
            hide_main,
            backspaces,
        });
        unsafe {
            let _ = post_boxed_message(hwnd_raw as HWND, WM_IMAGE_PASTE_READY, 0, payload);
        }
    });
}

unsafe fn queue_async_image_paste_if_needed(
    hwnd: HWND,
    _state: &mut AppState,
    item_ref: &ClipItem,
    target: HWND,
    hide_main: bool,
    backspaces: u8,
) -> bool {
    if item_ref.kind != ClipKind::Image
        || item_ref.id <= 0
        || item_ref.image_bytes.is_some()
        || item_ref.image_path.is_some()
    {
        return false;
    }
    if target.is_null() || IsWindow(target) == 0 {
        return false;
    }
    spawn_async_image_paste_load(hwnd, item_ref.id, target, hide_main, backspaces);
    true
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
        SendMessageW(
            state.hotkey_passthrough_edit,
            EM_SETSEL,
            caret as usize,
            caret,
        );
        set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_DIRECT_EDIT);
        clear_hotkey_passthrough_state(state);
    }
    ok
}

unsafe fn maybe_promote_pasted_item(hwnd: HWND, state: &mut AppState, item_id: i64) {
    if !state.settings.move_pasted_item_to_top || item_id <= 0 {
        return;
    }
    if let Ok(new_id) = db_promote_item_to_top(item_id) {
        let anchor = state.current_scroll_anchor();
        state.remove_cached_item(item_id);
        state.remove_cached_item(new_id);
        if !state.promote_loaded_item_to_top(item_id, new_id) {
            state.reload_state_from_db_preserve_scroll(anchor);
        } else {
            state.refilter();
            state.restore_scroll_anchor(anchor);
        }
        sync_peer_windows_from_db(hwnd);
    }
}

unsafe fn stop_search_debounce_timer(hwnd: HWND, state: &mut AppState) {
    stop_flagged_timer(
        hwnd,
        ID_TIMER_SEARCH_DEBOUNCE,
        &mut state.search_debounce_timer,
    );
}

unsafe fn apply_search_filter(hwnd: HWND, state: &mut AppState) {
    state.sel_idx = 0;
    state.scroll_y = 0;
    state.refilter();
    InvalidateRect(hwnd, null(), 1);
}

unsafe fn cancel_main_scroll_drag(hwnd: HWND, state: &mut AppState) {
    if state.scroll_dragging {
        state.scroll_dragging = false;
        ReleaseCapture();
        InvalidateRect(hwnd, null(), 0);
    }
}

fn hotkey_key_label_from_vk(vk: u32) -> Option<&'static str> {
    match vk {
        0x41..=0x5A => HOTKEY_KEY_OPTIONS.get((vk - 0x41) as usize).copied(),
        0x30..=0x39 => HOTKEY_KEY_OPTIONS.get((vk - 0x30 + 26) as usize).copied(),
        x if x == VK_SPACE as u32 => Some("Space"),
        x if x == VK_RETURN as u32 => Some("Enter"),
        x if x == VK_TAB as u32 => Some("Tab"),
        x if x == VK_ESCAPE as u32 => Some("Esc"),
        x if x == VK_BACK as u32 => Some("Backspace"),
        x if x == VK_DELETE as u32 => Some("Delete"),
        x if x == VK_INSERT as u32 => Some("Insert"),
        x if x == VK_UP as u32 => Some("Up"),
        x if x == VK_DOWN as u32 => Some("Down"),
        x if x == VK_LEFT as u32 => Some("Left"),
        x if x == VK_RIGHT as u32 => Some("Right"),
        x if x == VK_HOME as u32 => Some("Home"),
        x if x == VK_END as u32 => Some("End"),
        x if x == VK_PRIOR as u32 => Some("PageUp"),
        x if x == VK_NEXT as u32 => Some("PageDown"),
        _ => None,
    }
}

unsafe fn hotkey_mod_label_from_pressed_state() -> Option<String> {
    let ctrl = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;
    let alt = (GetAsyncKeyState(VK_MENU as i32) as u16 & 0x8000) != 0;
    let shift = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
    let win = (GetAsyncKeyState(VK_LWIN as i32) as u16 & 0x8000) != 0
        || (GetAsyncKeyState(VK_RWIN as i32) as u16 & 0x8000) != 0;
    if win && !ctrl && !alt && !shift {
        return Some("Win".to_string());
    }
    let mut parts = Vec::new();
    if ctrl {
        parts.push("Ctrl");
    }
    if alt {
        parts.push("Alt");
    }
    if shift {
        parts.push("Shift");
    }
    if parts.is_empty() {
        None
    } else {
        Some(parts.join("+"))
    }
}

unsafe fn settings_set_hotkey_recording(st: &mut SettingsWndState, recording: bool) {
    st.hotkey_recording = recording;
    if !st.btn_hk_record.is_null() {
        settings_set_text(
            st.btn_hk_record,
            if recording {
                tr("按下快捷键...", "Press shortcut...")
            } else {
                tr("录制热键", "Record Hotkey")
            },
        );
        InvalidateRect(st.btn_hk_record, null(), 1);
    }
    if !st.lb_hk_preview.is_null() {
        if recording {
            settings_set_text(
                st.lb_hk_preview,
                tr("请按修饰键 + 按键", "Press modifier + key"),
            );
        } else {
            settings_set_text(
                st.lb_hk_preview,
                &hotkey_preview_text(
                    &get_window_text(st.cb_hk_mod),
                    &get_window_text(st.cb_hk_key),
                ),
            );
        }
        InvalidateRect(st.lb_hk_preview, null(), 1);
    }
}

unsafe fn maybe_hide_after_paste(hwnd: HWND, state: &AppState) {
    if state.settings.click_hide {
        ShowWindow(hwnd, SW_HIDE);
    }
}

unsafe fn paste_selected(hwnd: HWND, state: &mut AppState) {
    let Some(item_ref) = state.current_item().cloned() else {
        return;
    };
    if try_apply_to_explorer_rename(state, &item_ref) {
        maybe_promote_pasted_item(hwnd, state, item_ref.id);
        if state.settings.paste_success_sound_enabled {
            play_paste_success_sound(
                &state.settings.paste_success_sound_kind,
                &state.settings.paste_success_sound_path,
            );
        }
        maybe_hide_after_paste(hwnd, state);
        state.clear_selection();
        clear_main_hover_state(hwnd);
        return;
    }
    let async_target = effective_paste_target(state, hwnd);
    if queue_async_image_paste_if_needed(
        hwnd,
        state,
        &item_ref,
        async_target,
        state.settings.click_hide,
        0,
    ) {
        maybe_promote_pasted_item(hwnd, state, item_ref.id);
        state.clear_selection();
        clear_main_hover_state(hwnd);
        if state.settings.click_hide {
            ShowWindow(hwnd, SW_HIDE);
        }
        return;
    }
    let applied = if state.plain_text_paste_mode {
        apply_item_to_clipboard_plain_text(state, &item_ref)
    } else {
        apply_item_to_clipboard(state, &item_ref)
    };
    if !applied {
        show_clipboard_write_failure_message(hwnd);
        return;
    }
    maybe_promote_pasted_item(hwnd, state, item_ref.id);
    state.plain_text_paste_mode = false;
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
    let item = state
        .vv_popup_items
        .get(index)
        .map(|entry| entry.item.clone());
    vv_popup_hide(hwnd, state);
    let Some(item) = item else {
        return;
    };
    if queue_async_image_paste_if_needed(
        hwnd,
        state,
        &item,
        target,
        state.settings.click_hide,
        backspaces,
    ) {
        maybe_promote_pasted_item(hwnd, state, item.id);
        return;
    }
    if !apply_item_to_clipboard(state, &item) {
        show_clipboard_write_failure_message(hwnd);
        return;
    }
    maybe_promote_pasted_item(hwnd, state, item.id);
    paste_after_clipboard_ready_to_target(
        hwnd,
        state,
        target,
        state.settings.click_hide,
        backspaces,
    );
}

fn strip_invisible_text_chars(input: &str) -> String {
    input
        .chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{feff}' | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}'
            )
        })
        .collect()
}

fn strip_markdown_links_and_images(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let is_image = bytes[i] == b'!' && i + 1 < bytes.len() && bytes[i + 1] == b'[';
        let is_link = bytes[i] == b'[';
        if is_image || is_link {
            let label_start = if is_image { i + 2 } else { i + 1 };
            if let Some(label_end_rel) = input[label_start..].find(']') {
                let label_end = label_start + label_end_rel;
                let after_label = label_end + 1;
                if after_label < bytes.len() && bytes[after_label] == b'(' {
                    if let Some(url_end_rel) = input[after_label + 1..].find(')') {
                        out.push_str(input[label_start..label_end].trim());
                        i = after_label + 2 + url_end_rel;
                        continue;
                    }
                }
            }
        }
        let Some(ch) = input[i..].chars().next() else {
            break;
        };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_markdown_prefix(line: &str) -> String {
    let trimmed = line.trim_start();
    let bytes = trimmed.as_bytes();
    let mut drop_bytes = 0usize;

    while drop_bytes < bytes.len() && bytes[drop_bytes] == b'>' {
        drop_bytes += 1;
        while drop_bytes < bytes.len() && bytes[drop_bytes] == b' ' {
            drop_bytes += 1;
        }
    }

    let trimmed = &trimmed[drop_bytes..];
    let bytes = trimmed.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() && bytes[idx] == b'#' {
        idx += 1;
    }
    if idx > 0 && idx < bytes.len() && bytes[idx] == b' ' {
        return trimmed[idx + 1..].to_string();
    }

    for prefix in [
        "- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "- ", "* ", "+ ",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.to_string();
        }
    }

    let mut digits = 0usize;
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() {
            digits += ch.len_utf8();
        } else {
            break;
        }
    }
    if digits > 0 {
        let rest = &trimmed[digits..];
        if let Some(stripped) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return stripped.to_string();
        }
    }

    trimmed.to_string()
}

fn looks_like_markdown_document(input: &str) -> bool {
    input.contains("```")
        || input.contains("](")
        || input.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('#')
                || trimmed.starts_with('>')
                || trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ ")
                || trimmed.starts_with("![")
                || trimmed.starts_with("- [")
                || trimmed.starts_with("* [")
                || trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count() > 0
                    && (trimmed.contains(". ") || trimmed.contains(") "))
        })
}

fn collapse_blank_lines(lines: impl IntoIterator<Item = String>, max_blank_lines: usize) -> String {
    let mut output = Vec::new();
    let mut blank = 0usize;
    for line in lines {
        if line.trim().is_empty() {
            blank += 1;
            if blank <= max_blank_lines {
                output.push(String::new());
            }
        } else {
            blank = 0;
            output.push(line);
        }
    }
    output.join("\n").trim().to_string()
}

fn clean_markdown_document(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut in_code_fence = false;
    let mut cleaned = Vec::new();
    for raw_line in normalized.lines() {
        let trimmed = raw_line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            continue;
        }
        let mut line = if in_code_fence {
            raw_line.trim_end().to_string()
        } else {
            strip_markdown_prefix(raw_line)
        };
        line = strip_markdown_links_and_images(&line);
        for marker in ["**", "__", "~~", "`"] {
            line = line.replace(marker, "");
        }
        cleaned.push(line.trim_end().to_string());
    }
    collapse_blank_lines(cleaned, 1)
}

fn ai_clean_text(input: &str) -> String {
    let normalized = strip_invisible_text_chars(input)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let cleaned = if looks_like_markdown_document(&normalized) {
        clean_markdown_document(&normalized)
    } else {
        let lines = normalized
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect::<Vec<_>>();
        collapse_blank_lines(lines, 1)
    };
    cleaned.trim().to_string()
}

unsafe fn maybe_ai_clean_text(state: &AppState, input: &str) -> String {
    if !state.settings.ai_clean_enabled {
        return input.to_string();
    }
    let shift_down = (GetAsyncKeyState(VK_SHIFT as i32) as u16 & 0x8000) != 0;
    if shift_down {
        return input.to_string();
    }
    if input.matches('\n').count() >= 4
        || input.contains("```")
        || input.chars().count() >= 280
        || looks_like_markdown_document(input)
    {
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
    let init_hr = OleInitialize(null());
    if init_hr < 0 && init_hr != RPC_E_CHANGED_MODE_HR {
        return false;
    }
    let Some(data_obj) = create_shell_data_object(paths) else {
        if init_hr >= 0 {
            OleUninitialize();
        }
        return false;
    };
    let drop_source = create_drop_source();
    if drop_source.is_null() {
        release_raw_com(data_obj);
        if init_hr >= 0 {
            OleUninitialize();
        }
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
    let mut s = name
        .chars()
        .take(40)
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim()
        .to_string();
    if s.is_empty() {
        s = fallback.to_string();
    }
    s
}

fn materialize_item_as_file(item: &ClipItem) -> Option<PathBuf> {
    let base = export_dir();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let suffix = if item.id > 0 {
        item.id.to_string()
    } else {
        ts.to_string()
    };
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let name = sanitize_export_name(&item.preview, "text");
            let path = base.join(format!("{}_{}_{}.txt", name, ts, suffix));
            let text = item.text.as_deref().unwrap_or(&item.preview);
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
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .and_then(|v| v.first())
            .map(PathBuf::from),
    }
}

fn drag_export_paths_for_item(item: &ClipItem) -> Vec<PathBuf> {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase | ClipKind::Image => {
            materialize_item_as_file(item).into_iter().collect()
        }
        ClipKind::Files => Vec::new(),
    }
}

pub(crate) fn is_supported_ocr_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff" | "webp"
            )
        })
        .unwrap_or(false)
}

pub(crate) struct OcrImageInput {
    pub(crate) path: PathBuf,
    pub(crate) delete_after: bool,
}

pub(crate) fn write_image_bytes_to_ocr_temp_path(
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Option<PathBuf> {
    let base = std::env::temp_dir().join("zsclip").join("ocr");
    let _ = fs::create_dir_all(&base);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let out = base.join(format!("ocr_{}.png", ts));
    write_image_bytes_to_path(&out, bytes, width, height)
}

pub(crate) fn image_input_for_ocr(item: &ClipItem) -> Option<OcrImageInput> {
    match item.kind {
        ClipKind::Image => {
            if let Some(path) = item.image_path.as_ref() {
                let src = PathBuf::from(path);
                if src.is_file() && is_supported_ocr_image_path(&src) {
                    return Some(OcrImageInput {
                        path: src,
                        delete_after: false,
                    });
                }
            }
            let (bytes, width, height) = ensure_item_image_bytes(item)?;
            let path = write_image_bytes_to_ocr_temp_path(&bytes, width as u32, height as u32)?;
            Some(OcrImageInput {
                path,
                delete_after: true,
            })
        }
        ClipKind::Files => item.file_paths.as_ref().and_then(|paths| {
            if paths.len() == 1 {
                let path = PathBuf::from(paths[0].clone());
                if path.is_file() && is_supported_ocr_image_path(&path) {
                    Some(OcrImageInput {
                        path,
                        delete_after: false,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }),
        _ => None,
    }
}

fn text_input_for_translate(item: &ClipItem) -> Option<String> {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => item
            .text
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .or_else(|| {
                let s = item.preview.trim();
                if s.is_empty() {
                    None
                } else {
                    Some(s.to_string())
                }
            }),
        _ => None,
    }
}

unsafe fn spawn_image_ocr_job(hwnd: HWND, settings: AppSettings, item: ClipItem) {
    let hwnd_value = hwnd as isize;
    std::thread::spawn(move || {
        let result = match settings.image_ocr_provider.as_str() {
            "baidu" => image_input_for_ocr(&item)
                .ok_or_else(|| {
                    tr(
                        "当前记录没有可识别的图片文件",
                        "This item does not contain a recognizable image file",
                    )
                    .to_string()
                })
                .and_then(|input| {
                    let result =
                        fs::read(&input.path)
                            .map_err(|e| e.to_string())
                            .and_then(|bytes| {
                                run_baidu_ocr_api(
                                    &settings.image_ocr_cloud_url,
                                    &settings.image_ocr_cloud_token,
                                    &bytes,
                                )
                            });
                    if input.delete_after {
                        let _ = fs::remove_file(&input.path);
                    }
                    result
                }),
            "winocr" => image_input_for_ocr(&item)
                .ok_or_else(|| {
                    tr(
                        "当前记录没有可识别的图片文件",
                        "This item does not contain a recognizable image file",
                    )
                    .to_string()
                })
                .and_then(|input| {
                    let result = run_winocr_dll_ocr(&input.path, &settings.image_ocr_wechat_dir);
                    if input.delete_after {
                        let _ = fs::remove_file(&input.path);
                    }
                    result
                }),
            _ => Err(tr(
                "请先在设置-插件中启用图片 OCR",
                "Please enable Image OCR in Settings > Plugins first",
            )
            .to_string()),
        };

        let payload = match result {
            Ok(text) => ImageOcrReadyResult {
                text: Some(text),
                error: None,
            },
            Err(err) => ImageOcrReadyResult {
                text: None,
                error: Some(err),
            },
        };
        let _ = unsafe {
            post_boxed_message(hwnd_value as HWND, WM_IMAGE_OCR_READY, 0, Box::new(payload))
        };
    });
}

unsafe fn spawn_text_translate_job(hwnd: HWND, settings: AppSettings, item: ClipItem) {
    let hwnd_value = hwnd as isize;
    std::thread::spawn(move || {
        let result = match settings.text_translate_provider.as_str() {
            "baidu" => text_input_for_translate(&item)
                .ok_or_else(|| {
                    tr(
                        "当前记录没有可翻译的文本",
                        "This item does not contain translatable text",
                    )
                    .to_string()
                })
                .and_then(|text| {
                    run_baidu_translate_api(
                        &settings.text_translate_app_id,
                        &settings.text_translate_secret,
                        &text,
                        &settings.text_translate_target_lang,
                    )
                }),
            _ => Err(tr(
                "请先在设置-插件中启用文本翻译",
                "Please enable Text Translate in Settings > Plugins first",
            )
            .to_string()),
        };

        let payload = match result {
            Ok(text) => TextTranslateReadyResult {
                text: Some(text),
                error: None,
            },
            Err(err) => TextTranslateReadyResult {
                text: None,
                error: Some(err),
            },
        };
        let _ = unsafe {
            post_boxed_message(
                hwnd_value as HWND,
                WM_TEXT_TRANSLATE_READY,
                0,
                Box::new(payload),
            )
        };
    });
}

unsafe fn begin_row_drag_export(hwnd: HWND, state: &mut AppState, visible_idx: i32) -> bool {
    if visible_idx < 0 {
        return false;
    }
    let Some(src_idx) = state.visible_src_idx(visible_idx as usize) else {
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
    ReleaseCapture();
    begin_file_drag(hwnd, &paths)
}

unsafe fn show_row_menu(
    hwnd: HWND,
    x: i32,
    y: i32,
    tab_index: usize,
    state: &AppState,
    selected_count: usize,
    has_unpinned: bool,
    current_kind: ClipKind,
    current_is_dir: bool,
    current_is_excel: bool,
    current_can_ocr: bool,
    current_can_translate: bool,
) -> usize {
    let menu = CreatePopupMenu();
    if menu.is_null() {
        return 0;
    }
    apply_theme_to_menu(menu as _);
    let groups = state.groups_for_tab(tab_index);
    let group_menu = if state.settings.grouping_enabled {
        CreatePopupMenu()
    } else {
        null_mut()
    };
    if !group_menu.is_null() {
        apply_theme_to_menu(group_menu as _);
        if groups.is_empty() {
            AppendMenuW(
                group_menu,
                MF_GRAYED | MF_STRING,
                0xFFFFusize,
                to_wide(translate("（暂无分组）").as_ref()).as_ptr(),
            );
        } else {
            for (idx, g) in groups.iter().enumerate() {
                AppendMenuW(
                    group_menu,
                    MF_STRING,
                    IDM_ROW_GROUP_BASE + idx,
                    to_wide(&g.name).as_ptr(),
                );
            }
        }
    }
    if selected_count > 1 {
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_ROW_COPY,
            to_wide(translate("合并复制").as_ref()).as_ptr(),
        );
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        let pin_text = if has_unpinned {
            "置顶所选"
        } else {
            "取消置顶"
        };
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_ROW_PIN,
            to_wide(translate(pin_text).as_ref()).as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_ROW_TO_PHRASE,
            to_wide(translate("添加到短语").as_ref()).as_ptr(),
        );
        if !group_menu.is_null() {
            AppendMenuW(
                menu,
                MF_POPUP,
                group_menu as usize,
                to_wide(translate("添加到分组").as_ref()).as_ptr(),
            );
        }
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_ROW_GROUP_REMOVE,
            to_wide(translate("移出分组").as_ref()).as_ptr(),
        );
        AppendMenuW(
            menu,
            MF_STRING,
            IDM_ROW_DELETE,
            to_wide(translate("删除所选").as_ref()).as_ptr(),
        );
        AppendMenuW(
            menu,
            if has_unpinned {
                MF_STRING
            } else {
                MF_GRAYED | MF_STRING
            },
            IDM_ROW_DELETE_UNPINNED,
            to_wide(translate("删除除置顶以外").as_ref()).as_ptr(),
        );
    } else {
        match current_kind {
            ClipKind::Image => {
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_STICKER,
                    to_wide(translate("贴图").as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_SAVE_IMAGE,
                    to_wide(translate("另存为 PNG").as_ref()).as_ptr(),
                );
                if current_can_ocr {
                    AppendMenuW(
                        menu,
                        MF_STRING,
                        IDM_ROW_IMAGE_OCR,
                        to_wide(tr("图片转文字", "Image OCR")).as_ptr(),
                    );
                }
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_EXPORT_FILE,
                    to_wide(translate("导出为文件").as_ref()).as_ptr(),
                );
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                let pin_text = if has_unpinned {
                    "置顶"
                } else {
                    "取消置顶"
                };
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_PIN,
                    to_wide(translate(pin_text).as_ref()).as_ptr(),
                );
                if !group_menu.is_null() {
                    AppendMenuW(
                        menu,
                        MF_POPUP,
                        group_menu as usize,
                        to_wide(translate("添加到分组").as_ref()).as_ptr(),
                    );
                }
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_GROUP_REMOVE,
                    to_wide(translate("移出分组").as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_DELETE,
                    to_wide(translate("删除").as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    if has_unpinned {
                        MF_STRING
                    } else {
                        MF_GRAYED | MF_STRING
                    },
                    IDM_ROW_DELETE_UNPINNED,
                    to_wide(translate("删除除置顶以外").as_ref()).as_ptr(),
                );
            }
            ClipKind::Files => {
                let open_text = if current_is_dir {
                    "打开文件夹"
                } else {
                    "打开文件"
                };
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_OPEN_PATH,
                    to_wide(translate(open_text).as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_OPEN_FOLDER,
                    to_wide(translate("打开所在文件夹").as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_COPY_PATH,
                    to_wide(translate("复制路径").as_ref()).as_ptr(),
                );
                if current_can_ocr {
                    AppendMenuW(
                        menu,
                        MF_STRING,
                        IDM_ROW_IMAGE_OCR,
                        to_wide(tr("图片转文字", "Image OCR")).as_ptr(),
                    );
                }
                if state.settings.qr_quick_enabled {
                    AppendMenuW(
                        menu,
                        MF_STRING,
                        IDM_ROW_QR_IMAGE,
                        to_wide(tr("快捷转换二维码", "Quick QR Convert")).as_ptr(),
                    );
                }
                if current_is_excel && state.settings.super_mail_merge_enabled {
                    AppendMenuW(
                        menu,
                        MF_STRING,
                        IDM_ROW_MAIL_MERGE,
                        to_wide(translate("超级邮件合并").as_ref()).as_ptr(),
                    );
                }
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                let pin_text = if has_unpinned {
                    "置顶"
                } else {
                    "取消置顶"
                };
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_PIN,
                    to_wide(translate(pin_text).as_ref()).as_ptr(),
                );
                if !group_menu.is_null() {
                    AppendMenuW(
                        menu,
                        MF_POPUP,
                        group_menu as usize,
                        to_wide(translate("添加到分组").as_ref()).as_ptr(),
                    );
                }
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_GROUP_REMOVE,
                    to_wide(translate("移出分组").as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_DELETE,
                    to_wide(translate("删除").as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    if has_unpinned {
                        MF_STRING
                    } else {
                        MF_GRAYED | MF_STRING
                    },
                    IDM_ROW_DELETE_UNPINNED,
                    to_wide(translate("删除除置顶以外").as_ref()).as_ptr(),
                );
            }
            _ => {
                let pin_text = if has_unpinned {
                    "置顶"
                } else {
                    "取消置顶"
                };
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_EDIT,
                    to_wide(translate("编辑").as_ref()).as_ptr(),
                );
                if state.settings.quick_search_enabled {
                    AppendMenuW(
                        menu,
                        MF_STRING,
                        IDM_ROW_QUICK_SEARCH,
                        to_wide(translate("快速搜索").as_ref()).as_ptr(),
                    );
                }
                if current_can_translate {
                    AppendMenuW(
                        menu,
                        MF_STRING,
                        IDM_ROW_TEXT_TRANSLATE,
                        to_wide(tr("文本翻译", "Text Translate")).as_ptr(),
                    );
                }
                if state.settings.qr_quick_enabled {
                    AppendMenuW(
                        menu,
                        MF_STRING,
                        IDM_ROW_QR_IMAGE,
                        to_wide(tr("快捷转换二维码", "Quick QR Convert")).as_ptr(),
                    );
                }
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_EXPORT_FILE,
                    to_wide(translate("导出为文件").as_ref()).as_ptr(),
                );
                AppendMenuW(menu, MF_SEPARATOR, 0, null());
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_PIN,
                    to_wide(translate(pin_text).as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_TO_PHRASE,
                    to_wide(translate("添加到短语").as_ref()).as_ptr(),
                );
                if !group_menu.is_null() {
                    AppendMenuW(
                        menu,
                        MF_POPUP,
                        group_menu as usize,
                        to_wide(translate("添加到分组").as_ref()).as_ptr(),
                    );
                }
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_GROUP_REMOVE,
                    to_wide(translate("移出分组").as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    MF_STRING,
                    IDM_ROW_DELETE,
                    to_wide(translate("删除").as_ref()).as_ptr(),
                );
                AppendMenuW(
                    menu,
                    if has_unpinned {
                        MF_STRING
                    } else {
                        MF_GRAYED | MF_STRING
                    },
                    IDM_ROW_DELETE_UNPINNED,
                    to_wide(translate("删除除置顶以外").as_ref()).as_ptr(),
                );
            }
        }
    }

    let mut rc: RECT = zeroed();
    GetWindowRect(hwnd, &mut rc);
    let pt = POINT {
        x: rc.left + x,
        y: rc.top + y,
    };
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
    vv_set_popup_menu_active(false);
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
    cmd
}

unsafe fn show_group_filter_menu(
    hwnd: HWND,
    x: i32,
    y: i32,
    tab_index: usize,
    state: &AppState,
) -> usize {
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
    let all_flags = if cur_gid == 0 {
        MF_STRING | MF_CHECKED
    } else {
        MF_STRING
    };
    AppendMenuW(
        menu,
        all_flags,
        IDM_GROUP_FILTER_ALL,
        to_wide(translate("全部").as_ref()).as_ptr(),
    );
    if !groups.is_empty() {
        AppendMenuW(menu, MF_SEPARATOR, 0, null());
        for (idx, g) in groups.iter().enumerate() {
            let flags = if cur_gid == g.id {
                MF_STRING | MF_CHECKED
            } else {
                MF_STRING
            };
            AppendMenuW(
                menu,
                flags,
                IDM_GROUP_FILTER_BASE + idx,
                to_wide(&g.name).as_ptr(),
            );
        }
    }
    SetForegroundWindow(hwnd);
    vv_set_popup_menu_active(true);
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
    vv_set_popup_menu_active(false);
    cmd
}

fn clear_hotkey_passthrough_state(state: &mut AppState) {
    state.hotkey_passthrough_active = false;
    state.hotkey_passthrough_target = null_mut();
    state.hotkey_passthrough_focus = null_mut();
    state.hotkey_passthrough_edit = null_mut();
}

unsafe fn restore_hotkey_focus_target(state: &AppState, target: HWND) {
    let focus = state.hotkey_passthrough_focus;
    if target.is_null() || focus.is_null() || IsWindow(focus) == 0 {
        return;
    }
    if GetAncestor(focus, GA_ROOT) != target {
        return;
    }

    let current_thread = GetCurrentThreadId();
    let target_thread = GetWindowThreadProcessId(target, null_mut());
    let focus_thread = GetWindowThreadProcessId(focus, null_mut());

    // 如果目标窗口当前已经有有效的子控件焦点，就尊重当前输入点，
    // 不再把焦点强拉回热键弹出瞬间保存的旧控件。
    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = size_of::<GUITHREADINFO>() as u32;
    if target_thread != 0 && GetGUIThreadInfo(target_thread, &mut info) != 0 {
        let current_focus = info.hwndFocus;
        if !current_focus.is_null()
            && IsWindow(current_focus) != 0
            && GetAncestor(current_focus, GA_ROOT) == target
            && current_focus != target
        {
            return;
        }
    }

    let attach_target = target_thread != 0
        && target_thread != current_thread
        && AttachThreadInput(current_thread, target_thread, 1) != 0;
    let attach_focus = focus_thread != 0
        && focus_thread != current_thread
        && focus_thread != target_thread
        && AttachThreadInput(current_thread, focus_thread, 1) != 0;

    let _ = SetFocus(focus);

    if attach_focus {
        let _ = AttachThreadInput(current_thread, focus_thread, 0);
    }
    if attach_target {
        let _ = AttachThreadInput(current_thread, target_thread, 0);
    }
}

unsafe fn can_send_ctrl_v_to_target(state: &AppState, target: HWND) -> bool {
    if target.is_null() || IsWindow(target) == 0 {
        return false;
    }
    if GetForegroundWindow() != target {
        return false;
    }
    let target_thread = GetWindowThreadProcessId(target, null_mut());
    if target_thread == 0 {
        return true;
    }
    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = size_of::<GUITHREADINFO>() as u32;
    if GetGUIThreadInfo(target_thread, &mut info) == 0 {
        return true;
    }
    let focus = info.hwndFocus;
    if focus.is_null() {
        return true;
    }
    GetAncestor(focus, GA_ROOT) == target
        || focus == target
        || focus == state.hotkey_passthrough_focus
}

unsafe fn paste_failure_message_for_target(state: &AppState, target: HWND) -> String {
    let detail = if target.is_null() || IsWindow(target) == 0 {
        tr(
            "目标窗口已经关闭。",
            "The target window is no longer available.",
        )
    } else if GetForegroundWindow() != target {
        tr(
            "未能把目标窗口切到前台。",
            "The target window could not be brought to the foreground.",
        )
    } else {
        let target_thread = GetWindowThreadProcessId(target, null_mut());
        let mut info: GUITHREADINFO = zeroed();
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        if target_thread == 0 || GetGUIThreadInfo(target_thread, &mut info) == 0 {
            tr(
                "目标窗口当前没有可用输入焦点。",
                "The target window does not currently expose a usable input focus.",
            )
        } else if info.hwndFocus.is_null() {
            tr(
                "目标窗口没有活动输入框。",
                "The target window does not have an active input control.",
            )
        } else if GetAncestor(info.hwndFocus, GA_ROOT) != target
            && info.hwndFocus != target
            && info.hwndFocus != state.hotkey_passthrough_focus
        {
            tr(
                "当前焦点不在目标输入区域里。",
                "The current focus is no longer inside the target input area.",
            )
        } else {
            tr(
                "目标窗口拒绝了粘贴。",
                "The target window rejected the paste action.",
            )
        }
    };
    format!(
        "{}\r\n\r\n{}",
        detail,
        tr(
            "内容已经写入剪贴板，你可以回到目标窗口后手动粘贴。",
            "The content is already in the clipboard. You can switch back and paste it manually.",
        )
    )
}

unsafe fn show_paste_failure_message(hwnd: HWND, state: &AppState, target: HWND) {
    let text = paste_failure_message_for_target(state, target);
    MessageBoxW(
        hwnd,
        to_wide(&text).as_ptr(),
        to_wide(translate("粘贴失败").as_ref()).as_ptr(),
        MB_OK | MB_ICONWARNING,
    );
}

unsafe fn show_clipboard_write_failure_message(hwnd: HWND) {
    MessageBoxW(
        hwnd,
        to_wide(translate("内容未能写入系统剪贴板，请重试一次。").as_ref()).as_ptr(),
        to_wide(translate("复制失败").as_ref()).as_ptr(),
        MB_OK | MB_ICONWARNING,
    );
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

unsafe fn paste_after_clipboard_ready_to_target(
    hwnd: HWND,
    state: &mut AppState,
    target: HWND,
    hide_main: bool,
    backspaces: u8,
) {
    state.paste_target_override = target;
    state.paste_backspace_count = backspaces;
    if !target.is_null() {
        if hide_main {
            ShowWindow(hwnd, SW_HIDE);
        }
        let _ = force_foreground_window(target);
        restore_hotkey_focus_target(state, target);
        KillTimer(hwnd, ID_TIMER_PASTE);
        SetTimer(hwnd, ID_TIMER_PASTE, 150, None);
    } else {
        clear_hotkey_passthrough_state(state);
        SetForegroundWindow(hwnd);
        if state.search_on {
            SetFocus(state.search_hwnd);
        }
        MessageBoxW(
            hwnd,
            to_wide(translate("没有找到可粘贴的目标窗口，内容已经保留在剪贴板中。").as_ref())
                .as_ptr(),
            to_wide(translate("粘贴失败").as_ref()).as_ptr(),
            MB_OK | MB_ICONWARNING,
        );
    }
}

unsafe fn is_window_enabled_compat(hwnd: HWND) -> bool {
    (GetWindowLongW(hwnd, GWL_STYLE) as u32 & WS_DISABLED) == 0
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

    let layout = state.layout();
    let min_pad = scale_for_window(state.hwnd, 6);
    let app_size = (((layout.title_h * 24) / 35).max(scale_for_window(state.hwnd, 20)))
        .min((layout.title_h - min_pad * 2).max(scale_for_window(state.hwnd, 20)));
    let app_icon = icon_handle_for(IconAssetKind::App, app_size);
    if app_icon != 0 {
        let app_pad_x = ((layout.title_h - app_size) / 2).max(min_pad);
        let app_pad_y = ((layout.title_h - app_size) / 2).max(min_pad.saturating_sub(1));
        draw_icon_tinted_soft(
            memdc as _, app_pad_x, app_pad_y, app_icon, app_size, app_size, false, 0,
        );
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
                let col = if down {
                    th.button_pressed
                } else {
                    th.button_hover
                };
                draw_round_rect(memdc as _, &inflate_rect(&rc, -2, -2), col, 0, 6);
            }
        }
        let slot = rc.right - rc.left;
        let iw = ((slot * 18 / 36).max(scale_for_window(state.hwnd, 18)))
            .min((slot - scale_for_window(state.hwnd, 4)).max(scale_for_window(state.hwnd, 18)));
        let icon = match key {
            "search" => icon_handle_for(IconAssetKind::Search, iw),
            "setting" => icon_handle_for(IconAssetKind::Setting, iw),
            "min" => icon_handle_for(IconAssetKind::Min, iw),
            _ => icon_handle_for(IconAssetKind::Close, iw),
        };
        if icon != 0 {
            let ih = iw;
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
        left: layout.seg_x,
        top: layout.seg_y,
        right: layout.seg_x + layout.seg_w,
        bottom: layout.seg_y + layout.seg_h,
    };
    let (tab0, tab1) = state.segment_rects();
    draw_main_segment_bar(
        memdc as _,
        &seg_rc,
        &tab0,
        &tab1,
        state.tab_index as i32,
        state.hover_tab,
        th,
    );

    let list_rc = RECT {
        left: layout.list_x,
        top: layout.list_y,
        right: layout.list_x + layout.list_w,
        bottom: layout.list_y + layout.list_h,
    };
    draw_round_rect(memdc as _, &list_rc, th.surface, th.stroke, 10);

    let saved_clip = SaveDC(memdc);
    IntersectClipRect(
        memdc,
        layout.list_x + 1,
        layout.list_y + 1,
        layout.list_x + layout.list_w - 1,
        layout.list_y + layout.list_h - 1,
    );
    if state.visible_count() == 0 {
        let text_rect = RECT {
            left: layout.list_x + 20,
            top: layout.list_y + 20,
            right: layout.list_x + layout.list_w - 20,
            bottom: layout.list_y + layout.list_h - 20,
        };
        let msg = if state.active_load_state().loading {
            tr("正在加载...", "Loading...")
        } else if state.active_load_state().error.is_some() {
            tr("加载失败，请稍后重试", "Loading failed. Please try again.")
        } else if state.settings.grouping_enabled && state.current_group_filter != 0 {
            tr("当前分组暂无记录", "No records in this group")
        } else if state.tab_index == 0 {
            tr("暂无剪贴板记录", "No clipboard records yet")
        } else {
            tr("暂无短语", "No phrases yet")
        };
        draw_text(
            memdc as _,
            msg,
            &text_rect,
            th.text_muted,
            layout
                .row_muted_text_size()
                .max(scale_for_window(state.hwnd, 13)),
            false,
            true,
        );
    } else {
        let view_top = layout.list_y + layout.list_pad;
        let view_bottom = layout.list_y + layout.list_h - layout.list_pad;
        let start_idx = max(0, state.scroll_y / layout.row_h);
        let end_idx = min(
            state.visible_count() as i32,
            (state.scroll_y + state.list_view_height()) / layout.row_h + 2,
        );

        for i in start_idx..end_idx {
            let Some(mut row_rc) = state.row_rect(i) else {
                continue;
            };
            if row_rc.bottom <= view_top || row_rc.top >= view_bottom {
                continue;
            }
            let item = state.active_items()[i as usize].clone();

            if state.row_is_selected(i) {
                let br = CreateSolidBrush(th.item_selected);
                FillRect(memdc, &row_rc, br);
                DeleteObject(br as _);
            } else if i == state.hover_idx {
                let br = CreateSolidBrush(th.item_hover);
                FillRect(memdc, &row_rc, br);
                DeleteObject(br as _);
            }

            let mut text_left = row_rc.left + (layout.row_h * 12 / 44).clamp(10, 20);
            if let Some(icon_rc) = layout.row_icon_rect(i, state.visible_count(), state.scroll_y) {
                let icon_w = icon_rc.right - icon_rc.left;
                let icon_h = icon_rc.bottom - icon_rc.top;
                let icon = item_icon_handle(&item, icon_w.max(icon_h));
                if icon != 0 {
                    draw_icon_tinted(
                        memdc as _,
                        icon_rc.left,
                        icon_rc.top,
                        icon,
                        icon_w,
                        icon_h,
                        dark,
                    );
                }
                text_left = text_left.max(icon_rc.right + (layout.row_h * 12 / 44).clamp(10, 18));
            }

            if item.pinned {
                if let Some(pin_rc) = layout.row_pin_rect(i, state.visible_count(), state.scroll_y)
                {
                    let pin_w = pin_rc.right - pin_rc.left;
                    let pin_h = pin_rc.bottom - pin_rc.top;
                    if pin_rc.top >= (view_top + 2) && pin_rc.bottom <= (view_bottom - 2) {
                        let pin_icon = icon_handle_for(IconAssetKind::Pin, pin_w.max(pin_h));
                        if pin_icon != 0 {
                            draw_icon_tinted(
                                memdc as _,
                                pin_rc.left,
                                pin_rc.top,
                                pin_icon,
                                pin_w,
                                pin_h,
                                dark,
                            );
                        }
                    }
                    text_left = text_left.max(pin_rc.right + (layout.row_h * 8 / 44).clamp(8, 16));
                }
            }

            if let Some(del_rc) = row_quick_delete_rect(state, i, &item) {
                let bg = inflate_rect(&del_rc, 2, 2);
                draw_round_rect(memdc as _, &bg, th.surface, th.stroke, 10);
                let del_size = (del_rc.right - del_rc.left)
                    .max(del_rc.bottom - del_rc.top)
                    .max(scale_for_window(state.hwnd, 16));
                let del_icon = icon_handle_for(IconAssetKind::Delete, del_size);
                if del_icon != 0 {
                    draw_icon_tinted(
                        memdc as _,
                        del_rc.left,
                        del_rc.top,
                        del_icon,
                        del_rc.right - del_rc.left,
                        del_rc.bottom - del_rc.top,
                        dark,
                    );
                }
            }

            row_rc.left = text_left;
            row_rc.right -= row_text_right_padding(state, i);
            if let Some(preview_rc) = row_inline_preview_rect(&row_rc, &item, &state.settings) {
                let bg = inflate_rect(&preview_rc, 2, 2);
                draw_round_rect(memdc as _, &bg, th.surface2, th.stroke, 8);
                let thumb_px = ((preview_rc.right - preview_rc.left)
                    .max(preview_rc.bottom - preview_rc.top)
                    + 8)
                .clamp(32, 96) as usize;
                if let Some((bytes, width, height)) =
                    ensure_item_thumbnail_bytes(state, &item, thumb_px)
                {
                    draw_rgba_image_fit(memdc as _, &bytes, width, height, &preview_rc);
                }
                row_rc.left = preview_rc.right + (layout.row_h * 10 / 44).clamp(10, 16);
            }
            // 图片条目：显示截图时间（本地时间），让用户快速识别
            let display_preview: String;
            let preview_str = if item.kind == ClipKind::Image {
                display_preview = format_created_at_local(&item.created_at, &item.preview);
                &display_preview
            } else {
                &item.preview
            };
            draw_text(
                memdc as _,
                preview_str,
                &row_rc,
                th.text,
                layout.row_text_size(),
                false,
                false,
            );
        }

        if state.active_load_state().loading {
            let loading_rc = RECT {
                left: layout.list_x + 18,
                top: layout.list_y + layout.list_h - (layout.row_h * 36 / 44).clamp(28, 44),
                right: layout.list_x + layout.list_w - 18,
                bottom: layout.list_y + layout.list_h - 12,
            };
            draw_text(
                memdc as _,
                "继续加载中...",
                &loading_rc,
                th.text_muted,
                layout.row_muted_text_size(),
                false,
                true,
            );
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
            draw_text_ex(
                memdc as _,
                "↑",
                &top_rc,
                th.text,
                18,
                true,
                true,
                "Segoe UI Variable Display",
            );
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
    let one_line = text.replace(['\r', '\n'], " ").trim().to_string();
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
    let names: Vec<String> = paths
        .iter()
        .map(|path| {
            let parsed = Path::new(path);
            parsed
                .file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(path)
                .to_string()
        })
        .collect();
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        2 => format!("{} + {}", names[0], names[1]),
        _ => format!("{} + {} 等 {} 项", names[0], names[1], names.len()),
    }
}

fn file_paths_signature(paths: &[String]) -> String {
    let merged = paths.join("\n");
    format!("files:{}", hash_bytes(merged.as_bytes()))
}

fn text_payload_for_qr(item: &ClipItem) -> Option<String> {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => item
            .text
            .as_ref()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty()),
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .map(|paths| {
                paths
                    .iter()
                    .map(|s| s.trim())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>()
                    .join("\n")
            })
            .filter(|s| !s.is_empty()),
        _ => None,
    }
}

fn build_qr_clip_item(text: &str) -> Option<(ClipItem, String)> {
    use qrcodegen::{QrCode, QrCodeEcc};

    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let qr = QrCode::encode_text(text, QrCodeEcc::Medium).ok()?;
    let module_count = qr.size().max(1) as usize;
    let border = 4usize;
    let total_modules = module_count + border * 2;
    let scale = (512usize / total_modules).clamp(4, 16);
    let side = total_modules * scale;
    let mut bytes = vec![255u8; side * side * 4];
    for y in 0..side {
        for x in 0..side {
            let mx = x / scale;
            let my = y / scale;
            let dark = mx >= border
                && my >= border
                && mx < border + module_count
                && my < border + module_count
                && qr.get_module((mx - border) as i32, (my - border) as i32);
            if dark {
                let idx = (y * side + x) * 4;
                bytes[idx] = 0;
                bytes[idx + 1] = 0;
                bytes[idx + 2] = 0;
                bytes[idx + 3] = 255;
            }
        }
    }
    let preview_text: String = text.chars().take(32).collect();
    let sig = format!("img:{}", hash_bytes(bytes.as_slice()));
    Some((
        ClipItem {
            id: 0,
            kind: ClipKind::Image,
            preview: format!("{} {}", tr("二维码", "QR"), preview_text),
            text: None,
            source_app: String::new(),
            file_paths: None,
            image_bytes: Some(bytes),
            image_path: None,
            image_width: side,
            image_height: side,
            pinned: false,
            group_id: 0,
            created_at: now_utc_sqlite(),
        },
        sig,
    ))
}

fn output_image_path() -> PathBuf {
    let base = data_dir().join("images");
    let _ = fs::create_dir_all(&base);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
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

fn write_image_bytes_to_path(out: &Path, bytes: &[u8], width: u32, height: u32) -> Option<PathBuf> {
    use std::fs::File;
    use std::io::BufWriter;

    let file = File::create(out).ok()?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header().ok()?;
    png_writer.write_image_data(bytes).ok()?;
    Some(out.to_path_buf())
}

fn write_image_bytes_to_output_path(bytes: &[u8], width: u32, height: u32) -> Option<PathBuf> {
    let out = output_image_path();
    write_image_bytes_to_path(&out, bytes, width, height)
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
    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
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
    full.image_path
        .as_deref()
        .and_then(load_image_bytes_from_path)
}

fn build_image_thumbnail_rgba(
    bytes: &[u8],
    width: usize,
    height: usize,
    max_side: usize,
) -> Option<ImageThumbnail> {
    if bytes.len() < 4 || width == 0 || height == 0 || max_side == 0 {
        return None;
    }
    if width <= max_side && height <= max_side {
        return Some(ImageThumbnail {
            bytes: bytes.to_vec(),
            width,
            height,
        });
    }
    let scale = (max_side as f32 / width as f32).min(max_side as f32 / height as f32);
    let out_w = ((width as f32 * scale).round() as usize).max(1);
    let out_h = ((height as f32 * scale).round() as usize).max(1);
    let mut out = vec![0u8; out_w * out_h * 4];
    for y in 0..out_h {
        let src_y = y * height / out_h;
        for x in 0..out_w {
            let src_x = x * width / out_w;
            let src_idx = (src_y * width + src_x) * 4;
            let dst_idx = (y * out_w + x) * 4;
            out[dst_idx..dst_idx + 4].copy_from_slice(&bytes[src_idx..src_idx + 4]);
        }
    }
    Some(ImageThumbnail {
        bytes: out,
        width: out_w,
        height: out_h,
    })
}

fn spawn_image_thumbnail_load(hwnd: HWND, item_id: i64, path: String, max_side: usize) {
    let hwnd_raw = hwnd as isize;
    std::thread::spawn(move || {
        let image = load_image_bytes_from_path(&path).and_then(|(bytes, width, height)| {
            build_image_thumbnail_rgba(&bytes, width, height, max_side)
        });
        let payload = Box::new(ImageThumbReadyResult { item_id, image });
        unsafe {
            let _ = post_boxed_message(hwnd_raw as HWND, WM_IMAGE_THUMB_READY, 0, payload);
        }
    });
}

fn ensure_item_thumbnail_bytes(
    state: &mut AppState,
    item: &ClipItem,
    max_side: usize,
) -> Option<(Vec<u8>, usize, usize)> {
    if item.id > 0 {
        if let Some(image) = state.image_thumb_cache.get(item.id) {
            return Some((image.bytes, image.width, image.height));
        }
    }
    let (bytes, width, height) = if let Some(bytes) = &item.image_bytes {
        (bytes.clone(), item.image_width, item.image_height)
    } else if let Some(path) = item.image_path.as_ref() {
        if item.id > 0 && state.image_thumb_loading.insert(item.id) {
            spawn_image_thumbnail_load(state.hwnd, item.id, path.clone(), max_side);
        }
        return None;
    } else {
        return None;
    };
    let thumb = build_image_thumbnail_rgba(&bytes, width, height, max_side)?;
    if item.id > 0 {
        state.image_thumb_cache.put(item.id, thumb.clone());
    }
    Some((thumb.bytes, thumb.width, thumb.height))
}

fn normalize_captured_image_rgba(
    bytes: Vec<u8>,
    width: usize,
    height: usize,
) -> Option<(Vec<u8>, usize, usize)> {
    if width == 0 || height == 0 {
        return None;
    }
    let expected = width.checked_mul(height)?.checked_mul(4)?;
    if bytes.len() < expected {
        return None;
    }
    if width <= MAX_CAPTURE_SIDE
        && height <= MAX_CAPTURE_SIDE
        && width.saturating_mul(height) <= MAX_CAPTURE_PIXELS
    {
        return Some((bytes, width, height));
    }

    let scale_by_pixels = ((MAX_CAPTURE_PIXELS as f64) / ((width as f64) * (height as f64))).sqrt();
    let scale_by_side =
        (MAX_CAPTURE_SIDE as f64 / width as f64).min(MAX_CAPTURE_SIDE as f64 / height as f64);
    let scale = scale_by_pixels.min(scale_by_side).min(1.0);
    let out_w = ((width as f64 * scale).round() as usize).max(1);
    let out_h = ((height as f64 * scale).round() as usize).max(1);
    let mut out = vec![0u8; out_w.checked_mul(out_h)?.checked_mul(4)?];
    for y in 0..out_h {
        let src_y = y * height / out_h;
        for x in 0..out_w {
            let src_x = x * width / out_w;
            let src_idx = (src_y * width + src_x) * 4;
            let dst_idx = (y * out_w + x) * 4;
            out[dst_idx..dst_idx + 4].copy_from_slice(&bytes[src_idx..src_idx + 4]);
        }
    }
    Some((out, out_w, out_h))
}

fn read_windows_clipboard_bitmap_rgba() -> Option<(Vec<u8>, usize, usize)> {
    let bmp_bytes: Vec<u8> = clipboard_win::get_clipboard(clipboard_win::formats::Bitmap).ok()?;
    let (declared_w, declared_h) = clipboard_bitmap_dimensions(&bmp_bytes)?;
    let declared_pixels = declared_w.checked_mul(declared_h)?;
    if declared_w > MAX_CAPTURE_SIDE
        || declared_h > MAX_CAPTURE_SIDE
        || declared_pixels > MAX_CLIPBOARD_BITMAP_DECODE_PIXELS
    {
        return None;
    }
    let image = image::load_from_memory_with_format(&bmp_bytes, ImageFormat::Bmp).ok()?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some((rgba.into_raw(), width as usize, height as usize))
}

fn clipboard_bitmap_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 16 {
        return None;
    }
    if bytes.starts_with(b"BM") && bytes.len() >= 26 {
        let width = i32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?);
        let height = i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?);
        return bitmap_dimension_pair(width, height);
    }
    let header_size = u32::from_le_bytes(bytes.get(0..4)?.try_into().ok()?);
    match header_size {
        12 if bytes.len() >= 8 => {
            let width = u16::from_le_bytes(bytes.get(4..6)?.try_into().ok()?) as i32;
            let height = u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?) as i32;
            bitmap_dimension_pair(width, height)
        }
        40..=124 if bytes.len() >= 12 => {
            let width = i32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?);
            let height = i32::from_le_bytes(bytes.get(8..12)?.try_into().ok()?);
            bitmap_dimension_pair(width, height)
        }
        _ => None,
    }
}

fn bitmap_dimension_pair(width: i32, height: i32) -> Option<(usize, usize)> {
    if width == 0 || height == 0 {
        return None;
    }
    Some((
        width.unsigned_abs() as usize,
        height.unsigned_abs() as usize,
    ))
}

fn hash_bytes(data: &[u8]) -> String {
    const OFFSET_A: u64 = 0xcbf29ce484222325;
    const OFFSET_B: u64 = 0x84222325cbf29ce4;
    const PRIME_A: u64 = 0x100000001b3;
    const PRIME_B: u64 = 0x10000000161;
    let mut a = OFFSET_A ^ (data.len() as u64);
    let mut b = OFFSET_B ^ ((data.len() as u64).rotate_left(17));
    for &byte in data {
        a ^= byte as u64;
        a = a.wrapping_mul(PRIME_A);
        b ^= (byte as u64).rotate_left(1);
        b = b.wrapping_mul(PRIME_B);
    }
    format!("{:016x}{:016x}", a, b)
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
    let layout = state.layout();
    let inner = RECT {
        left: layout.list_x + layout.list_pad,
        top: layout.list_y + layout.list_pad,
        right: layout.list_x + layout.list_w - layout.list_pad,
        bottom: layout.list_y + layout.list_h - layout.list_pad,
    };
    if !pt_in_rect(x, y, &inner) {
        return -1;
    }
    let yy = y - inner.top + state.scroll_y;
    let idx = yy / layout.row_h;
    if idx < 0 || idx >= state.visible_count() as i32 {
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
    let layout = state.layout();
    if row_shows_delete_button(state, visible_idx) {
        (layout.row_h * 42 / 44).max(42)
    } else {
        (layout.row_h * 18 / 44).max(18)
    }
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
    let size = ((row_rc.bottom - row_rc.top) - 8).max(24);
    let left = row_rc.left + 2;
    let top = row_rc.top + ((row_rc.bottom - row_rc.top - size) / 2);
    Some(RECT {
        left,
        top,
        right: left + size,
        bottom: top + size,
    })
}

fn scroll_to_top_visible(state: &AppState) -> bool {
    state.scroll_y > state.layout().row_h
}

unsafe fn hovered_item_clone(state: &AppState) -> Option<ClipItem> {
    if state.hover_idx < 0 {
        return None;
    }
    state.active_items().get(state.hover_idx as usize).cloned()
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

    let bgra = crate::ui::rgba_to_bgra(bytes);

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
        bgra.as_ptr() as _,
        &bmi,
        DIB_RGB_COLORS,
        SRCCOPY,
    );
}
