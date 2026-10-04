use super::prelude::*;

pub(super) unsafe fn settings_collect_group_to_draft(st: &mut SettingsWndState) {
    if !st.ui.is_built(SettingsPage::Hotkey.index()) && !st.ui.is_built(SettingsPage::Group.index()) {
        return;
    }
    st.draft.vv_source_tab = settings_vv_source_current(st);
    let vv_groups = settings_groups_cache_for_tab(st, st.draft.vv_source_tab);
    st.draft.vv_group_id =
        if st.vv_group_selected > 0 && vv_groups.iter().any(|g| g.id == st.vv_group_selected) {
            st.vv_group_selected
        } else {
            0
        };
}
