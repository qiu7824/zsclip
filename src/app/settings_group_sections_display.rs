use super::prelude::*;

pub(super) unsafe fn settings_group_current_filter_text(st: &SettingsWndState) -> String {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() {
        return tr("全部记录", "All Records").to_string();
    }
    let app = &*pst;
    let view_tab = normalize_source_tab(st.group_view_tab);
    let gid = app.tab_group_filters.get(view_tab).copied().unwrap_or(0);
    if gid == 0 {
        return if view_tab == 0 {
            tr("全部记录", "All Records").to_string()
        } else {
            tr("全部短语", "All Phrases").to_string()
        };
    }
    app.groups_for_tab(view_tab)
        .iter()
        .find(|g| g.id == gid)
        .map(|g| g.name.clone())
        .unwrap_or_else(|| format!("{} #{}", tr("分组", "Group"), gid))
}

pub(super) unsafe fn settings_sync_vv_source_display(st: &mut SettingsWndState) {
    st.vv_source_selected = normalize_source_tab(st.vv_source_selected);
    if !st.cb_vv_source.is_null() {
        settings_set_text(st.cb_vv_source, source_tab_label(st.vv_source_selected));
    }
}

pub(super) unsafe fn settings_sync_vv_group_display(st: &mut SettingsWndState) {
    let source_tab = settings_vv_source_current(st);
    let selected = st.vv_group_selected;
    let exists = if selected > 0 {
        settings_groups_cache_for_tab(st, source_tab)
            .iter()
            .any(|g| g.id == selected)
    } else {
        true
    };
    if selected > 0 && !exists {
        st.vv_group_selected = 0;
    }
    if !st.cb_vv_group.is_null() {
        let groups = settings_groups_cache_for_tab(st, source_tab);
        settings_set_text(
            st.cb_vv_group,
            &group_name_for_display(
                groups,
                st.vv_group_selected,
                source_tab_all_label(source_tab),
            ),
        );
    }
}

pub(super) unsafe fn settings_sync_group_view_tabs(st: &SettingsWndState) {
    if !st.btn_group_view_records.is_null() {
        platform_gdi::invalidate_rect(st.btn_group_view_records, null(), 1);
    }
    if !st.btn_group_view_phrases.is_null() {
        platform_gdi::invalidate_rect(st.btn_group_view_phrases, null(), 1);
    }
}

pub(super) unsafe fn settings_sync_group_overview(st: &mut SettingsWndState) {
    st.group_view_tab = normalize_source_tab(st.group_view_tab);
    let current_filter = settings_group_current_filter_text(st);
    let text = settings_group_overview_text(st.group_view_tab, &current_filter);
    if !st.lb_group_current.is_null() {
        settings_set_text(st.lb_group_current, &text);
    }
    let pst = get_state_ptr(st.parent_hwnd);
    let gid = if pst.is_null() {
        0
    } else {
        (&*pst)
            .tab_group_filters
            .get(st.group_view_tab)
            .copied()
            .unwrap_or(0)
    };
    settings_groups_refresh_list(st, gid);
    settings_sync_group_view_tabs(st);
}
