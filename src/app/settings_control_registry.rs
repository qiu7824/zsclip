use super::prelude::*;

pub(super) unsafe fn settings_register_ctrl(
    st: &mut SettingsWndState,
    page: usize,
    hwnd: HWND,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    scrollable: bool,
) {
    if hwnd.is_null() {
        return;
    }
    st.ui
        .register(SettingsCtrlReg::new(hwnd, page, x, y, w, h, scrollable));
}

pub(super) fn settings_page_control_scrollable(st: &SettingsWndState, page: usize) -> bool {
    match SettingsPage::from_index(page) {
        SettingsPage::Plugin => !st.plugin_sections.is_empty(),
        SettingsPage::Cloud => !st.multi_sync_sections.is_empty(),
        _ => crate::settings_model::settings_page_scrollable(page),
    }
}

pub(super) unsafe fn settings_page_push_ctrl(
    st: &mut SettingsWndState,
    page: usize,
    hwnd: HWND,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    let scrollable = settings_page_control_scrollable(st, page);
    settings_register_ctrl(st, page, hwnd, x, y, w, h, scrollable);
}
