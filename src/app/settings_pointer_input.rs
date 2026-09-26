use super::prelude::*;

pub(super) unsafe fn cancel_settings_scroll_drag(hwnd: HWND, st: &mut SettingsWndState) {
    if st.scroll_dragging {
        st.scroll_dragging = false;
        release_settings_pointer(hwnd);
        invalidate_settings_scrollbar_and_mask(hwnd);
    }
}

pub(super) fn cancel_settings_scroll_frame(hwnd: HWND, st: &mut SettingsWndState) {
    timer::stop(hwnd, ID_TIMER_SETTINGS_SCROLL_FRAME);
    st.scroll_frame_posted = false;
    st.pending_scroll_delta = 0;
    st.scroll_wheel_remainder = 0;
}

fn scroll_frame_step(remaining: i32) -> i32 {
    if remaining == 0 {
        0
    } else {
        let step = remaining / 3;
        if step == 0 {
            remaining.signum()
        } else {
            step
        }
    }
}

fn wheel_pixels(delta: i32, remainder: &mut i32, pixels_per_notch: i32) -> i32 {
    let accumulated = i64::from(*remainder) + i64::from(delta) * i64::from(pixels_per_notch);
    *remainder = (accumulated % 120) as i32;
    (-(accumulated / 120)).clamp(i32::MIN as i64, i32::MAX as i64) as i32
}

pub(super) unsafe fn handle_settings_scroll_frame(hwnd: HWND) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return 0;
    }
    let st = &mut *st_ptr;
    let delta = std::mem::take(&mut st.pending_scroll_delta);
    if delta != 0 {
        let before = st.content_scroll_y;
        settings_scroll(hwnd, st, scroll_frame_step(delta));
        let actual = st.content_scroll_y - before;
        if actual != 0 {
            st.pending_scroll_delta = delta.saturating_sub(actual);
        }
    }
    if st.pending_scroll_delta == 0 {
        timer::stop(hwnd, ID_TIMER_SETTINGS_SCROLL_FRAME);
        st.scroll_frame_posted = false;
    } else if SetTimer(hwnd, ID_TIMER_SETTINGS_SCROLL_FRAME, 16, None) == 0 {
        let rest = std::mem::take(&mut st.pending_scroll_delta);
        settings_scroll(hwnd, st, rest);
        st.scroll_frame_posted = false;
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
            cancel_settings_scroll_frame(hwnd, st);
            st.scroll_dragging = true;
            st.scroll_drag_start_y = drag_start_y;
            st.scroll_drag_start_scroll = drag_start_scroll;
            settings_scrollbar_show(hwnd, st);
            capture_settings_pointer(hwnd);
            invalidate_settings_scrollbar_and_mask(hwnd);
            return 0;
        }
        SettingsPointerDownTarget::ScrollbarTrack { scroll_y } => {
            cancel_settings_scroll_frame(hwnd, st);
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
    let scroll_delta = wheel_pixels(delta, &mut st.scroll_wheel_remainder, settings_scale(60));
    if scroll_delta == 0 {
        return 0;
    }
    st.pending_scroll_delta = st.pending_scroll_delta.saturating_add(scroll_delta);
    if !st.scroll_frame_posted {
        st.scroll_frame_posted = true;
        if SetTimer(hwnd, ID_TIMER_SETTINGS_SCROLL_FRAME, 16, None) == 0 {
            // A resource-starved timer must not swallow input.
            platform_window::post_message(hwnd as isize, WM_SETTINGS_SCROLL_FRAME, 0, 0);
        }
    }
    0
}

#[cfg(test)]
mod scroll_motion_tests {
    use super::*;
    #[test]
    fn high_resolution_wheel_preserves_total_motion_and_zero_is_idle() {
        let mut remainder = 0;
        let sum: i32 = (0..120).map(|_| wheel_pixels(1, &mut remainder, 60)).sum();
        assert_eq!(sum, -60);
        assert_eq!(remainder, 0);
        assert_eq!(wheel_pixels(0, &mut remainder, 60), 0);
        assert_eq!(wheel_pixels(-240, &mut remainder, 60), 120);
        let scaled: i32 = (0..120).map(|_| wheel_pixels(1, &mut remainder, 75)).sum();
        assert_eq!(scaled, -75);
    }
    #[test]
    fn scroll_frames_ease_towards_target_without_overshoot_or_stalling() {
        for total in [-600, -60, -1, 0, 1, 60, 600] {
            let mut remaining = total;
            let mut sum = 0;
            let mut count = 0;
            while remaining != 0 {
                let step = scroll_frame_step(remaining);
                assert_eq!(step.signum(), remaining.signum());
                assert!(step.abs() <= remaining.abs());
                sum += step;
                remaining -= step;
                count += 1;
                assert!(count < 24);
            }
            assert_eq!(sum, total);
        }
    }
}
