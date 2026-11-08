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

    let hotkey_conflict = state.settings.hotkey_enabled
        && !state.hotkey_registered
        && try_register_hotkey_for(hwnd, state) == Err(hotkey::ERROR_HOTKEY_ALREADY_REGISTERED);

    if state.settings.plain_paste_hotkey_enabled && !state.plain_paste_hotkey_registered {
        register_plain_paste_hotkey_for(hwnd, state);
    }

    if state.settings.vv_mode_enabled && !vv_hook_registered() {
        let _ = update_vv_mode_hook(hwnd, true);
    }

    if !state.clipboard_listener_registered {
        register_clipboard_listener_for(hwnd, state);
    }

    if startup_integrations_need_retry(state) {
        state.startup_recovery_ticks -= 1;
    } else {
        state.startup_recovery_ticks = 0;
    }
    if state.startup_recovery_ticks != 0 {
        return;
    }

    timer::stop(hwnd, ID_TIMER_STARTUP_RECOVERY);
    if hotkey_conflict && state.tray_icon_registered && !state.hotkey_conflict_notified {
        state.hotkey_conflict_notified = true;
        crate::platform::tray_icon::notify(
            hwnd, TRAY_UID, translate("快捷键冲突").as_ref(), &hotkey_conflict_message(state),
        );
    }
    if startup_can_hide(&state.settings)
        && !state.tray_icon_registered
        && !state.hotkey_registered
        && !startup_mouse_binding_available(&state.settings)
        && !platform_window::is_visible(hwnd)
    {
        platform_window::show_no_activate(hwnd);
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
        timer::start(state.hwnd, ID_TIMER_STARTUP_RECOVERY, 500);
    }
}

pub(super) unsafe fn notify_update_state_changed() {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() {
            continue;
        }
        platform_window::post_hwnd_message(hwnd, WM_UPDATE_CHECK_READY, 0, 0);
    }
}
