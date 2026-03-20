use super::*;
use crate::win_system_ui::{
    create_settings_dropdown_button as settings_create_dropdown_btn,
    create_settings_edit as host_create_settings_edit,
    create_settings_label as host_create_settings_label,
    create_settings_label_auto as host_create_settings_label_auto,
    create_settings_listbox as host_create_settings_listbox,
    create_settings_password_edit as host_create_settings_password_edit,
    create_settings_small_button as settings_create_small_btn,
    create_settings_toggle_plain as settings_create_toggle_plain,
    draw_settings_button_component,
    draw_settings_toggle_component,
    set_settings_font as settings_set_font,
    SettingsComponentKind,
};

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

pub(super) fn set_window_host(role: WindowRole, hwnd: HWND) {
    if let Ok(mut hosts) = window_hosts().lock() {
        match role {
            WindowRole::Main => hosts.main = hwnd as isize,
            WindowRole::Quick => hosts.quick = hwnd as isize,
        }
    }
}

pub(super) fn clear_window_host(role: WindowRole, hwnd: HWND) {
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

pub(super) fn window_host_hwnds() -> [HWND; 2] {
    window_hosts()
        .lock()
        .ok()
        .map(|hosts| [hosts.main as HWND, hosts.quick as HWND])
        .unwrap_or([null_mut(), null_mut()])
}

fn window_host_hwnds_try() -> [HWND; 2] {
    window_hosts()
        .try_lock()
        .ok()
        .map(|hosts| [hosts.main as HWND, hosts.quick as HWND])
        .unwrap_or([null_mut(), null_mut()])
}

pub(super) unsafe fn set_ignore_clipboard_for_all_hosts(duration_ms: u64) {
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

pub(super) fn is_app_window(hwnd: HWND) -> bool {
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

unsafe fn window_class_name(hwnd: HWND) -> String {
    if hwnd.is_null() {
        return String::new();
    }
    let mut class_buf = [0u16; 64];
    let class_len = GetClassNameW(hwnd, class_buf.as_mut_ptr(), class_buf.len() as i32);
    if class_len > 0 {
        String::from_utf16_lossy(&class_buf[..class_len as usize])
    } else {
        String::new()
    }
}

unsafe fn screen_point_hits_popup_menu(pt: POINT) -> bool {
    let hwnd = WindowFromPoint(pt);
    if hwnd.is_null() {
        return false;
    }
    let root = GetAncestor(hwnd, GA_ROOT);
    let target = if root.is_null() { hwnd } else { root };
    window_class_name(target) == "#32768"
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

unsafe fn any_visible_window_requires_outside_hide_try() -> bool {
    for hwnd in window_host_hwnds_try() {
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

unsafe fn any_visible_window_requires_quick_escape_try() -> bool {
    for hwnd in window_host_hwnds_try() {
        if hwnd.is_null() || IsWindowVisible(hwnd) == 0 {
            continue;
        }
        let ptr = get_state_ptr(hwnd);
        if ptr.is_null() {
            continue;
        }
        let state = &*ptr;
        if state.role == WindowRole::Quick || state.main_window_noactivate {
            return true;
        }
    }
    false
}

unsafe fn should_ignore_outside_click_for_point(pt: POINT) -> bool {
    if screen_point_hits_popup_menu(pt) {
        return true;
    }
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
                let st_ptr =
                    GetWindowLongPtrW((*ptr).settings_hwnd, GWLP_USERDATA) as *mut SettingsWndState;
                if !st_ptr.is_null()
                    && screen_point_hits_window_scope((*st_ptr).dropdown_popup, pt)
                {
                    return true;
                }
            }
        }
    }
    let popup = current_vv_popup_hwnd();
    screen_point_hits_window_scope(popup, pt)
}

unsafe extern "system" fn outside_hide_mouse_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let popup_menu_active = vv_hook_state()
        .try_lock()
        .ok()
        .map(|guard| {
            guard.popup_menu_active
                || guard
                    .popup_menu_grace_until
                    .map(|until| until > Instant::now())
                    .unwrap_or(false)
        })
        .unwrap_or(false);
    if popup_menu_active {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    if code >= 0
        && matches!(wparam as u32, WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN)
        && any_visible_window_requires_outside_hide_try()
    {
        let data = &*(lparam as *const MSLLHOOKSTRUCT);
        let pt = data.pt;
        if !should_ignore_outside_click_for_point(pt) {
            for hwnd in window_host_hwnds_try() {
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

pub(super) unsafe fn ensure_outside_hide_mouse_hook() {
    let Ok(mut handle) = outside_hide_mouse_hook_handle().lock() else {
        return;
    };
    if *handle == 0 {
        *handle = SetWindowsHookExW(
            WH_MOUSE_LL,
            Some(outside_hide_mouse_hook_proc),
            GetModuleHandleW(null()),
            0,
        ) as isize;
    }
}

unsafe fn disable_outside_hide_mouse_hook() {
    let Ok(mut handle) = outside_hide_mouse_hook_handle().lock() else {
        return;
    };
    if *handle != 0 {
        UnhookWindowsHookEx(*handle as _);
        *handle = 0;
    }
}

unsafe extern "system" fn quick_escape_keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if code < 0 || (wparam as u32 != WM_KEYDOWN && wparam as u32 != WM_SYSKEYDOWN) {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    let data = &*(lparam as *const KBDLLHOOKSTRUCT);
    if (data.flags & LLKHF_INJECTED_FLAG) != 0 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    let [main, quick] = window_host_hwnds_try();

    let ctrl_down = (GetAsyncKeyState(VK_CONTROL as i32) as u16 & 0x8000) != 0;
    if data.vkCode == 0x46 && ctrl_down {
        if !quick.is_null() && IsWindowVisible(quick) != 0 {
            let _ = PostMessageW(quick, WM_KEYDOWN, 0x46usize, 0);
            return 1;
        }
        if !main.is_null() && IsWindowVisible(main) != 0 {
            let ptr = get_state_ptr(main);
            if !ptr.is_null() && (*ptr).main_window_noactivate {
                let _ = PostMessageW(main, WM_KEYDOWN, 0x46usize, 0);
                return 1;
            }
        }
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }

    if data.vkCode != VK_ESCAPE as u32 {
        return CallNextHookEx(null_mut(), code, wparam, lparam);
    }
    if !quick.is_null() && IsWindowVisible(quick) != 0 {
        let _ = PostMessageW(quick, WM_KEYDOWN, VK_ESCAPE as usize, 0);
        return 1;
    }

    if !main.is_null() && IsWindowVisible(main) != 0 {
        let ptr = get_state_ptr(main);
        if !ptr.is_null() && (*ptr).main_window_noactivate {
            let _ = PostMessageW(main, WM_KEYDOWN, VK_ESCAPE as usize, 0);
            return 1;
        }
    }

    CallNextHookEx(null_mut(), code, wparam, lparam)
}

pub(super) unsafe fn ensure_quick_escape_keyboard_hook() {
    let Ok(mut handle) = quick_escape_keyboard_hook_handle().lock() else {
        return;
    };
    if *handle == 0 {
        *handle = SetWindowsHookExW(
            WH_KEYBOARD_LL,
            Some(quick_escape_keyboard_hook_proc),
            GetModuleHandleW(null()),
            0,
        ) as isize;
    }
}

unsafe fn disable_quick_escape_keyboard_hook() {
    let Ok(mut handle) = quick_escape_keyboard_hook_handle().lock() else {
        return;
    };
    if *handle != 0 {
        UnhookWindowsHookEx(*handle as _);
        *handle = 0;
    }
}

pub(crate) unsafe fn refresh_low_level_input_hooks() {
    if any_visible_window_requires_outside_hide() {
        ensure_outside_hide_mouse_hook();
    } else {
        disable_outside_hide_mouse_hook();
    }

    if any_visible_window_requires_quick_escape_try() {
        ensure_quick_escape_keyboard_hook();
    } else {
        disable_quick_escape_keyboard_hook();
    }
}

pub(crate) unsafe fn shutdown_low_level_input_hooks() {
    disable_outside_hide_mouse_hook();
    disable_quick_escape_keyboard_hook();
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

pub(crate) unsafe fn get_state_ptr(hwnd: HWND) -> *mut AppState {
    GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut AppState
}

unsafe extern "system" fn enum_visible_windows(hwnd: HWND, lparam: LPARAM) -> i32 {
    let list = &mut *(lparam as *mut Vec<HWND>);
    if hwnd.is_null() || IsWindowVisible(hwnd) == 0 || !is_window_enabled_compat(hwnd) || IsIconic(hwnd) != 0 {
        return 1;
    }
    list.push(hwnd);
    1
}

pub(super) unsafe fn is_viable_paste_window(hwnd: HWND, app_hwnd: HWND) -> bool {
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

pub(super) unsafe fn find_next_paste_target(app_hwnd: HWND) -> HWND {
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

fn clear_edge_dock_state(state: &mut AppState) {
    state.edge_hidden = false;
    state.edge_hidden_side = EDGE_AUTO_HIDE_NONE;
    state.edge_docked_left = 0;
    state.edge_docked_top = 0;
    state.edge_docked_right = 0;
    state.edge_docked_bottom = 0;
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

pub(super) unsafe fn restore_edge_hidden_window(hwnd: HWND, state: &mut AppState) {
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

pub(super) unsafe fn hide_edge_docked_window(hwnd: HWND, state: &mut AppState) {
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

pub(super) unsafe fn handle_edge_auto_hide_tick(hwnd: HWND) {
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

pub(super) unsafe fn ensure_mouse_leave_tracking(hwnd: HWND) {
    let mut tme = TRACKMOUSEEVENT {
        cb_size: core::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dw_flags: TME_LEAVE | TME_HOVER,
        hwnd_track: hwnd,
        dw_hover_time: system_mouse_hover_time_ms(),
    };
    TrackMouseEvent(&mut tme);
}

pub(super) unsafe fn hover_preview_blocked_at_point(state: &AppState, x: i32, y: i32) -> bool {
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

pub(super) unsafe fn handle_mouse_hover_main(hwnd: HWND, lparam: LPARAM) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &*ptr;
    refresh_hover_preview(hwnd, state, get_x_lparam(lparam), get_y_lparam(lparam));
}

pub(super) unsafe fn handle_mouse_leave_main(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let mut dirty = false;
    if !state.hover_btn.is_empty() {
        state.hover_btn = "";
        dirty = true;
    }
    if state.hover_tab != -1 {
        state.hover_tab = -1;
        dirty = true;
    }
    if state.hover_idx != -1 {
        state.hover_idx = -1;
        dirty = true;
    }
    if state.hover_scroll {
        state.hover_scroll = false;
        dirty = true;
    }
    if state.hover_to_top {
        state.hover_to_top = false;
        dirty = true;
    }
    hide_hover_preview();
    if dirty {
        InvalidateRect(hwnd, null(), 0);
    }
    hide_edge_docked_window(hwnd, state);
}

pub(super) unsafe fn handle_outside_click_hide(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    if !(*ptr).settings.auto_hide_on_blur {
        return;
    }
    if vv_popup_menu_active() {
        return;
    }
    let mut pt: POINT = zeroed();
    if GetCursorPos(&mut pt) != 0 && should_ignore_outside_click_for_point(pt) {
        return;
    }
    hide_hover_preview();
    ShowWindow(hwnd, SW_HIDE);
    refresh_low_level_input_hooks();
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
        refresh_low_level_input_hooks();
        return;
    }
    SetWindowLongW(hwnd, GWL_EXSTYLE, desired as i32);
    let flags =
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER | SWP_FRAMECHANGED | if enable { SWP_NOACTIVATE } else { 0 };
    SetWindowPos(hwnd, null_mut(), 0, 0, 0, 0, flags);
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        (*ptr).main_window_noactivate = enable;
    }
    refresh_low_level_input_hooks();
}

pub(super) unsafe fn get_state_mut(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}

pub(super) unsafe fn clear_main_hover_state(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let mut dirty = false;
    if !state.hover_btn.is_empty() {
        state.hover_btn = "";
        dirty = true;
    }
    if state.hover_tab != -1 {
        state.hover_tab = -1;
        dirty = true;
    }
    if state.hover_idx != -1 {
        state.hover_idx = -1;
        dirty = true;
    }
    if state.hover_to_top {
        state.hover_to_top = false;
        dirty = true;
    }
    if state.down_to_top {
        state.down_to_top = false;
        dirty = true;
    }
    if state.down_row != -1 {
        state.down_row = -1;
        state.down_x = 0;
        state.down_y = 0;
        dirty = true;
    }
    hide_hover_preview();
    if dirty {
        InvalidateRect(hwnd, null(), 0);
    }
}

pub(super) unsafe fn main_window_should_stay_noactivate(state: &AppState, x: i32, y: i32) -> bool {
    hit_test_row(state, x, y) >= 0
}

pub(super) fn trim_process_working_set() {
    unsafe {
        let process = GetCurrentProcess();
        if !process.is_null() {
            let _ = EmptyWorkingSet(process);
        }
    }
}

pub(super) fn taskbar_created_message() -> u32 {
    *TASKBAR_CREATED_MESSAGE.get_or_init(|| unsafe {
        RegisterWindowMessageW(to_wide("TaskbarCreated").as_ptr())
    })
}

pub(super) unsafe fn sync_main_tray_icon(hwnd: HWND, state: &mut AppState) {
    remove_tray_icon(hwnd);
    state.tray_icon_registered = false;
    if tray_mode_enabled(&state.settings) && state.icons.app != 0 {
        state.tray_icon_registered = add_tray_icon_localized(hwnd, state.icons.app);
    }
}

pub(super) unsafe fn retry_startup_integrations(hwnd: HWND, state: &mut AppState) {
    if state.role != WindowRole::Main || state.startup_recovery_ticks == 0 {
        return;
    }

    if tray_mode_enabled(&state.settings) && state.icons.app != 0 && !state.tray_icon_registered {
        sync_main_tray_icon(hwnd, state);
    }

    if state.settings.hotkey_enabled && !state.hotkey_registered {
        register_hotkey_for(hwnd, state);
    }

    let tray_ready =
        !tray_mode_enabled(&state.settings) || state.icons.app == 0 || state.tray_icon_registered;
    let hotkey_ready = !state.settings.hotkey_enabled || state.hotkey_registered;
    if tray_ready && hotkey_ready {
        state.startup_recovery_ticks = 0;
    } else {
        state.startup_recovery_ticks = state.startup_recovery_ticks.saturating_sub(1);
    }
}

pub(super) unsafe fn notify_update_state_changed() {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() {
            continue;
        }
        let _ = PostMessageW(hwnd, WM_UPDATE_CHECK_READY, 0, 0);
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null()
            && !(*ptr).settings_hwnd.is_null()
            && IsWindow((*ptr).settings_hwnd) != 0
        {
            InvalidateRect((*ptr).settings_hwnd, null(), 1);
        }
    }
}

pub(super) unsafe fn refresh_settings_window_from_app(app: &mut AppState) {
    if app.settings_hwnd.is_null() || IsWindow(app.settings_hwnd) == 0 {
        return;
    }
    let st_ptr = GetWindowLongPtrW(app.settings_hwnd, GWLP_USERDATA) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        settings_apply_from_app(&mut *st_ptr);
        InvalidateRect(app.settings_hwnd, null(), 1);
    }
}

pub(super) unsafe fn settings_set_text(hwnd: HWND, s: &str) {
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

pub(super) fn settings_groups_cache_for_tab(st: &SettingsWndState, tab: usize) -> &Vec<ClipGroup> {
    if normalize_source_tab(tab) == 0 {
        &st.record_groups_cache
    } else {
        &st.phrase_groups_cache
    }
}

pub(super) fn settings_groups_cache_for_tab_mut(
    st: &mut SettingsWndState,
    tab: usize,
) -> &mut Vec<ClipGroup> {
    if normalize_source_tab(tab) == 0 {
        &mut st.record_groups_cache
    } else {
        &mut st.phrase_groups_cache
    }
}

pub(super) unsafe fn settings_group_current_filter_text(st: &SettingsWndState) -> String {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() {
        return tr("全部记录", "All Records").to_string();
    }
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

pub(super) unsafe fn settings_sync_vv_source_display(st: &mut SettingsWndState) {
    st.vv_source_selected = normalize_source_tab(st.vv_source_selected);
    if !st.cb_vv_source.is_null() {
        settings_set_text(st.cb_vv_source, source_tab_label(st.vv_source_selected));
    }
}

pub(super) unsafe fn settings_sync_vv_group_display(st: &mut SettingsWndState) {
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

pub(super) unsafe fn settings_sync_group_view_tabs(st: &SettingsWndState) {
    if !st.btn_group_view_records.is_null() {
        InvalidateRect(st.btn_group_view_records, null(), 1);
    }
    if !st.btn_group_view_phrases.is_null() {
        InvalidateRect(st.btn_group_view_phrases, null(), 1);
    }
}

pub(super) unsafe fn settings_sync_group_overview(st: &mut SettingsWndState) {
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

pub(super) fn settings_vv_source_current(st: &SettingsWndState) -> usize {
    normalize_source_tab(st.vv_source_selected)
}

pub(super) fn settings_group_view_current(st: &SettingsWndState) -> usize {
    normalize_source_tab(st.group_view_tab)
}

pub(super) unsafe fn settings_vv_source_from_app(st: &mut SettingsWndState) {
    st.vv_source_selected = normalize_source_tab(st.draft.vv_source_tab);
}

pub(super) unsafe fn settings_group_view_from_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    st.group_view_tab = if pst.is_null() {
        0
    } else {
        normalize_source_tab((&*pst).tab_index)
    };
}

pub(super) unsafe fn settings_sync_group_page(st: &mut SettingsWndState) {
    st.record_groups_cache = db_load_groups(0);
    st.phrase_groups_cache = db_load_groups(1);
    settings_vv_source_from_app(st);
    settings_sync_vv_source_display(st);
    st.vv_group_selected = st.draft.vv_group_id;
    settings_sync_vv_group_display(st);
    settings_group_view_from_app(st);
    settings_sync_group_overview(st);
}

pub(super) unsafe fn settings_invalidate_page_ctrls(
    hwnd: HWND,
    st: &SettingsWndState,
    page: usize,
) {
    for reg in st.ui.page_regs(page) {
        if !reg.hwnd.is_null() {
            InvalidateRect(reg.hwnd, null(), 1);
        }
    }
    let mut rc: RECT = core::mem::zeroed();
    if GetClientRect(hwnd, &mut rc) != 0 {
        let viewport = settings_viewport_rect(&rc);
        InvalidateRect(hwnd, &viewport, 0);
    }
}

pub(super) unsafe fn settings_sync_page_state(st: &mut SettingsWndState, page: usize) {
    match SettingsPage::from_index(page) {
        SettingsPage::General => settings_sync_pos_fields_enabled(st),
        SettingsPage::Hotkey => {
            let s = &st.draft;
            settings_set_text(st.cb_hk_mod, &normalize_hotkey_mod(&s.hotkey_mod));
            settings_set_text(st.cb_hk_key, &normalize_hotkey_key(&s.hotkey_key));
            settings_set_text(st.lb_hk_preview, &hotkey_preview_text(&s.hotkey_mod, &s.hotkey_key));
            if !st.btn_hk_record.is_null() {
            settings_set_text(
                st.btn_hk_record,
                if st.hotkey_recording {
                    tr("按下快捷键...", "Press shortcut...")
                } else {
                    tr("录制热键", "Record Hotkey")
                },
            );
        }
        }
        SettingsPage::Plugin => {
            let s = &st.draft;
            settings_set_text(st.cb_engine, &search_engine_display(&s.search_engine));
            settings_set_text(st.ed_tpl, &s.search_template);
        }
        SettingsPage::Group => settings_sync_group_page(st),
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
    if trimmed.is_empty() || trimmed == "未同步" {
        return tr("未同步", "Not synced").to_string();
    }
    if let Some(rest) = trimmed.strip_prefix("失败：") {
        return format!("{}{}", tr("失败：", "Failed: "), rest);
    }
    translate(trimmed).into_owned()
}

pub(super) unsafe fn settings_refresh_theme_resources(st: &mut SettingsWndState) {
    if !st.bg_brush.is_null() {
        DeleteObject(st.bg_brush as _);
    }
    if !st.surface_brush.is_null() {
        DeleteObject(st.surface_brush as _);
    }
    if !st.control_brush.is_null() {
        DeleteObject(st.control_brush as _);
    }
    if !st.nav_brush.is_null() {
        DeleteObject(st.nav_brush as _);
    }
    let th = Theme::default();
    st.bg_brush = CreateSolidBrush(th.bg) as _;
    st.surface_brush = CreateSolidBrush(th.surface) as _;
    st.control_brush = CreateSolidBrush(th.control_bg) as _;
    st.nav_brush = CreateSolidBrush(th.nav_bg) as _;
}

pub(super) unsafe fn settings_register_ctrl(
    st: &mut SettingsWndState,
    page: usize,
    hwnd: HWND,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    scrollable: bool,
) {
    if hwnd.is_null() {
        return;
    }
    st.ui
        .register(SettingsCtrlReg::new(hwnd, page, x, y, w, h, scrollable));
}

pub(super) unsafe fn settings_page_push_ctrl(
    st: &mut SettingsWndState,
    page: usize,
    hwnd: HWND,
) {
    settings_register_ctrl(st, page, hwnd, 0, 0, 0, 0, false);
}

pub(super) unsafe fn settings_page0_push_ctrl(
    st: &mut SettingsWndState,
    hwnd: HWND,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    settings_register_ctrl(st, 0, hwnd, x, y, w, h, true);
}

pub(super) unsafe fn settings_repos_controls(hwnd: HWND, st: &SettingsWndState, redraw_children: bool) {
    if st.ui.scroll_ctrls().is_empty() || st.cur_page != SettingsPage::General.index() {
        return;
    }

    let mut crc: RECT = core::mem::zeroed();
    GetClientRect(hwnd, &mut crc);
    let viewport = settings_viewport_rect(&crc);
    let mut dirty: Vec<RECT> = Vec::with_capacity(st.ui.scroll_ctrls().len() * 2);

    let hdwp = BeginDeferWindowPos(st.ui.scroll_ctrls().len() as i32);
    if hdwp.is_null() {
        return;
    }
    let mut hdwp = hdwp;
    for slot in st.ui.scroll_ctrls() {
        let hchild = slot.hwnd;
        let ox = slot.bounds.left;
        let oy = slot.bounds.top;
        let ow = slot.bounds.right - slot.bounds.left;
        let oh = slot.bounds.bottom - slot.bounds.top;
        if hchild.is_null() {
            continue;
        }

        let mut wr: RECT = core::mem::zeroed();
        if GetWindowRect(hchild, &mut wr) != 0 {
            let mut tl = POINT {
                x: wr.left,
                y: wr.top,
            };
            let mut br = POINT {
                x: wr.right,
                y: wr.bottom,
            };
            ScreenToClient(hwnd, &mut tl);
            ScreenToClient(hwnd, &mut br);
            dirty.push(RECT {
                left: tl.x,
                top: tl.y,
                right: br.x,
                bottom: br.y,
            });
        }

        let new_y = oy - st.content_scroll_y;
        let visible = settings_child_visible(new_y, oh, &viewport);
        dirty.push(RECT {
            left: ox,
            top: new_y,
            right: ox + ow,
            bottom: new_y + oh,
        });

        let flags = SWP_NOZORDER
            | SWP_NOACTIVATE
            | if visible { SWP_SHOWWINDOW } else { SWP_HIDEWINDOW };
        let r = DeferWindowPos(hdwp, hchild, null_mut(), ox, new_y, ow, oh, flags);
        if !r.is_null() {
            hdwp = r;
        }
    }
    EndDeferWindowPos(hdwp);

    if redraw_children {
        for slot in st.ui.scroll_ctrls() {
            let hchild = slot.hwnd;
            let oy = slot.bounds.top;
            let oh = slot.bounds.bottom - slot.bounds.top;
            if hchild.is_null() {
                continue;
            }
            let new_y = oy - st.content_scroll_y;
            if settings_child_visible(new_y, oh, &viewport) {
                InvalidateRect(hchild, null(), 0);
            }
        }
    }

    for mut rc in dirty {
        if rc.right <= rc.left || rc.bottom <= rc.top {
            continue;
        }
        if rc.left < viewport.left {
            rc.left = viewport.left;
        }
        if rc.top < viewport.top {
            rc.top = viewport.top;
        }
        if rc.right > viewport.right {
            rc.right = viewport.right;
        }
        if rc.bottom > viewport.bottom {
            rc.bottom = viewport.bottom;
        }
        if rc.right > rc.left && rc.bottom > rc.top {
            InvalidateRect(hwnd, &rc, 0);
        }
    }
}

pub(super) unsafe fn settings_scroll_to(hwnd: HWND, st: &mut SettingsWndState, new_y: i32) {
    let mut crc: RECT = core::mem::zeroed();
    GetClientRect(hwnd, &mut crc);
    let view_h = (crc.bottom - crc.top) - SETTINGS_CONTENT_Y;
    let new_y = new_y.clamp(0, settings_page_max_scroll(st.cur_page, view_h));
    if new_y == st.content_scroll_y {
        return;
    }
    let old_y = st.content_scroll_y;
    st.content_scroll_y = new_y;
    settings_scrollbar_show(hwnd, st);

    let viewport = settings_viewport_rect(&crc);
    let delta_y = old_y - new_y;
    if delta_y != 0 {
        ScrollWindowEx(
            hwnd,
            0,
            delta_y,
            &viewport,
            &viewport,
            null_mut(),
            null_mut(),
            SW_INVALIDATE | SW_SCROLLCHILDREN,
        );
    }
    settings_repos_controls(hwnd, st, false);

    let mask = settings_viewport_mask_rect(&crc);
    InvalidateRect(hwnd, &mask, 0);
    let scroll_strip = RECT {
        left: crc.right - SCROLL_BAR_W_ACTIVE - SCROLL_BAR_MARGIN - 4,
        top: SETTINGS_CONTENT_Y,
        right: crc.right,
        bottom: crc.bottom,
    };
    InvalidateRect(hwnd, &scroll_strip, 0);
    RedrawWindow(
        hwnd,
        &viewport,
        null_mut(),
        RDW_INVALIDATE | RDW_UPDATENOW | RDW_ALLCHILDREN,
    );
}

pub(super) unsafe fn settings_scrollbar_show(hwnd: HWND, st: &mut SettingsWndState) {
    st.scroll_bar_visible = true;
    if st.scroll_hide_timer {
        KillTimer(hwnd, ID_TIMER_SETTINGS_SCROLLBAR);
    }
    st.scroll_hide_timer = true;
    SetTimer(hwnd, ID_TIMER_SETTINGS_SCROLLBAR, 1500, None);
}

pub(super) unsafe fn settings_scroll(hwnd: HWND, st: &mut SettingsWndState, delta: i32) {
    settings_scroll_to(hwnd, st, st.content_scroll_y + delta);
}

pub(super) unsafe fn settings_show_page(hwnd: HWND, st: &mut SettingsWndState, page: usize) {
    let page = page.min(SETTINGS_PAGES.len().saturating_sub(1));
    let old_page = st.cur_page;
    if old_page == SettingsPage::Hotkey.index() && page != old_page && st.hotkey_recording {
        st.hotkey_recording = false;
        if !st.btn_hk_record.is_null() {
            settings_set_text(st.btn_hk_record, tr("录制热键", "Record Hotkey"));
            InvalidateRect(st.btn_hk_record, null(), 1);
        }
        if !st.lb_hk_preview.is_null() {
            settings_set_text(
                st.lb_hk_preview,
                &hotkey_preview_text(&get_window_text(st.cb_hk_mod), &get_window_text(st.cb_hk_key)),
            );
            InvalidateRect(st.lb_hk_preview, null(), 1);
        }
    }
    if old_page == page && st.ui.is_built(page) {
        settings_sync_page_state(st, page);
        return;
    }

    SendMessageW(hwnd, WM_SETREDRAW, 0, 0);
    settings_ensure_page(hwnd, st, page);
    st.cur_page = page;

    for reg in st.ui.page_regs(old_page) {
        if !reg.hwnd.is_null() {
            ShowWindow(reg.hwnd, SW_HIDE);
        }
    }
    for reg in st.ui.page_regs(st.cur_page) {
        if !reg.hwnd.is_null() {
            ShowWindow(reg.hwnd, SW_SHOW);
        }
    }

    st.content_scroll_y = 0;
    st.scroll_bar_visible = false;
    if st.cur_page == SettingsPage::General.index() {
        settings_repos_controls(hwnd, st, true);
    }

    settings_sync_page_state(st, page);
    SendMessageW(hwnd, WM_SETREDRAW, 1, 0);
    let mut rc: RECT = core::mem::zeroed();
    if GetClientRect(hwnd, &mut rc) != 0 {
        let viewport = settings_viewport_rect(&rc);
        RedrawWindow(
            hwnd,
            &viewport,
            null_mut(),
            RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
        );
    } else {
        InvalidateRect(hwnd, null(), 1);
    }
}

pub(super) unsafe fn settings_apply_from_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() {
        return;
    }
    let app = &mut *pst;
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
    if st.ui.is_built(SettingsPage::Hotkey.index()) {
        settings_sync_page_state(st, SettingsPage::Hotkey.index());
    }
    if st.ui.is_built(SettingsPage::Plugin.index()) {
        settings_sync_page_state(st, SettingsPage::Plugin.index());
    }
    if st.ui.is_built(SettingsPage::Group.index()) {
        settings_sync_page_state(st, SettingsPage::Group.index());
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) {
        settings_sync_page_state(st, SettingsPage::Cloud.index());
    }
}

pub(super) unsafe fn settings_sync_pos_fields_enabled(st: &SettingsWndState) {
    let mode = settings_dropdown_pos_mode_from_label(&get_window_text(st.cb_pos));
    let is_follow = mode == "mouse";
    let is_fixed = mode == "fixed";
    if !st.ed_dx.is_null() {
        EnableWindow(st.ed_dx, if is_follow { 1 } else { 0 });
    }
    if !st.ed_dy.is_null() {
        EnableWindow(st.ed_dy, if is_follow { 1 } else { 0 });
    }
    if !st.ed_fx.is_null() {
        EnableWindow(st.ed_fx, if is_fixed { 1 } else { 0 });
    }
    if !st.ed_fy.is_null() {
        EnableWindow(st.ed_fy, if is_fixed { 1 } else { 0 });
    }
}

pub(super) unsafe fn settings_collect_to_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() {
        return;
    }
    st.draft.max_items = settings_dropdown_max_items_from_label(&get_window_text(st.cb_max));
    st.draft.show_mouse_dx = get_window_text(st.ed_dx).parse::<i32>().ok().unwrap_or(12);
    st.draft.show_mouse_dy = get_window_text(st.ed_dy).parse::<i32>().ok().unwrap_or(12);
    st.draft.show_fixed_x = get_window_text(st.ed_fx).parse::<i32>().ok().unwrap_or(120);
    st.draft.show_fixed_y = get_window_text(st.ed_fy).parse::<i32>().ok().unwrap_or(120);
    st.draft.show_pos_mode = settings_dropdown_pos_mode_from_label(&get_window_text(st.cb_pos));
    if st.ui.is_built(SettingsPage::Hotkey.index()) && !st.cb_hk_mod.is_null() && !st.cb_hk_key.is_null() {
        st.draft.hotkey_mod = normalize_hotkey_mod(&get_window_text(st.cb_hk_mod));
        st.draft.hotkey_key = normalize_hotkey_key(&get_window_text(st.cb_hk_key));
    }
    if st.ui.is_built(SettingsPage::Plugin.index()) && !st.cb_engine.is_null() {
        st.draft.search_engine = search_engine_key_from_display(&get_window_text(st.cb_engine)).to_string();
        st.draft.search_template = {
            let tpl = get_window_text(st.ed_tpl);
            if tpl.trim().is_empty() {
                search_engine_template(&st.draft.search_engine).to_string()
            } else {
                tpl
            }
        };
    }
    st.draft.vv_source_tab = settings_vv_source_current(st);
    let vv_groups = settings_groups_cache_for_tab(st, st.draft.vv_source_tab);
    st.draft.vv_group_id = if st.vv_group_selected > 0
        && vv_groups.iter().any(|g| g.id == st.vv_group_selected)
    {
        st.vv_group_selected
    } else {
        0
    };
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.cb_cloud_interval.is_null() {
        st.draft.cloud_sync_interval = {
            let label = get_window_text(st.cb_cloud_interval);
            if label.trim().is_empty() {
                "1灏忔椂".to_string()
            } else {
                label
            }
        };
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_cloud_url.is_null() {
        st.draft.cloud_webdav_url = get_window_text(st.ed_cloud_url);
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_cloud_user.is_null() {
        st.draft.cloud_webdav_user = get_window_text(st.ed_cloud_user);
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_cloud_pass.is_null() {
        st.draft.cloud_webdav_pass = get_window_text(st.ed_cloud_pass);
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_cloud_dir.is_null() {
        st.draft.cloud_remote_dir = {
            let dir = get_window_text(st.ed_cloud_dir);
            if dir.trim().is_empty() {
                "ZSClip".to_string()
            } else {
                dir
            }
        };
    }
    let app = &mut *pst;
    let grouping_old = app.settings.grouping_enabled;
    let autostart_old = app.settings.auto_start;
    let tray_icon_old = app.settings.tray_icon_enabled;
    let hotkey_old = format!(
        "{}+{}+{}",
        app.settings.hotkey_enabled, app.settings.hotkey_mod, app.settings.hotkey_key
    );
    let edge_hide_old = app.settings.edge_auto_hide;
    let vv_mode_old = app.settings.vv_mode_enabled;
    app.settings = st.draft.clone();
    if !app.settings.grouping_enabled {
        app.current_group_filter = 0;
        app.tab_group_filters = [0, 0];
    }
    save_settings(&app.settings);
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
    let hotkey_new = format!(
        "{}+{}+{}",
        app.settings.hotkey_enabled, app.settings.hotkey_mod, app.settings.hotkey_key
    );
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
    let new_max = app.settings.max_items;
    if new_max > 0 {
        db_prune_items(new_max);
        reload_state_from_db(app);
    }
    if edge_hide_old && !app.settings.edge_auto_hide {
        restore_edge_hidden_window(st.parent_hwnd, app);
    }
    refresh_low_level_input_hooks();
    app.refilter();
    sync_peer_windows_from_settings(st.parent_hwnd);
    InvalidateRect(st.parent_hwnd, null(), 1);
}

pub(super) unsafe fn settings_toggle_get(st: &SettingsWndState, cid: isize) -> bool {
    match cid {
        IDC_SET_AUTOSTART => st.draft.auto_start,
        IDC_SET_SILENTSTART => st.draft.silent_start,
        IDC_SET_TRAYICON => st.draft.tray_icon_enabled,
        IDC_SET_CLOSETRAY => st.draft.close_without_exit,
        IDC_SET_CLICK_HIDE => st.draft.click_hide,
        IDC_SET_PASTE_MOVE_TOP => st.draft.move_pasted_item_to_top,
        IDC_SET_DEDUPE_FILTER => st.draft.dedupe_filter_enabled,
        IDC_SET_AUTOHIDE_BLUR => st.draft.auto_hide_on_blur,
        IDC_SET_EDGEHIDE => st.draft.edge_auto_hide,
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

pub(super) unsafe fn settings_toggle_flip(st: &mut SettingsWndState, cid: isize) {
    match cid {
        IDC_SET_AUTOSTART => st.draft.auto_start = !st.draft.auto_start,
        IDC_SET_SILENTSTART => st.draft.silent_start = !st.draft.silent_start,
        IDC_SET_TRAYICON => st.draft.tray_icon_enabled = !st.draft.tray_icon_enabled,
        IDC_SET_CLOSETRAY => st.draft.close_without_exit = !st.draft.close_without_exit,
        IDC_SET_CLICK_HIDE => st.draft.click_hide = !st.draft.click_hide,
        IDC_SET_PASTE_MOVE_TOP => {
            st.draft.move_pasted_item_to_top = !st.draft.move_pasted_item_to_top
        }
        IDC_SET_DEDUPE_FILTER => {
            st.draft.dedupe_filter_enabled = !st.draft.dedupe_filter_enabled
        }
        IDC_SET_AUTOHIDE_BLUR => st.draft.auto_hide_on_blur = !st.draft.auto_hide_on_blur,
        IDC_SET_EDGEHIDE => st.draft.edge_auto_hide = !st.draft.edge_auto_hide,
        IDC_SET_HOVERPREVIEW => st.draft.hover_preview = !st.draft.hover_preview,
        IDC_SET_VV_MODE => st.draft.vv_mode_enabled = !st.draft.vv_mode_enabled,
        IDC_SET_IMAGE_PREVIEW => {
            st.draft.image_preview_enabled = !st.draft.image_preview_enabled
        }
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

struct SettingsPageBuilder {
    hwnd: HWND,
    page: usize,
    font: *mut core::ffi::c_void,
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

pub(super) unsafe fn settings_create_label(parent: HWND, text: &str, x: i32, y: i32, w: i32, h: i32, font: *mut core::ffi::c_void) -> HWND {
    host_create_settings_label(parent, text, x, y, w, h, font)
}

pub(super) unsafe fn settings_create_label_auto(parent: HWND, text: &str, x: i32, y: i32, w: i32, min_h: i32, font: *mut core::ffi::c_void) -> (HWND, i32) {
    host_create_settings_label_auto(parent, text, x, y, w, min_h, font)
}

pub(super) unsafe fn settings_create_toggle(parent: HWND, st: &mut SettingsWndState, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    let (label, btn, lx, ly, lw, lh, btn_x, btn_y) = settings_create_toggle_plain(parent, text, id, x, y, w, font);
    settings_page0_push_ctrl(st, label, lx, ly, lw, lh);
    settings_page0_push_ctrl(st, btn, btn_x, btn_y, 44, 24);
    if !btn.is_null() { st.ownerdraw_ctrls.push(btn); }
    btn
}

pub(super) unsafe fn settings_create_edit(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    host_create_settings_edit(parent, text, id, x, y, w, font)
}

pub(super) unsafe fn settings_create_password_edit(parent: HWND, text: &str, id: isize, x: i32, y: i32, w: i32, font: *mut core::ffi::c_void) -> HWND {
    host_create_settings_password_edit(parent, text, id, x, y, w, font)
}

pub(super) unsafe fn settings_create_general_page(hwnd: HWND, st: &mut SettingsWndState) {
    let ui_font = st.ui_font;
    let sec0 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 0, 0);
    let sec1 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 1, 130);
    let sec2 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 2, 0);
    let sec3 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 3, 138);
    let sec4 = SettingsFormSectionLayout::new(SettingsPage::General.index(), 4, 0);

    st.chk_autostart = settings_create_toggle(hwnd, st, "开机自启", IDC_SET_AUTOSTART, sec0.left(), sec0.row_y(0), sec0.full_w(), ui_font);
    st.chk_silent_start = settings_create_toggle(hwnd, st, "静默启动（打开默认不显示）", IDC_SET_SILENTSTART, sec0.left(), sec0.row_y(1), sec0.full_w(), ui_font);
    st.chk_tray_icon = settings_create_toggle(hwnd, st, "右下角图标开启/关闭", IDC_SET_TRAYICON, sec0.left(), sec0.row_y(2), sec0.full_w(), ui_font);
    st.chk_close_tray = settings_create_toggle(hwnd, st, "关闭不退出（托盘驻留）", IDC_SET_CLOSETRAY, sec0.left(), sec0.row_y(3), sec0.full_w(), ui_font);
    st.chk_auto_hide_on_blur = settings_create_toggle(hwnd, st, "呼出后点击外部自动隐藏", IDC_SET_AUTOHIDE_BLUR, sec0.left(), sec0.row_y(4), sec0.full_w(), ui_font);
    st.chk_edge_hide = settings_create_toggle(hwnd, st, "贴边自动隐藏", IDC_SET_EDGEHIDE, sec0.left(), sec0.row_y(5), sec0.full_w(), ui_font);
    st.chk_hover_preview = settings_create_toggle(hwnd, st, "悬停预览", IDC_SET_HOVERPREVIEW, sec0.left(), sec0.row_y(6), sec0.full_w(), ui_font);
    let _ = settings_create_toggle(hwnd, st, "VV 模式", IDC_SET_VV_MODE, sec0.left(), sec0.row_y(7), sec0.full_w(), ui_font);
    let _ = settings_create_toggle(hwnd, st, "显示图片记录", IDC_SET_IMAGE_PREVIEW, sec0.left(), sec0.row_y(8), sec0.full_w(), ui_font);
    let _ = settings_create_toggle(hwnd, st, "快速删除按钮", IDC_SET_QUICK_DELETE, sec0.left(), sec0.row_y(9), sec0.full_w(), ui_font);

    let lbl_max = settings_create_label(hwnd, "最大保存条数：", sec1.left(), sec1.label_y(0, 24), sec1.label_w(), 24, ui_font);
    settings_page0_push_ctrl(st, lbl_max, sec1.left(), sec1.label_y(0, 24), sec1.label_w(), 24);
    st.cb_max = settings_create_dropdown_btn(hwnd, "200", IDC_SET_MAX, sec1.field_x(), sec1.row_y(0), 150, ui_font);
    settings_page0_push_ctrl(st, st.cb_max, sec1.field_x(), sec1.row_y(0), 150, 32);
    if !st.cb_max.is_null() { st.ownerdraw_ctrls.push(st.cb_max); }

    st.chk_click_hide = settings_create_toggle(hwnd, st, "单击后隐藏主窗口", IDC_SET_CLICK_HIDE, sec2.left(), sec2.row_y(0), sec2.full_w(), ui_font);
    st.chk_move_pasted_to_top = settings_create_toggle(hwnd, st, "粘贴后上移到首行", IDC_SET_PASTE_MOVE_TOP, sec2.left(), sec2.row_y(1), sec2.full_w(), ui_font);
    let _ = settings_create_toggle(hwnd, st, "重复内容过滤并提升到首行", IDC_SET_DEDUPE_FILTER, sec2.left(), sec2.row_y(2), sec2.full_w(), ui_font);

    let lbl_pos = settings_create_label(hwnd, "弹出位置：", sec3.left(), sec3.label_y(0, 24), sec3.label_w(), 24, ui_font);
    settings_page0_push_ctrl(st, lbl_pos, sec3.left(), sec3.label_y(0, 24), sec3.label_w(), 24);
    st.cb_pos = settings_create_dropdown_btn(hwnd, "跟随鼠标", IDC_SET_POSMODE, sec3.field_x(), sec3.row_y(0), 170, ui_font);
    settings_page0_push_ctrl(st, st.cb_pos, sec3.field_x(), sec3.row_y(0), 170, 32);
    if !st.cb_pos.is_null() { st.ownerdraw_ctrls.push(st.cb_pos); }

    let lbl_mouse = settings_create_label(hwnd, "鼠标偏移 dx/dy：", sec3.left(), sec3.label_y(1, 24), sec3.label_w(), 24, ui_font);
    settings_page0_push_ctrl(st, lbl_mouse, sec3.left(), sec3.label_y(1, 24), sec3.label_w(), 24);
    let mouse_x = sec3.field_x();
    st.ed_dx = settings_create_edit(hwnd, "", IDC_SET_DX, mouse_x, sec3.row_y(1), 64, ui_font);
    st.ed_dy = settings_create_edit(hwnd, "", IDC_SET_DY, mouse_x + 74, sec3.row_y(1), 64, ui_font);
    settings_page0_push_ctrl(st, st.ed_dx, mouse_x, sec3.row_y(1), 64, 28);
    settings_page0_push_ctrl(st, st.ed_dy, mouse_x + 74, sec3.row_y(1), 64, 28);

    let lbl_fixed = settings_create_label(hwnd, "固定位置 x/y：", sec3.left(), sec3.label_y(2, 24), sec3.label_w(), 24, ui_font);
    settings_page0_push_ctrl(st, lbl_fixed, sec3.left(), sec3.label_y(2, 24), sec3.label_w(), 24);
    let fixed_x = sec3.field_x();
    st.ed_fx = settings_create_edit(hwnd, "", IDC_SET_FX, fixed_x, sec3.row_y(2), 64, ui_font);
    st.ed_fy = settings_create_edit(hwnd, "", IDC_SET_FY, fixed_x + 74, sec3.row_y(2), 64, ui_font);
    settings_page0_push_ctrl(st, st.ed_fx, fixed_x, sec3.row_y(2), 64, 28);
    settings_page0_push_ctrl(st, st.ed_fy, fixed_x + 74, sec3.row_y(2), 64, 28);

    let btn_y = sec4.row_y(0);
    st.btn_open_cfg = settings_create_small_btn(hwnd, "打开设置文件", IDC_SET_BTN_OPENCFG, sec4.action_x(0, 130), btn_y, 130, ui_font);
    st.btn_open_db = settings_create_small_btn(hwnd, "打开数据库文件", IDC_SET_BTN_OPENDB, sec4.action_x(1, 130), btn_y, 130, ui_font);
    st.btn_open_data = settings_create_small_btn(hwnd, "打开数据目录", IDC_SET_BTN_OPENDATA, sec4.action_x(2, 130), btn_y, 130, ui_font);
    settings_page0_push_ctrl(st, st.btn_open_cfg, sec4.action_x(0, 130), btn_y, 130, 32);
    settings_page0_push_ctrl(st, st.btn_open_db, sec4.action_x(1, 130), btn_y, 130, 32);
    settings_page0_push_ctrl(st, st.btn_open_data, sec4.action_x(2, 130), btn_y, 130, 32);
    for &hh in &[st.btn_open_cfg, st.btn_open_db, st.btn_open_data] {
        if !hh.is_null() { st.ownerdraw_ctrls.push(hh); }
    }
    st.ui.mark_built(SettingsPage::General.index());
}

pub(super) unsafe fn settings_create_listbox(parent: HWND, id: isize, x: i32, y: i32, w: i32, h: i32, font: *mut core::ffi::c_void) -> HWND {
    host_create_settings_listbox(parent, id, x, y, w, h, font)
}

pub(super) unsafe fn settings_groups_refresh_list(st: &mut SettingsWndState, select_gid: i64) {
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
    let item_h = SendMessageW(st.lb_groups, LB_GETITEMHEIGHT, 0, 0) as i32;
    let mut rc: RECT = core::mem::zeroed();
    GetClientRect(st.lb_groups, &mut rc);
    let view_h = (rc.bottom - rc.top).max(0);
    let needs_vscroll = item_h > 0 && (groups.len() as i32 * item_h) > view_h;
    ShowScrollBar(st.lb_groups, SB_VERT, if needs_vscroll { 1 } else { 0 });
    ShowScrollBar(st.lb_groups, SB_HORZ, 0);
    settings_sync_vv_group_display(st);
}

pub(super) unsafe fn settings_groups_selected(st: &SettingsWndState) -> Option<(usize, ClipGroup)> {
    if st.lb_groups.is_null() { return None; }
    let row = SendMessageW(st.lb_groups, LB_GETCURSEL, 0, 0) as i32;
    if row < 0 { return None; }
    settings_groups_cache_for_tab(st, settings_group_view_current(st))
        .get(row as usize)
        .cloned()
        .map(|g| (row as usize, g))
}

pub(super) unsafe fn settings_groups_sync_name(_st: &mut SettingsWndState) {}

pub(super) unsafe fn settings_groups_move(st: &mut SettingsWndState, step: i32) {
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

pub(super) unsafe fn settings_create_hotkey_page(hwnd: HWND, st: &mut SettingsWndState) {
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
    st.lb_hk_preview = b.label(st, "当前设置：Win + V", sec0.left(), sec0.label_y(2, 24), sec0.full_w() - 124, 24);
    st.btn_hk_record = b.button(st, "录制热键", IDC_SET_HK_RECORD, sec0.left() + sec0.full_w() - 110, sec0.row_y(2) - 2, 110);
    if !st.btn_hk_record.is_null() { st.ownerdraw_ctrls.push(st.btn_hk_record); }

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

pub(super) unsafe fn settings_create_plugin_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Plugin.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 110);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 0);
    let sec2 = SettingsFormSectionLayout::new(page, 2, 0);

    let (_qs_lbl, qs_btn) = b.toggle_row(st, "启用快速搜索", 7102, sec0.left(), sec0.row_y(0), sec0.full_w());
    st.chk_qs = qs_btn;
    if !st.chk_qs.is_null() { st.ownerdraw_ctrls.push(st.chk_qs); }
    b.label(st, "搜索引擎：", sec0.left(), sec0.label_y(1, 24), sec0.label_w(), 24);
    st.cb_engine = b.dropdown(st, "筑森搜索（zxx.vip）", 7201, sec0.field_x(), sec0.row_y(1), 240);
    if !st.cb_engine.is_null() { st.ownerdraw_ctrls.push(st.cb_engine); }
    b.label(st, "URL 模板：", sec0.left(), sec0.label_y(2, 24), sec0.label_w(), 24);
    st.ed_tpl = b.edit(st, "", 7202, sec0.field_x(), sec0.row_y(2), sec0.field_w());
    let btn_restore_tpl = b.button(st, "恢复预设模板", 7203, sec0.left(), sec0.row_y(3), 130);
    if !btn_restore_tpl.is_null() { st.ownerdraw_ctrls.push(btn_restore_tpl); }
    let _ = b.label_auto(st, "占位符：{q}=编码后关键词，{raw}=原文", sec0.left() + 146, sec0.row_y(3) + 4, sec0.field_w_from(sec0.left() + 146), 24);
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

pub(super) unsafe fn settings_create_group_page(hwnd: HWND, st: &mut SettingsWndState) {
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

    let lbl_vv_source = settings_create_label(hwnd, "VV 来源：", sec0.left(), sec0.label_y(1, 24), sec0.label_w(), 24, ui_font);
    push(st, lbl_vv_source);
    st.cb_vv_source = settings_create_dropdown_btn(hwnd, "复制记录", IDC_SET_VV_SOURCE, sec0.field_x(), sec0.row_y(1), 180, ui_font);
    if !st.cb_vv_source.is_null() {
        settings_page_push_ctrl(st, page, st.cb_vv_source);
        st.ownerdraw_ctrls.push(st.cb_vv_source);
    }

    let lbl_vv_group = settings_create_label(hwnd, "VV 默认分组：", sec0.left(), sec0.label_y(2, 24), sec0.label_w(), 24, ui_font);
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
    st.btn_group_add = settings_create_small_btn(hwnd, "新建分组", IDC_SET_GROUP_ADD, x0, btn_y, bw, ui_font);
    st.btn_group_rename = settings_create_small_btn(hwnd, "重命名", IDC_SET_GROUP_RENAME, x0 + (bw + gap), btn_y, bw, ui_font);
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

pub(super) unsafe fn settings_create_cloud_page(hwnd: HWND, st: &mut SettingsWndState) {
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
    b.label(st, "同步间隔：", sec0.left(), sec0.label_y(1, 24), sec0.label_w(), 24);
    st.cb_cloud_interval = b.dropdown(st, "1小时", IDC_SET_CLOUD_INTERVAL, sec0.field_x(), sec0.row_y(1), 150);
    st.lb_cloud_status = b.label(st, "上次同步：未同步", sec0.left(), sec0.label_y(2, 24), sec0.full_w(), 24);

    b.label(st, "WebDAV 地址：", sec1.left(), sec1.label_y(0, 24), sec1.label_w(), 24);
    st.ed_cloud_url = b.edit(st, "", IDC_SET_CLOUD_URL, sec1.field_x(), sec1.row_y(0), sec1.field_w());
    b.label(st, "用户名：", sec1.left(), sec1.label_y(1, 24), sec1.label_w(), 24);
    st.ed_cloud_user = b.edit(st, "", IDC_SET_CLOUD_USER, sec1.field_x(), sec1.row_y(1), sec1.field_w());
    b.label(st, "密码：", sec1.left(), sec1.label_y(2, 24), sec1.label_w(), 24);
    st.ed_cloud_pass = b.add(st, settings_create_password_edit(hwnd, "", IDC_SET_CLOUD_PASS, sec1.field_x(), sec1.row_y(2), sec1.field_w(), st.ui_font));
    b.label(st, "远程目录：", sec1.left(), sec1.label_y(3, 24), sec1.label_w(), 24);
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

pub(super) unsafe fn settings_create_about_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::About.index();
    let b = SettingsPageBuilder { hwnd, page, font: st.ui_font };
    let sec = SettingsFormSectionLayout::new(page, 0, 0);
    let update_state = update_check_state_snapshot();
    let lines = [
        format!("{}{}", tr("版本：", "Version: "), env!("CARGO_PKG_VERSION")),
        tr(
            "设置界面现在统一使用同一套 section/form 布局。",
            "The settings window now uses a unified section/form layout.",
        )
        .to_string(),
        tr(
            "新增设置项时可以直接复用卡片、字段列、按钮行和统一间距。",
            "New settings can reuse the same cards, field columns, action rows, and spacing.",
        )
        .to_string(),
    ];
    let mut y = sec.row_y(0);
    for line in lines.iter() {
        let (_, h) = b.label_auto(st, line, sec.left(), y, sec.full_w(), 24);
        y += h + 10;
    }

    let (_, label_h) = b.label_auto(st, tr("开源地址：", "Source: "), sec.left(), y, 72, 24);
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
        tr("检查更新中…", "Checking for updates...").to_string()
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
            tr("点击下载最新版本", "Click to download latest version")
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

pub(super) unsafe fn settings_button_hover(st: &SettingsWndState, hwnd_item: HWND) -> bool {
    if hwnd_item.is_null() { return false; }
    let mut pt: POINT = zeroed();
    if GetCursorPos(&mut pt) == 0 { return false; }
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd_item, &mut rc) == 0 { return false; }
    pt.x >= rc.left && pt.x < rc.right && pt.y >= rc.top && pt.y < rc.bottom && st.hot_ownerdraw == hwnd_item
}

pub(super) unsafe fn settings_ensure_page(hwnd: HWND, st: &mut SettingsWndState, page: usize) {
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

pub(super) unsafe fn settings_draw_button_item(st: &SettingsWndState, dis: &DRAWITEMSTRUCT) {
    let th = Theme::default();
    let hdc = dis.hDC;
    let rc = dis.rcItem;
    let cid = dis.CtlID as isize;
    let pressed = (dis.itemState & ODS_SELECTED) != 0;
    let hover = settings_button_hover(st, dis.hwndItem);
    let text = get_window_text(dis.hwndItem);

    if cid == IDC_SET_AUTOSTART || cid == IDC_SET_SILENTSTART || cid == IDC_SET_TRAYICON || cid == IDC_SET_CLOSETRAY
        || cid == IDC_SET_CLICK_HIDE || cid == IDC_SET_PASTE_MOVE_TOP || cid == IDC_SET_DEDUPE_FILTER || cid == IDC_SET_AUTOHIDE_BLUR || cid == IDC_SET_EDGEHIDE
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
        DrawTextW(hdc, text_w.as_ptr(), -1, &mut text_rc, DT_LEFT | DT_VCENTER | DT_SINGLELINE);
        if !old_font.is_null() {
            SelectObject(hdc, old_font);
        }
        if !font.is_null() {
            DeleteObject(font as _);
        }
        return;
    }

    let kind = if cid == IDC_SET_MAX
        || cid == IDC_SET_POSMODE
        || cid == IDC_SET_CLOUD_INTERVAL
        || cid == IDC_SET_VV_GROUP
        || cid == IDC_SET_VV_SOURCE
        || cid == 6102
        || cid == 6103
        || cid == 7201
    {
        SettingsComponentKind::Dropdown
    } else if (cid == IDC_SET_GROUP_VIEW_RECORDS && settings_group_view_current(st) == 0)
        || (cid == IDC_SET_GROUP_VIEW_PHRASES && settings_group_view_current(st) == 1)
        || cid == IDC_SET_SAVE
    {
        SettingsComponentKind::AccentButton
    } else {
        SettingsComponentKind::Button
    };
    draw_settings_button_component(hdc as _, &rc, &text, kind, hover, pressed, th);
}

pub(super) unsafe fn apply_loaded_settings(hwnd: HWND, state: &mut AppState) {
    let old_edge_hide = state.settings.edge_auto_hide;
    let mut loaded = load_settings();
    loaded.auto_start = is_autostart_enabled();
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

pub(super) unsafe fn refresh_window_state(hwnd: HWND, reload_settings: bool) {
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

pub(super) unsafe fn sync_peer_windows_from_db(source_hwnd: HWND) {
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

pub(super) unsafe fn sync_peer_windows_from_settings(source_hwnd: HWND) {
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

