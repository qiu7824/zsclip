use super::prelude::*;
use crate::settings_model::multi_sync_mode_display;

pub(super) unsafe fn settings_sync_cloud_page_state(st: &mut SettingsWndState) {
    let s = &st.draft;
    let mode = multi_sync_mode_from_settings(s);
    let webdav_enabled = mode == "webdav";
    let lan_enabled = mode == "lan";
    settings_set_text(st.cb_multi_sync_mode, multi_sync_mode_display(mode));
    settings_set_text(
        st.lb_multi_sync_summary,
        crate::multi_sync::transport_status_label(s.cloud_sync_enabled, s.lan_sync_enabled),
    );
    settings_sync_cloud_webdav_state(st, webdav_enabled);
    #[cfg(feature = "lan-sync")]
    settings_sync_cloud_lan_state(st, lan_enabled);
}
