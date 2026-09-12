use super::prelude::*;

pub(super) unsafe fn settings_collect_current_page_to_draft(st: &mut SettingsWndState) {
    if !st.ui.is_built(st.cur_page) {
        return;
    }
    match SettingsPage::from_index(st.cur_page) {
        SettingsPage::General => settings_collect_general_to_draft(st),
        SettingsPage::Hotkey => settings_collect_hotkey_to_draft(st),
        SettingsPage::Plugin => settings_collect_plugin_to_draft(st),
        SettingsPage::Group => settings_collect_group_to_draft(st),
        SettingsPage::Cloud => settings_collect_cloud_to_draft(st),
        SettingsPage::About => {}
    }
}

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
