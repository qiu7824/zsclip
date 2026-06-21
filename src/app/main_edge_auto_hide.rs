use super::prelude::*;
use std::time::Duration;

pub(crate) fn clear_edge_dock_state(state: &mut AppState) {
    state.edge_hidden = false;
    state.edge_hidden_side = EDGE_AUTO_HIDE_NONE;
    state.edge_docked_left = 0;
    state.edge_docked_top = 0;
    state.edge_docked_right = 0;
    state.edge_docked_bottom = 0;
    state.edge_hide_armed = false;
    state.edge_hide_pending_until = None;
    state.edge_hide_grace_until = None;
    state.edge_restore_wait_leave = false;
    state.edge_anim_until = None;
}

fn set_edge_docked_rect(state: &mut AppState, rc: &RECT) {
    state.edge_docked_left = rc.left;
    state.edge_docked_top = rc.top;
    state.edge_docked_right = rc.right;
    state.edge_docked_bottom = rc.bottom;
}

fn edge_docked_rect(state: &AppState) -> RECT {
    RECT {
        left: state.edge_docked_left,
        top: state.edge_docked_top,
        right: state.edge_docked_right,
        bottom: state.edge_docked_bottom,
    }
}

fn edge_docked_rect_valid(state: &AppState) -> bool {
    state.edge_docked_right > state.edge_docked_left
        && state.edge_docked_bottom > state.edge_docked_top
}

fn edge_side_valid(side: i32) -> bool {
    matches!(
        side,
        EDGE_AUTO_HIDE_LEFT | EDGE_AUTO_HIDE_RIGHT | EDGE_AUTO_HIDE_TOP | EDGE_AUTO_HIDE_BOTTOM
    )
}

unsafe fn edge_auto_hide_peek(hwnd: HWND) -> i32 {
    platform_dpi::scale_for_window(hwnd, EDGE_AUTO_HIDE_PEEK).max(6)
}

fn edge_set_grace(state: &mut AppState, ms: u64) {
    state.edge_hide_grace_until = Some(Instant::now() + Duration::from_millis(ms));
}

pub(crate) fn edge_interaction_grace_ms() -> u64 {
    0
}

fn edge_set_hide_pending(state: &mut AppState, ms: u64) {
    let now = Instant::now();
    let base = state
        .edge_hide_grace_until
        .filter(|until| *until > now)
        .unwrap_or(now);
    state.edge_hide_pending_until = Some(base + Duration::from_millis(ms));
}

fn edge_grace_active(state: &AppState) -> bool {
    state
        .edge_hide_grace_until
        .map(|until| until > Instant::now())
        .unwrap_or(false)
}

pub(crate) fn edge_animation_active(state: &AppState) -> bool {
    state
        .edge_anim_until
        .map(|until| until > Instant::now())
        .unwrap_or(false)
}

unsafe fn begin_edge_animation(
    hwnd: HWND,
    state: &mut AppState,
    from_x: i32,
    from_y: i32,
    to_x: i32,
    to_y: i32,
    duration_ms: u64,
) {
    state.edge_anim_from_x = from_x;
    state.edge_anim_from_y = from_y;
    state.edge_anim_to_x = to_x;
    state.edge_anim_to_y = to_y;
    state.edge_anim_until = Some(Instant::now() + Duration::from_millis(duration_ms));
    platform_window::set_pos(
        hwnd,
        null_mut(),
        from_x,
        from_y,
        0,
        0,
        SWP_NOSIZE
            | SWP_NOACTIVATE
            | SWP_NOZORDER
            | SWP_NOOWNERZORDER
            | SWP_NOSENDCHANGING
            | SWP_ASYNCWINDOWPOS
            | SWP_SHOWWINDOW,
    );
}

unsafe fn edge_step_animation(hwnd: HWND, state: &mut AppState) -> bool {
    let Some(until) = state.edge_anim_until else {
        return false;
    };
    let now = Instant::now();
    let total_ms = EDGE_AUTO_HIDE_ANIM_MS as f32;
    let remain_ms = until.saturating_duration_since(now).as_millis() as f32;
    let t = if remain_ms <= 0.0 {
        1.0
    } else {
        (1.0 - (remain_ms / total_ms)).clamp(0.0, 1.0)
    };
    let eased = 1.0 - (1.0 - t) * (1.0 - t);
    let x = state.edge_anim_from_x
        + ((state.edge_anim_to_x - state.edge_anim_from_x) as f32 * eased).round() as i32;
    let y = state.edge_anim_from_y
        + ((state.edge_anim_to_y - state.edge_anim_from_y) as f32 * eased).round() as i32;
    platform_window::set_pos(
        hwnd,
        null_mut(),
        x,
        y,
        0,
        0,
        SWP_NOSIZE
            | SWP_NOACTIVATE
            | SWP_NOZORDER
            | SWP_NOOWNERZORDER
            | SWP_NOSENDCHANGING
            | SWP_ASYNCWINDOWPOS
            | SWP_SHOWWINDOW,
    );
    if t >= 1.0 {
        platform_window::set_pos(
            hwnd,
            null_mut(),
            state.edge_anim_to_x,
            state.edge_anim_to_y,
            0,
            0,
            SWP_NOSIZE
                | SWP_NOACTIVATE
                | SWP_NOZORDER
                | SWP_NOOWNERZORDER
                | SWP_NOSENDCHANGING
                | SWP_ASYNCWINDOWPOS
                | SWP_SHOWWINDOW,
        );
        state.edge_anim_until = None;
        return false;
    }
    true
}

fn edge_hide_pending_active(state: &AppState) -> bool {
    state
        .edge_hide_pending_until
        .map(|until| until > Instant::now())
        .unwrap_or(false)
}

unsafe fn edge_detect_margin_v(hwnd: HWND) -> i32 {
    platform_dpi::scale_for_window(hwnd, EDGE_AUTO_HIDE_MARGIN).max(12)
}

unsafe fn edge_detect_margin_h(hwnd: HWND) -> i32 {
    edge_detect_margin_v(hwnd).max(platform_dpi::scale_for_window(hwnd, 24))
}

unsafe fn edge_choose_dock_side(hwnd: HWND, rc: &RECT) -> Option<(i32, RECT)> {
    let work = platform_monitor::nearest_work_rect_for_window(hwnd);
    let monitor = platform_monitor::nearest_rect_for_window(hwnd);
    let margin_v = edge_detect_margin_v(hwnd);
    let margin_h = edge_detect_margin_h(hwnd);

    let candidates = [
        (
            (rc.left - work.left).abs(),
            EDGE_AUTO_HIDE_LEFT,
            work,
            margin_h,
        ),
        (
            (rc.left - monitor.left).abs(),
            EDGE_AUTO_HIDE_LEFT,
            monitor,
            margin_h,
        ),
        (
            (work.right - rc.right).abs(),
            EDGE_AUTO_HIDE_RIGHT,
            work,
            margin_h,
        ),
        (
            (monitor.right - rc.right).abs(),
            EDGE_AUTO_HIDE_RIGHT,
            monitor,
            margin_h,
        ),
        (
            (rc.top - work.top).abs(),
            EDGE_AUTO_HIDE_TOP,
            work,
            margin_v,
        ),
        (
            (rc.top - monitor.top).abs(),
            EDGE_AUTO_HIDE_TOP,
            monitor,
            margin_v,
        ),
        (
            (work.bottom - rc.bottom).abs(),
            EDGE_AUTO_HIDE_BOTTOM,
            work,
            margin_v,
        ),
        (
            (monitor.bottom - rc.bottom).abs(),
            EDGE_AUTO_HIDE_BOTTOM,
            monitor,
            margin_v,
        ),
    ];

    let mut best: Option<(i32, i32, RECT)> = None;
    for (dist, side, base, limit) in candidates {
        if dist > limit {
            continue;
        }
        match best {
            Some((best_dist, _, _)) if best_dist <= dist => {}
            _ => best = Some((dist, side, base)),
        }
    }
    let _ = monitor;
    best.map(|(_, side, base)| (side, base))
}

unsafe fn update_edge_dock_state(hwnd: HWND, state: &mut AppState, rc: &RECT) -> bool {
    if let Some((side, base)) = edge_choose_dock_side(hwnd, rc) {
        state.edge_hidden_side = side;
        set_edge_docked_rect(state, &base);
        if !state.edge_hidden {
            state.edge_restore_x = rc.left;
            state.edge_restore_y = rc.top;
        }
        true
    } else {
        clear_edge_dock_state(state);
        false
    }
}

unsafe fn ensure_edge_dock_state(hwnd: HWND, state: &mut AppState) -> bool {
    if edge_side_valid(state.edge_hidden_side) && edge_docked_rect_valid(state) {
        return true;
    }
    let rc = platform_window::dock_rect(hwnd);
    update_edge_dock_state(hwnd, state, &rc)
}

unsafe fn edge_hotzone_rect(hwnd: HWND, state: &AppState) -> Option<RECT> {
    if !edge_side_valid(state.edge_hidden_side) || !edge_docked_rect_valid(state) {
        return None;
    }
    let docked = edge_docked_rect(state);
    let monitor = platform_monitor::nearest_rect_for_window(hwnd);
    let hot = edge_detect_margin_v(hwnd);
    Some(match state.edge_hidden_side {
        EDGE_AUTO_HIDE_LEFT => RECT {
            left: docked.left,
            top: docked.top,
            right: docked.left + hot,
            bottom: docked.bottom,
        },
        EDGE_AUTO_HIDE_RIGHT => RECT {
            left: docked.right - hot,
            top: docked.top,
            right: docked.right,
            bottom: docked.bottom,
        },
        EDGE_AUTO_HIDE_TOP => RECT {
            left: docked.left,
            top: docked.top,
            right: docked.right,
            bottom: docked.top + hot,
        },
        EDGE_AUTO_HIDE_BOTTOM => RECT {
            left: docked.left,
            top: monitor.bottom - hot,
            right: docked.right,
            bottom: monitor.bottom,
        },
        _ => docked,
    })
}

unsafe fn edge_hidden_position(hwnd: HWND, state: &AppState, rc: &RECT) -> Option<(i32, i32)> {
    if !edge_side_valid(state.edge_hidden_side) || !edge_docked_rect_valid(state) {
        return None;
    }
    let docked = edge_docked_rect(state);
    let monitor = platform_monitor::nearest_rect_for_window(hwnd);
    let width = (rc.right - rc.left).max(1);
    let height = (rc.bottom - rc.top).max(1);
    let peek = edge_auto_hide_peek(hwnd);
    let x = match state.edge_hidden_side {
        EDGE_AUTO_HIDE_LEFT => docked.left + peek - width,
        EDGE_AUTO_HIDE_RIGHT => docked.right - peek,
        EDGE_AUTO_HIDE_TOP | EDGE_AUTO_HIDE_BOTTOM => state
            .edge_restore_x
            .clamp(docked.left, (docked.right - width).max(docked.left)),
        _ => rc.left,
    };
    let y = match state.edge_hidden_side {
        EDGE_AUTO_HIDE_TOP => docked.top + peek - height,
        EDGE_AUTO_HIDE_BOTTOM => monitor.bottom - peek,
        EDGE_AUTO_HIDE_LEFT | EDGE_AUTO_HIDE_RIGHT => state
            .edge_restore_y
            .clamp(docked.top, (docked.bottom - height).max(docked.top)),
        _ => rc.top,
    };
    Some((x, y))
}

pub(super) unsafe fn restore_edge_hidden_window(hwnd: HWND, state: &mut AppState) {
    if !state.edge_hidden {
        return;
    }
    let rc = platform_window::dock_rect(hwnd);
    begin_edge_animation(
        hwnd,
        state,
        rc.left,
        rc.top,
        state.edge_restore_x,
        state.edge_restore_y,
        EDGE_AUTO_HIDE_ANIM_MS,
    );
    state.edge_hidden = false;
    state.edge_hide_armed = false;
    state.edge_hide_pending_until = None;
    state.edge_restore_wait_leave = false;
    edge_set_grace(state, EDGE_AUTO_HIDE_RESTORE_GRACE_MS);
    ensure_mouse_leave_tracking(hwnd);
    refresh_low_level_input_hooks();
}

pub(crate) unsafe fn try_restore_edge_hidden_window(hwnd: HWND) -> bool {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return false;
    }
    let state = &mut *ptr;
    if !state.edge_hidden {
        return false;
    }
    restore_edge_hidden_window(hwnd, state);
    platform_gdi::invalidate_rect(hwnd, null(), 1);
    true
}

unsafe fn hide_edge_docked_window_with_scope(hwnd: HWND, state: &mut AppState, check_scope: bool) {
    if !state.settings.edge_auto_hide || state.edge_hidden {
        return;
    }

    if !ensure_edge_dock_state(hwnd, state) {
        return;
    }
    let rc = platform_window::dock_rect(hwnd);

    let cursor = platform_input::cursor_pos().unwrap_or(POINT { x: 0, y: 0 });
    if check_scope && edge_window_scope_contains_point(hwnd, cursor) {
        return;
    }
    let wait_leave_after_hide = edge_hotzone_rect(hwnd, state)
        .map(|hot| platform_window::point_in_rect_screen(&cursor, &hot))
        .unwrap_or(false);

    state.edge_restore_x = rc.left;
    state.edge_restore_y = rc.top;
    let (hide_x, hide_y) = match edge_hidden_position(hwnd, state, &rc) {
        Some(pos) => pos,
        None => return,
    };
    state.edge_hidden = true;
    state.edge_hide_armed = false;
    state.edge_hide_pending_until = None;
    state.edge_restore_wait_leave = wait_leave_after_hide;
    begin_edge_animation(
        hwnd,
        state,
        rc.left,
        rc.top,
        hide_x,
        hide_y,
        EDGE_AUTO_HIDE_ANIM_MS,
    );
    hide_hover_preview();
    platform_gdi::invalidate_rect(hwnd, null(), 0);
    refresh_low_level_input_hooks();
}

pub(super) unsafe fn handle_edge_auto_hide_tick(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() || !platform_window::is_visible(hwnd) {
        return;
    }
    let state = &mut *ptr;
    if state.edge_anim_until.is_some() {
        if !edge_step_animation(hwnd, state) {
            refresh_low_level_input_hooks();
        }
        return;
    }
    if !state.settings.edge_auto_hide {
        restore_edge_hidden_window(hwnd, state);
        clear_edge_dock_state(state);
        return;
    }

    let cursor = platform_input::cursor_pos().unwrap_or(POINT { x: 0, y: 0 });

    if state.edge_hidden {
        if let Some(hot) = edge_hotzone_rect(hwnd, state) {
            let in_hot = platform_window::point_in_rect_screen(&cursor, &hot);
            if state.edge_restore_wait_leave {
                if !in_hot {
                    state.edge_restore_wait_leave = false;
                }
                return;
            }
            if in_hot {
                restore_edge_hidden_window(hwnd, state);
                platform_gdi::invalidate_rect(hwnd, null(), 0);
            }
        } else {
            restore_edge_hidden_window(hwnd, state);
        }
        return;
    }

    let inside = edge_window_scope_contains_point(hwnd, cursor);
    if inside {
        state.edge_hide_armed = true;
        state.edge_hide_pending_until = None;
        return;
    }
    if edge_grace_active(state) {
        return;
    }
    if !state.edge_hide_armed {
        return;
    }
    if state.edge_hide_pending_until.is_none() {
        edge_set_hide_pending(state, EDGE_AUTO_HIDE_DELAY_MS);
        return;
    }
    if edge_hide_pending_active(state) {
        return;
    }
    hide_edge_docked_window_with_scope(hwnd, state, false);
    refresh_low_level_input_hooks();
}

pub(crate) unsafe fn note_window_moved_for_edge_hide(hwnd: HWND, state: &mut AppState) {
    if !state.settings.edge_auto_hide
        || state.edge_hidden
        || edge_animation_active(state)
        || !platform_window::is_visible(hwnd)
    {
        return;
    }
    let rc = platform_window::dock_rect(hwnd);
    if update_edge_dock_state(hwnd, state, &rc) {
        state.edge_hide_armed = false;
        state.edge_hide_pending_until = None;
        edge_set_grace(state, edge_interaction_grace_ms());
    } else {
        clear_edge_dock_state(state);
    }
}
