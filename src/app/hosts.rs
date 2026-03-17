use super::*;

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

unsafe extern "system" fn quick_escape_keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
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

pub(super) unsafe fn apply_loaded_settings(hwnd: HWND, state: &mut AppState) {
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
