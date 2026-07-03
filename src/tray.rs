use std::mem::zeroed;
use std::ptr::{null, null_mut};

use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    UI::WindowsAndMessaging::*,
};

use crate::app::state::{apply_shared_tab_view_state, AppSettings};
use crate::app::{get_state_ptr, AppState};
use crate::app::{TRAY_UID, WM_TRAYICON};
use crate::app_core::{
    clamp_window_pos_to_rect, main_edge_restore_position, main_remember_window_position,
    main_show_prepare_plan, main_show_window_state_plan, main_tray_menu_plan,
    main_window_hotkey_visibility_plan, main_window_position_anchor, main_window_position_plan,
    main_window_toggle_visibility_plan, native_host_status_menu_item_specs,
    MainEdgeRestorePositionInput, MainHotkeyPassthroughPlan, MainRememberWindowPositionInput,
    MainShowPrepareInput, MainShowPreparePlan, MainShowSearchAction, MainShowWindowDockAction,
    MainShowWindowKind, MainShowWindowStateInput, MainShowWindowStatePlan, MainTrayMenuInput,
    MainTrayMenuItem, MainTrayMenuText, MainUiLayout, MainWindowHotkeyVisibilityInput,
    MainWindowHotkeyVisibilityStep, MainWindowPositionInput, MainWindowPositionMode,
    MainWindowVisibilityInput, MainWindowVisibilityStep, NativeMainWindowPresentMode,
    StatusItemHost, StatusMenuEntry, UiRect,
};
use crate::i18n::{app_title, translate};
use crate::platform::{
    dpi as platform_dpi, gdi as platform_gdi, input as platform_input, monitor as platform_monitor,
    tray_icon::WindowsStatusItemHost, window as platform_window,
};

fn parse_main_window_pos_mode(mode: &str) -> MainWindowPositionMode {
    match mode {
        "fixed" => MainWindowPositionMode::Fixed,
        "last" => MainWindowPositionMode::Last,
        "mouse" => MainWindowPositionMode::Mouse,
        _ => MainWindowPositionMode::Center,
    }
}

fn tray_menu_text_label(text: MainTrayMenuText) -> &'static str {
    match text {
        MainTrayMenuText::ToggleWindow => "显示/隐藏",
        MainTrayMenuText::EnableClipboardCapture => "开启剪贴板捕捉",
        MainTrayMenuText::DisableClipboardCapture => "关闭剪贴板捕捉",
        MainTrayMenuText::LanSyncOn => "局域网同步：开",
        MainTrayMenuText::LanSyncOff => "局域网同步：关",
        MainTrayMenuText::Exit => "退出",
    }
}

fn windows_status_menu_entries_from_native_specs(
    clipboard_capture_enabled: bool,
    lan_sync_enabled: bool,
) -> Vec<StatusMenuEntry> {
    let planned = main_tray_menu_plan(MainTrayMenuInput {
        clipboard_capture_enabled,
        lan_sync_enabled,
    });
    let mut entries = Vec::new();

    for spec in native_host_status_menu_item_specs() {
        let action = spec.action;
        let tray_action = action.tray_action();
        if spec.starts_section {
            entries.push(StatusMenuEntry::Separator);
        }
        let label = planned
            .iter()
            .find_map(|item| match item {
                MainTrayMenuItem::Command { action, text } if *action == tray_action => {
                    Some(translate(tray_menu_text_label(*text)).into_owned())
                }
                _ => None,
            })
            .unwrap_or_else(|| translate(spec.label).into_owned());
        entries.push(StatusMenuEntry::Command {
            action: tray_action,
            label,
            icon_name: spec.icon_name.to_string(),
        });
    }

    entries
}

fn main_window_layout_for_point(pt: POINT) -> MainUiLayout {
    unsafe { MainUiLayout::zsclip().scaled(platform_dpi::layout_dpi_for_point(pt)) }
}

fn main_window_size_for_point(pt: POINT) -> (i32, i32) {
    let layout = main_window_layout_for_point(pt);
    (layout.win_w, layout.list_y + layout.list_h + 7)
}

fn edge_preferred_position(state: &AppState) -> Option<(i32, i32)> {
    main_edge_restore_position(MainEdgeRestorePositionInput {
        edge_auto_hide: state.settings.edge_auto_hide,
        edge_hidden_side: state.edge_hidden_side,
        edge_docked_left: state.edge_docked_left,
        edge_docked_top: state.edge_docked_top,
        edge_docked_right: state.edge_docked_right,
        edge_docked_bottom: state.edge_docked_bottom,
        edge_restore_x: state.edge_restore_x,
        edge_restore_y: state.edge_restore_y,
        last_window_x: state.settings.last_window_x,
        last_window_y: state.settings.last_window_y,
    })
    .map(|anchor| (anchor.x, anchor.y))
}

unsafe fn position_window_at_restore(hwnd: HWND, restore_x: i32, restore_y: i32) {
    let anchor = POINT {
        x: restore_x,
        y: restore_y,
    };
    let (win_w, win_h) = main_window_size_for_point(anchor);
    let work = platform_monitor::nearest_work_rect_for_point(anchor);
    let monitor = platform_monitor::nearest_rect_for_point(anchor);
    let clamp_rect = RECT {
        left: std::cmp::max(work.left, monitor.left),
        top: std::cmp::max(work.top, monitor.top),
        right: std::cmp::min(work.right, monitor.right),
        bottom: std::cmp::min(work.bottom, monitor.bottom),
    };
    let (x, y) = clamp_window_pos_to_rect(
        restore_x,
        restore_y,
        UiRect::new(
            clamp_rect.left,
            clamp_rect.top,
            clamp_rect.right,
            clamp_rect.bottom,
        ),
        win_w,
        win_h,
    );
    crate::app::set_main_window_bounds(hwnd, UiRect::new(x, y, x + win_w, y + win_h));
}

fn resolve_main_window_position(
    settings: &AppSettings,
    by_hotkey: bool,
    cursor: POINT,
) -> (i32, i32, i32, i32) {
    let requested = parse_main_window_pos_mode(settings.show_pos_mode.as_str());
    let (win_w, win_h) = main_window_size_for_point(cursor);
    let cursor_work = platform_monitor::nearest_work_rect_for_point(cursor);
    let cursor_monitor = platform_monitor::nearest_rect_for_point(cursor);
    let cursor_bounds = UiRect::new(
        std::cmp::max(cursor_work.left, cursor_monitor.left),
        std::cmp::max(cursor_work.top, cursor_monitor.top),
        std::cmp::min(cursor_work.right, cursor_monitor.right),
        std::cmp::min(cursor_work.bottom, cursor_monitor.bottom),
    );
    let base_input = MainWindowPositionInput {
        mode: requested,
        by_hotkey,
        cursor_x: cursor.x,
        cursor_y: cursor.y,
        mouse_dx: settings.show_mouse_dx,
        mouse_dy: settings.show_mouse_dy,
        fixed_x: settings.show_fixed_x,
        fixed_y: settings.show_fixed_y,
        last_x: settings.last_window_x,
        last_y: settings.last_window_y,
        bounds: cursor_bounds,
        win_w,
        win_h,
    };
    let anchor = main_window_position_anchor(base_input);
    let anchor_point = POINT {
        x: anchor.x,
        y: anchor.y,
    };
    let (win_w, win_h) = main_window_size_for_point(anchor_point);
    let work = platform_monitor::nearest_work_rect_for_point(anchor_point);
    let monitor = platform_monitor::nearest_rect_for_point(anchor_point);
    let plan = main_window_position_plan(MainWindowPositionInput {
        win_w,
        win_h,
        bounds: UiRect::new(
            std::cmp::max(work.left, monitor.left),
            std::cmp::max(work.top, monitor.top),
            std::cmp::min(work.right, monitor.right),
            std::cmp::min(work.bottom, monitor.bottom),
        ),
        ..base_input
    });
    (plan.x, plan.y, plan.width, plan.height)
}

unsafe fn window_class_name(hwnd: HWND) -> String {
    platform_window::class_name(hwnd)
}

unsafe fn explorer_rename_target() -> Option<(HWND, HWND)> {
    let (fg, focus) = foreground_focus_snapshot()?;
    let fg_class = window_class_name(fg);
    if !matches!(
        fg_class.as_str(),
        "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW"
    ) {
        return None;
    }
    explorer_rename_edit_from_focus_or_caret(fg, focus).map(|edit| (fg, edit))
}

unsafe fn explorer_rename_edit_from_focus_or_caret(fg: HWND, focus: HWND) -> Option<HWND> {
    if !focus.is_null() && matches!(window_class_name(focus).as_str(), "Edit") {
        return Some(focus);
    }

    let thread_id = platform_window::window_thread_id(fg);
    if thread_id == 0 {
        return None;
    }

    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if !platform_window::gui_thread_info(thread_id, &mut info) {
        return None;
    }

    for candidate in [info.hwndFocus, info.hwndCaret] {
        if candidate.is_null() || platform_window::root_ancestor(candidate) != fg {
            continue;
        }
        if matches!(window_class_name(candidate).as_str(), "Edit") {
            return Some(candidate);
        }
    }
    None
}

unsafe fn foreground_focus_snapshot() -> Option<(HWND, HWND)> {
    let fg = platform_window::foreground();
    if fg.is_null() {
        return None;
    }

    let thread_id = platform_window::window_thread_id(fg);
    if thread_id == 0 {
        return Some((fg, null_mut()));
    }

    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if !platform_window::gui_thread_info(thread_id, &mut info) {
        return Some((fg, null_mut()));
    }

    let focus = if !info.hwndFocus.is_null() {
        info.hwndFocus
    } else {
        info.hwndCaret
    };
    if !focus.is_null() && platform_window::root_ancestor(focus) == fg {
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
    let (capture_enabled, lan_enabled) = {
        let ptr = get_state_ptr(hwnd);
        if ptr.is_null() {
            (true, false)
        } else {
            (
                (*ptr).settings.clipboard_capture_enabled,
                (*ptr).settings.lan_sync_enabled,
            )
        }
    };
    let entries = windows_status_menu_entries_from_native_specs(capture_enabled, lan_enabled);

    let mut host = WindowsStatusItemHost::new(hwnd, TRAY_UID, WM_TRAYICON, 0);
    host.present_menu(&entries);
}

pub(crate) unsafe fn add_tray_icon_localized(hwnd: HWND, icon: isize) -> bool {
    let mut host = WindowsStatusItemHost::new(hwnd, TRAY_UID, WM_TRAYICON, icon);
    host.install(app_title())
}

unsafe fn position_main_window_for_state(hwnd: HWND, state: &AppState, by_hotkey: bool) {
    let pt = platform_input::cursor_pos().unwrap_or_else(|| zeroed());
    if state.settings.edge_auto_hide {
        if let Some((restore_x, restore_y)) = edge_preferred_position(state) {
            position_window_at_restore(hwnd, restore_x, restore_y);
            return;
        }
        let work = platform_monitor::nearest_work_rect_for_point(pt);
        let (win_w, win_h) = main_window_size_for_point(pt);
        let x = work.left + ((work.right - work.left - win_w) / 2);
        let y = work.top + ((work.bottom - work.top - win_h) / 3);
        crate::app::set_main_window_bounds(hwnd, UiRect::new(x, y, x + win_w, y + win_h));
        return;
    }
    let (x, y, win_w, win_h) = resolve_main_window_position(&state.settings, by_hotkey, pt);
    crate::app::set_main_window_bounds(hwnd, UiRect::new(x, y, x + win_w, y + win_h));
}

pub(crate) unsafe fn position_main_window(hwnd: HWND, settings: &AppSettings, by_hotkey: bool) {
    let pt = platform_input::cursor_pos().unwrap_or_else(|| zeroed());
    let (x, y, win_w, win_h) = resolve_main_window_position(settings, by_hotkey, pt);
    crate::app::set_main_window_bounds(hwnd, UiRect::new(x, y, x + win_w, y + win_h));
}

unsafe fn clear_hotkey_passthrough_state_fields(state: &mut AppState) {
    state.hotkey_passthrough_active = false;
    state.hotkey_passthrough_target = null_mut();
    state.hotkey_passthrough_focus = null_mut();
    state.hotkey_passthrough_edit = null_mut();
}

unsafe fn apply_show_window_state_plan(
    hwnd: HWND,
    state: &mut AppState,
    plan: MainShowWindowStatePlan,
    foreground_snapshot: Option<(HWND, HWND)>,
) {
    if plan.reset_edge_hidden_state {
        state.edge_hidden = false;
        state.edge_hide_pending_until = None;
        state.edge_restore_wait_leave = false;
        state.edge_anim_until = None;
    }
    match plan.dock_action {
        MainShowWindowDockAction::NoteMovedForEdgeHide => {
            crate::app::note_window_moved_for_edge_hide(hwnd, state);
        }
        MainShowWindowDockAction::ClearEdgeDockState => {
            crate::app::clear_edge_dock_state(state);
        }
    }
    match plan.hotkey_passthrough {
        MainHotkeyPassthroughPlan::Clear => clear_hotkey_passthrough_state_fields(state),
        MainHotkeyPassthroughPlan::UseForegroundSnapshot => {
            if let Some((target, focus)) = foreground_snapshot {
                state.hotkey_passthrough_active = true;
                state.hotkey_passthrough_target = target;
                state.hotkey_passthrough_focus = focus;
                state.hotkey_passthrough_edit = explorer_rename_target()
                    .map(|(_, edit)| edit)
                    .unwrap_or(null_mut());
            } else {
                clear_hotkey_passthrough_state_fields(state);
            }
        }
    }
    state.plain_text_paste_mode = plan.plain_text_paste_mode;
}

unsafe fn apply_show_prepare_plan(hwnd: HWND, state: &mut AppState, plan: MainShowPreparePlan) {
    if plan.clear_selection {
        state.clear_selection();
    }
    if plan.reset_scroll {
        state.scroll_y = 0;
    }
    if plan.refilter {
        state.refilter();
    }
    match plan.search_action {
        MainShowSearchAction::ShowPersistent => {
            state.search_on = true;
            crate::app::layout_children(hwnd);
            platform_gdi::invalidate_rect(hwnd, null(), 0);
        }
        MainShowSearchAction::Reset => {
            crate::app::reset_search_ui_state(state);
        }
    }
}

unsafe fn prepare_main_window_for_show(hwnd: HWND, state: &mut AppState) {
    let shared_tab_changed = apply_shared_tab_view_state(state);
    let plan = main_show_prepare_plan(MainShowPrepareInput {
        shared_tab_changed,
        persistent_search_box: state.settings.persistent_search_box,
    });
    apply_show_prepare_plan(hwnd, state, plan);
}

pub(crate) unsafe fn show_main_window(hwnd: HWND, by_hotkey: bool) {
    let pst = crate::app::get_state_ptr(hwnd);
    if !pst.is_null() {
        let state = &mut *pst;
        crate::app::refresh_window_for_show(hwnd);
        prepare_main_window_for_show(hwnd, state);
        position_main_window_for_state(hwnd, state, by_hotkey);
        crate::app::refresh_main_window_layout_for_monitor(hwnd, state, None);
        let plan = main_show_window_state_plan(MainShowWindowStateInput {
            kind: MainShowWindowKind::Main,
            by_hotkey,
            edge_auto_hide: state.settings.edge_auto_hide,
            foreground_snapshot_available: false,
            plain_text_paste_mode: false,
        });
        apply_show_window_state_plan(hwnd, state, plan, None);
    }
    crate::app::set_main_window_activation_policy(hwnd, true);
    crate::app::present_main_window(hwnd, NativeMainWindowPresentMode::ActivateAndFocus);
    crate::app::refresh_low_level_input_hooks();
}

pub(crate) unsafe fn show_quick_window(by_hotkey: bool, plain_text_mode: bool) {
    let hwnd = crate::app::quick_window_hwnd();
    if hwnd.is_null() {
        return;
    }
    crate::app::refresh_window_for_show(hwnd);
    let foreground_snapshot = by_hotkey.then(|| foreground_focus_snapshot()).flatten();
    let pst = crate::app::get_state_ptr(hwnd);
    if !pst.is_null() {
        let state = &mut *pst;
        prepare_main_window_for_show(hwnd, state);
        position_main_window_for_state(hwnd, state, by_hotkey);
        crate::app::refresh_main_window_layout_for_monitor(hwnd, state, None);
        let plan = main_show_window_state_plan(MainShowWindowStateInput {
            kind: MainShowWindowKind::Quick,
            by_hotkey,
            edge_auto_hide: state.settings.edge_auto_hide,
            foreground_snapshot_available: foreground_snapshot.is_some(),
            plain_text_paste_mode: plain_text_mode,
        });
        apply_show_window_state_plan(hwnd, state, plan, foreground_snapshot);
    }
    crate::app::set_main_window_activation_policy(hwnd, false);
    crate::app::present_main_window(hwnd, NativeMainWindowPresentMode::NoActivate);
    crate::app::refresh_low_level_input_hooks();
}

unsafe fn window_edge_hidden(hwnd: HWND) -> bool {
    let ptr = crate::app::get_state_ptr(hwnd);
    !ptr.is_null() && (*ptr).edge_hidden
}

unsafe fn clear_hotkey_passthrough_for_window(hwnd: HWND) {
    let pst = crate::app::get_state_ptr(hwnd);
    if pst.is_null() {
        return;
    }
    clear_hotkey_passthrough_state_fields(&mut *pst);
}

pub(crate) unsafe fn toggle_window_visibility(hwnd: HWND) {
    let quick = crate::app::quick_window_hwnd();
    let plan = main_window_toggle_visibility_plan(MainWindowVisibilityInput {
        main_visible: platform_window::is_visible(hwnd),
        quick_visible: platform_window::is_visible(quick),
        main_edge_hidden: window_edge_hidden(hwnd),
        quick_edge_hidden: window_edge_hidden(quick),
    });
    for step in plan.steps {
        match step {
            MainWindowVisibilityStep::TryRestoreQuickEdge => {
                if crate::app::try_restore_edge_hidden_window(quick) {
                    crate::app::refresh_low_level_input_hooks();
                    return;
                }
            }
            MainWindowVisibilityStep::HideQuick => {
                crate::app::set_main_window_activation_policy(quick, true);
                crate::app::hide_main_window(quick);
            }
            MainWindowVisibilityStep::TryRestoreMainEdge => {
                if crate::app::try_restore_edge_hidden_window(hwnd) {
                    crate::app::refresh_low_level_input_hooks();
                    return;
                }
            }
            MainWindowVisibilityStep::HideMain => {
                clear_hotkey_passthrough_for_window(hwnd);
                crate::app::set_main_window_activation_policy(hwnd, true);
                crate::app::hide_main_window(hwnd);
                crate::app::refresh_low_level_input_hooks();
            }
            MainWindowVisibilityStep::ShowMain => show_main_window(hwnd, false),
        }
    }
}

pub(crate) unsafe fn remember_window_pos(hwnd: HWND) {
    let pst = crate::app::get_state_ptr(hwnd);
    if pst.is_null() || platform_window::is_minimized(hwnd) {
        return;
    }
    if let Some(rc) = platform_window::window_rect(hwnd) {
        let anchor = main_remember_window_position(MainRememberWindowPositionInput {
            edge_auto_hide: (*pst).settings.edge_auto_hide,
            edge_hidden: (*pst).edge_hidden,
            edge_restore_x: (*pst).edge_restore_x,
            edge_restore_y: (*pst).edge_restore_y,
            window_left: rc.left,
            window_top: rc.top,
        });
        (*pst).settings.last_window_x = anchor.x;
        (*pst).settings.last_window_y = anchor.y;
    }
}

pub(crate) unsafe fn toggle_window_visibility_hotkey(hwnd: HWND) {
    let mut plain_text_mode = false;
    let main_ptr = crate::app::get_state_ptr(hwnd);
    if !main_ptr.is_null() {
        plain_text_mode = (*main_ptr).plain_text_paste_mode;
    }
    let quick = crate::app::quick_window_hwnd();
    let plan = main_window_hotkey_visibility_plan(MainWindowHotkeyVisibilityInput {
        main_visible: platform_window::is_visible(hwnd),
        quick_visible: platform_window::is_visible(quick),
        main_edge_hidden: window_edge_hidden(hwnd),
        quick_edge_hidden: window_edge_hidden(quick),
        plain_text_paste_mode: plain_text_mode,
    });
    for step in plan.steps {
        match step {
            MainWindowHotkeyVisibilityStep::TryRestoreQuickEdge => {
                if crate::app::try_restore_edge_hidden_window(quick) {
                    crate::app::refresh_low_level_input_hooks();
                    return;
                }
            }
            MainWindowHotkeyVisibilityStep::HideQuick => {
                clear_hotkey_passthrough_for_window(quick);
                let pst = crate::app::get_state_ptr(quick);
                if !pst.is_null() {
                    (*pst).plain_text_paste_mode = false;
                }
                crate::app::set_main_window_activation_policy(quick, true);
                crate::app::hide_main_window(quick);
                crate::app::refresh_low_level_input_hooks();
            }
            MainWindowHotkeyVisibilityStep::TryRestoreMainEdge => {
                if crate::app::try_restore_edge_hidden_window(hwnd) {
                    crate::app::refresh_low_level_input_hooks();
                    return;
                }
            }
            MainWindowHotkeyVisibilityStep::HideMain => {
                clear_hotkey_passthrough_for_window(hwnd);
                let pst = crate::app::get_state_ptr(hwnd);
                if !pst.is_null() {
                    (*pst).plain_text_paste_mode = false;
                }
                crate::app::set_main_window_activation_policy(hwnd, true);
                crate::app::hide_main_window(hwnd);
                crate::app::refresh_low_level_input_hooks();
            }
            MainWindowHotkeyVisibilityStep::ShowQuick {
                plain_text_paste_mode,
            } => show_quick_window(true, plain_text_paste_mode),
        }
    }
}

pub(crate) unsafe fn remove_tray_icon(hwnd: HWND) {
    let mut host = WindowsStatusItemHost::new(hwnd, TRAY_UID, WM_TRAYICON, 0);
    host.remove();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clipboard_capture_tray_label_is_an_action() {
        assert_eq!(
            tray_menu_text_label(MainTrayMenuText::DisableClipboardCapture),
            "关闭剪贴板捕捉"
        );
        assert_eq!(
            tray_menu_text_label(MainTrayMenuText::EnableClipboardCapture),
            "开启剪贴板捕捉"
        );
    }

    #[test]
    fn tray_show_paths_use_main_window_activation_policy_facade() {
        let source = include_str!("tray.rs").replace("\r\n", "\n");
        let production = source
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .unwrap_or(&source);

        assert!(production.contains("set_main_window_activation_policy(hwnd, true)"));
        assert!(production.contains("set_main_window_activation_policy(hwnd, false)"));
        assert!(production.contains("set_main_window_activation_policy(quick, true)"));
        assert!(production.contains("hide_main_window(hwnd)"));
        assert!(production.contains("hide_main_window(quick)"));
        assert!(production
            .contains("present_main_window(hwnd, NativeMainWindowPresentMode::ActivateAndFocus)"));
        assert!(production
            .contains("present_main_window(hwnd, NativeMainWindowPresentMode::NoActivate)"));
        assert!(production.contains("set_main_window_bounds(hwnd, UiRect::new("));
        assert!(!production.contains("set_main_window_noactivate_mode"));
        assert!(!production.contains("platform_window::hide("));
        assert!(!production.contains("platform_window::set_pos"));
        assert!(!production.contains("platform_window::show_no_activate"));
        assert!(!production.contains("platform_window::set_foreground"));
        assert!(!production.contains("platform_input::set_focus"));
    }
}
