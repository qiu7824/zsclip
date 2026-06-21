use super::prelude::*;
use crate::win_system_ui::settings_host_set_enabled;

pub(super) unsafe fn settings_sync_cloud_webdav_state(
    st: &mut SettingsWndState,
    webdav_enabled: bool,
) {
    let s = &st.draft;
    settings_set_text(st.cb_cloud_interval, &s.cloud_sync_interval);
    settings_set_text(st.ed_cloud_url, &s.cloud_webdav_url);
    settings_set_text(st.ed_cloud_user, &s.cloud_webdav_user);
    settings_set_text(st.ed_cloud_pass, &s.cloud_webdav_pass);
    settings_set_text(st.ed_cloud_dir, &s.cloud_remote_dir);
    settings_set_text(
        st.lb_cloud_status,
        &format!(
            "{}{}",
            tr("上次同步：", "Last sync: "),
            localized_cloud_status_text(&s.cloud_last_sync_status)
        ),
    );
    for hwnd in [
        st.cb_cloud_interval,
        st.ed_cloud_url,
        st.ed_cloud_user,
        st.ed_cloud_pass,
        st.ed_cloud_dir,
    ] {
        if !hwnd.is_null() {
            settings_host_set_enabled(hwnd, webdav_enabled);
        }
    }
}
