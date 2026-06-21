use super::prelude::*;

pub(super) unsafe fn settings_commit_collected_app_settings(
    st: &mut SettingsWndState,
    app: &mut AppState,
) {
    let baseline = SettingsAppEffectBaseline::capture(app);
    crate::lan_sync::ensure_device_identity(&mut st.draft);
    app.settings = st.draft.clone();
    if app.settings.edge_auto_hide {
        if let Some(rc) = platform_window::window_rect(st.parent_hwnd) {
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
    if baseline.grouping_enabled != app.settings.grouping_enabled {
        app.clear_selection();
    }
    settings_refresh_integrations_after_commit(st, app, &baseline);
    settings_refresh_data_after_commit(st, app);
    settings_refresh_windows_after_commit(st, app, &baseline);
}
