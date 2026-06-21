use super::prelude::*;
use crate::win_system_ui::settings_host_text;

pub(super) unsafe fn settings_collect_cloud_to_draft(st: &mut SettingsWndState) {
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.cb_cloud_interval.is_null() {
        st.draft.cloud_sync_interval = {
            let label = settings_host_text(st.cb_cloud_interval);
            if label.trim().is_empty() {
                "1小时".to_string()
            } else {
                label
            }
        };
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.cb_multi_sync_mode.is_null() {
        let mode = multi_sync_mode_from_label(&settings_host_text(st.cb_multi_sync_mode));
        settings_apply_multi_sync_mode(&mut st.draft, mode);
    } else {
        settings_normalize_multi_sync_mode(&mut st.draft);
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_cloud_url.is_null() {
        st.draft.cloud_webdav_url = settings_host_text(st.ed_cloud_url);
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_cloud_user.is_null() {
        st.draft.cloud_webdav_user = settings_host_text(st.ed_cloud_user);
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_cloud_pass.is_null() {
        st.draft.cloud_webdav_pass = settings_host_text(st.ed_cloud_pass);
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_cloud_dir.is_null() {
        st.draft.cloud_remote_dir = {
            let dir = settings_host_text(st.ed_cloud_dir);
            if dir.trim().is_empty() {
                "ZSClip".to_string()
            } else {
                dir
            }
        };
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_lan_name.is_null() {
        st.draft.lan_device_name = settings_host_text(st.ed_lan_name)
            .trim()
            .chars()
            .take(48)
            .collect();
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_lan_tcp_port.is_null() {
        st.draft.lan_tcp_port = settings_host_text(st.ed_lan_tcp_port)
            .trim()
            .parse::<u16>()
            .ok()
            .filter(|port| *port > 0)
            .unwrap_or(crate::lan_sync::LAN_TCP_PORT_DEFAULT);
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.ed_lan_manual_host.is_null() {
        st.draft.lan_manual_host = settings_host_text(st.ed_lan_manual_host).trim().to_string();
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) && !st.cb_lan_receive_mode.is_null() {
        st.draft.lan_receive_mode =
            lan_receive_mode_from_label(&settings_host_text(st.cb_lan_receive_mode)).to_string();
    }
}
