use super::prelude::*;

pub(super) fn settings_toggle_cloud_get(st: &SettingsWndState, cid: isize) -> Option<bool> {
    match cid {
        IDC_SET_CLOUD_ENABLE => Some(st.draft.cloud_sync_enabled),
        IDC_SET_LAN_ENABLE => Some(st.draft.lan_sync_enabled),
        _ => None,
    }
}

pub(super) fn settings_toggle_cloud_flip(st: &mut SettingsWndState, cid: isize) -> bool {
    match cid {
        IDC_SET_CLOUD_ENABLE => st.draft.cloud_sync_enabled = !st.draft.cloud_sync_enabled,
        IDC_SET_LAN_ENABLE => st.draft.lan_sync_enabled = !st.draft.lan_sync_enabled,
        _ => return false,
    }
    true
}
