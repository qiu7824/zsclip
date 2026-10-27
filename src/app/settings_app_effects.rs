use super::prelude::*;

pub(super) unsafe fn settings_commit_collected_app_settings(
    st: &mut SettingsWndState,
    app: &mut AppState,
) {
    let expected_generation = st.app_data_generation;
    if app.app_data_generation != expected_generation
        || crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
            settings_commit_collected_app_settings_locked(st, app);
        })
        .is_none()
    {
        apply_loaded_settings(st.parent_hwnd, app);
        settings_apply_from_app(st);
    }
}

unsafe fn settings_commit_collected_app_settings_locked(
    st: &mut SettingsWndState,
    app: &mut AppState,
) {
    let baseline = SettingsAppEffectBaseline::capture(app);
    let categories_changed=app.settings.category_tabs!=st.draft.category_tabs;
    let removed_tabs: Vec<String> = app.settings.category_tabs.iter()
        .filter(|old| !st.draft.category_tabs.iter().any(|next| next.id == old.id))
        .map(|tab| tab.id.clone()).collect();
    #[cfg(feature = "lan-sync")]
    crate::lan_sync::ensure_device_identity(&mut st.draft);
    #[cfg(not(feature = "lan-sync"))]
    {
        st.draft.lan_sync_enabled = false;
    }
    st.draft.clipboard_window_width = app.settings.clipboard_window_width;
    st.draft.clipboard_window_height = app.settings.clipboard_window_height;
    app.settings = st.draft.clone();
    if categories_changed { app.reconcile_category(); }
    if app.settings.edge_auto_hide {
        if let Some(rc) = platform_window::window_rect(st.parent_hwnd) {
            app.settings.last_window_x = rc.left;
            app.settings.last_window_y = rc.top;
            st.draft.last_window_x = rc.left;
            st.draft.last_window_y = rc.top;
        }
    }
    if !app.settings.grouping_enabled && app.active_category_id.is_empty() {
        app.current_group_filter = 0;
        app.tab_group_filters = [0, 0];
        remember_shared_tab_view_state(app);
    }
    save_state_settings(app);
    if !removed_tabs.is_empty() {
        // Only remove membership metadata once the saved configuration confirms deletion.
        if fs::read_to_string(settings_file()).ok()
            .and_then(|text| serde_json::from_str::<AppSettings>(&text).ok())
            .is_some_and(|saved| saved.category_tabs == app.settings.category_tabs) {
            let _ = crate::clip_tags::delete_memberships(&removed_tabs);
        }
    }
    if baseline.grouping_enabled != app.settings.grouping_enabled {
        app.clear_selection();
    }
    settings_refresh_integrations_after_commit(st, app, &baseline);
    settings_refresh_data_after_commit(st, app);
    settings_refresh_windows_after_commit(st, app, &baseline);
}
