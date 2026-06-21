use super::prelude::*;

pub(super) unsafe fn ensure_mouse_leave_tracking(hwnd: HWND) {
    platform_input::track_mouse_leave_and_hover(
        hwnd,
        platform_system_parameters::mouse_hover_time_ms(),
    );
}

pub(super) unsafe fn hover_preview_blocked_at_point(state: &AppState, x: i32, y: i32) -> bool {
    if scroll_to_top_visible(state) && pt_in_rect(x, y, &state.scroll_to_top_rect()) {
        return true;
    }
    let Some(item) = hovered_item_clone(state) else {
        return false;
    };
    row_quick_delete_rect(state, state.hover_idx, &item)
        .map(|rc| pt_in_rect(x, y, &rc))
        .unwrap_or(false)
}

unsafe fn refresh_hover_preview(hwnd: HWND, state: &AppState, x: i32, y: i32) {
    if !state.settings.hover_preview || state.edge_hidden {
        hide_hover_preview();
        return;
    }
    let Some(item) = hovered_item_clone(state) else {
        hide_hover_preview();
        return;
    };
    if hover_preview_blocked_at_point(state, x, y) {
        return;
    }
    let Some(win_rc) = platform_window::window_rect(hwnd) else {
        hide_hover_preview();
        return;
    };
    show_hover_preview(&item, win_rc.left + x, win_rc.top + y);
}

pub(super) unsafe fn handle_mouse_hover_main(hwnd: HWND, position: UiPoint) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &*ptr;
    refresh_hover_preview(hwnd, state, position.x, position.y);
}

pub(super) unsafe fn handle_mouse_leave_main(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let transition = main_hover_target_from_state(state).clear_transition(true);
    if transition.changed {
        apply_main_hover_target(state, transition.next);
    }
    hide_hover_preview();
    if state.settings.edge_auto_hide && !state.edge_hidden && !vv_popup_menu_active() {
        if let Some(pt) = platform_input::cursor_pos() {
            if edge_window_scope_contains_point(hwnd, pt) {
                ensure_mouse_leave_tracking(hwnd);
            }
        }
    }
    if transition.changed {
        platform_gdi::invalidate_rect(hwnd, null(), 0);
    }
}

pub(super) unsafe fn clear_main_hover_state(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let transition = main_hover_target_from_state(state).clear_transition(false);
    let mut dirty = transition.changed;
    if transition.changed {
        apply_main_hover_target(state, transition.next);
    }
    if state.down_to_top {
        state.down_to_top = false;
        dirty = true;
    }
    if state.down_row != -1 {
        state.down_row = -1;
        state.down_x = 0;
        state.down_y = 0;
        dirty = true;
    }
    hide_hover_preview();
    if dirty {
        platform_gdi::invalidate_rect(hwnd, null(), 0);
    }
}

pub(super) unsafe fn main_window_should_stay_noactivate(state: &AppState, x: i32, y: i32) -> bool {
    hit_test_row(state, x, y) >= 0
}
