use super::prelude::*;

pub(super) unsafe fn settings_create_cloud_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Cloud.index();
    let b = SettingsPageBuilder {
        hwnd,
        page,
        font: st.ui_font,
    };
    settings_refresh_multi_sync_cards(st);
    let mode = multi_sync_mode_from_settings(&st.draft);
    let sec0 = settings_multi_sync_layout(st, 0, 110);
    let line_h = settings_scale(24);
    let lan_btn_w = settings_scale(150);
    let small_btn_w = settings_scale(96);

    b.form_label(st, &sec0, 0, "同步方案：");
    st.cb_multi_sync_mode = b.form_dropdown(
        st,
        &sec0,
        0,
        multi_sync_mode_display(mode),
        IDC_SET_MULTI_SYNC_MODE,
        settings_scale(150),
    );
    if !st.cb_multi_sync_mode.is_null() {
        st.ownerdraw_ctrls.push(st.cb_multi_sync_mode);
    }
    st.lb_multi_sync_summary = b.label(
        st,
        "多端同步：未选择同步方案。",
        sec0.left(),
        sec0.label_y(1, line_h),
        sec0.full_w(),
        line_h,
    );
    if mode == "webdav" {
        settings_create_cloud_webdav_page(st, &b, line_h);
    }

    if mode == "lan" {
        settings_create_cloud_lan_page(st, &b, line_h, lan_btn_w, small_btn_w);
    }

    st.ui.mark_built(page);
}
