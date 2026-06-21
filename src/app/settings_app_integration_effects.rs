use super::prelude::*;

pub(super) unsafe fn settings_refresh_integrations_after_commit(
    st: &mut SettingsWndState,
    app: &mut AppState,
    baseline: &SettingsAppEffectBaseline,
) {
    if baseline.auto_start != app.settings.auto_start {
        app.settings.auto_start = apply_autostart(app.settings.auto_start);
        st.draft.auto_start = app.settings.auto_start;
        save_settings(&app.settings);
    }
    if baseline.tray_icon_enabled != app.settings.tray_icon_enabled {
        let main_hwnd = main_window_hwnd();
        if !main_hwnd.is_null() {
            sync_main_tray_icon(main_hwnd, app);
        }
    }
    let hotkey_new = format!(
        "{}+{}+{}",
        app.settings.hotkey_enabled, app.settings.hotkey_mod, app.settings.hotkey_key
    );
    if baseline.hotkey != hotkey_new {
        register_hotkey_for(st.parent_hwnd, app);
    }
    let plain_hotkey_new = format!(
        "{}+{}+{}",
        app.settings.plain_paste_hotkey_enabled,
        app.settings.plain_paste_hotkey_mod,
        app.settings.plain_paste_hotkey_key
    );
    if baseline.plain_hotkey != plain_hotkey_new {
        register_plain_paste_hotkey_for(st.parent_hwnd, app);
    }
    if baseline.vv_mode_enabled != app.settings.vv_mode_enabled {
        let _ = update_vv_mode_hook(st.parent_hwnd, app.settings.vv_mode_enabled);
        if !app.settings.vv_mode_enabled {
            vv_popup_hide(st.parent_hwnd, app);
        }
    }
    arm_startup_recovery_if_needed(app);
}
