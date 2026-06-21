use super::prelude::*;

pub(super) unsafe fn settings_ensure_page(hwnd: HWND, st: &mut SettingsWndState, page: usize) {
    let page = page.min(SETTINGS_PAGE_LABELS.len().saturating_sub(1));
    if st.ui.is_built(page) {
        return;
    }
    match SettingsPage::from_index(page) {
        SettingsPage::General => {
            settings_create_general_page(hwnd, st);
            st.ui.mark_built(page);
        }
        SettingsPage::Hotkey => settings_create_hotkey_page(hwnd, st),
        SettingsPage::Plugin => settings_create_plugin_page(hwnd, st),
        SettingsPage::Group => settings_create_group_page(hwnd, st),
        SettingsPage::Cloud => settings_create_cloud_page(hwnd, st),
        SettingsPage::About => settings_create_about_page(hwnd, st),
    }
}
