use super::prelude::*;
use crate::platform::gdi as platform_gdi;

pub(super) unsafe fn settings_repos_controls(
    hwnd: HWND,
    st: &SettingsWndState,
    redraw_children: bool,
) {
    let slots: Vec<_> = st.ui.scroll_ctrls_for_page(st.cur_page).collect();
    if slots.is_empty() || !crate::settings_model::settings_page_scrollable(st.cur_page) {
        return;
    }

    let Some(crc) = platform_window::client_rect(hwnd) else {
        return;
    };
    let viewport = settings_viewport_rect(&crc);
    let mut dirty: Vec<RECT> = Vec::with_capacity(slots.len() * 2);
    let mut moves = Vec::with_capacity(slots.len());
    for slot in slots.iter() {
        let hchild = slot.hwnd;
        let ox = slot.bounds.left;
        let oy = slot.bounds.top;
        let ow = slot.bounds.right - slot.bounds.left;
        let oh = slot.bounds.bottom - slot.bounds.top;
        if hchild.is_null() {
            continue;
        }

        if let Some(wr) = platform_window::window_rect(hchild) {
            let mut tl = POINT {
                x: wr.left,
                y: wr.top,
            };
            let mut br = POINT {
                x: wr.right,
                y: wr.bottom,
            };
            platform_window::screen_to_client(hwnd, &mut tl);
            platform_window::screen_to_client(hwnd, &mut br);
            dirty.push(RECT {
                left: tl.x,
                top: tl.y,
                right: br.x,
                bottom: br.y,
            });
        }

        let new_y = oy - st.content_scroll_y;
        let visible = slot.visible && settings_child_visible(new_y, oh, &viewport);
        if slot.visible {
            dirty.push(RECT {
                left: ox,
                top: new_y,
                right: ox + ow,
                bottom: new_y + oh,
            });
        }

        moves.push(platform_window::DeferredWindowPos {
            hwnd: hchild,
            x: ox,
            y: new_y,
            width: ow,
            height: oh,
            visible,
        });
    }
    platform_window::defer_move_windows(&moves);

    if redraw_children {
        for slot in slots.iter() {
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

    for mut rc in dirty {
        if rc.right <= rc.left || rc.bottom <= rc.top {
            continue;
        }
        if rc.left < viewport.left {
            rc.left = viewport.left;
        }
        if rc.top < viewport.top {
            rc.top = viewport.top;
        }
        if rc.right > viewport.right {
            rc.right = viewport.right;
        }
        if rc.bottom > viewport.bottom {
            rc.bottom = viewport.bottom;
        }
        if rc.right > rc.left && rc.bottom > rc.top {
            platform_gdi::invalidate_rect(hwnd, &rc, 0);
        }
    }
}
