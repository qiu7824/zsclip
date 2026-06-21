use super::prelude::*;

pub(super) unsafe fn open_settings_group_dropdown(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
) -> bool {
    match control_id {
        IDC_SET_VV_SOURCE => {
            let rc = settings_control_screen_rect_or_empty(st.cb_vv_source);
            let current = settings_vv_source_current(st);
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_VV_SOURCE,
                &rc,
                &["复制记录", "常用短语"],
                current,
                200,
            );
            true
        }
        IDC_SET_VV_GROUP => {
            let rc = settings_control_screen_rect_or_empty(st.cb_vv_group);
            let vv_source = settings_vv_source_current(st);
            let groups = settings_groups_cache_for_tab(st, vv_source);
            let labels_owned: Vec<String> =
                std::iter::once(source_tab_all_label(vv_source).to_string())
                    .chain(groups.iter().map(|g| g.name.clone()))
                    .collect();
            let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
            let current = if st.vv_group_selected == 0 {
                0
            } else {
                groups
                    .iter()
                    .position(|g| g.id == st.vv_group_selected)
                    .map(|idx| idx + 1)
                    .unwrap_or(0)
            };
            st.dropdown_popup =
                present_settings_dropdown_popup(hwnd, IDC_SET_VV_GROUP, &rc, &labels, current, 260);
            true
        }
        _ => false,
    }
}
