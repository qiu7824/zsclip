use super::prelude::*;
use crate::platform::gdi as platform_gdi;
use crate::settings_model::lan_receive_mode_display;
use crate::win_system_ui::settings_host_set_enabled;

pub(super) unsafe fn settings_sync_cloud_lan_state(st: &mut SettingsWndState, lan_enabled: bool) {
    let s = &st.draft;
    settings_set_text(st.ed_lan_name, &s.lan_device_name);
    settings_set_text(st.ed_lan_tcp_port, &s.lan_tcp_port.to_string());
    settings_set_text(st.ed_lan_manual_host, &s.lan_manual_host);
    settings_set_text(
        st.cb_lan_receive_mode,
        lan_receive_mode_display(&s.lan_receive_mode),
    );
    settings_set_text(st.lb_lan_status, &crate::lan_sync::status_summary(s));
    settings_set_text(st.lb_lan_trusted, &lan_trusted_value_text());
    settings_lan_refresh_lists(st);
    for hwnd in [
        st.ed_lan_name,
        st.ed_lan_tcp_port,
        st.ed_lan_manual_host,
        st.cb_lan_receive_mode,
        st.btn_lan_pair,
        st.btn_lan_refresh,
        st.lb_lan_devices,
        st.btn_lan_accept_pair,
        st.btn_lan_reject_pair,
        st.btn_lan_docs,
        st.qr_lan_android,
        st.qr_lan_ios,
        st.btn_lan_copy_pair,
        st.btn_lan_copy_setup,
    ] {
        if !hwnd.is_null() {
            settings_host_set_enabled(hwnd, lan_enabled);
            platform_gdi::invalidate_rect(hwnd, null(), 1);
        }
    }
}

fn lan_trusted_value_text() -> String {
    lan_trusted_summary_value_text(&crate::lan_sync::trusted_summary())
}
