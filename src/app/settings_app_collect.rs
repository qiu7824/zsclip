use super::prelude::*;

pub(super) unsafe fn settings_collect_to_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() {
        return;
    }
    settings_collect_general_to_draft(st);
    settings_collect_hotkey_to_draft(st);
    settings_collect_plugin_to_draft(st);
    settings_collect_group_to_draft(st);
    settings_collect_cloud_to_draft(st);
    settings_commit_collected_app_settings(st, &mut *pst);
}
