use super::prelude::*;

pub(super) fn settings_multi_sync_layout(
    st: &SettingsWndState,
    index: usize,
    label_w: i32,
) -> SettingsFormSectionLayout {
    crate::settings_model::settings_form_layout_for_section(
        SettingsPage::Cloud.index(),
        index,
        label_w,
        &st.multi_sync_sections,
    )
}

unsafe fn settings_reset_cloud_page_handles(st: &mut SettingsWndState) {
    st.cb_multi_sync_mode = null_mut();
    st.cb_cloud_interval = null_mut();
    st.lb_multi_sync_summary = null_mut();
    st.ed_cloud_url = null_mut();
    st.ed_cloud_user = null_mut();
    st.ed_cloud_pass = null_mut();
    st.ed_cloud_dir = null_mut();
    st.lb_cloud_status = null_mut();
    st.ed_lan_name = null_mut();
    st.ed_lan_tcp_port = null_mut();
    st.lb_lan_status = null_mut();
    st.cb_lan_receive_mode = null_mut();
    st.ed_lan_manual_host = null_mut();
    st.btn_lan_pair = null_mut();
    st.btn_lan_refresh = null_mut();
    st.btn_lan_docs = null_mut();
    st.qr_lan_android_bounds = UiRect::new(0, 0, 0, 0);
    st.qr_lan_ios_bounds = UiRect::new(0, 0, 0, 0);
    st.btn_lan_copy_pair = null_mut();
    st.btn_lan_copy_setup = null_mut();
    st.lb_lan_devices = null_mut();
    st.lb_lan_trusted = null_mut();
    st.btn_lan_accept_pair = null_mut();
    st.btn_lan_reject_pair = null_mut();
    st.ownerdraw_ctrls
        .retain(|hwnd| platform_window::exists(*hwnd));
}

pub(super) unsafe fn settings_refresh_multi_sync_cards(st: &mut SettingsWndState) {
    let mode = multi_sync_mode_from_settings(&st.draft);
    st.multi_sync_sections = crate::settings_model::settings_multi_sync_cards_for_mode(mode);
}

pub(super) unsafe fn settings_rebuild_cloud_page(hwnd: HWND, st: &mut SettingsWndState) {
    let page = SettingsPage::Cloud.index();
    platform_window::send_message(hwnd, WM_SETREDRAW, 0, 0);
    set_settings_viewport_child_visible(st.viewport_hwnd, false);
    st.ui.clear_page(page);
    settings_reset_cloud_page_handles(st);
    settings_create_cloud_page(hwnd, st);
    settings_sync_page_state(st, page);
    if st.cur_page == page {
        for reg in st.ui.page_regs(page) {
            if !reg.hwnd.is_null() {
                settings_host_set_visible(reg.hwnd, reg.visible);
            }
        }
        settings_repos_controls(hwnd, st, true);
    }
    set_settings_viewport_child_visible(st.viewport_hwnd, true);
    platform_window::send_message(hwnd, WM_SETREDRAW, 1, 0);
    platform_gdi::invalidate_rect(hwnd, null(), 1);
    platform_gdi::redraw_window(
        hwnd,
        null(),
        null_mut(),
        RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
    );
}
