use super::prelude::*;

pub(super) unsafe fn refresh_settings_window_from_app(app: &mut AppState) {
    if !platform_window::exists(app.settings_hwnd) {
        return;
    }
    let st_ptr = platform_window::user_data(app.settings_hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        settings_apply_from_app(&mut *st_ptr);
        platform_gdi::invalidate_rect(app.settings_hwnd, null(), 1);
    }
}

pub(super) unsafe fn apply_loaded_settings(hwnd: HWND, state: &mut AppState) {
    crate::db_runtime::with_shared_app_data(|| apply_loaded_settings_locked(hwnd, state));
}

unsafe fn apply_loaded_settings_locked(hwnd: HWND, state: &mut AppState) {
    let old_edge_hide = state.settings.edge_auto_hide;
    let (mut loaded, app_data_generation) = load_settings_with_generation();
    loaded.auto_start = is_autostart_enabled();
    #[cfg(not(feature = "lan-sync"))]
    {
        loaded.lan_sync_enabled = false;
    }
    #[cfg(feature = "lan-sync")]
    crate::lan_sync::ensure_device_identity(&mut loaded);
    settings_normalize_multi_sync_mode(&mut loaded);
    state.settings = loaded;
    platform_appearance::set_dark_mode_enabled(state.settings.dark_mode_enabled);
    state.theme = Theme::default();
    refresh_search_theme_resources(state);
    WindowsMainWindowHost::new(Some(wnd_proc)).apply_main_window_appearance(hwnd);
    state.app_data_generation = app_data_generation;
    save_state_settings(state);
    schedule_cloud_sync(state, false);
    #[cfg(feature = "lan-sync")]
    refresh_lan_latest_from_db(&state.settings);
    #[cfg(feature = "lan-sync")]
    crate::lan_sync::refresh_service(hwnd, &state.settings);
    if state.role == WindowRole::Main {
        sync_main_tray_icon(hwnd, state);
        register_hotkey_for(hwnd, state);
        register_plain_paste_hotkey_for(hwnd, state);
        let _ = update_vv_mode_hook(hwnd, state.settings.vv_mode_enabled);
        register_clipboard_listener_for(hwnd, state);
        arm_startup_recovery_if_needed(state);
        if !state.settings.edge_auto_hide {
            position_main_window(hwnd, &state.settings, false);
        } else if platform_window::is_visible(hwnd) {
            clear_edge_dock_state(state);
            note_window_moved_for_edge_hide(hwnd, state);
        }
    }
    if old_edge_hide && !state.settings.edge_auto_hide {
        restore_edge_hidden_window(hwnd, state);
    }
    reload_state_from_db_persisting(state);
    layout_children(hwnd);
    platform_gdi::invalidate_rect(hwnd, null(), 1);
    refresh_low_level_input_hooks();
    refresh_settings_window_from_app(state);
}

pub(super) unsafe fn refresh_window_state(hwnd: HWND, reload_settings: bool) {
    crate::db_runtime::with_shared_app_data(|| refresh_window_state_locked(hwnd, reload_settings));
}

unsafe fn refresh_window_state_locked(hwnd: HWND, reload_settings: bool) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    if reload_settings {
        let (settings, app_data_generation) = load_settings_with_generation();
        state.settings = settings;
        platform_appearance::set_dark_mode_enabled(state.settings.dark_mode_enabled);
        state.theme = Theme::default();
        refresh_search_theme_resources(state);
        WindowsMainWindowHost::new(Some(wnd_proc)).apply_main_window_appearance(hwnd);
        state.app_data_generation = app_data_generation;
        state.settings.auto_start = is_autostart_enabled();
        #[cfg(not(feature = "lan-sync"))]
        {
            state.settings.lan_sync_enabled = false;
        }
        settings_normalize_multi_sync_mode(&mut state.settings);
        schedule_cloud_sync(state, false);
        if state.role == WindowRole::Main {
            sync_main_tray_icon(hwnd, state);
            register_hotkey_for(hwnd, state);
            register_plain_paste_hotkey_for(hwnd, state);
            let _ = update_vv_mode_hook(hwnd, state.settings.vv_mode_enabled);
            register_clipboard_listener_for(hwnd, state);
            arm_startup_recovery_if_needed(state);
        }
    }
    reload_state_from_db_persisting(state);
    layout_children(hwnd);
    platform_gdi::invalidate_rect(hwnd, null(), 1);
    refresh_low_level_input_hooks();
}

pub(super) unsafe fn sync_peer_windows_from_db(source_hwnd: HWND) {
    for target in window_host_hwnds() {
        if target == source_hwnd || !platform_window::exists(target) {
            continue;
        }
        refresh_window_state(target, false);
    }
}

pub(super) unsafe fn sync_peer_windows_from_settings(source_hwnd: HWND) {
    for target in window_host_hwnds() {
        if target == source_hwnd || !platform_window::exists(target) {
            continue;
        }
        refresh_window_state(target, true);
    }
}

pub(crate) unsafe fn refresh_window_for_show(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    cancel_hidden_memory_reclaim(hwnd, state);
    state.refilter();
    layout_children(hwnd);
    platform_gdi::invalidate_rect(hwnd, null(), 1);
}
