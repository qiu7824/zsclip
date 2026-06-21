use super::prelude::*;

pub(super) unsafe fn settings_refresh_data_after_commit(st: &SettingsWndState, app: &mut AppState) {
    schedule_cloud_sync(app, false);
    refresh_lan_latest_from_db(&app.settings);
    crate::lan_sync::refresh_service(st.parent_hwnd, &app.settings);
    let new_max = app.settings.max_items;
    let mut reload_needed = false;
    if new_max > 0 {
        db_prune_items(0, new_max);
        reload_needed = true;
    }
    if db_reconcile_dedupe_signatures(0, !app.settings.dedupe_filter_enabled)
        .map(|removed| removed > 0)
        .unwrap_or(false)
    {
        reload_needed = true;
    }
    if reload_needed {
        reload_state_from_db_persisting(app);
    }
    refresh_lan_latest_from_db(&app.settings);
}
