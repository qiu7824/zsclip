use super::prelude::*;
use crate::platform::gdi as platform_gdi;
use crate::settings_model::{
    settings_normalized_page_index, settings_page_switch_plan, SettingsPageSwitchMode,
};
use crate::win_system_ui::{settings_host_set_visible, settings_host_text};

pub(super) unsafe fn settings_show_page(hwnd: HWND, st: &mut SettingsWndState, page: usize) {
    let page_count = SETTINGS_PAGE_LABELS.len();
    let target_page = settings_normalized_page_index(page, page_count);
    let plan = settings_page_switch_plan(
        st.cur_page,
        page,
        page_count,
        st.ui.is_built(target_page),
        st.hotkey_recording,
        st.scroll_dragging,
        !st.dropdown_popup.is_null(),
        st.page_scroll_y.get(target_page).copied().unwrap_or(0),
    );
    let page = plan.target_page;
    if plan.cancel_hotkey_recording {
        st.hotkey_recording = false;
        if !st.btn_hk_record.is_null() {
            settings_set_text(st.btn_hk_record, tr("录制热键", "Record Hotkey"));
            platform_gdi::invalidate_rect(st.btn_hk_record, null(), 1);
        }
        if !st.lb_hk_preview.is_null() {
            settings_set_text(
                st.lb_hk_preview,
                &hotkey_preview_text(
                    &settings_host_text(st.cb_hk_mod),
                    &settings_host_text(st.cb_hk_key),
                ),
            );
            platform_gdi::invalidate_rect(st.lb_hk_preview, null(), 1);
        }
    }
    if plan.mode == SettingsPageSwitchMode::SyncOnly {
        settings_sync_page_state(st, page);
        return;
    }

    if plan.cancel_scroll_drag {
        cancel_settings_scroll_drag(hwnd, st);
    }
    cancel_settings_scroll_frame(hwnd, st);
    if plan.close_dropdown {
        if platform_window::exists(st.dropdown_popup) {
            platform_window::destroy(st.dropdown_popup);
        }
        st.dropdown_popup = null_mut();
    }

    platform_window::send_message(hwnd, WM_SETREDRAW, 0, 0);
    set_settings_viewport_child_visible(st.viewport_hwnd, false);
    st.cur_page = page;
    if let Some(scroll_state) = plan.scroll_state {
        st.content_scroll_y = scroll_state.content_scroll_y;
        st.page_scroll_y[scroll_state.page] = scroll_state.page_scroll_y;
        st.scroll_bar_visible = scroll_state.scroll_bar_visible;
    }
    sync_settings_viewport_child_bounds(hwnd, st.viewport_hwnd);
    settings_ensure_page(hwnd, st, page);

    for other_page in 0..SETTINGS_PAGE_LABELS.len() {
        for reg in st.ui.page_regs(other_page) {
            if reg.hwnd.is_null() {
                continue;
            }
            settings_host_set_visible(reg.hwnd, other_page == st.cur_page && reg.visible);
        }
    }

    if plan.reposition_controls {
        settings_repos_controls(hwnd, st, true);
    }

    settings_sync_page_state(st, page);
    set_settings_viewport_child_visible(st.viewport_hwnd, true);
    platform_window::send_message(hwnd, WM_SETREDRAW, 1, 0);
    platform_gdi::invalidate_rect(hwnd, null(), 1);
    platform_gdi::redraw_window(
        hwnd,
        null(),
        null_mut(),
        RDW_INVALIDATE | RDW_ERASE | RDW_ALLCHILDREN | RDW_UPDATENOW,
    );
}
