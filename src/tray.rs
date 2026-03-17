use std::mem::zeroed;
use std::ptr::{null, null_mut};

use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    UI::{
        Input::KeyboardAndMouse::SetFocus,
        Shell::{Shell_NotifyIconW, NOTIFYICONDATAW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE},
        WindowsAndMessaging::*,
    },
};

use crate::app::{AppSettings, IDM_TRAY_EXIT, IDM_TRAY_TOGGLE, TRAY_UID, WM_TRAYICON};
use crate::i18n::{app_title, translate};
use crate::win_system_ui::{apply_theme_to_menu, to_wide};
use crate::window_position::resolve_main_window_position;

unsafe fn window_class_name(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut buf = [0u16; 64];
    let len = GetClassNameW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    if len <= 0 {
        String::new()
    } else {
        String::from_utf16_lossy(&buf[..len as usize])
    }
}

unsafe fn explorer_rename_target() -> Option<(HWND, HWND)> {
    let fg = GetForegroundWindow();
    if fg.is_null() {
        return None;
    }

    let fg_class = window_class_name(fg);
    if !matches!(fg_class.as_str(), "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW") {
        return None;
    }

    let thread_id = GetWindowThreadProcessId(fg, null_mut());
    if thread_id == 0 {
        return None;
    }

    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if GetGUIThreadInfo(thread_id, &mut info) == 0 {
        return None;
    }

    let focus = if !info.hwndFocus.is_null() {
        info.hwndFocus
    } else {
        info.hwndCaret
    };
    if matches!(window_class_name(focus).as_str(), "Edit") {
        Some((fg, focus))
    } else {
        None
    }
}

pub(crate) unsafe fn handle_tray(hwnd: HWND, msg: u32) {
    match msg {
        WM_LBUTTONUP | WM_LBUTTONDBLCLK => toggle_window_visibility(hwnd),
        WM_RBUTTONUP | WM_CONTEXTMENU => show_tray_menu_localized(hwnd),
        _ => {}
    }
}

pub(crate) unsafe fn show_tray_menu_localized(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() { return; }
    apply_theme_to_menu(menu as _);
    AppendMenuW(menu, MF_STRING, IDM_TRAY_TOGGLE, to_wide(translate("显示/隐藏").as_ref()).as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, IDM_TRAY_EXIT, to_wide(translate("退出").as_ref()).as_ptr());

    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(menu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN, pt.x, pt.y, 0, hwnd, null());
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
}

pub(crate) unsafe fn add_tray_icon_localized(hwnd: HWND, icon: isize) -> bool {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = icon as _;
    let tip = to_wide(app_title());
    let n = core::cmp::min(tip.len(), nid.szTip.len());
    nid.szTip[..n].copy_from_slice(&tip[..n]);
    Shell_NotifyIconW(NIM_ADD, &mut nid) != 0
}

pub(crate) unsafe fn position_main_window(hwnd: HWND, settings: &AppSettings, by_hotkey: bool) {
    let sw = GetSystemMetrics(SM_CXSCREEN);
    let sh = GetSystemMetrics(SM_CYSCREEN);
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    let (x, y) = resolve_main_window_position(settings, by_hotkey, sw, sh, pt);
    SetWindowPos(hwnd, null_mut(), x, y, 0, 0, SWP_NOSIZE | SWP_NOZORDER | SWP_NOACTIVATE);
}

pub(crate) unsafe fn show_main_window(hwnd: HWND, by_hotkey: bool) {
    let pst = crate::app::get_state_ptr(hwnd);
    if !pst.is_null() {
        crate::app::refresh_window_for_show(hwnd);
        position_main_window(hwnd, &(*pst).settings, by_hotkey);
        (*pst).edge_hidden = false;
        (*pst).edge_hidden_side = -1;
        (*pst).hotkey_passthrough_active = false;
        (*pst).hotkey_passthrough_target = null_mut();
        (*pst).hotkey_passthrough_edit = null_mut();
    }
    crate::app::set_main_window_noactivate_mode(hwnd, false);
    ShowWindow(hwnd, SW_SHOW);
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
}

pub(crate) unsafe fn show_quick_window(by_hotkey: bool) {
    let hwnd = crate::app::quick_window_hwnd();
    if hwnd.is_null() {
        return;
    }
    crate::app::refresh_window_for_show(hwnd);
    let pst = crate::app::get_state_ptr(hwnd);
    if !pst.is_null() {
        position_main_window(hwnd, &(*pst).settings, by_hotkey);
        (*pst).edge_hidden = false;
        (*pst).edge_hidden_side = -1;
        if by_hotkey {
            if let Some((target, edit)) = explorer_rename_target() {
                (*pst).hotkey_passthrough_active = true;
                (*pst).hotkey_passthrough_target = target;
                (*pst).hotkey_passthrough_edit = edit;
            } else {
                (*pst).hotkey_passthrough_active = false;
                (*pst).hotkey_passthrough_target = null_mut();
                (*pst).hotkey_passthrough_edit = null_mut();
            }
        } else {
            (*pst).hotkey_passthrough_active = false;
            (*pst).hotkey_passthrough_target = null_mut();
            (*pst).hotkey_passthrough_edit = null_mut();
        }
    }
    crate::app::set_main_window_noactivate_mode(hwnd, true);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
}

pub(crate) unsafe fn toggle_window_visibility(hwnd: HWND) {
    let quick = crate::app::quick_window_hwnd();
    if !quick.is_null() && IsWindowVisible(quick) != 0 {
        crate::app::set_main_window_noactivate_mode(quick, false);
        ShowWindow(quick, SW_HIDE);
    }
    if IsWindowVisible(hwnd) != 0 {
        let pst = crate::app::get_state_ptr(hwnd);
        if !pst.is_null() {
            (*pst).hotkey_passthrough_active = false;
            (*pst).hotkey_passthrough_target = null_mut();
            (*pst).hotkey_passthrough_edit = null_mut();
        }
        crate::app::set_main_window_noactivate_mode(hwnd, false);
        ShowWindow(hwnd, SW_HIDE);
    } else {
        show_main_window(hwnd, false);
    }
}

pub(crate) unsafe fn remember_window_pos(hwnd: HWND) {
    let pst = crate::app::get_state_ptr(hwnd);
    if pst.is_null() || IsIconic(hwnd) != 0 {
        return;
    }
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd, &mut rc) != 0 {
        (*pst).settings.last_window_x = rc.left;
        (*pst).settings.last_window_y = rc.top;
        crate::app::save_settings(&(*pst).settings);
    }
}

pub(crate) unsafe fn toggle_window_visibility_hotkey(hwnd: HWND) {
    let quick = crate::app::quick_window_hwnd();
    if !quick.is_null() && IsWindowVisible(quick) != 0 {
        let pst = crate::app::get_state_ptr(quick);
        if !pst.is_null() {
            (*pst).hotkey_passthrough_active = false;
            (*pst).hotkey_passthrough_target = null_mut();
            (*pst).hotkey_passthrough_edit = null_mut();
        }
        crate::app::set_main_window_noactivate_mode(quick, false);
        ShowWindow(quick, SW_HIDE);
    } else if IsWindowVisible(hwnd) != 0 {
        let pst = crate::app::get_state_ptr(hwnd);
        if !pst.is_null() {
            (*pst).hotkey_passthrough_active = false;
            (*pst).hotkey_passthrough_target = null_mut();
            (*pst).hotkey_passthrough_edit = null_mut();
        }
        crate::app::set_main_window_noactivate_mode(hwnd, false);
        ShowWindow(hwnd, SW_HIDE);
    } else {
        show_quick_window(true);
    }
}

#[allow(dead_code)]
pub(crate) unsafe fn show_tray_menu(hwnd: HWND) {
    let menu = CreatePopupMenu();
    if menu.is_null() { return; }
    apply_theme_to_menu(menu as _);
    AppendMenuW(menu, MF_STRING, IDM_TRAY_TOGGLE, to_wide("显示/隐藏").as_ptr());
    AppendMenuW(menu, MF_SEPARATOR, 0, null());
    AppendMenuW(menu, MF_STRING, IDM_TRAY_EXIT, to_wide("退出").as_ptr());

    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    SetForegroundWindow(hwnd);
    TrackPopupMenu(menu, TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN, pt.x, pt.y, 0, hwnd, null());
    PostMessageW(hwnd, WM_NULL, 0, 0);
    DestroyMenu(menu);
}

#[allow(dead_code)]
pub(crate) unsafe fn add_tray_icon(hwnd: HWND, icon: isize) {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    nid.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    nid.uCallbackMessage = WM_TRAYICON;
    nid.hIcon = icon as _;
    let tip = to_wide("剪贴板");
    let n = core::cmp::min(tip.len(), nid.szTip.len());
    nid.szTip[..n].copy_from_slice(&tip[..n]);
    Shell_NotifyIconW(NIM_ADD, &mut nid);
}

pub(crate) unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    Shell_NotifyIconW(NIM_DELETE, &mut nid);
}
