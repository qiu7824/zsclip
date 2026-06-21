use super::prelude::*;

static TASKBAR_CREATED_MESSAGE: OnceLock<u32> = OnceLock::new();

pub(super) fn taskbar_created_message() -> u32 {
    *TASKBAR_CREATED_MESSAGE
        .get_or_init(|| platform_window::register_window_message("TaskbarCreated"))
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

    if state.settings.plain_paste_hotkey_enabled && !state.plain_paste_hotkey_registered {
        register_plain_paste_hotkey_for(hwnd, state);
    }

    if state.settings.vv_mode_enabled && !vv_hook_registered() {
        let _ = update_vv_mode_hook(hwnd, true);
    }

    if !state.clipboard_listener_registered {
        register_clipboard_listener_for(hwnd, state);
    }

    let tray_ready =
        !tray_mode_enabled(&state.settings) || state.icons.app == 0 || state.tray_icon_registered;
    let hotkey_ready = !state.settings.hotkey_enabled || state.hotkey_registered;
    let plain_hotkey_ready =
        !state.settings.plain_paste_hotkey_enabled || state.plain_paste_hotkey_registered;
    let vv_ready = !state.settings.vv_mode_enabled || vv_hook_registered();
    let clipboard_ready = state.clipboard_listener_registered;
    if tray_ready && hotkey_ready && plain_hotkey_ready && vv_ready && clipboard_ready {
        state.startup_recovery_ticks = 0;
    } else {
        state.startup_recovery_ticks = state.startup_recovery_ticks.saturating_sub(1);
    }
}

fn startup_integrations_need_retry(state: &AppState) -> bool {
    state.role == WindowRole::Main
        && ((tray_mode_enabled(&state.settings)
            && state.icons.app != 0
            && !state.tray_icon_registered)
            || (state.settings.hotkey_enabled && !state.hotkey_registered)
            || (state.settings.plain_paste_hotkey_enabled && !state.plain_paste_hotkey_registered)
            || (state.settings.vv_mode_enabled && !vv_hook_registered())
            || !state.clipboard_listener_registered)
}

pub(super) fn arm_startup_recovery_if_needed(state: &mut AppState) {
    if startup_integrations_need_retry(state) {
        state.startup_recovery_ticks = STARTUP_RECOVERY_TICKS;
    }
}

pub(super) unsafe fn notify_update_state_changed() {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() {
            continue;
        }
        platform_window::post_hwnd_message(hwnd, WM_UPDATE_CHECK_READY, 0, 0);
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() && platform_window::exists((*ptr).settings_hwnd) {
            platform_gdi::invalidate_rect((*ptr).settings_hwnd, null(), 1);
        }
    }
}
