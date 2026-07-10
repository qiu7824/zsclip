use super::prelude::*;

pub(super) unsafe fn cancel_settings_scroll_drag(hwnd: HWND, st: &mut SettingsWndState) {
    if st.scroll_dragging {
        st.scroll_dragging = false;
        release_settings_pointer(hwnd);
        invalidate_settings_scrollbar_and_mask(hwnd);
    }
}

pub(super) fn cancel_settings_scroll_frame(_hwnd: HWND, st: &mut SettingsWndState) {
    st.scroll_frame_posted = false;
    st.pending_scroll_delta = 0;
}

pub(super) unsafe fn handle_settings_scroll_frame(hwnd: HWND) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return 0;
    }
    let st = &mut *st_ptr;
    st.scroll_frame_posted = false;
    let delta = std::mem::take(&mut st.pending_scroll_delta);
    if delta != 0 {
        settings_scroll(hwnd, st, delta);
    }
    0
}

pub(super) unsafe fn handle_settings_pointer_move(hwnd: HWND, position: UiPoint) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return platform_window::default_window_proc(hwnd, WM_MOUSEMOVE, 0, 0);
    }
    let _ = settings_window_track_pointer_leave(hwnd);
    let st = &mut *st_ptr;
    let x = position.x;
    let y = position.y;
    let crc: RECT = settings_window_client_bounds(hwnd)
        .map(Into::into)
        .unwrap_or_else(|| zeroed());
    let transition = settings_pointer_move_transition(
        x,
        y,
        SETTINGS_PAGE_LABELS.len(),
        st.nav_hot,
        st.scroll_dragging,
        settings_scroll_layout_for_state(st, &crc, SCROLL_BAR_W_ACTIVE),
        st.scroll_drag_start_y,
        st.scroll_drag_start_scroll,
    );
    if st.scroll_dragging {
        if let Some(new_y) = transition.drag_scroll_y {
            settings_scroll_to(hwnd, st, new_y);
        }
        return 0;
    }

    if let Some(hover) = transition.nav_hover {
        if hover.next_hot != st.nav_hot {
            st.nav_hot = hover.next_hot;
            for rect in hover.invalidate_rects {
                repaint_settings_window_area(hwnd, Some(rect), false);
            }
        }
    }

    let hot_ctrl = settings_host_control_at_point(hwnd, position)
        .filter(|control| st.ownerdraw_ctrls.contains(control))
        .unwrap_or(null_mut());
    if hot_ctrl != st.hot_ownerdraw {
        if !st.hot_ownerdraw.is_null() {
            repaint_settings_control(st.hot_ownerdraw);
        }
        st.hot_ownerdraw = hot_ctrl;
        if !st.hot_ownerdraw.is_null() {
            repaint_settings_control(st.hot_ownerdraw);
        }
    }
    0
}

pub(super) unsafe fn handle_settings_pointer_leave(hwnd: HWND) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        let st = &mut *st_ptr;
        let hover = settings_nav_hover_transition(st.nav_hot, -1, SETTINGS_PAGE_LABELS.len());
        if hover.next_hot != st.nav_hot {
            st.nav_hot = hover.next_hot;
            for rect in hover.invalidate_rects {
                repaint_settings_window_area(hwnd, Some(rect), false);
            }
        }
        if !st.hot_ownerdraw.is_null() {
            let old = st.hot_ownerdraw;
            st.hot_ownerdraw = null_mut();
            repaint_settings_control(old);
        }
    }
    0
}

pub(super) unsafe fn handle_settings_lbutton_down(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    position: UiPoint,
) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return 0;
    }
    let st = &mut *st_ptr;
    let mx = position.x;
    let my = position.y;
    if settings_dropdown_popup_exists(st.dropdown_popup) {
        if let Some(prc) = settings_dropdown_popup_bounds(st.dropdown_popup) {
            let pt = settings_window_client_to_screen(hwnd, UiPoint { x: mx, y: my })
                .unwrap_or(UiPoint { x: mx, y: my });
            if !(pt.x >= prc.left && pt.x <= prc.right && pt.y >= prc.top && pt.y <= prc.bottom) {
                destroy_settings_dropdown_popup(st.dropdown_popup);
                st.dropdown_popup = null_mut();
            }
        } else {
            destroy_settings_dropdown_popup(st.dropdown_popup);
            st.dropdown_popup = null_mut();
        }
    }

    let crc: RECT = settings_window_client_bounds(hwnd)
        .map(Into::into)
        .unwrap_or_else(|| zeroed());
    let target = settings_pointer_down_target(
        mx,
        my,
        SETTINGS_PAGE_LABELS.len(),
        settings_scroll_layout_for_state(st, &crc, SCROLL_BAR_W_ACTIVE),
        st.content_scroll_y,
        4,
        4,
        2,
    );
    match target {
        SettingsPointerDownTarget::NavPage(page) => {
            settings_show_page(hwnd, st, page);
            let viewport = settings_viewport_rect(&crc);
            repaint_settings_window_area(hwnd, Some((&viewport).into()), false);
            repaint_settings_window(hwnd, false);
            return 0;
        }
        SettingsPointerDownTarget::ScrollbarThumb {
            drag_start_y,
            drag_start_scroll,
        } => {
            st.scroll_dragging = true;
            st.scroll_drag_start_y = drag_start_y;
            st.scroll_drag_start_scroll = drag_start_scroll;
            settings_scrollbar_show(hwnd, st);
            capture_settings_pointer(hwnd);
            invalidate_settings_scrollbar_and_mask(hwnd);
            return 0;
        }
        SettingsPointerDownTarget::ScrollbarTrack { scroll_y } => {
            settings_scroll_to(hwnd, st, scroll_y);
            return 0;
        }
        SettingsPointerDownTarget::None => {}
    }
    platform_window::default_window_proc(hwnd, msg, wparam, lparam)
}

pub(super) unsafe fn handle_settings_lbutton_up(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() && (*st_ptr).scroll_dragging {
        cancel_settings_scroll_drag(hwnd, &mut *st_ptr);
    }
    platform_window::default_window_proc(hwnd, msg, wparam, lparam)
}

pub(super) unsafe fn handle_settings_pointer_cancel(hwnd: HWND) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        cancel_settings_scroll_drag(hwnd, &mut *st_ptr);
    }
    0
}

pub(super) unsafe fn handle_settings_mouse_wheel(hwnd: HWND, delta: i32) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return 0;
    }
    let st = &mut *st_ptr;
    let scroll_delta = settings_scroll_delta_for_wheel(delta);
    if scroll_delta == 0 {
        return 0;
    }
    st.pending_scroll_delta = st.pending_scroll_delta.saturating_add(scroll_delta);
    if !st.scroll_frame_posted {
        st.scroll_frame_posted = true;
        platform_window::post_message(hwnd as isize, WM_SETTINGS_SCROLL_FRAME, 0, 0);
    }
    0
}
