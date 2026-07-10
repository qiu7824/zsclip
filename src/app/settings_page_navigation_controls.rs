use super::prelude::*;
use crate::platform::gdi as platform_gdi;

pub(super) unsafe fn settings_repos_controls(
    hwnd: HWND,
    st: &mut SettingsWndState,
    redraw_children: bool,
) {
    if !settings_page_control_scrollable(st, st.cur_page) {
        return;
    }

    let Some(crc) = platform_window::client_rect(hwnd) else {
        return;
    };
    let viewport = settings_viewport_rect(&crc);
    st.scroll_moves.clear();
    for slot in st.ui.scroll_ctrls_for_page(st.cur_page) {
        let hchild = slot.hwnd;
        if hchild.is_null() {
            continue;
        }

        let original = slot.bounds;
        let new_y = original.top - st.content_scroll_y;
        let intersects_viewport = settings_child_visible(new_y, original.height(), &viewport);
        let visible = slot.visible && (!st.viewport_hwnd.is_null() || intersects_viewport);
        let was_visible = platform_window::is_visible(hchild);
        if !visible && !was_visible {
            continue;
        }
        let bounds = if !st.viewport_hwnd.is_null() {
            settings_viewport_child_control_bounds(original, st.content_scroll_y, viewport)
        } else {
            UiRect::new(
                original.left,
                new_y,
                original.right,
                new_y + original.height(),
            )
        };

        st.scroll_moves.push(platform_window::DeferredWindowPos {
            hwnd: hchild,
            x: bounds.left,
            y: bounds.top,
            width: original.width(),
            height: original.height(),
            visible,
            was_visible,
        });
    }
    platform_window::defer_move_windows(&st.scroll_moves);

    if redraw_children {
        for slot in st.ui.scroll_ctrls_for_page(st.cur_page) {
            let hchild = slot.hwnd;
            let oy = slot.bounds.top;
            let oh = slot.bounds.bottom - slot.bounds.top;
            if hchild.is_null() {
                continue;
            }
            let new_y = oy - st.content_scroll_y;
            if slot.visible && settings_child_visible(new_y, oh, &viewport) {
                platform_gdi::invalidate_rect(hchild, null(), 0);
            }
        }
    }
}
