use super::prelude::*;
use crate::platform::string::to_wide;

pub(super) unsafe fn settings_groups_refresh_list(st: &mut SettingsWndState, select_gid: i64) {
    if st.lb_groups.is_null() {
        return;
    }
    let category = source_tab_category(settings_group_view_current(st));
    platform_window::send_message(st.lb_groups, LB_RESETCONTENT, 0, 0);
    *settings_groups_cache_for_tab_mut(st, settings_group_view_current(st)) =
        db_load_groups(category);
    let groups = settings_groups_cache_for_tab(st, settings_group_view_current(st));
    let mut sel_idx: i32 = -1;
    for (i, g) in groups.iter().enumerate() {
        platform_window::send_message(
            st.lb_groups,
            LB_ADDSTRING,
            0,
            to_wide(&g.name).as_ptr() as LPARAM,
        );
        if g.id == select_gid {
            sel_idx = i as i32;
        }
    }
    if sel_idx < 0 && !groups.is_empty() {
        sel_idx = 0;
    }
    if sel_idx >= 0 {
        platform_window::send_message(st.lb_groups, LB_SETCURSEL, sel_idx as WPARAM, 0);
    }
    let item_h = platform_window::send_message(st.lb_groups, LB_GETITEMHEIGHT, 0, 0) as i32;
    let Some(rc) = platform_window::client_rect(st.lb_groups) else {
        return;
    };
    let view_h = (rc.bottom - rc.top).max(0);
    let needs_vscroll = item_h > 0 && (groups.len() as i32 * item_h) > view_h;
    platform_window::show_scrollbar(st.lb_groups, SB_VERT, needs_vscroll);
    platform_window::show_scrollbar(st.lb_groups, SB_HORZ, false);
    settings_sync_vv_group_display(st);
}

pub(super) unsafe fn settings_groups_selected(st: &SettingsWndState) -> Option<(usize, ClipGroup)> {
    if st.lb_groups.is_null() {
        return None;
    }
    let row = platform_window::send_message(st.lb_groups, LB_GETCURSEL, 0, 0) as i32;
    if row < 0 {
        return None;
    }
    settings_groups_cache_for_tab(st, settings_group_view_current(st))
        .get(row as usize)
        .cloned()
        .map(|g| (row as usize, g))
}

pub(super) unsafe fn settings_groups_sync_name(_st: &mut SettingsWndState) {}

pub(super) unsafe fn settings_groups_move(st: &mut SettingsWndState, step: i32) {
    let Some((idx, _)) = settings_groups_selected(st) else {
        return;
    };
    let tab = settings_group_view_current(st);
    let category = source_tab_category(tab);
    let groups = settings_groups_cache_for_tab(st, tab);
    let new_idx = idx as i32 + step;
    if new_idx < 0 || new_idx >= groups.len() as i32 {
        return;
    }
    let mut ids: Vec<i64> = groups.iter().map(|g| g.id).collect();
    let item = ids.remove(idx);
    ids.insert(new_idx as usize, item);
    if db_set_groups_order(category, &ids).is_ok() {
        settings_groups_refresh_list(st, ids[new_idx as usize]);
        let pst = get_state_ptr(st.parent_hwnd);
        if !pst.is_null() {
            reload_state_from_db_persisting(&mut *pst);
            platform_gdi::invalidate_rect(st.parent_hwnd, null(), 1);
        }
    }
}
