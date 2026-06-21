use super::prelude::*;

pub(super) unsafe fn handle_settings_group_selection(
    st: &mut SettingsWndState,
    control_id: isize,
    index: usize,
) {
    match control_id {
        IDC_SET_VV_SOURCE => {
            st.vv_source_selected = normalize_source_tab(index);
            settings_sync_vv_source_display(st);
            settings_sync_vv_group_display(st);
            repaint_settings_control(st.cb_vv_source);
            repaint_settings_control(st.cb_vv_group);
        }
        IDC_SET_VV_GROUP => {
            let vv_source = settings_vv_source_current(st);
            let groups = settings_groups_cache_for_tab(st, vv_source);
            if index == 0 {
                st.vv_group_selected = 0;
            } else if let Some(group) = groups.get(index - 1) {
                st.vv_group_selected = group.id;
            }
            settings_sync_vv_group_display(st);
            repaint_settings_control(st.cb_vv_group);
        }
        _ => {}
    }
}
