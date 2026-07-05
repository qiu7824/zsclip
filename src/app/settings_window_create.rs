use super::prelude::*;

pub(super) unsafe fn create_settings_window_state(
    hwnd: HWND,
    parent_hwnd: HWND,
) -> Box<SettingsWndState> {
    set_settings_ui_dpi(settings_window_layout_dpi(hwnd));
    let (nav_font, ui_font, title_font) = create_settings_fonts(hwnd);
    let mut st = Box::new(SettingsWndState::new(
        parent_hwnd,
        settings_window_layout_dpi(hwnd).max(96),
        nav_font,
        ui_font,
        title_font,
    ));

    st.viewport_hwnd = create_settings_viewport_child(hwnd);
    sync_settings_viewport_child_bounds(hwnd, st.viewport_hwnd);
    settings_refresh_theme_resources(&mut st);
    st.btn_save = settings_create_btn(hwnd, "保存", IDC_SET_SAVE, 984, 24, 72, st.ui_font);
    st.btn_close = settings_create_btn(hwnd, "关闭", IDC_SET_CLOSE, 900, 24, 64, st.ui_font);
    for &control in &[st.btn_save, st.btn_close] {
        if !control.is_null() {
            st.ownerdraw_ctrls.push(control);
        }
    }
    refresh_settings_window_metrics(hwnd, &mut st);
    settings_ensure_page(hwnd, &mut st, SettingsPage::General.index());
    settings_apply_from_app(&mut st);
    settings_show_page(hwnd, &mut st, 0);
    st
}
