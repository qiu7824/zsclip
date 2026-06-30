use super::prelude::*;

pub(super) fn settings_toggle_group_get(st: &SettingsWndState, cid: isize) -> Option<bool> {
    match cid {
        IDC_SET_GROUP_ENABLE => Some(st.draft.grouping_enabled),
        IDC_SET_GROUP_TYPE_FILTER => Some(st.draft.group_type_filter_enabled),
        _ => None,
    }
}

pub(super) fn settings_toggle_group_flip(st: &mut SettingsWndState, cid: isize) -> bool {
    match cid {
        IDC_SET_GROUP_ENABLE => st.draft.grouping_enabled = !st.draft.grouping_enabled,
        IDC_SET_GROUP_TYPE_FILTER => {
            st.draft.group_type_filter_enabled = !st.draft.group_type_filter_enabled
        }
        _ => return false,
    }
    true
}
