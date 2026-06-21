use super::prelude::*;

const VV_TRIGGER_TIMEOUT_MS: u128 = 300;

pub(super) unsafe fn window_process_name(hwnd: HWND) -> String {
    WindowsWindowIdentityHost::new().process_name(hwnd)
}

pub(super) unsafe fn send_escape_key() {
    platform_input::tap_key(hotkey::escape_key_u8());
}

unsafe fn vv_target_is_ignored(hwnd: HWND, main_hwnd: HWND) -> bool {
    if hwnd.is_null() || hwnd == main_hwnd {
        return true;
    }
    let popup = current_vv_popup_hwnd();
    if hwnd == popup {
        return true;
    }
    WindowsWindowIdentityHost::new().is_current_process_window(hwnd)
}

pub(super) unsafe fn vv_window_class_name(hwnd: HWND) -> String {
    WindowsWindowIdentityHost::new().class_name(hwnd)
}

pub(super) fn vv_is_qq_wps_process(process_name: &str) -> bool {
    let process = process_name.trim().to_ascii_lowercase();
    matches!(
        process.as_str(),
        "qq.exe"
            | "qq"
            | "qqnt.exe"
            | "qqnt"
            | "tim.exe"
            | "tim"
            | "wps.exe"
            | "wps"
            | "wpp.exe"
            | "wpp"
            | "et.exe"
            | "et"
    )
}

fn vv_is_qq_process(process_name: &str) -> bool {
    let process = process_name.trim().to_ascii_lowercase();
    matches!(
        process.as_str(),
        "qq.exe" | "qq" | "qqnt.exe" | "qqnt" | "tim.exe" | "tim"
    )
}

fn vv_is_browser_process(process_name: &str) -> bool {
    source_app_is_browser(process_name)
}

fn vv_is_browser_window_class(class_name: &str) -> bool {
    let class_name = class_name.trim().to_ascii_lowercase();
    matches!(
        class_name.as_str(),
        "chrome_renderwidgethosthwnd"
            | "chrome_widgetwin_0"
            | "chrome_widgetwin_1"
            | "mozillawindowclass"
            | "windows.ui.composition.desktopwindowcontentbridge"
    )
}

pub(super) fn vv_backspace_count_for_target_identity(
    process_name: &str,
    root_process_name: &str,
    target_class_name: &str,
    replaces_ime: bool,
) -> u8 {
    if replaces_ime
        || vv_is_qq_process(process_name)
        || vv_is_qq_process(root_process_name)
        || vv_is_browser_process(process_name)
        || vv_is_browser_process(root_process_name)
        || vv_is_browser_window_class(target_class_name)
    {
        0
    } else {
        2
    }
}

pub(super) unsafe fn vv_backspace_count_for_target_window(target: HWND, replaces_ime: bool) -> u8 {
    let process_name = window_process_name(target);
    let root = WindowsWindowIdentityHost::new().root_handle(target);
    let root_process_name = if root.is_null() || root == target {
        String::new()
    } else {
        window_process_name(root)
    };
    let target_class_name = vv_window_class_name(target);
    vv_backspace_count_for_target_identity(
        &process_name,
        &root_process_name,
        &target_class_name,
        replaces_ime,
    )
}

unsafe fn vv_target_is_text_input_ready(target: HWND) -> bool {
    WindowsPasteTargetHost::new().paste_target_text_input_ready(target)
}

unsafe extern "system" fn vv_keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let Some(event) = platform_hook::keyboard_event(code, wparam, lparam) else {
        return platform_hook::call_next(code, wparam, lparam);
    };
    if event.is_injected_or_lower_integrity() {
        return platform_hook::call_next(code, wparam, lparam);
    }

    let Ok(mut hook) = vv_hook_state().try_lock() else {
        return platform_hook::call_next(code, wparam, lparam);
    };
    if !hook.enabled || hook.main_hwnd == 0 {
        return platform_hook::call_next(code, wparam, lparam);
    }
    let main_hwnd = hook.main_hwnd as HWND;
    let trigger_vk = hook.trigger_vk;
    let menu_active = hook.popup_menu_active
        || hook
            .popup_menu_grace_until
            .map(|until| until > Instant::now())
            .unwrap_or(false);

    let identity_host = WindowsWindowIdentityHost::new();
    let fg = identity_host.foreground_handle();
    if vv_target_is_ignored(fg, main_hwnd) {
        if hook.popup_active && menu_active {
            return platform_hook::call_next(code, wparam, lparam);
        }
        hook.last_was_v = false;
        hook.last_v_target = 0;
        hook.last_v_at = None;
        if hook.popup_active {
            hook.popup_active = false;
            hook.popup_target = 0;
            platform_window::post_hwnd_message(main_hwnd, WM_VV_HIDE, 0, 0);
        }
        return platform_hook::call_next(code, wparam, lparam);
    }

    let has_mod = hotkey::command_modifier_pressed();

    if hook.popup_active {
        if menu_active {
            return platform_hook::call_next(code, wparam, lparam);
        }
        if fg as isize != hook.popup_target {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_v_at = None;
            platform_window::post_hwnd_message(main_hwnd, WM_VV_HIDE, 0, 0);
            return platform_hook::call_next(code, wparam, lparam);
        }
        if let Some(idx) = hotkey::digit_index_1_to_9_from_vk(event.vk_code) {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            platform_window::post_hwnd_message(main_hwnd, WM_VV_SELECT, idx, 0);
            return 1;
        }
        if hotkey::is_escape_vk(event.vk_code) {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            platform_window::post_hwnd_message(main_hwnd, WM_VV_HIDE, 0, 0);
            return 1;
        }
        if hotkey::is_backspace_vk(event.vk_code) {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_v_at = None;
            platform_window::post_hwnd_message(main_hwnd, WM_VV_HIDE, 0, 0);
            return platform_hook::call_next(code, wparam, lparam);
        }
        if !hotkey::is_modifier_vk(event.vk_code) {
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            platform_window::post_hwnd_message(main_hwnd, WM_VV_HIDE, 0, 0);
        }
        return platform_hook::call_next(code, wparam, lparam);
    }

    if has_mod {
        hook.last_was_v = false;
        hook.last_v_target = 0;
        hook.last_v_at = None;
        return platform_hook::call_next(code, wparam, lparam);
    }

    if event.vk_code == trigger_vk {
        let within_timeout = hook
            .last_v_at
            .map(|t| t.elapsed().as_millis() <= VV_TRIGGER_TIMEOUT_MS)
            .unwrap_or(false);
        if hook.last_was_v && hook.last_v_target == fg as isize && within_timeout {
            let target = if identity_host.exists(fg) {
                fg
            } else {
                hook.last_v_target as HWND
            };
            hook.popup_active = false;
            hook.popup_target = 0;
            hook.last_was_v = false;
            hook.last_v_target = 0;
            hook.last_v_at = None;
            if identity_host.exists(target) && vv_target_is_text_input_ready(target) {
                platform_window::post_hwnd_message(main_hwnd, WM_VV_SHOW, target as usize, 0);
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
    } else if !hotkey::is_modifier_vk(event.vk_code) {
        hook.last_was_v = false;
        hook.last_v_target = 0;
        hook.last_v_at = None;
    }

    platform_hook::call_next(code, wparam, lparam)
}

pub(super) unsafe fn update_vv_mode_hook(main_hwnd: HWND, enabled: bool) -> bool {
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
        return false;
    };
    if enabled {
        if *handle == 0 {
            *handle = platform_hook::install_low_level_keyboard(Some(vv_keyboard_hook_proc));
        }
        *handle != 0
    } else if *handle != 0 {
        platform_hook::uninstall(*handle);
        *handle = 0;
        true
    } else {
        true
    }
}
