use super::prelude::*;
use crate::settings_model::multi_sync_mode_label;

pub(super) unsafe fn settings_sync_cloud_page_state(st: &mut SettingsWndState) {
    let s = &st.draft;
    let mode = multi_sync_mode_from_settings(s);
    let webdav_enabled = mode == "webdav";
    let lan_enabled = matches!(mode, "lan" | "qinput");
    settings_set_text(st.cb_multi_sync_mode, multi_sync_mode_label(mode));
    settings_set_text(
        st.lb_multi_sync_summary,
        crate::multi_sync::transport_status_label(s.cloud_sync_enabled, s.lan_sync_enabled),
    );
    settings_sync_cloud_webdav_state(st, webdav_enabled);
    #[cfg(feature="lan-sync")]
    super::main_qq_cloud::sync_settings_section(st);
    #[cfg(feature = "lan-sync")]
    settings_sync_cloud_lan_state(st, lan_enabled);
}
