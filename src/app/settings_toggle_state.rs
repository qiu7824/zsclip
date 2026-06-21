use super::prelude::*;

pub(super) unsafe fn settings_toggle_get(st: &SettingsWndState, cid: isize) -> bool {
    settings_toggle_general_get(st, cid)
        .or_else(|| settings_toggle_cloud_get(st, cid))
        .or_else(|| settings_toggle_hotkey_get(st, cid))
        .or_else(|| settings_toggle_plugin_get(st, cid))
        .or_else(|| settings_toggle_group_get(st, cid))
        .unwrap_or(false)
}

pub(super) unsafe fn settings_toggle_flip(st: &mut SettingsWndState, cid: isize) {
    let _ = settings_toggle_general_flip(st, cid)
        || settings_toggle_cloud_flip(st, cid)
        || settings_toggle_hotkey_flip(st, cid)
        || settings_toggle_plugin_flip(st, cid)
        || settings_toggle_group_flip(st, cid);
}
