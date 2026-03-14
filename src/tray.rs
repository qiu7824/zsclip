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
use crate::win_system_ui::{apply_theme_to_menu, to_wide};
use crate::window_position::resolve_main_window_position;

pub(crate) unsafe fn handle_tray(hwnd: HWND, msg: u32) {
    match msg {
        WM_LBUTTONUP | WM_LBUTTONDBLCLK => toggle_window_visibility(hwnd),
        WM_RBUTTONUP | WM_CONTEXTMENU => show_tray_menu(hwnd),
        _ => {}
    }
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
        position_main_window(hwnd, &(*pst).settings, by_hotkey);
    }
    ShowWindow(hwnd, SW_SHOW);
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
}

pub(crate) unsafe fn toggle_window_visibility(hwnd: HWND) {
    if IsWindowVisible(hwnd) != 0 {
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
    if IsWindowVisible(hwnd) != 0 && GetForegroundWindow() == hwnd {
        ShowWindow(hwnd, SW_HIDE);
    } else {
        show_main_window(hwnd, true);
    }
}

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
