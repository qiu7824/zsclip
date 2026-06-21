use super::prelude::*;

pub(super) fn settings_toggle_plugin_get(st: &SettingsWndState, cid: isize) -> Option<bool> {
    match cid {
        7102 => Some(st.draft.quick_search_enabled),
        7101 => Some(st.draft.ai_clean_enabled),
        7103 => Some(st.draft.super_mail_merge_enabled),
        7106 => Some(st.draft.wps_taskpane_enabled),
        7104 => Some(st.draft.qr_quick_enabled),
        _ => None,
    }
}

pub(super) fn settings_toggle_plugin_flip(st: &mut SettingsWndState, cid: isize) -> bool {
    match cid {
        7102 => st.draft.quick_search_enabled = !st.draft.quick_search_enabled,
        7101 => st.draft.ai_clean_enabled = !st.draft.ai_clean_enabled,
        7103 => st.draft.super_mail_merge_enabled = !st.draft.super_mail_merge_enabled,
        7106 => st.draft.wps_taskpane_enabled = !st.draft.wps_taskpane_enabled,
        7104 => st.draft.qr_quick_enabled = !st.draft.qr_quick_enabled,
        _ => return false,
    }
    true
}
