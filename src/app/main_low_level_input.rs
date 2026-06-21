use super::prelude::*;

unsafe fn screen_point_hits_window_scope(hwnd: HWND, pt: POINT) -> bool {
    if !platform_window::exists(hwnd) || !platform_window::is_visible(hwnd) {
        return false;
    }
    if platform_window::point_in_rect_screen(&pt, &platform_window::dock_rect(hwnd)) {
        return true;
    }
    platform_window::cursor_over_window_tree(hwnd, pt)
}

unsafe fn window_class_name(hwnd: HWND) -> String {
    platform_window::class_name(hwnd)
}

unsafe fn screen_point_hits_popup_menu(pt: POINT) -> bool {
    let hwnd = platform_window::window_from_point(pt);
    if hwnd.is_null() {
        return false;
    }
    let root = platform_window::root_ancestor(hwnd);
    let target = if root.is_null() { hwnd } else { root };
    window_class_name(target) == "#32768"
}

unsafe fn any_visible_window_requires_quick_escape_try() -> bool {
    for hwnd in window_host_hwnds_try() {
        if !platform_window::is_visible(hwnd) {
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
                    platform_window::user_data((*ptr).settings_hwnd) as *mut SettingsWndState;
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
    if !platform_window::exists(hwnd) || !platform_window::is_visible(hwnd) {
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
        if !platform_window::exists(hwnd) {
            continue;
        }
        if window_needs_outside_hide_timer(hwnd) {
            timer::start(hwnd, ID_TIMER_OUTSIDE_HIDE, 120);
        } else {
            timer::stop(hwnd, ID_TIMER_OUTSIDE_HIDE);
        }
    }
}

unsafe fn window_needs_edge_auto_hide_timer(hwnd: HWND) -> bool {
    if !platform_window::exists(hwnd) || !platform_window::is_visible(hwnd) {
        return false;
    }
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return false;
    }
    if platform_window::is_visible((*ptr).settings_hwnd) {
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
        if !platform_window::exists(hwnd) {
            continue;
        }
        if window_needs_edge_auto_hide_timer(hwnd) {
            timer::start(
                hwnd,
                ID_TIMER_EDGE_AUTO_HIDE,
                edge_auto_hide_timer_interval(hwnd),
            );
        } else {
            timer::stop(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
        }
    }
}

unsafe extern "system" fn quick_escape_keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some(event) = platform_hook::keyboard_event(code, wparam, lparam) else {
        return platform_hook::call_next(code, wparam, lparam);
    };
    if event.is_injected() {
        return platform_hook::call_next(code, wparam, lparam);
    }

    let [main, quick] = window_host_hwnds_try();

    let ctrl_down = platform_hotkey::control_pressed();
    if platform_hotkey::is_find_vk(event.vk_code) && ctrl_down {
        if platform_window::is_visible(quick) {
            platform_window::post_hwnd_message(
                quick,
                WM_KEYDOWN,
                platform_hotkey::find_wparam(),
                0,
            );
            return 1;
        }
        if platform_window::is_visible(main) {
            let ptr = get_state_ptr(main);
            if !ptr.is_null() && (*ptr).main_window_noactivate {
                platform_window::post_hwnd_message(
                    main,
                    WM_KEYDOWN,
                    platform_hotkey::find_wparam(),
                    0,
                );
                return 1;
            }
        }
        return platform_hook::call_next(code, wparam, lparam);
    }

    if !platform_hotkey::is_escape_vk(event.vk_code) {
        return platform_hook::call_next(code, wparam, lparam);
    }
    if platform_window::is_visible(quick) {
        platform_window::post_hwnd_message(quick, WM_KEYDOWN, platform_hotkey::escape_wparam(), 0);
        return 1;
    }

    if platform_window::is_visible(main) {
        let ptr = get_state_ptr(main);
        if !ptr.is_null() && (*ptr).main_window_noactivate {
            platform_window::post_hwnd_message(
                main,
                WM_KEYDOWN,
                platform_hotkey::escape_wparam(),
                0,
            );
            return 1;
        }
    }

    platform_hook::call_next(code, wparam, lparam)
}

pub(super) unsafe fn ensure_quick_escape_keyboard_hook() {
    let Ok(mut handle) = quick_escape_keyboard_hook_handle().lock() else {
        return;
    };
    if *handle == 0 {
        *handle = platform_hook::install_low_level_keyboard(Some(quick_escape_keyboard_hook_proc));
    }
}

unsafe fn disable_quick_escape_keyboard_hook() {
    let Ok(mut handle) = quick_escape_keyboard_hook_handle().lock() else {
        return;
    };
    if *handle != 0 {
        platform_hook::uninstall(*handle);
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
        if platform_window::exists(hwnd) {
            timer::stop(hwnd, ID_TIMER_OUTSIDE_HIDE);
            timer::stop(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
        }
    }
    disable_quick_escape_keyboard_hook();
}

pub(super) unsafe fn handle_outside_hide_tick(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() || !platform_window::is_visible(hwnd) {
        return;
    }
    let state = &mut *ptr;
    if !state.settings.auto_hide_on_blur {
        timer::stop(hwnd, ID_TIMER_OUTSIDE_HIDE);
        return;
    }
    if !(state.role == WindowRole::Quick || state.main_window_noactivate) {
        return;
    }
    if vv_popup_menu_active() {
        return;
    }
    if let Some(pt) = platform_input::cursor_pos() {
        if should_ignore_outside_click_for_point(pt) {
            return;
        }
    }
    if !platform_input::any_mouse_button_down() {
        return;
    }
    hide_hover_preview();
    platform_window::hide(hwnd);
    refresh_low_level_input_hooks();
}
