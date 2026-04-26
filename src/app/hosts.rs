use super::*;
use crate::settings_render::{
    IDC_SET_TRANSLATE_APP_ID, IDC_SET_TRANSLATE_PROVIDER, IDC_SET_TRANSLATE_SECRET,
    IDC_SET_TRANSLATE_TARGET,
};
use crate::shell::{image_ocr_status_text, text_translate_status_text};
use crate::win_system_ui::{
    create_settings_dropdown_button as settings_create_dropdown_btn,
    create_settings_edit as host_create_settings_edit,
    create_settings_label as host_create_settings_label,
    create_settings_label_auto as host_create_settings_label_auto,
    create_settings_listbox as host_create_settings_listbox,
    create_settings_password_edit as host_create_settings_password_edit,
    create_settings_small_button as settings_create_small_btn,
    create_settings_toggle_plain as settings_create_toggle_plain, draw_settings_button_component,
    draw_settings_toggle_component, SettingsComponentKind,
};
use std::time::Duration;

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

pub(super) unsafe fn skip_next_clipboard_update_for_all_hosts() {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() {
            continue;
        }
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            (*ptr).skip_next_clipboard_update_once = true;
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
    if point_in_rect_screen(&pt, &window_rect_for_dock(hwnd)) {
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
                if !st_ptr.is_null() && screen_point_hits_window_scope((*st_ptr).dropdown_popup, pt)
                {
                    return true;
                }
            }
        }
    }
    let popup = current_vv_popup_hwnd();
    screen_point_hits_window_scope(popup, pt)
}

pub(super) unsafe fn edge_window_scope_contains_point(hwnd: HWND, pt: POINT) -> bool {
    if screen_point_hits_window_scope(hwnd, pt) {
        return true;
    }
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        let popup = current_vv_popup_hwnd();
        return screen_point_hits_window_scope(popup, pt);
    }
    let popup = current_vv_popup_hwnd();
    screen_point_hits_window_scope(popup, pt)
}

unsafe fn window_needs_outside_hide_timer(hwnd: HWND) -> bool {
    if hwnd.is_null() || IsWindow(hwnd) == 0 || IsWindowVisible(hwnd) == 0 {
        return false;
    }
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return false;
    }
    let state = &*ptr;
    state.settings.auto_hide_on_blur
        && (state.role == WindowRole::Quick || state.main_window_noactivate)
}

unsafe fn refresh_outside_hide_timers() {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() || IsWindow(hwnd) == 0 {
            continue;
        }
        if window_needs_outside_hide_timer(hwnd) {
            SetTimer(hwnd, ID_TIMER_OUTSIDE_HIDE, 120, None);
        } else {
            KillTimer(hwnd, ID_TIMER_OUTSIDE_HIDE);
        }
    }
}

unsafe fn window_needs_edge_auto_hide_timer(hwnd: HWND) -> bool {
    if hwnd.is_null() || IsWindow(hwnd) == 0 || IsWindowVisible(hwnd) == 0 {
        return false;
    }
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return false;
    }
    if !(*ptr).settings_hwnd.is_null() && IsWindowVisible((*ptr).settings_hwnd) != 0 {
        return false;
    }
    (*ptr).settings.edge_auto_hide
}

unsafe fn edge_auto_hide_timer_interval(hwnd: HWND) -> u32 {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return EDGE_AUTO_HIDE_TIMER_MS;
    }
    if edge_animation_active(&*ptr) {
        EDGE_AUTO_HIDE_ANIM_TIMER_MS
    } else {
        EDGE_AUTO_HIDE_TIMER_MS
    }
}

unsafe fn refresh_edge_auto_hide_timers() {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() || IsWindow(hwnd) == 0 {
            continue;
        }
        if window_needs_edge_auto_hide_timer(hwnd) {
            SetTimer(
                hwnd,
                ID_TIMER_EDGE_AUTO_HIDE,
                edge_auto_hide_timer_interval(hwnd),
                None,
            );
        } else {
            KillTimer(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
        }
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
    refresh_outside_hide_timers();
    refresh_edge_auto_hide_timers();

    if any_visible_window_requires_quick_escape_try() {
        ensure_quick_escape_keyboard_hook();
    } else {
        disable_quick_escape_keyboard_hook();
    }
}

pub(crate) unsafe fn shutdown_low_level_input_hooks() {
    for hwnd in window_host_hwnds() {
        if !hwnd.is_null() && IsWindow(hwnd) != 0 {
            KillTimer(hwnd, ID_TIMER_OUTSIDE_HIDE);
            KillTimer(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
        }
    }
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
    if hwnd.is_null()
        || IsWindowVisible(hwnd) == 0
        || !is_window_enabled_compat(hwnd)
        || IsIconic(hwnd) != 0
    {
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

pub(crate) fn clear_edge_dock_state(state: &mut AppState) {
    state.edge_hidden = false;
    state.edge_hidden_side = EDGE_AUTO_HIDE_NONE;
    state.edge_docked_left = 0;
    state.edge_docked_top = 0;
    state.edge_docked_right = 0;
    state.edge_docked_bottom = 0;
    state.edge_hide_armed = false;
    state.edge_hide_pending_until = None;
    state.edge_hide_grace_until = None;
    state.edge_restore_wait_leave = false;
    state.edge_anim_until = None;
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

fn edge_docked_rect_valid(state: &AppState) -> bool {
    state.edge_docked_right > state.edge_docked_left
        && state.edge_docked_bottom > state.edge_docked_top
}

fn edge_side_valid(side: i32) -> bool {
    matches!(
        side,
        EDGE_AUTO_HIDE_LEFT | EDGE_AUTO_HIDE_RIGHT | EDGE_AUTO_HIDE_TOP | EDGE_AUTO_HIDE_BOTTOM
    )
}

unsafe fn edge_auto_hide_peek(hwnd: HWND) -> i32 {
    scale_for_window(hwnd, EDGE_AUTO_HIDE_PEEK).max(6)
}

fn edge_set_grace(state: &mut AppState, ms: u64) {
    state.edge_hide_grace_until = Some(Instant::now() + Duration::from_millis(ms));
}

pub(crate) fn edge_interaction_grace_ms() -> u64 {
    0
}

fn edge_set_hide_pending(state: &mut AppState, ms: u64) {
    let now = Instant::now();
    let base = state
        .edge_hide_grace_until
        .filter(|until| *until > now)
        .unwrap_or(now);
    state.edge_hide_pending_until = Some(base + Duration::from_millis(ms));
}

fn edge_grace_active(state: &AppState) -> bool {
    state
        .edge_hide_grace_until
        .map(|until| until > Instant::now())
        .unwrap_or(false)
}

pub(crate) fn edge_animation_active(state: &AppState) -> bool {
    state
        .edge_anim_until
        .map(|until| until > Instant::now())
        .unwrap_or(false)
}

unsafe fn begin_edge_animation(
    hwnd: HWND,
    state: &mut AppState,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    duration_ms: u64,
) {
    state.edge_anim_from_x = from_x;
    state.edge_anim_from_y = from_y;
    state.edge_anim_to_x = to_x;
    state.edge_anim_to_y = to_y;
    state.edge_anim_until = Some(Instant::now() + Duration::from_millis(duration_ms));
    SetWindowPos(
        hwnd,
        null_mut(),
        from_x,
        from_y,
        0,
        0,
        SWP_NOSIZE
            | SWP_NOACTIVATE
            | SWP_NOZORDER
            | SWP_NOOWNERZORDER
            | SWP_NOSENDCHANGING
            | SWP_ASYNCWINDOWPOS
            | SWP_SHOWWINDOW,
    );
}

unsafe fn edge_step_animation(hwnd: HWND, state: &mut AppState) -> bool {
    let Some(until) = state.edge_anim_until else {
        return false;
    };
    let now = Instant::now();
    let total_ms = EDGE_AUTO_HIDE_ANIM_MS as f32;
    let remain_ms = until.saturating_duration_since(now).as_millis() as f32;
    let t = if remain_ms <= 0.0 {
        1.0
    } else {
        (1.0 - (remain_ms / total_ms)).clamp(0.0, 1.0)
    };
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    let x = state.edge_anim_from_x
        + ((state.edge_anim_to_x - state.edge_anim_from_x) as f32 * eased).round() as i32;
    let y = state.edge_anim_from_y
        + ((state.edge_anim_to_y - state.edge_anim_from_y) as f32 * eased).round() as i32;
    SetWindowPos(
        hwnd,
        null_mut(),
        x,
        y,
        0,
        0,
        SWP_NOSIZE
            | SWP_NOACTIVATE
            | SWP_NOZORDER
            | SWP_NOOWNERZORDER
            | SWP_NOSENDCHANGING
            | SWP_ASYNCWINDOWPOS
            | SWP_SHOWWINDOW,
    );
    if t >= 1.0 {
        SetWindowPos(
            hwnd,
            null_mut(),
            state.edge_anim_to_x,
            state.edge_anim_to_y,
            0,
            0,
            SWP_NOSIZE
                | SWP_NOACTIVATE
                | SWP_NOZORDER
                | SWP_NOOWNERZORDER
                | SWP_NOSENDCHANGING
                | SWP_ASYNCWINDOWPOS
                | SWP_SHOWWINDOW,
        );
        state.edge_anim_until = None;
        return false;
    }
    true
}

fn edge_hide_pending_active(state: &AppState) -> bool {
    state
        .edge_hide_pending_until
        .map(|until| until > Instant::now())
        .unwrap_or(false)
}

unsafe fn edge_detect_margin_v(hwnd: HWND) -> i32 {
    scale_for_window(hwnd, EDGE_AUTO_HIDE_MARGIN).max(12)
}

unsafe fn edge_detect_margin_h(hwnd: HWND) -> i32 {
    edge_detect_margin_v(hwnd).max(scale_for_window(hwnd, 24))
}

unsafe fn edge_choose_dock_side(hwnd: HWND, rc: &RECT) -> Option<(i32, RECT)> {
    let work = nearest_monitor_work_rect_for_window(hwnd);
    let monitor = nearest_monitor_rect_for_window(hwnd);
    let margin_v = edge_detect_margin_v(hwnd);
    let margin_h = edge_detect_margin_h(hwnd);

    let candidates = [
        (
            (rc.left - work.left).abs(),
            EDGE_AUTO_HIDE_LEFT,
            work,
            margin_h,
        ),
        (
            (rc.left - monitor.left).abs(),
            EDGE_AUTO_HIDE_LEFT,
            monitor,
            margin_h,
        ),
        (
            (work.right - rc.right).abs(),
            EDGE_AUTO_HIDE_RIGHT,
            work,
            margin_h,
        ),
        (
            (monitor.right - rc.right).abs(),
            EDGE_AUTO_HIDE_RIGHT,
            monitor,
            margin_h,
        ),
        (
            (rc.top - work.top).abs(),
            EDGE_AUTO_HIDE_TOP,
            work,
            margin_v,
        ),
        (
            (rc.top - monitor.top).abs(),
            EDGE_AUTO_HIDE_TOP,
            monitor,
            margin_v,
        ),
        (
            (work.bottom - rc.bottom).abs(),
            EDGE_AUTO_HIDE_BOTTOM,
            work,
            margin_v,
        ),
        (
            (monitor.bottom - rc.bottom).abs(),
            EDGE_AUTO_HIDE_BOTTOM,
            monitor,
            margin_v,
        ),
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
    let _ = monitor;
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

unsafe fn ensure_edge_dock_state(hwnd: HWND, state: &mut AppState) -> bool {
    if edge_side_valid(state.edge_hidden_side) && edge_docked_rect_valid(state) {
        return true;
    }
    let rc = window_rect_for_dock(hwnd);
    update_edge_dock_state(hwnd, state, &rc)
}

unsafe fn edge_hotzone_rect(hwnd: HWND, state: &AppState) -> Option<RECT> {
    if !edge_side_valid(state.edge_hidden_side) || !edge_docked_rect_valid(state) {
        return None;
    }
    let docked = edge_docked_rect(state);
    let monitor = nearest_monitor_rect_for_window(hwnd);
    let hot = edge_detect_margin_v(hwnd);
    Some(match state.edge_hidden_side {
        EDGE_AUTO_HIDE_LEFT => RECT {
            left: docked.left,
            top: docked.top,
            right: docked.left + hot,
            bottom: docked.bottom,
        },
        EDGE_AUTO_HIDE_RIGHT => RECT {
            left: docked.right - hot,
            top: docked.top,
            right: docked.right,
            bottom: docked.bottom,
        },
        EDGE_AUTO_HIDE_TOP => RECT {
            left: docked.left,
            top: docked.top,
            right: docked.right,
            bottom: docked.top + hot,
        },
        EDGE_AUTO_HIDE_BOTTOM => RECT {
            left: docked.left,
            top: monitor.bottom - hot,
            right: docked.right,
            bottom: monitor.bottom,
        },
        _ => docked,
    })
}

unsafe fn edge_hidden_position(hwnd: HWND, state: &AppState, rc: &RECT) -> Option<(i32, i32)> {
    if !edge_side_valid(state.edge_hidden_side) || !edge_docked_rect_valid(state) {
        return None;
    }
    let docked = edge_docked_rect(state);
    let monitor = nearest_monitor_rect_for_window(hwnd);
    let width = (rc.right - rc.left).max(1);
    let height = (rc.bottom - rc.top).max(1);
    let peek = edge_auto_hide_peek(hwnd);
    let x = match state.edge_hidden_side {
        EDGE_AUTO_HIDE_LEFT => docked.left + peek - width,
        EDGE_AUTO_HIDE_RIGHT => docked.right - peek,
        EDGE_AUTO_HIDE_TOP | EDGE_AUTO_HIDE_BOTTOM => state
            .edge_restore_x
            .clamp(docked.left, (docked.right - width).max(docked.left)),
        _ => rc.left,
    };
    let y = match state.edge_hidden_side {
        EDGE_AUTO_HIDE_TOP => docked.top + peek - height,
        EDGE_AUTO_HIDE_BOTTOM => monitor.bottom - peek,
        EDGE_AUTO_HIDE_LEFT | EDGE_AUTO_HIDE_RIGHT => state
            .edge_restore_y
            .clamp(docked.top, (docked.bottom - height).max(docked.top)),
        _ => rc.top,
    };
    Some((x, y))
}

pub(super) unsafe fn restore_edge_hidden_window(hwnd: HWND, state: &mut AppState) {
    if !state.edge_hidden {
        return;
    }
    let rc = window_rect_for_dock(hwnd);
    begin_edge_animation(
        hwnd,
        state,
        rc.left,
        rc.top,
        state.edge_restore_x,
        state.edge_restore_y,
        EDGE_AUTO_HIDE_ANIM_MS,
    );
    state.edge_hidden = false;
    state.edge_hide_armed = false;
    state.edge_hide_pending_until = None;
    state.edge_restore_wait_leave = false;
    edge_set_grace(state, EDGE_AUTO_HIDE_RESTORE_GRACE_MS);
    ensure_mouse_leave_tracking(hwnd);
    refresh_low_level_input_hooks();
}

pub(crate) unsafe fn try_restore_edge_hidden_window(hwnd: HWND) -> bool {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return false;
    }
    let state = &mut *ptr;
    if !state.edge_hidden {
        return false;
    }
    restore_edge_hidden_window(hwnd, state);
    InvalidateRect(hwnd, null(), 1);
    true
}

unsafe fn hide_edge_docked_window_with_scope(hwnd: HWND, state: &mut AppState, check_scope: bool) {
    if !state.settings.edge_auto_hide || state.edge_hidden {
        return;
    }

    if !ensure_edge_dock_state(hwnd, state) {
        return;
    }
    let rc = window_rect_for_dock(hwnd);

    let mut cursor: POINT = zeroed();
    GetCursorPos(&mut cursor);
    if check_scope && edge_window_scope_contains_point(hwnd, cursor) {
        return;
    }
    let wait_leave_after_hide = edge_hotzone_rect(hwnd, state)
        .map(|hot| point_in_rect_screen(&cursor, &hot))
        .unwrap_or(false);

    state.edge_restore_x = rc.left;
    state.edge_restore_y = rc.top;
    let (hide_x, hide_y) = match edge_hidden_position(hwnd, state, &rc) {
        Some(pos) => pos,
        None => return,
    };
    state.edge_hidden = true;
    state.edge_hide_armed = false;
    state.edge_hide_pending_until = None;
    state.edge_restore_wait_leave = wait_leave_after_hide;
    begin_edge_animation(
        hwnd,
        state,
        rc.left,
        rc.top,
        hide_x,
        hide_y,
        EDGE_AUTO_HIDE_ANIM_MS,
    );
    hide_hover_preview();
    InvalidateRect(hwnd, null(), 0);
    refresh_low_level_input_hooks();
}

pub(super) unsafe fn handle_edge_auto_hide_tick(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() || IsWindowVisible(hwnd) == 0 {
        return;
    }
    let state = &mut *ptr;
    if state.edge_anim_until.is_some() {
        if !edge_step_animation(hwnd, state) {
            refresh_low_level_input_hooks();
        }
        return;
    }
    if !state.settings.edge_auto_hide {
        restore_edge_hidden_window(hwnd, state);
        clear_edge_dock_state(state);
        return;
    }

    let mut cursor: POINT = zeroed();
    GetCursorPos(&mut cursor);

    if state.edge_hidden {
        if let Some(hot) = edge_hotzone_rect(hwnd, state) {
            let in_hot = point_in_rect_screen(&cursor, &hot);
            if state.edge_restore_wait_leave {
                if !in_hot {
                    state.edge_restore_wait_leave = false;
                }
                return;
            }
            if in_hot {
                restore_edge_hidden_window(hwnd, state);
                InvalidateRect(hwnd, null(), 0);
            }
        } else {
            restore_edge_hidden_window(hwnd, state);
        }
        return;
    }

    let inside = edge_window_scope_contains_point(hwnd, cursor);
    if inside {
        state.edge_hide_armed = true;
        state.edge_hide_pending_until = None;
        return;
    }
    if edge_grace_active(state) {
        return;
    }
    if !state.edge_hide_armed {
        return;
    }
    if state.edge_hide_pending_until.is_none() {
        edge_set_hide_pending(state, EDGE_AUTO_HIDE_DELAY_MS);
        return;
    }
    if edge_hide_pending_active(state) {
        return;
    }
    hide_edge_docked_window_with_scope(hwnd, state, false);
    refresh_low_level_input_hooks();
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
    if state.settings.edge_auto_hide && !state.edge_hidden && !vv_popup_menu_active() {
        let mut pt: POINT = zeroed();
        if GetCursorPos(&mut pt) != 0 {
            if edge_window_scope_contains_point(hwnd, pt) {
                ensure_mouse_leave_tracking(hwnd);
            }
        }
    }
    if dirty {
        InvalidateRect(hwnd, null(), 0);
    }
}

pub(super) unsafe fn handle_outside_hide_tick(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() || IsWindowVisible(hwnd) == 0 {
        return;
    }
    let state = &mut *ptr;
    if !state.settings.auto_hide_on_blur {
        KillTimer(hwnd, ID_TIMER_OUTSIDE_HIDE);
        return;
    }
    if !(state.role == WindowRole::Quick || state.main_window_noactivate) {
        return;
    }
    if vv_popup_menu_active() {
        return;
    }
    let mut pt: POINT = zeroed();
    if GetCursorPos(&mut pt) != 0 && should_ignore_outside_click_for_point(pt) {
        return;
    }
    let mouse_down = (GetAsyncKeyState(VK_LBUTTON as i32) as u16 & 0x8000) != 0
        || (GetAsyncKeyState(VK_RBUTTON as i32) as u16 & 0x8000) != 0
        || (GetAsyncKeyState(VK_MBUTTON as i32) as u16 & 0x8000) != 0;
    if !mouse_down {
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
    let flags = SWP_NOMOVE
        | SWP_NOSIZE
        | SWP_NOZORDER
        | SWP_FRAMECHANGED
        | if enable { SWP_NOACTIVATE } else { 0 };
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

pub(crate) unsafe fn note_window_moved_for_edge_hide(hwnd: HWND, state: &mut AppState) {
    if !state.settings.edge_auto_hide
        || state.edge_hidden
        || edge_animation_active(state)
        || IsWindowVisible(hwnd) == 0
    {
        return;
    }
    let rc = window_rect_for_dock(hwnd);
    if update_edge_dock_state(hwnd, state, &rc) {
        state.edge_hide_armed = false;
        state.edge_hide_pending_until = None;
        edge_set_grace(state, edge_interaction_grace_ms());
    } else {
        clear_edge_dock_state(state);
    }
}

pub(super) unsafe fn main_window_should_stay_noactivate(state: &AppState, x: i32, y: i32) -> bool {
    hit_test_row(state, x, y) >= 0
}

pub(super) fn taskbar_created_message() -> u32 {
    *TASKBAR_CREATED_MESSAGE
        .get_or_init(|| unsafe { RegisterWindowMessageW(to_wide("TaskbarCreated").as_ptr()) })
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
        if !ptr.is_null() && !(*ptr).settings_hwnd.is_null() && IsWindow((*ptr).settings_hwnd) != 0
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
            &group_name_for_display(
                groups,
                st.vv_group_selected,
                source_tab_all_label(source_tab),
            ),
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
        SettingsPage::General => {
            settings_sync_pos_fields_enabled(st);
            let sound_enabled = st.draft.paste_success_sound_enabled;
            if !st.cb_paste_sound.is_null() {
                settings_set_text(
                    st.cb_paste_sound,
                    &paste_sound_display(&st.draft.paste_success_sound_kind),
                );
                EnableWindow(st.cb_paste_sound, if sound_enabled { 1 } else { 0 });
            }
            if !st.btn_paste_sound_pick.is_null() {
                settings_set_text(
                    st.btn_paste_sound_pick,
                    &paste_sound_file_button_text(&st.draft.paste_success_sound_path),
                );
                EnableWindow(
                    st.btn_paste_sound_pick,
                    if sound_enabled && st.draft.paste_success_sound_kind == "custom" {
                        1
                    } else {
                        0
                    },
                );
            }
        }
        SettingsPage::Hotkey => {
            let s = &st.draft;
            settings_set_text(st.cb_hk_mod, &normalize_hotkey_mod(&s.hotkey_mod));
            settings_set_text(st.cb_hk_key, &normalize_hotkey_key(&s.hotkey_key));
            settings_set_text(
                st.lb_hk_preview,
                &hotkey_preview_text(&s.hotkey_mod, &s.hotkey_key),
            );
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
            if !st.cb_plain_hk_mod.is_null() {
                settings_set_text(
                    st.cb_plain_hk_mod,
                    &normalize_hotkey_mod(&s.plain_paste_hotkey_mod),
                );
                EnableWindow(
                    st.cb_plain_hk_mod,
                    if s.plain_paste_hotkey_enabled { 1 } else { 0 },
                );
            }
            if !st.cb_plain_hk_key.is_null() {
                settings_set_text(
                    st.cb_plain_hk_key,
                    &normalize_hotkey_key(&s.plain_paste_hotkey_key),
                );
                EnableWindow(
                    st.cb_plain_hk_key,
                    if s.plain_paste_hotkey_enabled { 1 } else { 0 },
                );
            }
            if !st.lb_plain_hk_preview.is_null() {
                settings_set_text(
                    st.lb_plain_hk_preview,
                    &hotkey_preview_text(&s.plain_paste_hotkey_mod, &s.plain_paste_hotkey_key),
                );
            }
        }
        SettingsPage::Plugin => {
            let s = &st.draft;
            settings_set_text(st.cb_engine, &search_engine_display(&s.search_engine));
            settings_set_text(st.ed_tpl, &s.search_template);
            settings_set_text(
                st.cb_ocr_provider,
                &image_ocr_provider_display(&s.image_ocr_provider),
            );
            let baidu_enabled = s.image_ocr_provider == "baidu";
            let winocr_enabled = s.image_ocr_provider == "winocr";
            let ocr_fields_visible = baidu_enabled || winocr_enabled;
            settings_set_text(
                st.lb_ocr_primary,
                if winocr_enabled {
                    tr("微信目录：", "WeChat directory:")
                } else {
                    tr("API Key：", "API Key:")
                },
            );
            settings_set_text(st.lb_ocr_secondary, tr("Secret Key：", "Secret Key:"));
            settings_set_text(
                st.ed_ocr_cloud_url,
                if winocr_enabled {
                    &s.image_ocr_wechat_dir
                } else {
                    &s.image_ocr_cloud_url
                },
            );
            settings_set_text(st.ed_ocr_cloud_token, &s.image_ocr_cloud_token);
            if !st.lb_ocr_primary.is_null() {
                ShowWindow(
                    st.lb_ocr_primary,
                    if ocr_fields_visible { SW_SHOW } else { SW_HIDE },
                );
            }
            if !st.ed_ocr_cloud_url.is_null() {
                EnableWindow(
                    st.ed_ocr_cloud_url,
                    if baidu_enabled || winocr_enabled {
                        1
                    } else {
                        0
                    },
                );
                ShowWindow(
                    st.ed_ocr_cloud_url,
                    if ocr_fields_visible { SW_SHOW } else { SW_HIDE },
                );
            }
            if !st.ed_ocr_cloud_token.is_null() {
                EnableWindow(st.ed_ocr_cloud_token, if baidu_enabled { 1 } else { 0 });
            }
            if !st.lb_ocr_secondary.is_null() {
                ShowWindow(
                    st.lb_ocr_secondary,
                    if baidu_enabled { SW_SHOW } else { SW_HIDE },
                );
            }
            if !st.ed_ocr_cloud_token.is_null() {
                ShowWindow(
                    st.ed_ocr_cloud_token,
                    if baidu_enabled { SW_SHOW } else { SW_HIDE },
                );
            }
            if !st.btn_ocr_detect.is_null() {
                ShowWindow(
                    st.btn_ocr_detect,
                    if winocr_enabled { SW_SHOW } else { SW_HIDE },
                );
                EnableWindow(st.btn_ocr_detect, if winocr_enabled { 1 } else { 0 });
            }
            settings_set_text(
                st.lb_ocr_status,
                &image_ocr_status_text(
                    &s.image_ocr_provider,
                    &s.image_ocr_cloud_url,
                    &s.image_ocr_cloud_token,
                    &s.image_ocr_wechat_dir,
                ),
            );
            settings_set_text(
                st.cb_translate_provider,
                &text_translate_provider_display(&s.text_translate_provider),
            );
            settings_set_text(st.ed_translate_app_id, &s.text_translate_app_id);
            settings_set_text(st.ed_translate_secret, &s.text_translate_secret);
            settings_set_text(
                st.cb_translate_target,
                &text_translate_target_display(&s.text_translate_target_lang),
            );
            let translate_enabled = s.text_translate_provider == "baidu";
            if !st.lb_translate_primary.is_null() {
                ShowWindow(
                    st.lb_translate_primary,
                    if translate_enabled { SW_SHOW } else { SW_HIDE },
                );
            }
            if !st.ed_translate_app_id.is_null() {
                ShowWindow(
                    st.ed_translate_app_id,
                    if translate_enabled { SW_SHOW } else { SW_HIDE },
                );
                EnableWindow(
                    st.ed_translate_app_id,
                    if translate_enabled { 1 } else { 0 },
                );
            }
            if !st.lb_translate_secondary.is_null() {
                ShowWindow(
                    st.lb_translate_secondary,
                    if translate_enabled { SW_SHOW } else { SW_HIDE },
                );
            }
            if !st.ed_translate_secret.is_null() {
                ShowWindow(
                    st.ed_translate_secret,
                    if translate_enabled { SW_SHOW } else { SW_HIDE },
                );
                EnableWindow(
                    st.ed_translate_secret,
                    if translate_enabled { 1 } else { 0 },
                );
            }
            if !st.lb_translate_target.is_null() {
                ShowWindow(
                    st.lb_translate_target,
                    if translate_enabled { SW_SHOW } else { SW_HIDE },
                );
            }
            if !st.cb_translate_target.is_null() {
                ShowWindow(
                    st.cb_translate_target,
                    if translate_enabled { SW_SHOW } else { SW_HIDE },
                );
                EnableWindow(
                    st.cb_translate_target,
                    if translate_enabled { 1 } else { 0 },
                );
            }
            if !st.lb_translate_status.is_null() {
                settings_set_text(
                    st.lb_translate_status,
                    &text_translate_status_text(
                        &s.text_translate_provider,
                        &s.text_translate_app_id,
                        &s.text_translate_secret,
                        &s.text_translate_target_lang,
                    ),
                );
            }
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
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    settings_register_ctrl(
        st,
        page,
        hwnd,
        x,
        y,
        w,
        h,
        crate::settings_model::settings_page_scrollable(page),
    );
}

pub(super) unsafe fn settings_repos_controls(
    hwnd: HWND,
    st: &SettingsWndState,
    redraw_children: bool,
) {
    let slots: Vec<_> = st.ui.scroll_ctrls_for_page(st.cur_page).copied().collect();
    if slots.is_empty() || !crate::settings_model::settings_page_scrollable(st.cur_page) {
        return;
    }

    let mut crc: RECT = core::mem::zeroed();
    GetClientRect(hwnd, &mut crc);
    let viewport = settings_viewport_rect(&crc);
    let mut dirty: Vec<RECT> = Vec::with_capacity(slots.len() * 2);

    let hdwp = BeginDeferWindowPos(slots.len() as i32);
    if hdwp.is_null() {
        return;
    }
    let mut hdwp = hdwp;
    for slot in slots.iter() {
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
            | if visible {
                SWP_SHOWWINDOW
            } else {
                SWP_HIDEWINDOW
            };
        let r = DeferWindowPos(hdwp, hchild, null_mut(), ox, new_y, ow, oh, flags);
        if !r.is_null() {
            hdwp = r;
        }
    }
    EndDeferWindowPos(hdwp);

    if redraw_children {
        for slot in slots.iter() {
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
    let content_y = settings_content_y_scaled();
    let view_h = (crc.bottom - crc.top) - content_y;
    let new_y = new_y.clamp(
        0,
        settings_page_max_scroll_for_state(st, st.cur_page, view_h),
    );
    if new_y == st.content_scroll_y {
        return;
    }
    let old_y = st.content_scroll_y;
    st.content_scroll_y = new_y;
    st.page_scroll_y[st.cur_page] = new_y;
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
        top: content_y,
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
    start_flagged_timer(
        hwnd,
        ID_TIMER_SETTINGS_SCROLLBAR,
        1500,
        &mut st.scroll_hide_timer,
    );
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
                &hotkey_preview_text(
                    &get_window_text(st.cb_hk_mod),
                    &get_window_text(st.cb_hk_key),
                ),
            );
            InvalidateRect(st.lb_hk_preview, null(), 1);
        }
    }
    if old_page == page && st.ui.is_built(page) {
        settings_sync_page_state(st, page);
        return;
    }

    if st.scroll_dragging {
        cancel_settings_scroll_drag(hwnd, st);
    }
    if !st.dropdown_popup.is_null() {
        if IsWindow(st.dropdown_popup) != 0 {
            DestroyWindow(st.dropdown_popup);
        }
        st.dropdown_popup = null_mut();
    }

    SendMessageW(hwnd, WM_SETREDRAW, 0, 0);
    st.cur_page = page;
    if crate::settings_model::settings_page_scrollable(page) {
        st.content_scroll_y = st.page_scroll_y[page];
    } else {
        st.page_scroll_y[page] = 0;
        st.content_scroll_y = 0;
    }
    st.scroll_bar_visible = false;
    settings_ensure_page(hwnd, st, page);

    for other_page in 0..SETTINGS_PAGES.len() {
        for reg in st.ui.page_regs(other_page) {
            if reg.hwnd.is_null() {
                continue;
            }
            ShowWindow(
                reg.hwnd,
                if other_page == st.cur_page {
                    SW_SHOW
                } else {
                    SW_HIDE
                },
            );
        }
    }

    if crate::settings_model::settings_page_scrollable(st.cur_page) {
        settings_repos_controls(hwnd, st, true);
    }

    settings_sync_page_state(st, page);
    SendMessageW(hwnd, WM_SETREDRAW, 1, 0);
    InvalidateRect(hwnd, null(), 1);
    RedrawWindow(
        hwnd,
        null(),
        null_mut(),
        RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
    );
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
    settings_set_text(
        st.cb_max,
        settings_dropdown_label_for_max_items(s.max_items),
    );
    settings_set_text(st.ed_dx, &s.show_mouse_dx.to_string());
    settings_set_text(st.ed_dy, &s.show_mouse_dy.to_string());
    settings_set_text(st.ed_fx, &s.show_fixed_x.to_string());
    settings_set_text(st.ed_fy, &s.show_fixed_y.to_string());
    settings_set_text(
        st.cb_pos,
        settings_dropdown_label_for_pos_mode(&s.show_pos_mode),
    );
    if !st.cb_paste_sound.is_null() {
        settings_set_text(
            st.cb_paste_sound,
            &paste_sound_display(&s.paste_success_sound_kind),
        );
    }
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
    let edge_hide = st.draft.edge_auto_hide;
    let mode = settings_dropdown_pos_mode_from_label(&get_window_text(st.cb_pos));
    let is_follow = !edge_hide && mode == "mouse";
    let is_fixed = !edge_hide && mode == "fixed";
    if !st.cb_pos.is_null() {
        EnableWindow(st.cb_pos, if edge_hide { 0 } else { 1 });
    }
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
    if st.ui.is_built(SettingsPage::General.index()) && !st.cb_paste_sound.is_null() {
        st.draft.paste_success_sound_kind =
            paste_sound_key_from_display(&get_window_text(st.cb_paste_sound)).to_string();
    }
    if st.ui.is_built(SettingsPage::Hotkey.index())
        && !st.cb_hk_mod.is_null()
        && !st.cb_hk_key.is_null()
    {
        st.draft.hotkey_mod = normalize_hotkey_mod(&get_window_text(st.cb_hk_mod));
        st.draft.hotkey_key = normalize_hotkey_key(&get_window_text(st.cb_hk_key));
        if !st.cb_plain_hk_mod.is_null() && !st.cb_plain_hk_key.is_null() {
            st.draft.plain_paste_hotkey_mod =
                normalize_hotkey_mod(&get_window_text(st.cb_plain_hk_mod));
            st.draft.plain_paste_hotkey_key =
                normalize_hotkey_key(&get_window_text(st.cb_plain_hk_key));
        }
    }
    if st.ui.is_built(SettingsPage::Plugin.index()) && !st.cb_engine.is_null() {
        st.draft.search_engine =
            search_engine_key_from_display(&get_window_text(st.cb_engine)).to_string();
        st.draft.search_template = {
            let tpl = get_window_text(st.ed_tpl);
            if tpl.trim().is_empty() {
                search_engine_template(&st.draft.search_engine).to_string()
            } else {
                tpl
            }
        };
        st.draft.image_ocr_provider =
            image_ocr_provider_key_from_display(&get_window_text(st.cb_ocr_provider)).to_string();
        if st.draft.image_ocr_provider == "winocr" {
            st.draft.image_ocr_wechat_dir = get_window_text(st.ed_ocr_cloud_url);
        } else {
            st.draft.image_ocr_cloud_url = get_window_text(st.ed_ocr_cloud_url);
            st.draft.image_ocr_cloud_token = get_window_text(st.ed_ocr_cloud_token);
        }
        st.draft.text_translate_provider =
            text_translate_provider_key_from_display(&get_window_text(st.cb_translate_provider))
                .to_string();
        if st.draft.text_translate_provider == "baidu" {
            st.draft.text_translate_app_id = get_window_text(st.ed_translate_app_id);
            st.draft.text_translate_secret = get_window_text(st.ed_translate_secret);
            st.draft.text_translate_target_lang =
                text_translate_target_key_from_display(&get_window_text(st.cb_translate_target))
                    .to_string();
        }
    }
    st.draft.vv_source_tab = settings_vv_source_current(st);
    let vv_groups = settings_groups_cache_for_tab(st, st.draft.vv_source_tab);
    st.draft.vv_group_id =
        if st.vv_group_selected > 0 && vv_groups.iter().any(|g| g.id == st.vv_group_selected) {
            st.vv_group_selected
        } else {
            0
        };
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.cb_cloud_interval.is_null() {
        st.draft.cloud_sync_interval = {
            let label = get_window_text(st.cb_cloud_interval);
            if label.trim().is_empty() {
                "1小时".to_string()
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
    let plain_hotkey_old = format!(
        "{}+{}+{}",
        app.settings.plain_paste_hotkey_enabled,
        app.settings.plain_paste_hotkey_mod,
        app.settings.plain_paste_hotkey_key
    );
    let edge_hide_old = app.settings.edge_auto_hide;
    let vv_mode_old = app.settings.vv_mode_enabled;
    let persistent_search_old = app.settings.persistent_search_box;
    app.settings = st.draft.clone();
    if app.settings.edge_auto_hide {
        let mut rc: RECT = zeroed();
        if GetWindowRect(st.parent_hwnd, &mut rc) != 0 {
            app.settings.last_window_x = rc.left;
            app.settings.last_window_y = rc.top;
            st.draft.last_window_x = rc.left;
            st.draft.last_window_y = rc.top;
        }
    }
    if !app.settings.grouping_enabled {
        app.current_group_filter = 0;
        app.tab_group_filters = [0, 0];
        remember_shared_tab_view_state(app);
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
    let plain_hotkey_new = format!(
        "{}+{}+{}",
        app.settings.plain_paste_hotkey_enabled,
        app.settings.plain_paste_hotkey_mod,
        app.settings.plain_paste_hotkey_key
    );
    if plain_hotkey_old != plain_hotkey_new {
        register_plain_paste_hotkey_for(st.parent_hwnd, app);
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
        db_prune_items(0, new_max);
        reload_state_from_db_persisting(app);
    }
    if edge_hide_old && !app.settings.edge_auto_hide {
        restore_edge_hidden_window(st.parent_hwnd, app);
    } else if !edge_hide_old && app.settings.edge_auto_hide && IsWindowVisible(st.parent_hwnd) != 0
    {
        clear_edge_dock_state(app);
        note_window_moved_for_edge_hide(st.parent_hwnd, app);
    }
    refresh_low_level_input_hooks();
    app.refilter();
    if persistent_search_old != app.settings.persistent_search_box {
        prepare_search_ui_for_show(st.parent_hwnd, app);
    }
    layout_children(st.parent_hwnd);
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
        IDC_SET_PERSIST_SEARCH => st.draft.persistent_search_box,
        IDC_SET_PASTE_SOUND_ENABLE => st.draft.paste_success_sound_enabled,
        IDC_SET_AUTOHIDE_BLUR => st.draft.auto_hide_on_blur,
        IDC_SET_EDGEHIDE => st.draft.edge_auto_hide,
        IDC_SET_HOVERPREVIEW => st.draft.hover_preview,
        IDC_SET_VV_MODE => st.draft.vv_mode_enabled,
        IDC_SET_IMAGE_PREVIEW => st.draft.image_preview_enabled,
        IDC_SET_QUICK_DELETE => st.draft.quick_delete_button,
        IDC_SET_GROUP_ENABLE => st.draft.grouping_enabled,
        IDC_SET_CLOUD_ENABLE => st.draft.cloud_sync_enabled,
        6101 => st.draft.hotkey_enabled,
        IDC_SET_PLAIN_HK_ENABLE => st.draft.plain_paste_hotkey_enabled,
        7102 => st.draft.quick_search_enabled,
        7101 => st.draft.ai_clean_enabled,
        7103 => st.draft.super_mail_merge_enabled,
        7104 => st.draft.qr_quick_enabled,
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
        IDC_SET_DEDUPE_FILTER => st.draft.dedupe_filter_enabled = !st.draft.dedupe_filter_enabled,
        IDC_SET_PERSIST_SEARCH => st.draft.persistent_search_box = !st.draft.persistent_search_box,
        IDC_SET_PASTE_SOUND_ENABLE => {
            st.draft.paste_success_sound_enabled = !st.draft.paste_success_sound_enabled
        }
        IDC_SET_AUTOHIDE_BLUR => st.draft.auto_hide_on_blur = !st.draft.auto_hide_on_blur,
        IDC_SET_EDGEHIDE => st.draft.edge_auto_hide = !st.draft.edge_auto_hide,
        IDC_SET_HOVERPREVIEW => st.draft.hover_preview = !st.draft.hover_preview,
        IDC_SET_VV_MODE => st.draft.vv_mode_enabled = !st.draft.vv_mode_enabled,
        IDC_SET_IMAGE_PREVIEW => st.draft.image_preview_enabled = !st.draft.image_preview_enabled,
        IDC_SET_QUICK_DELETE => st.draft.quick_delete_button = !st.draft.quick_delete_button,
        IDC_SET_GROUP_ENABLE => st.draft.grouping_enabled = !st.draft.grouping_enabled,
        IDC_SET_CLOUD_ENABLE => st.draft.cloud_sync_enabled = !st.draft.cloud_sync_enabled,
        6101 => st.draft.hotkey_enabled = !st.draft.hotkey_enabled,
        IDC_SET_PLAIN_HK_ENABLE => {
            st.draft.plain_paste_hotkey_enabled = !st.draft.plain_paste_hotkey_enabled
        }
        7102 => st.draft.quick_search_enabled = !st.draft.quick_search_enabled,
        7101 => st.draft.ai_clean_enabled = !st.draft.ai_clean_enabled,
        7103 => st.draft.super_mail_merge_enabled = !st.draft.super_mail_merge_enabled,
        7104 => st.draft.qr_quick_enabled = !st.draft.qr_quick_enabled,
        _ => {}
    }
}

struct SettingsPageBuilder {
    hwnd: HWND,
    page: usize,
    font: *mut core::ffi::c_void,
}

impl SettingsPageBuilder {
    unsafe fn add(
        &self,
        st: &mut SettingsWndState,
        hwnd: HWND,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        if !hwnd.is_null() {
            settings_page_push_ctrl(st, self.page, hwnd, x, y, w, h);
        }
        hwnd
    }

    unsafe fn label(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_label(self.hwnd, text, x, y, w, h, self.font),
            x,
            y,
            w,
            h,
        )
    }

    unsafe fn label_auto(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        x: i32,
        y: i32,
        w: i32,
        min_h: i32,
    ) -> (HWND, i32) {
        let (hwnd, h) = settings_create_label_auto(self.hwnd, text, x, y, w, min_h, self.font);
        (self.add(st, hwnd, x, y, w, h), h)
    }

    unsafe fn button(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_small_btn(self.hwnd, text, id, x, y, w, self.font),
            x,
            y,
            w,
            settings_scale(32),
        )
    }

    unsafe fn dropdown(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_dropdown_btn(self.hwnd, text, id, x, y, w, self.font),
            x,
            y,
            w,
            settings_scale(32),
        )
    }

    unsafe fn edit(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_edit(self.hwnd, text, id, x, y, w, self.font),
            x,
            y,
            w,
            settings_scale(28),
        )
    }

    unsafe fn password_edit(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_password_edit(self.hwnd, text, id, x, y, w, self.font),
            x,
            y,
            w,
            settings_scale(28),
        )
    }

    unsafe fn listbox(
        &self,
        st: &mut SettingsWndState,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
        h: i32,
    ) -> HWND {
        self.add(
            st,
            settings_create_listbox(self.hwnd, id, x, y, w, h, self.font),
            x,
            y,
            w,
            h,
        )
    }

    unsafe fn toggle_row(
        &self,
        st: &mut SettingsWndState,
        text: &str,
        id: isize,
        x: i32,
        y: i32,
        w: i32,
    ) -> (HWND, HWND) {
        let (label, btn, ..) =
            settings_create_toggle_plain(self.hwnd, text, id, x, y, w, self.font);
        let label_h = settings_scale(24);
        let btn_w = settings_scale(44);
        let btn_h = settings_scale(24);
        let btn_x = x + w - btn_w;
        let btn_y = y + ((settings_scale(32) - btn_h).max(0) / 2);
        (
            self.add(st, label, x, y, w - btn_w - settings_scale(16), label_h),
            self.add(st, btn, btn_x, btn_y, btn_w, btn_h),
        )
    }
}

pub(super) unsafe fn settings_create_label(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    host_create_settings_label(parent, text, x, y, w, h, font)
}

pub(super) unsafe fn settings_create_label_auto(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    min_h: i32,
    font: *mut core::ffi::c_void,
) -> (HWND, i32) {
    host_create_settings_label_auto(parent, text, x, y, w, min_h, font)
}

pub(super) unsafe fn settings_create_edit(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    host_create_settings_edit(parent, text, id, x, y, w, font)
}

pub(super) unsafe fn settings_create_password_edit(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    host_create_settings_password_edit(parent, text, id, x, y, w, font)
}

pub(super) unsafe fn settings_create_general_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::General.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 0);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 130);
    let sec2 = SettingsFormSectionLayout::new(page, 2, 0);
    let sec3 = SettingsFormSectionLayout::new(page, 3, 138);
    let sec4 = SettingsFormSectionLayout::new(page, 4, 0);

    let (_, btn) = b.toggle_row(
        st,
        "开机自启",
        IDC_SET_AUTOSTART,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.chk_autostart = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "静默启动（打开默认不显示）",
        IDC_SET_SILENTSTART,
        sec0.left(),
        sec0.row_y(1),
        sec0.full_w(),
    );
    st.chk_silent_start = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "右下角图标开启/关闭",
        IDC_SET_TRAYICON,
        sec0.left(),
        sec0.row_y(2),
        sec0.full_w(),
    );
    st.chk_tray_icon = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "关闭不退出（托盘驻留）",
        IDC_SET_CLOSETRAY,
        sec0.left(),
        sec0.row_y(3),
        sec0.full_w(),
    );
    st.chk_close_tray = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "呼出后点击外部自动隐藏",
        IDC_SET_AUTOHIDE_BLUR,
        sec0.left(),
        sec0.row_y(4),
        sec0.full_w(),
    );
    st.chk_auto_hide_on_blur = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "贴边自动隐藏",
        IDC_SET_EDGEHIDE,
        sec0.left(),
        sec0.row_y(5),
        sec0.full_w(),
    );
    st.chk_edge_hide = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "悬停预览",
        IDC_SET_HOVERPREVIEW,
        sec0.left(),
        sec0.row_y(6),
        sec0.full_w(),
    );
    st.chk_hover_preview = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        tr("VV 模式", "VV Mode"),
        IDC_SET_VV_MODE,
        sec0.left(),
        sec0.row_y(7),
        sec0.full_w(),
    );
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        tr("显示图片缩略图", "Show image thumbnails"),
        IDC_SET_IMAGE_PREVIEW,
        sec0.left(),
        sec0.row_y(8),
        sec0.full_w(),
    );
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        tr("快速删除按钮", "Quick delete button"),
        IDC_SET_QUICK_DELETE,
        sec0.left(),
        sec0.row_y(9),
        sec0.full_w(),
    );
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }

    b.label(
        st,
        "最大保存条数：",
        sec1.left(),
        sec1.label_y(0, settings_scale(24)),
        sec1.label_w(),
        settings_scale(24),
    );
    st.cb_max = b.dropdown(
        st,
        "200",
        IDC_SET_MAX,
        sec1.field_x(),
        sec1.row_y(0),
        settings_scale(150),
    );
    if !st.cb_max.is_null() {
        st.ownerdraw_ctrls.push(st.cb_max);
    }

    let (_, btn) = b.toggle_row(
        st,
        "单击后隐藏主窗口",
        IDC_SET_CLICK_HIDE,
        sec2.left(),
        sec2.row_y(0),
        sec2.full_w(),
    );
    st.chk_click_hide = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "粘贴后上移到首行",
        IDC_SET_PASTE_MOVE_TOP,
        sec2.left(),
        sec2.row_y(1),
        sec2.full_w(),
    );
    st.chk_move_pasted_to_top = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "重复内容过滤并提升到首行",
        IDC_SET_DEDUPE_FILTER,
        sec2.left(),
        sec2.row_y(2),
        sec2.full_w(),
    );
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "常驻搜索框",
        IDC_SET_PERSIST_SEARCH,
        sec2.left(),
        sec2.row_y(3),
        sec2.full_w(),
    );
    st.chk_persistent_search = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    let (_, btn) = b.toggle_row(
        st,
        "粘贴成功声音",
        IDC_SET_PASTE_SOUND_ENABLE,
        sec2.left(),
        sec2.row_y(4),
        sec2.full_w(),
    );
    st.chk_paste_sound = btn;
    if !btn.is_null() {
        st.ownerdraw_ctrls.push(btn);
    }
    b.label(
        st,
        "提示音：",
        sec2.left(),
        sec2.label_y(5, settings_scale(24)),
        sec2.label_w(),
        settings_scale(24),
    );
    st.cb_paste_sound = b.dropdown(
        st,
        &paste_sound_display("default"),
        IDC_SET_PASTE_SOUND_KIND,
        sec2.field_x(),
        sec2.row_y(5),
        settings_scale(170),
    );
    if !st.cb_paste_sound.is_null() {
        st.ownerdraw_ctrls.push(st.cb_paste_sound);
    }
    b.label(
        st,
        "声音文件：",
        sec2.left(),
        sec2.label_y(6, settings_scale(24)),
        sec2.label_w(),
        settings_scale(24),
    );
    st.btn_paste_sound_pick = b.button(
        st,
        &paste_sound_file_button_text(""),
        IDC_SET_PASTE_SOUND_PICK,
        sec2.field_x(),
        sec2.row_y(6),
        settings_scale(240),
    );
    if !st.btn_paste_sound_pick.is_null() {
        st.ownerdraw_ctrls.push(st.btn_paste_sound_pick);
    }

    b.label(
        st,
        "弹出位置：",
        sec3.left(),
        sec3.label_y(0, settings_scale(24)),
        sec3.label_w(),
        settings_scale(24),
    );
    st.cb_pos = b.dropdown(
        st,
        "跟随鼠标",
        IDC_SET_POSMODE,
        sec3.field_x(),
        sec3.row_y(0),
        settings_scale(170),
    );
    if !st.cb_pos.is_null() {
        st.ownerdraw_ctrls.push(st.cb_pos);
    }

    b.label(
        st,
        "鼠标偏移 dx/dy：",
        sec3.left(),
        sec3.label_y(1, settings_scale(24)),
        sec3.label_w(),
        settings_scale(24),
    );
    let mouse_x = sec3.field_x();
    st.ed_dx = b.edit(
        st,
        "",
        IDC_SET_DX,
        mouse_x,
        sec3.row_y(1),
        settings_scale(64),
    );
    st.ed_dy = b.edit(
        st,
        "",
        IDC_SET_DY,
        mouse_x + settings_scale(74),
        sec3.row_y(1),
        settings_scale(64),
    );

    b.label(
        st,
        "固定位置 x/y：",
        sec3.left(),
        sec3.label_y(2, settings_scale(24)),
        sec3.label_w(),
        settings_scale(24),
    );
    let fixed_x = sec3.field_x();
    st.ed_fx = b.edit(
        st,
        "",
        IDC_SET_FX,
        fixed_x,
        sec3.row_y(2),
        settings_scale(64),
    );
    st.ed_fy = b.edit(
        st,
        "",
        IDC_SET_FY,
        fixed_x + settings_scale(74),
        sec3.row_y(2),
        settings_scale(64),
    );

    let btn_y = sec4.row_y(0);
    st.btn_open_cfg = b.button(
        st,
        "打开设置文件",
        IDC_SET_BTN_OPENCFG,
        sec4.action_x(0, settings_scale(130)),
        btn_y,
        settings_scale(130),
    );
    st.btn_open_db = b.button(
        st,
        "打开数据库文件",
        IDC_SET_BTN_OPENDB,
        sec4.action_x(1, settings_scale(130)),
        btn_y,
        settings_scale(130),
    );
    st.btn_open_data = b.button(
        st,
        "打开数据目录",
        IDC_SET_BTN_OPENDATA,
        sec4.action_x(2, settings_scale(130)),
        btn_y,
        settings_scale(130),
    );
    for &hh in &[st.btn_open_cfg, st.btn_open_db, st.btn_open_data] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }
    st.ui.mark_built(page);
}

pub(super) unsafe fn settings_create_listbox(
    parent: HWND,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    host_create_settings_listbox(parent, id, x, y, w, h, font)
}

pub(super) unsafe fn settings_groups_refresh_list(st: &mut SettingsWndState, select_gid: i64) {
    if st.lb_groups.is_null() {
        return;
    }
    let category = source_tab_category(settings_group_view_current(st));
    SendMessageW(st.lb_groups, LB_RESETCONTENT, 0, 0);
    *settings_groups_cache_for_tab_mut(st, settings_group_view_current(st)) =
        db_load_groups(category);
    let groups = settings_groups_cache_for_tab(st, settings_group_view_current(st));
    let mut sel_idx: i32 = -1;
    for (i, g) in groups.iter().enumerate() {
        SendMessageW(
            st.lb_groups,
            LB_ADDSTRING,
            0,
            to_wide(&g.name).as_ptr() as LPARAM,
        );
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
    if st.lb_groups.is_null() {
        return None;
    }
    let row = SendMessageW(st.lb_groups, LB_GETCURSEL, 0, 0) as i32;
    if row < 0 {
        return None;
    }
    settings_groups_cache_for_tab(st, settings_group_view_current(st))
        .get(row as usize)
        .cloned()
        .map(|g| (row as usize, g))
}

pub(super) unsafe fn settings_groups_sync_name(_st: &mut SettingsWndState) {}

pub(super) unsafe fn settings_groups_move(st: &mut SettingsWndState, step: i32) {
    let Some((idx, _)) = settings_groups_selected(st) else {
        return;
    };
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
            reload_state_from_db_persisting(&mut *pst);
            InvalidateRect(st.parent_hwnd, null(), 1);
        }
    }
}

pub(super) unsafe fn settings_create_hotkey_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Hotkey.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 86);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 0);
    let sec2 = SettingsFormSectionLayout::new(page, 2, 0);
    let line_h = settings_scale(24);
    let note_h = settings_scale(40);
    let small_gap = settings_scale(6);

    let (_hk_lbl, hk_btn) = b.toggle_row(
        st,
        "启用快捷键",
        6101,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.chk_hk_enable = hk_btn;
    if !st.chk_hk_enable.is_null() {
        st.ownerdraw_ctrls.push(st.chk_hk_enable);
    }

    b.label(
        st,
        "修饰键：",
        sec0.left(),
        sec0.label_y(1, line_h),
        settings_scale(70),
        line_h,
    );
    st.cb_hk_mod = b.dropdown(
        st,
        "Win",
        6102,
        sec0.field_x(),
        sec0.row_y(1),
        settings_scale(170),
    );
    if !st.cb_hk_mod.is_null() {
        st.ownerdraw_ctrls.push(st.cb_hk_mod);
    }
    let key_label_x = sec0.field_x() + settings_scale(186);
    b.label(
        st,
        "按键：",
        key_label_x,
        sec0.label_y(1, line_h),
        settings_scale(50),
        line_h,
    );
    st.cb_hk_key = b.dropdown(
        st,
        "V",
        6103,
        key_label_x + settings_scale(50),
        sec0.row_y(1),
        settings_scale(120),
    );
    if !st.cb_hk_key.is_null() {
        st.ownerdraw_ctrls.push(st.cb_hk_key);
    }
    st.lb_hk_preview = b.label(
        st,
        "当前设置：Win + V",
        sec0.left(),
        sec0.label_y(2, line_h),
        sec0.full_w() - settings_scale(124),
        line_h,
    );
    st.btn_hk_record = b.button(
        st,
        tr("录制热键", "Record Hotkey"),
        IDC_SET_HK_RECORD,
        sec0.left() + sec0.full_w() - settings_scale(110),
        sec0.row_y(2) - settings_scale(2),
        settings_scale(110),
    );
    if !st.btn_hk_record.is_null() {
        st.ownerdraw_ctrls.push(st.btn_hk_record);
    }

    let (_plain_lbl, plain_btn) = b.toggle_row(
        st,
        tr("启用纯文本粘贴快捷键", "Enable plain-text paste hotkey"),
        IDC_SET_PLAIN_HK_ENABLE,
        sec0.left(),
        sec0.row_y(3),
        sec0.full_w(),
    );
    st.chk_plain_hk_enable = plain_btn;
    if !st.chk_plain_hk_enable.is_null() {
        st.ownerdraw_ctrls.push(st.chk_plain_hk_enable);
    }
    b.label(
        st,
        tr("纯文本修饰键：", "Plain modifiers:"),
        sec0.left(),
        sec0.label_y(4, line_h),
        settings_scale(110),
        line_h,
    );
    st.cb_plain_hk_mod = b.dropdown(
        st,
        "Ctrl+Shift",
        IDC_SET_PLAIN_HK_MOD,
        sec0.field_x(),
        sec0.row_y(4),
        settings_scale(170),
    );
    if !st.cb_plain_hk_mod.is_null() {
        st.ownerdraw_ctrls.push(st.cb_plain_hk_mod);
    }
    let plain_key_label_x = sec0.field_x() + settings_scale(186);
    b.label(
        st,
        tr("纯文本按键：", "Plain key:"),
        plain_key_label_x,
        sec0.label_y(4, line_h),
        settings_scale(90),
        line_h,
    );
    st.cb_plain_hk_key = b.dropdown(
        st,
        "V",
        IDC_SET_PLAIN_HK_KEY,
        plain_key_label_x + settings_scale(90),
        sec0.row_y(4),
        settings_scale(120),
    );
    if !st.cb_plain_hk_key.is_null() {
        st.ownerdraw_ctrls.push(st.cb_plain_hk_key);
    }
    st.lb_plain_hk_preview = b.label(
        st,
        &hotkey_preview_text("Ctrl+Shift", "V"),
        sec0.left(),
        sec0.label_y(5, line_h),
        sec0.full_w(),
        line_h,
    );

    let _ = b.label_auto(st, "说明：通过注册表 DisabledHotkeys 屏蔽或恢复 Win+V。修改后通常需要重启资源管理器或重新登录。", sec1.left(), sec1.row_y(0), sec1.full_w(), note_h);
    st.btn_clip_hist_block = b.button(
        st,
        "屏蔽 Win+V",
        6111,
        sec1.action_x(0, settings_scale(110)),
        sec1.row_y(1),
        settings_scale(110),
    );
    st.btn_clip_hist_restore = b.button(
        st,
        "恢复 Win+V",
        6112,
        sec1.action_x(1, settings_scale(110)),
        sec1.row_y(1),
        settings_scale(110),
    );
    st.btn_restart_explorer = b.button(
        st,
        "重启资源管理器",
        6113,
        sec1.action_x(2, settings_scale(130)),
        sec1.row_y(1),
        settings_scale(130),
    );
    for &hh in &[
        st.btn_clip_hist_block,
        st.btn_clip_hist_restore,
        st.btn_restart_explorer,
    ] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }

    let (_desc1, d1h) = b.label_auto(
        st,
        "说明：保存后会立即重新注册主快捷键。",
        sec2.left(),
        sec2.row_y(0),
        sec2.full_w(),
        line_h,
    );
    let _ = b.label_auto(
        st,
        "建议避免使用 Ctrl+C / Ctrl+V 等系统级常用组合。",
        sec2.left(),
        sec2.row_y(0) + d1h + small_gap,
        sec2.full_w(),
        line_h,
    );

    st.ui.mark_built(page);
}

pub(super) unsafe fn settings_create_plugin_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Plugin.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 110);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 110);
    let sec2 = SettingsFormSectionLayout::new(page, 2, 110);
    let sec3 = SettingsFormSectionLayout::new(page, 3, 0);
    let line_h = settings_scale(24);

    let (_qs_lbl, qs_btn) = b.toggle_row(
        st,
        "启用快速搜索",
        7102,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.chk_qs = qs_btn;
    if !st.chk_qs.is_null() {
        st.ownerdraw_ctrls.push(st.chk_qs);
    }
    b.label(
        st,
        "搜索引擎：",
        sec0.left(),
        sec0.label_y(1, line_h),
        sec0.label_w(),
        line_h,
    );
    st.cb_engine = b.dropdown(
        st,
        "筑森搜索（zxx.vip）",
        7201,
        sec0.field_x(),
        sec0.row_y(1),
        settings_scale(240),
    );
    if !st.cb_engine.is_null() {
        st.ownerdraw_ctrls.push(st.cb_engine);
    }
    b.label(
        st,
        "URL 模板：",
        sec0.left(),
        sec0.label_y(2, line_h),
        sec0.label_w(),
        line_h,
    );
    st.ed_tpl = b.edit(st, "", 7202, sec0.field_x(), sec0.row_y(2), sec0.field_w());
    let btn_restore_tpl = b.button(
        st,
        "恢复预设模板",
        7203,
        sec0.left(),
        sec0.row_y(3),
        settings_scale(130),
    );
    if !btn_restore_tpl.is_null() {
        st.ownerdraw_ctrls.push(btn_restore_tpl);
    }
    let _ = b.label_auto(
        st,
        "占位符：{q}=编码后关键词，{raw}=原文",
        sec0.left() + settings_scale(146),
        sec0.row_y(3) + settings_scale(4),
        sec0.field_w_from(sec0.left() + settings_scale(146)),
        line_h,
    );

    b.label(
        st,
        tr("识别来源：", "Provider:"),
        sec1.left(),
        sec1.label_y(0, line_h),
        sec1.label_w(),
        line_h,
    );
    st.cb_ocr_provider = b.dropdown(
        st,
        tr("关闭", "Off"),
        IDC_SET_OCR_PROVIDER,
        sec1.field_x(),
        sec1.row_y(0),
        settings_scale(220),
    );
    if !st.cb_ocr_provider.is_null() {
        st.ownerdraw_ctrls.push(st.cb_ocr_provider);
    }
    st.lb_ocr_status = b.label(
        st,
        tr("图片 OCR：已关闭", "Image OCR: disabled"),
        sec1.left(),
        sec1.label_y(1, line_h),
        sec1.full_w(),
        line_h,
    );
    st.lb_ocr_primary = b.label(
        st,
        tr("API Key：", "API Key:"),
        sec1.left(),
        sec1.label_y(2, line_h),
        sec1.label_w(),
        line_h,
    );
    st.ed_ocr_cloud_url = b.edit(
        st,
        "",
        IDC_SET_OCR_CLOUD_URL,
        sec1.field_x(),
        sec1.row_y(2),
        sec1.field_w(),
    );
    st.lb_ocr_secondary = b.label(
        st,
        tr("Secret Key：", "Secret Key:"),
        sec1.left(),
        sec1.label_y(3, line_h),
        sec1.label_w(),
        line_h,
    );
    st.ed_ocr_cloud_token = b.password_edit(
        st,
        "",
        IDC_SET_OCR_CLOUD_TOKEN,
        sec1.field_x(),
        sec1.row_y(3),
        sec1.field_w(),
    );
    st.btn_ocr_detect = b.button(
        st,
        tr("自动检测微信目录", "Auto-detect WeChat directory"),
        IDC_SET_OCR_WECHAT_DETECT,
        sec1.left(),
        sec1.row_y(3),
        settings_scale(180),
    );
    if !st.btn_ocr_detect.is_null() {
        st.ownerdraw_ctrls.push(st.btn_ocr_detect);
    }

    b.label(
        st,
        tr("翻译来源：", "Provider:"),
        sec2.left(),
        sec2.label_y(0, line_h),
        sec2.label_w(),
        line_h,
    );
    st.cb_translate_provider = b.dropdown(
        st,
        tr("关闭", "Off"),
        IDC_SET_TRANSLATE_PROVIDER,
        sec2.field_x(),
        sec2.row_y(0),
        settings_scale(220),
    );
    if !st.cb_translate_provider.is_null() {
        st.ownerdraw_ctrls.push(st.cb_translate_provider);
    }
    st.lb_translate_status = b.label(
        st,
        tr("文本翻译：已关闭", "Text translation: disabled"),
        sec2.left(),
        sec2.label_y(1, line_h),
        sec2.full_w(),
        line_h,
    );
    st.lb_translate_primary = b.label(
        st,
        tr("APP ID：", "APP ID:"),
        sec2.left(),
        sec2.label_y(2, line_h),
        sec2.label_w(),
        line_h,
    );
    st.ed_translate_app_id = b.edit(
        st,
        "",
        IDC_SET_TRANSLATE_APP_ID,
        sec2.field_x(),
        sec2.row_y(2),
        sec2.field_w(),
    );
    st.lb_translate_secondary = b.label(
        st,
        tr("密钥：", "Secret:"),
        sec2.left(),
        sec2.label_y(3, line_h),
        sec2.label_w(),
        line_h,
    );
    st.ed_translate_secret = b.password_edit(
        st,
        "",
        IDC_SET_TRANSLATE_SECRET,
        sec2.field_x(),
        sec2.row_y(3),
        sec2.field_w(),
    );
    st.lb_translate_target = b.label(
        st,
        tr("目标语言：", "Target language:"),
        sec2.left(),
        sec2.label_y(4, line_h),
        sec2.label_w(),
        line_h,
    );
    st.cb_translate_target = b.dropdown(
        st,
        tr("简体中文", "Simplified Chinese"),
        IDC_SET_TRANSLATE_TARGET,
        sec2.field_x(),
        sec2.row_y(4),
        settings_scale(180),
    );
    if !st.cb_translate_target.is_null() {
        st.ownerdraw_ctrls.push(st.cb_translate_target);
    }

    let (_ai_lbl, ai_btn) = b.toggle_row(
        st,
        "AI 文本清洗",
        7101,
        sec3.left(),
        sec3.row_y(0),
        sec3.full_w(),
    );
    st.chk_ai = ai_btn;
    if !st.chk_ai.is_null() {
        st.ownerdraw_ctrls.push(st.chk_ai);
    }
    let (_mm_lbl, mm_btn) = b.toggle_row(
        st,
        "启用超级邮件合并",
        7103,
        sec3.left(),
        sec3.row_y(1),
        sec3.full_w(),
    );
    st.chk_mm = mm_btn;
    if !st.chk_mm.is_null() {
        st.ownerdraw_ctrls.push(st.chk_mm);
    }
    let btn_mail_merge = b.button(
        st,
        "打开超级邮件合并",
        IDC_SET_PLUGIN_MAILMERGE,
        sec3.left(),
        sec3.row_y(2),
        settings_scale(170),
    );
    if !btn_mail_merge.is_null() {
        st.ownerdraw_ctrls.push(btn_mail_merge);
    }
    let (_qr_lbl, qr_btn) = b.toggle_row(
        st,
        "启用快捷转换二维码",
        7104,
        sec3.left(),
        sec3.row_y(3),
        sec3.full_w(),
    );
    st.chk_qr = qr_btn;
    if !st.chk_qr.is_null() {
        st.ownerdraw_ctrls.push(st.chk_qr);
    }
    st.btn_plugin_downloads = b.button(
        st,
        tr("独立插件下载", "Standalone plugin downloads"),
        IDC_SET_PLUGIN_DOWNLOADS,
        sec3.left(),
        sec3.row_y(4),
        settings_scale(170),
    );
    if !st.btn_plugin_downloads.is_null() {
        st.ownerdraw_ctrls.push(st.btn_plugin_downloads);
    }
    st.ui.mark_built(page);
}

pub(super) unsafe fn settings_create_group_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Group.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 104);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 0);
    let (_, btn) = b.toggle_row(
        st,
        "启用分组功能",
        IDC_SET_GROUP_ENABLE,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.chk_group_enable = btn;
    if !st.chk_group_enable.is_null() {
        st.ownerdraw_ctrls.push(st.chk_group_enable);
    }

    b.label(
        st,
        tr("VV 来源：", "VV Source:"),
        sec0.left(),
        sec0.label_y(1, settings_scale(24)),
        sec0.label_w(),
        settings_scale(24),
    );
    st.cb_vv_source = b.dropdown(
        st,
        source_tab_label(0),
        IDC_SET_VV_SOURCE,
        sec0.field_x(),
        sec0.row_y(1),
        settings_scale(180),
    );
    if !st.cb_vv_source.is_null() {
        st.ownerdraw_ctrls.push(st.cb_vv_source);
    }

    b.label(
        st,
        tr("VV 默认分组：", "VV Default Group:"),
        sec0.left(),
        sec0.label_y(2, settings_scale(24)),
        sec0.label_w(),
        settings_scale(24),
    );
    st.cb_vv_group = b.dropdown(
        st,
        source_tab_all_label(0),
        IDC_SET_VV_GROUP,
        sec0.field_x(),
        sec0.row_y(2),
        settings_scale(220),
    );
    if !st.cb_vv_group.is_null() {
        st.ownerdraw_ctrls.push(st.cb_vv_group);
    }

    let tab_w = settings_scale(118);
    st.btn_group_view_records = b.button(
        st,
        "复制记录",
        IDC_SET_GROUP_VIEW_RECORDS,
        sec1.left(),
        sec1.row_y(0),
        tab_w,
    );
    st.btn_group_view_phrases = b.button(
        st,
        "常用短语",
        IDC_SET_GROUP_VIEW_PHRASES,
        sec1.left() + tab_w + settings_scale(10),
        sec1.row_y(0),
        tab_w,
    );
    for &hh in &[st.btn_group_view_records, st.btn_group_view_phrases] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }

    st.lb_group_current = b.label(
        st,
        "当前分组：全部记录",
        sec1.left(),
        sec1.row_y(1),
        sec1.full_w(),
        settings_scale(24),
    );
    b.label(
        st,
        "分组列表：",
        sec1.left(),
        sec1.row_y(2),
        settings_scale(220),
        settings_scale(22),
    );

    st.lb_groups = b.listbox(
        st,
        IDC_SET_GROUP_LIST,
        sec1.left(),
        sec1.row_y(3),
        sec1.full_w(),
        settings_scale(170),
    );

    let btn_y = sec1.row_y(3) + settings_scale(186);
    let bw = settings_scale(90);
    let gap = settings_scale(10);
    let x0 = sec1.left();
    st.btn_group_add = b.button(st, "新建分组", IDC_SET_GROUP_ADD, x0, btn_y, bw);
    st.btn_group_rename = b.button(
        st,
        "重命名",
        IDC_SET_GROUP_RENAME,
        x0 + (bw + gap),
        btn_y,
        bw,
    );
    st.btn_group_delete = b.button(
        st,
        "删除",
        IDC_SET_GROUP_DELETE,
        x0 + (bw + gap) * 2,
        btn_y,
        bw,
    );
    st.btn_group_up = b.button(st, "上移", IDC_SET_GROUP_UP, x0 + (bw + gap) * 3, btn_y, bw);
    st.btn_group_down = b.button(
        st,
        "下移",
        IDC_SET_GROUP_DOWN,
        x0 + (bw + gap) * 4,
        btn_y,
        bw,
    );
    for &hh in &[
        st.btn_group_add,
        st.btn_group_rename,
        st.btn_group_delete,
        st.btn_group_up,
        st.btn_group_down,
    ] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }

    st.ui.mark_built(page);
}

pub(super) unsafe fn settings_create_cloud_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Cloud.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec0 = SettingsFormSectionLayout::new(page, 0, 110);
    let sec1 = SettingsFormSectionLayout::new(page, 1, 110);
    let sec2 = SettingsFormSectionLayout::new(page, 2, 0);
    let line_h = settings_scale(24);

    let (_, toggle) = b.toggle_row(
        st,
        "启用自动同步",
        IDC_SET_CLOUD_ENABLE,
        sec0.left(),
        sec0.row_y(0),
        sec0.full_w(),
    );
    st.chk_cloud_enable = toggle;
    if !st.chk_cloud_enable.is_null() {
        st.ownerdraw_ctrls.push(st.chk_cloud_enable);
    }
    b.label(
        st,
        "同步间隔：",
        sec0.left(),
        sec0.label_y(1, line_h),
        sec0.label_w(),
        line_h,
    );
    st.cb_cloud_interval = b.dropdown(
        st,
        "1小时",
        IDC_SET_CLOUD_INTERVAL,
        sec0.field_x(),
        sec0.row_y(1),
        settings_scale(150),
    );
    st.lb_cloud_status = b.label(
        st,
        "上次同步：未同步",
        sec0.left(),
        sec0.label_y(2, line_h),
        sec0.full_w(),
        line_h,
    );

    b.label(
        st,
        "WebDAV 地址：",
        sec1.left(),
        sec1.label_y(0, line_h),
        sec1.label_w(),
        line_h,
    );
    st.ed_cloud_url = b.edit(
        st,
        "",
        IDC_SET_CLOUD_URL,
        sec1.field_x(),
        sec1.row_y(0),
        sec1.field_w(),
    );
    b.label(
        st,
        "用户名：",
        sec1.left(),
        sec1.label_y(1, line_h),
        sec1.label_w(),
        line_h,
    );
    st.ed_cloud_user = b.edit(
        st,
        "",
        IDC_SET_CLOUD_USER,
        sec1.field_x(),
        sec1.row_y(1),
        sec1.field_w(),
    );
    b.label(
        st,
        "密码：",
        sec1.left(),
        sec1.label_y(2, line_h),
        sec1.label_w(),
        line_h,
    );
    st.ed_cloud_pass = b.password_edit(
        st,
        "",
        IDC_SET_CLOUD_PASS,
        sec1.field_x(),
        sec1.row_y(2),
        sec1.field_w(),
    );
    b.label(
        st,
        "远程目录：",
        sec1.left(),
        sec1.label_y(3, line_h),
        sec1.label_w(),
        line_h,
    );
    st.ed_cloud_dir = b.edit(
        st,
        "",
        IDC_SET_CLOUD_DIR,
        sec1.field_x(),
        sec1.row_y(3),
        sec1.field_w(),
    );

    let btn_w = settings_scale(130);
    let gap = settings_scale(14);
    let x0 = sec2.left();
    let x1 = x0 + btn_w + gap;
    let btn_sync = b.button(
        st,
        "立即同步",
        IDC_SET_CLOUD_SYNC_NOW,
        x0,
        sec2.row_y(0),
        btn_w,
    );
    let btn_upload = b.button(
        st,
        "上传配置",
        IDC_SET_CLOUD_UPLOAD_CFG,
        x1,
        sec2.row_y(0),
        btn_w,
    );
    let btn_apply = b.button(
        st,
        "应用云端配置",
        IDC_SET_CLOUD_APPLY_CFG,
        x0,
        sec2.row_y(1),
        btn_w,
    );
    let btn_restore = b.button(
        st,
        "云备份恢复",
        IDC_SET_CLOUD_RESTORE_BACKUP,
        x1,
        sec2.row_y(1),
        btn_w,
    );
    for &hh in &[btn_sync, btn_upload, btn_apply, btn_restore] {
        if !hh.is_null() {
            st.ownerdraw_ctrls.push(hh);
        }
    }

    st.ui.mark_built(page);
}

pub(super) unsafe fn settings_create_about_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::About.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    let sec = SettingsFormSectionLayout::new(page, 0, 96);
    let update_state = update_check_state_snapshot();
    let mut y = sec.row_y(0);
    let version_text = format!("{}{}", tr("版本：", "Version: "), env!("CARGO_PKG_VERSION"));
    let (_, version_h) = b.label_auto(
        st,
        &version_text,
        sec.left(),
        y,
        sec.full_w(),
        settings_scale(28),
    );
    y += version_h + settings_scale(8);

    let summary_text = format!(
        "{}\r\n{}",
        tr(
            "设置界面现在统一使用同一套 section/form 布局。",
            "The settings window now uses a unified section/form layout.",
        ),
        tr(
            "新增设置项时可以直接复用卡片、字段列、按钮行和统一间距。",
            "New settings can reuse the same cards, field columns, action rows, and spacing.",
        )
    );
    let (_, summary_h) = b.label_auto(
        st,
        &summary_text,
        sec.left(),
        y,
        sec.full_w(),
        settings_scale(72),
    );
    y += summary_h + settings_scale(10);

    let source_label_w = sec.label_w();
    let source_row_h = settings_scale(34);
    b.label(
        st,
        tr("开源地址：", "Source: "),
        sec.left(),
        y + settings_scale(2),
        source_label_w,
        settings_scale(24),
    );
    let link = b.button(
        st,
        open_source_url_display(),
        IDC_SET_OPEN_SOURCE,
        sec.field_x(),
        y,
        sec.field_w(),
    );
    if !link.is_null() {
        st.ownerdraw_ctrls.push(link);
    }
    y += source_row_h + settings_scale(10);

    let update_text = if update_state.checking {
        tr("检查更新中…", "Checking for updates...").to_string()
    } else if !update_state.started {
        tr(
            "点击下方按钮后再检查更新。",
            "Click the button below to check for updates.",
        )
        .to_string()
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
        format!(
            "{} {}",
            tr("更新检查失败：", "Update check failed: "),
            update_state.error
        )
    } else {
        tr(
            "当前已经是最新版本。",
            "You are already on the latest version.",
        )
        .to_string()
    };
    let (_, update_h) = b.label_auto(
        st,
        &update_text,
        sec.left(),
        y,
        sec.full_w(),
        settings_scale(44),
    );
    y += update_h + settings_scale(8);
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
        settings_scale(184),
    );
    if !st.btn_open_update.is_null() {
        st.ownerdraw_ctrls.push(st.btn_open_update);
    }
    y += settings_scale(42);

    let info_text = format!(
        "{}{}\r\n{}{}",
        tr("数据目录：", "Data directory: "),
        data_dir().to_string_lossy(),
        tr("数据库：", "Database: "),
        db_file().to_string_lossy(),
    );
    let _ = b.label_auto(
        st,
        &info_text,
        sec.left(),
        y,
        sec.full_w(),
        settings_scale(72),
    );
    st.ui.mark_built(page);
}

pub(super) unsafe fn settings_button_hover(st: &SettingsWndState, hwnd_item: HWND) -> bool {
    if hwnd_item.is_null() {
        return false;
    }
    let mut pt: POINT = zeroed();
    if GetCursorPos(&mut pt) == 0 {
        return false;
    }
    let mut rc: RECT = zeroed();
    if GetWindowRect(hwnd_item, &mut rc) == 0 {
        return false;
    }
    pt.x >= rc.left
        && pt.x < rc.right
        && pt.y >= rc.top
        && pt.y < rc.bottom
        && st.hot_ownerdraw == hwnd_item
}

pub(super) unsafe fn settings_ensure_page(hwnd: HWND, st: &mut SettingsWndState, page: usize) {
    let page = page.min(SETTINGS_PAGES.len().saturating_sub(1));
    if st.ui.is_built(page) {
        return;
    }
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

    if cid == IDC_SET_AUTOSTART
        || cid == IDC_SET_SILENTSTART
        || cid == IDC_SET_TRAYICON
        || cid == IDC_SET_CLOSETRAY
        || cid == IDC_SET_CLICK_HIDE
        || cid == IDC_SET_PASTE_MOVE_TOP
        || cid == IDC_SET_DEDUPE_FILTER
        || cid == IDC_SET_PERSIST_SEARCH
        || cid == IDC_SET_PASTE_SOUND_ENABLE
        || cid == IDC_SET_AUTOHIDE_BLUR
        || cid == IDC_SET_EDGEHIDE
        || cid == IDC_SET_HOVERPREVIEW
        || cid == IDC_SET_VV_MODE
        || cid == IDC_SET_IMAGE_PREVIEW
        || cid == IDC_SET_QUICK_DELETE
        || cid == IDC_SET_GROUP_ENABLE
        || cid == IDC_SET_CLOUD_ENABLE
        || cid == 6101
        || cid == 7102
        || cid == 7101
        || cid == 7103
        || cid == 7104
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
        let font_px = scale_for_window(dis.hwndItem, 14).max(12);
        let font = CreateFontW(
            -font_px,
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
        || cid == IDC_SET_PASTE_SOUND_KIND
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
        if !state.settings.edge_auto_hide {
            position_main_window(hwnd, &state.settings, false);
        } else if IsWindowVisible(hwnd) != 0 {
            clear_edge_dock_state(state);
            note_window_moved_for_edge_hide(hwnd, state);
        }
    }
    if old_edge_hide && !state.settings.edge_auto_hide {
        restore_edge_hidden_window(hwnd, state);
    }
    reload_state_from_db_persisting(state);
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
    refresh_low_level_input_hooks();
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
    reload_state_from_db_persisting(state);
    layout_children(hwnd);
    InvalidateRect(hwnd, null(), 1);
    refresh_low_level_input_hooks();
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
