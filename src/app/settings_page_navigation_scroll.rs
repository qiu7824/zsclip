use super::prelude::*;
use crate::platform::gdi as platform_gdi;
use crate::settings_model::settings_scroll_update_for_target;
use crate::win_system_ui::settings_viewport_mask_rect;

pub(super) unsafe fn invalidate_settings_scrollbar_and_mask(hwnd: HWND) {
    let Some(crc) = platform_window::client_rect(hwnd) else {
        return;
    };
    let content_y = settings_content_y_scaled();
    let mask = settings_viewport_mask_rect(&crc);
    platform_gdi::invalidate_rect(hwnd, &mask, 0);
    let scroll_strip = RECT {
        left: crc.right - SCROLL_BAR_W_ACTIVE - SCROLL_BAR_MARGIN - 4,
        top: content_y,
        right: crc.right,
        bottom: crc.bottom,
    };
    platform_gdi::invalidate_rect(hwnd, &scroll_strip, 0);
}

pub(super) unsafe fn settings_scroll_to(hwnd: HWND, st: &mut SettingsWndState, new_y: i32) {
    let Some(crc) = platform_window::client_rect(hwnd) else {
        return;
    };
    let content_y = settings_content_y_scaled();
    let view_h = (crc.bottom - crc.top) - content_y;
    let Some(update) = settings_scroll_update_for_target(
        st.cur_page,
        st.content_scroll_y,
        new_y,
        settings_page_content_total_h_for_state(st, st.cur_page),
        view_h,
    ) else {
        return;
    };
    st.content_scroll_y = update.content_scroll_y;
    st.page_scroll_y[update.page] = update.page_scroll_y;
    settings_scrollbar_show(hwnd, st);

    let viewport = settings_viewport_rect(&crc);
    settings_repos_controls(hwnd, st, false);
    if !st.viewport_hwnd.is_null() {
        platform_gdi::invalidate_rect(st.viewport_hwnd, null(), 0);
    }

    invalidate_settings_scrollbar_and_mask(hwnd);
    platform_gdi::redraw_window(hwnd, &viewport, null_mut(), RDW_INVALIDATE);
}

pub(super) unsafe fn settings_scrollbar_show(hwnd: HWND, st: &mut SettingsWndState) {
    st.scroll_bar_visible = true;
    start_flagged_timer(
        hwnd,
        ID_TIMER_SETTINGS_SCROLLBAR,
        1500,
        &mut st.scroll_hide_timer,
    );
}

pub(super) unsafe fn settings_scroll(hwnd: HWND, st: &mut SettingsWndState, delta: i32) {
    settings_scroll_to(hwnd, st, st.content_scroll_y + delta);
}
