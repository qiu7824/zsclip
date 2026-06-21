use super::prelude::*;

pub(super) fn settings_groups_cache_for_tab(st: &SettingsWndState, tab: usize) -> &Vec<ClipGroup> {
    if normalize_source_tab(tab) == 0 {
        &st.record_groups_cache
    } else {
        &st.phrase_groups_cache
    }
}

pub(super) fn settings_groups_cache_for_tab_mut(
    st: &mut SettingsWndState,
    tab: usize,
) -> &mut Vec<ClipGroup> {
    if normalize_source_tab(tab) == 0 {
        &mut st.record_groups_cache
    } else {
        &mut st.phrase_groups_cache
    }
}

pub(super) fn settings_vv_source_current(st: &SettingsWndState) -> usize {
    normalize_source_tab(st.vv_source_selected)
}

pub(super) fn settings_group_view_current(st: &SettingsWndState) -> usize {
    normalize_source_tab(st.group_view_tab)
}

pub(super) unsafe fn settings_vv_source_from_app(st: &mut SettingsWndState) {
    st.vv_source_selected = normalize_source_tab(st.draft.vv_source_tab);
}

pub(super) unsafe fn settings_group_view_from_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    st.group_view_tab = if pst.is_null() {
        0
    } else {
        normalize_source_tab((&*pst).tab_index)
    };
}
