use super::prelude::*;

pub(super) unsafe fn settings_sync_group_page(st: &mut SettingsWndState) {
    st.record_groups_cache = db_load_groups(0);
    st.phrase_groups_cache = db_load_groups(1);
    settings_vv_source_from_app(st);
    settings_sync_vv_source_display(st);
    st.vv_group_selected = st.draft.vv_group_id;
    settings_sync_vv_group_display(st);
    settings_group_view_from_app(st);
    settings_sync_group_overview(st);
}
