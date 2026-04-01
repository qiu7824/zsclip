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

use crate::app::{IDM_TRAY_EXIT, IDM_TRAY_TOGGLE, TRAY_UID, WM_TRAYICON};
use crate::app::state::{apply_shared_tab_view_state, AppSettings};
use crate::app::AppState;
use crate::i18n::{app_title, translate};
use crate::ui::MainUiLayout;
use crate::win_system_ui::{apply_theme_to_menu, monitor_dpi_for_point, nearest_monitor_rect_for_point, nearest_monitor_work_rect_for_point, to_wide};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum MainWindowPosMode {
    Mouse,
    Fixed,
    Last,
    Center,
}

fn parse_main_window_pos_mode(mode: &str) -> MainWindowPosMode {
    match mode {
        "fixed" => MainWindowPosMode::Fixed,
        "last" => MainWindowPosMode::Last,
        "mouse" => MainWindowPosMode::Mouse,
        _ => MainWindowPosMode::Center,
    }
}

fn main_window_layout_for_point(pt: POINT) -> MainUiLayout {
    unsafe { MainUiLayout::zsclip().scaled(monitor_dpi_for_point(pt)) }
}

fn main_window_size_for_point(pt: POINT) -> (i32, i32) {
    let layout = main_window_layout_for_point(pt);
    (layout.win_w, layout.list_y + layout.list_h + 7)
}

fn edge_preferred_position(state: &AppState) -> Option<(i32, i32)> {
    if !state.settings.edge_auto_hide {
        return None;
    }
    if state.edge_hidden_side >= 0
        && state.edge_docked_right > state.edge_docked_left
        && state.edge_docked_bottom > state.edge_docked_top
    {
        return Some((state.edge_restore_x, state.edge_restore_y));
    }
    if state.settings.last_window_x >= 0 && state.settings.last_window_y >= 0 {
        return Some((state.settings.last_window_x, state.settings.last_window_y));
    }
    None
}

unsafe fn position_window_at_restore(hwnd: HWND, restore_x: i32, restore_y: i32) {
    let anchor = POINT { x: restore_x, y: restore_y };
    let (win_w, win_h) = main_window_size_for_point(anchor);
    let work = nearest_monitor_work_rect_for_point(anchor);
    let monitor = nearest_monitor_rect_for_point(anchor);
    let clamp_rect = RECT {
        left: std::cmp::max(work.left, monitor.left),
        top: std::cmp::max(work.top, monitor.top),
        right: std::cmp::min(work.right, monitor.right),
        bottom: std::cmp::min(work.bottom, monitor.bottom),
    };
    let (x, y) = clamp_to_rect(restore_x, restore_y, &clamp_rect, win_w, win_h);
    SetWindowPos(hwnd, null_mut(), x, y, win_w, win_h, SWP_NOZORDER | SWP_NOACTIVATE);
}

fn clamp_to_rect(x: i32, y: i32, rc: &RECT, win_w: i32, win_h: i32) -> (i32, i32) {
    (
        std::cmp::max(rc.left, std::cmp::min(x, std::cmp::max(rc.left, rc.right - win_w))),
        std::cmp::max(rc.top, std::cmp::min(y, std::cmp::max(rc.top, rc.bottom - win_h))),
    )
}

fn mouse_anchor(settings: &AppSettings, cursor: POINT) -> (i32, i32) {
    (
        cursor.x + settings.show_mouse_dx,
        cursor.y + settings.show_mouse_dy,
    )
}

fn resolve_main_window_position(
    settings: &AppSettings,
    by_hotkey: bool,
    cursor: POINT,
) -> (i32, i32, i32, i32) {
    let requested = parse_main_window_pos_mode(settings.show_pos_mode.as_str());
    let (x, y) = match requested {
        MainWindowPosMode::Fixed => (settings.show_fixed_x, settings.show_fixed_y),
        MainWindowPosMode::Last if settings.last_window_x >= 0 && settings.last_window_y >= 0 => {
            (settings.last_window_x, settings.last_window_y)
        }
        MainWindowPosMode::Mouse => mouse_anchor(settings, cursor),
        MainWindowPosMode::Last if by_hotkey => mouse_anchor(settings, cursor),
        MainWindowPosMode::Center if by_hotkey => mouse_anchor(settings, cursor),
        _ => {
            let work = unsafe { nearest_monitor_work_rect_for_point(cursor) };
            let (win_w, win_h) = main_window_size_for_point(cursor);
            (
                work.left + ((work.right - work.left - win_w) / 2),
                work.top + ((work.bottom - work.top - win_h) / 3),
            )
        }
    };
    let anchor = POINT { x, y };
    let (win_w, win_h) = main_window_size_for_point(anchor);
    let work = unsafe { nearest_monitor_work_rect_for_point(anchor) };
    let monitor = unsafe { nearest_monitor_rect_for_point(anchor) };
    let clamp_rect = RECT {
        left: std::cmp::max(work.left, monitor.left),
        top: std::cmp::max(work.top, monitor.top),
        right: std::cmp::min(work.right, monitor.right),
        bottom: std::cmp::min(work.bottom, monitor.bottom),
    };
    let (x, y) = clamp_to_rect(x, y, &clamp_rect, win_w, win_h);
    (x, y, win_w, win_h)
}

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
    let (fg, focus) = foreground_focus_snapshot()?;
    let fg_class = window_class_name(fg);
    if !matches!(fg_class.as_str(), "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW") {
        return None;
    }
    if matches!(window_class_name(focus).as_str(), "Edit") {
        Some((fg, focus))
    } else {
        None
    }
}

unsafe fn foreground_focus_snapshot() -> Option<(HWND, HWND)> {
    let fg = GetForegroundWindow();
    if fg.is_null() {
        return None;
    }

    let thread_id = GetWindowThreadProcessId(fg, null_mut());
    if thread_id == 0 {
        return Some((fg, null_mut()));
    }

    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if GetGUIThreadInfo(thread_id, &mut info) == 0 {
        return Some((fg, null_mut()));
    }

    let focus = if !info.hwndFocus.is_null() {
        info.hwndFocus
    } else {
        info.hwndCaret
    };
    if !focus.is_null() && GetAncestor(focus, GA_ROOT) == fg {
        Some((fg, focus))
    } else {
        Some((fg, null_mut()))
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
    Shell_NotifyIconW(NIM_ADD, &nid) != 0
}

unsafe fn position_main_window_for_state(hwnd: HWND, state: &AppState, by_hotkey: bool) {
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    if state.settings.edge_auto_hide {
        if let Some((restore_x, restore_y)) = edge_preferred_position(state) {
            position_window_at_restore(hwnd, restore_x, restore_y);
            return;
        }
        let work = nearest_monitor_work_rect_for_point(pt);
        let (win_w, win_h) = main_window_size_for_point(pt);
        let x = work.left + ((work.right - work.left - win_w) / 2);
        let y = work.top + ((work.bottom - work.top - win_h) / 3);
        SetWindowPos(hwnd, null_mut(), x, y, win_w, win_h, SWP_NOZORDER | SWP_NOACTIVATE);
        return;
    }
    let (x, y, win_w, win_h) = resolve_main_window_position(&state.settings, by_hotkey, pt);
    SetWindowPos(hwnd, null_mut(), x, y, win_w, win_h, SWP_NOZORDER | SWP_NOACTIVATE);
}

pub(crate) unsafe fn position_main_window(hwnd: HWND, settings: &AppSettings, by_hotkey: bool) {
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    let (x, y, win_w, win_h) = resolve_main_window_position(settings, by_hotkey, pt);
    SetWindowPos(hwnd, null_mut(), x, y, win_w, win_h, SWP_NOZORDER | SWP_NOACTIVATE);
}

pub(crate) unsafe fn show_main_window(hwnd: HWND, by_hotkey: bool) {
    let pst = crate::app::get_state_ptr(hwnd);
    if !pst.is_null() {
        let state = &mut *pst;
        crate::app::refresh_window_for_show(hwnd);
        if apply_shared_tab_view_state(state) {
            state.clear_selection();
            state.scroll_y = 0;
            state.refilter();
        }
        crate::app::prepare_search_ui_for_show(hwnd, state);
        position_main_window_for_state(hwnd, state, by_hotkey);
        state.edge_hidden = false;
        state.edge_hide_pending_until = None;
        state.edge_restore_wait_leave = false;
        state.edge_anim_until = None;
        if state.settings.edge_auto_hide {
            crate::app::hosts::note_window_moved_for_edge_hide(hwnd, state);
        } else {
            crate::app::hosts::clear_edge_dock_state(state);
        }
        state.hotkey_passthrough_active = false;
        state.hotkey_passthrough_target = null_mut();
        state.hotkey_passthrough_focus = null_mut();
        state.hotkey_passthrough_edit = null_mut();
        state.plain_text_paste_mode = false;
    }
    crate::app::set_main_window_noactivate_mode(hwnd, false);
    ShowWindow(hwnd, SW_SHOW);
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
    SetForegroundWindow(hwnd);
    SetFocus(hwnd);
    crate::app::refresh_low_level_input_hooks();
}

pub(crate) unsafe fn show_quick_window(by_hotkey: bool, plain_text_mode: bool) {
    let hwnd = crate::app::quick_window_hwnd();
    if hwnd.is_null() {
        return;
    }
    crate::app::refresh_window_for_show(hwnd);
    let pst = crate::app::get_state_ptr(hwnd);
    if !pst.is_null() {
        let state = &mut *pst;
        if apply_shared_tab_view_state(state) {
            state.clear_selection();
            state.scroll_y = 0;
            state.refilter();
        }
        crate::app::prepare_search_ui_for_show(hwnd, state);
        position_main_window_for_state(hwnd, state, by_hotkey);
        state.edge_hidden = false;
        state.edge_hide_pending_until = None;
        state.edge_restore_wait_leave = false;
        state.edge_anim_until = None;
        if state.settings.edge_auto_hide {
            crate::app::hosts::note_window_moved_for_edge_hide(hwnd, state);
        } else {
            crate::app::hosts::clear_edge_dock_state(state);
        }
        if by_hotkey {
            if let Some((target, focus)) = foreground_focus_snapshot() {
                state.hotkey_passthrough_active = true;
                state.hotkey_passthrough_target = target;
                state.hotkey_passthrough_focus = focus;
                state.hotkey_passthrough_edit =
                    explorer_rename_target().map(|(_, edit)| edit).unwrap_or(null_mut());
            } else {
                state.hotkey_passthrough_active = false;
                state.hotkey_passthrough_target = null_mut();
                state.hotkey_passthrough_focus = null_mut();
                state.hotkey_passthrough_edit = null_mut();
            }
        } else {
            state.hotkey_passthrough_active = false;
            state.hotkey_passthrough_target = null_mut();
            state.hotkey_passthrough_focus = null_mut();
            state.hotkey_passthrough_edit = null_mut();
        }
        state.plain_text_paste_mode = plain_text_mode;
    }
    crate::app::set_main_window_noactivate_mode(hwnd, true);
    ShowWindow(hwnd, SW_SHOWNOACTIVATE);
    SetWindowPos(hwnd, HWND_TOPMOST, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW);
    crate::app::refresh_low_level_input_hooks();
}

pub(crate) unsafe fn toggle_window_visibility(hwnd: HWND) {
    let quick = crate::app::quick_window_hwnd();
    if !quick.is_null() && IsWindowVisible(quick) != 0 {
        if crate::app::hosts::try_restore_edge_hidden_window(quick) {
            crate::app::refresh_low_level_input_hooks();
            return;
        }
        crate::app::set_main_window_noactivate_mode(quick, false);
        ShowWindow(quick, SW_HIDE);
    }
    if crate::app::hosts::try_restore_edge_hidden_window(hwnd) {
        crate::app::refresh_low_level_input_hooks();
        return;
    }
    if IsWindowVisible(hwnd) != 0 {
        let pst = crate::app::get_state_ptr(hwnd);
        if !pst.is_null() {
            (*pst).hotkey_passthrough_active = false;
            (*pst).hotkey_passthrough_target = null_mut();
            (*pst).hotkey_passthrough_focus = null_mut();
            (*pst).hotkey_passthrough_edit = null_mut();
        }
        crate::app::set_main_window_noactivate_mode(hwnd, false);
        ShowWindow(hwnd, SW_HIDE);
        crate::app::refresh_low_level_input_hooks();
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
        let (save_x, save_y) = if (*pst).settings.edge_auto_hide && (*pst).edge_hidden {
            ((*pst).edge_restore_x, (*pst).edge_restore_y)
        } else {
            (rc.left, rc.top)
        };
        (*pst).settings.last_window_x = save_x;
        (*pst).settings.last_window_y = save_y;
    }
}

pub(crate) unsafe fn toggle_window_visibility_hotkey(hwnd: HWND) {
    let mut plain_text_mode = false;
    let main_ptr = crate::app::get_state_ptr(hwnd);
    if !main_ptr.is_null() {
        plain_text_mode = (*main_ptr).plain_text_paste_mode;
    }
    let quick = crate::app::quick_window_hwnd();
    if !quick.is_null() && IsWindowVisible(quick) != 0 {
        if crate::app::hosts::try_restore_edge_hidden_window(quick) {
            crate::app::refresh_low_level_input_hooks();
            return;
        }
        let pst = crate::app::get_state_ptr(quick);
        if !pst.is_null() {
            (*pst).hotkey_passthrough_active = false;
            (*pst).hotkey_passthrough_target = null_mut();
            (*pst).hotkey_passthrough_focus = null_mut();
            (*pst).hotkey_passthrough_edit = null_mut();
            (*pst).plain_text_paste_mode = false;
        }
        crate::app::set_main_window_noactivate_mode(quick, false);
        ShowWindow(quick, SW_HIDE);
        crate::app::refresh_low_level_input_hooks();
    } else if IsWindowVisible(hwnd) != 0 {
        let pst = crate::app::get_state_ptr(hwnd);
        if crate::app::hosts::try_restore_edge_hidden_window(hwnd) {
            crate::app::refresh_low_level_input_hooks();
            return;
        }
        if !pst.is_null() {
            (*pst).hotkey_passthrough_active = false;
            (*pst).hotkey_passthrough_target = null_mut();
            (*pst).hotkey_passthrough_focus = null_mut();
            (*pst).hotkey_passthrough_edit = null_mut();
            (*pst).plain_text_paste_mode = false;
        }
        crate::app::set_main_window_noactivate_mode(hwnd, false);
        ShowWindow(hwnd, SW_HIDE);
        crate::app::refresh_low_level_input_hooks();
    } else {
        show_quick_window(true, plain_text_mode);
    }
}

pub(crate) unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut nid: NOTIFYICONDATAW = zeroed();
    nid.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    nid.hWnd = hwnd;
    nid.uID = TRAY_UID;
    Shell_NotifyIconW(NIM_DELETE, &nid);
}

