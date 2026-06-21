use super::prelude::*;

unsafe fn reclaim_window_state_memory(hwnd: HWND, state: &mut AppState) {
    hide_hover_preview();
    clear_page_load_results_for_hwnd(hwnd);
    clear_cloud_sync_results_for_hwnd(hwnd);
    state.release_list_memory();
    crate::win_ui_render::release_idle_memory();
    if !state.settings.lan_sync_enabled {
        lan_sync::release_idle_memory();
    }
}

unsafe fn reclaim_hidden_peer_window_memory(current_hwnd: HWND) {
    for hwnd in window_host_hwnds() {
        if !platform_window::exists(hwnd)
            || hwnd == current_hwnd
            || platform_window::is_visible(hwnd)
        {
            continue;
        }
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            reclaim_window_state_memory(hwnd, &mut *ptr);
        }
    }
}

pub(super) unsafe fn reclaim_hidden_window_memory(hwnd: HWND, state: &mut AppState) {
    reclaim_window_state_memory(hwnd, state);
    reclaim_hidden_peer_window_memory(hwnd);
    platform_process::trim_current_working_set();
}

pub(super) unsafe fn schedule_hidden_memory_reclaim(hwnd: HWND, state: &mut AppState) {
    start_flagged_timer(
        hwnd,
        ID_TIMER_HIDDEN_RECLAIM,
        800,
        &mut state.hidden_reclaim_timer,
    );
}

pub(super) unsafe fn cancel_hidden_memory_reclaim(hwnd: HWND, state: &mut AppState) {
    stop_flagged_timer(
        hwnd,
        ID_TIMER_HIDDEN_RECLAIM,
        &mut state.hidden_reclaim_timer,
    );
}

pub(super) unsafe fn show_main_scrollbar_feedback(hwnd: HWND, state: &mut AppState, erase: bool) {
    state.scroll_fade_alpha = 255;
    start_flagged_timer(hwnd, ID_TIMER_SCROLL_FADE, 50, &mut state.scroll_fade_timer);
    repaint_main_window(hwnd, erase);
}

pub(super) fn main_layout_for_dpi(dpi: u32) -> MainUiLayout {
    MAIN_UI_LAYOUT.scaled(dpi.max(96))
}

fn main_window_size_for_dpi(dpi: u32) -> (i32, i32) {
    let layout = main_layout_for_dpi(dpi);
    (layout.win_w, layout.list_y + layout.list_h + 7)
}

pub(super) unsafe fn main_layout_for_window(hwnd: HWND) -> MainUiLayout {
    main_layout_for_dpi(main_window_layout_dpi(hwnd))
}

pub(super) unsafe fn handle_main_window_size(hwnd: HWND, _size: UiSize, minimized: bool) {
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        let state = &mut *ptr;
        if minimized {
            reclaim_hidden_window_memory(hwnd, state);
            schedule_hidden_memory_reclaim(hwnd, state);
        } else {
            cancel_hidden_memory_reclaim(hwnd, state);
        }
    }
    WindowsMainWindowHost::new(Some(wnd_proc)).apply_main_window_appearance(hwnd);
    layout_children(hwnd);
    repaint_main_window(hwnd, true);
}

pub(super) unsafe fn handle_main_app_activation_changed(hwnd: HWND, active: bool) {
    if active {
        return;
    }
    clear_main_hover_state(hwnd);
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        let state = &mut *ptr;
        if state.settings.auto_hide_on_blur
            && !state.main_window_noactivate
            && state.role != WindowRole::Quick
            && !vv_popup_menu_active()
            && platform_input::cursor_pos()
                .map(|pt| !edge_window_scope_contains_point(hwnd, pt))
                .unwrap_or(true)
        {
            hide_hover_preview();
            WindowsMainWindowHost::new(Some(wnd_proc)).hide_main_window(hwnd);
            refresh_low_level_input_hooks();
        }
    }
}

pub(super) unsafe fn handle_main_system_metrics_changed(hwnd: HWND) {
    refresh_main_window_metrics(hwnd);
    timer::start(hwnd, ID_TIMER_DPI_FIT, 60);
}

pub(super) unsafe fn handle_main_window_moved(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        if (*ptr).role == WindowRole::Main && !edge_animation_active(&*ptr) {
            remember_window_pos(hwnd);
        }
        note_window_moved_for_edge_hide(hwnd, &mut *ptr);
        let dpi = main_window_layout_dpi(hwnd);
        if (*ptr).ui_dpi != dpi.max(96) {
            refresh_main_window_layout_for_monitor(hwnd, &mut *ptr, Some(dpi));
        }
        if !platform_dpi::is_per_monitor_aware() {
            timer::start(hwnd, ID_TIMER_DPI_FIT, 60);
        }
    }
}

pub(super) unsafe fn handle_main_window_move_completed(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        ensure_main_window_size_for_monitor(hwnd, &mut *ptr);
        if (*ptr).role == WindowRole::Main && !edge_animation_active(&*ptr) {
            remember_window_pos(hwnd);
        }
    }
}

pub(super) unsafe fn handle_main_close_requested(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() && close_to_tray_enabled(&(*ptr).settings) {
        hide_main_window(hwnd);
        return;
    }
    destroy_main_window(hwnd);
}

pub(super) unsafe fn handle_main_lifecycle_event(hwnd: HWND, event: LifecycleEvent) {
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        let state = &mut *ptr;
        state.ui_lifecycle.apply(event);
        match event {
            LifecycleEvent::Suspend => {
                reclaim_hidden_window_memory(hwnd, state);
                schedule_hidden_memory_reclaim(hwnd, state);
            }
            LifecycleEvent::Resume => {
                cancel_hidden_memory_reclaim(hwnd, state);
                timer::start(hwnd, ID_TIMER_DPI_FIT, 60);
            }
            LifecycleEvent::Unmount => {
                handle_main_destroy(hwnd, state);
            }
            LifecycleEvent::Mount => {}
        }
    }
    if matches!(event, LifecycleEvent::Suspend | LifecycleEvent::Resume) {
        refresh_low_level_input_hooks();
    }
}

unsafe fn handle_main_destroy(hwnd: HWND, state: &mut AppState) {
    cancel_main_scroll_drag(hwnd, state);
    clear_page_load_results_for_hwnd(hwnd);
    clear_cloud_sync_results_for_hwnd(hwnd);
    match state.role {
        WindowRole::Main => {
            save_settings(&state.settings);
            timer::stop(hwnd, ID_TIMER_STARTUP_RECOVERY);
            timer::stop(hwnd, ID_TIMER_VV_WATCH);
            timer::stop(hwnd, ID_TIMER_VV_SHOW);
            timer::stop(hwnd, ID_TIMER_PASTE);
            timer::stop(hwnd, ID_TIMER_SEARCH_DEBOUNCE);
            timer::stop(hwnd, ID_TIMER_HIDDEN_RECLAIM);
            timer::stop(hwnd, ID_TIMER_CLIPBOARD_RETRY);
            timer::stop(hwnd, ID_TIMER_DPI_FIT);
            timer::stop(hwnd, ID_TIMER_SCROLL_FADE);
            timer::stop(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
            timer::stop(hwnd, ID_TIMER_OUTSIDE_HIDE);
            timer::stop(hwnd, ID_TIMER_CLOUD_SYNC);
            let popup = current_vv_popup_hwnd();
            if platform_window::exists(popup) {
                destroy_vv_popup_window(popup);
            }
            let _ = update_vv_mode_hook(hwnd, false);
            lan_sync::stop_service();
            shutdown_low_level_input_hooks();
            unregister_clipboard_listener_for(hwnd, state);
            unregister_hotkey_for(hwnd, state);
            unregister_plain_paste_hotkey_for(hwnd, state);
            remove_tray_icon(hwnd);
            let quick = quick_window_hwnd();
            if quick != hwnd && platform_window::exists(quick) {
                destroy_main_window(quick);
            }
            platform_window::post_quit_message(0);
        }
        WindowRole::Quick => {
            timer::stop(hwnd, ID_TIMER_PASTE);
            timer::stop(hwnd, ID_TIMER_SEARCH_DEBOUNCE);
            timer::stop(hwnd, ID_TIMER_HIDDEN_RECLAIM);
            timer::stop(hwnd, ID_TIMER_CLIPBOARD_RETRY);
            timer::stop(hwnd, ID_TIMER_DPI_FIT);
            timer::stop(hwnd, ID_TIMER_SCROLL_FADE);
            timer::stop(hwnd, ID_TIMER_EDGE_AUTO_HIDE);
            timer::stop(hwnd, ID_TIMER_OUTSIDE_HIDE);
            refresh_low_level_input_hooks();
        }
    }
}

pub(super) unsafe fn handle_main_dpi_changed(hwnd: HWND, dpi: u32) {
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        reset_main_dpi_compensation(&mut *ptr);
        refresh_main_window_layout_for_monitor(hwnd, &mut *ptr, Some(dpi));
    }
}

unsafe fn refresh_main_window_metrics(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    if let Some(rc) = main_window_bounds(hwnd) {
        let layout = main_layout_for_window(hwnd);
        let win_h = layout.list_y + layout.list_h + 7;
        set_main_window_bounds(
            hwnd,
            UiRect::new(rc.left, rc.top, rc.left + layout.win_w, rc.top + win_h),
        );
    }
    refresh_main_window_layout_only(hwnd, state);
}

pub(super) unsafe fn ensure_main_window_size_for_monitor(hwnd: HWND, state: &mut AppState) {
    if edge_animation_active(state) || state.edge_hidden {
        return;
    }
    if !platform_dpi::is_per_monitor_aware() {
        if apply_main_system_dpi_compensation(hwnd, state) {
            return;
        }
        refresh_main_window_layout_for_monitor(hwnd, state, Some(main_window_layout_dpi(hwnd)));
        return;
    }
    let dpi = main_window_layout_dpi(hwnd).max(96);
    let (win_w, win_h) = main_window_size_for_dpi(dpi);
    let Some(rc) = main_window_bounds(hwnd) else {
        refresh_main_window_layout_for_monitor(hwnd, state, Some(dpi));
        return;
    };
    if rc.right <= rc.left || rc.bottom <= rc.top {
        refresh_main_window_layout_for_monitor(hwnd, state, Some(dpi));
        return;
    }
    let cur_w = rc.right - rc.left;
    let cur_h = rc.bottom - rc.top;
    if state.ui_dpi != dpi || (cur_w - win_w).abs() > 2 || (cur_h - win_h).abs() > 2 {
        let work = platform_monitor::nearest_work_rect_for_window(hwnd);
        let (x, y) = clamp_window_pos_to_rect(rc.left, rc.top, (&work).into(), win_w, win_h);
        set_main_window_bounds(hwnd, UiRect::new(x, y, x + win_w, y + win_h));
        refresh_main_window_layout_for_monitor(hwnd, state, Some(dpi));
    }
}

unsafe fn reset_main_dpi_compensation(state: &mut AppState) {
    state.dpi_comp.reset();
}

unsafe fn apply_main_system_dpi_compensation(hwnd: HWND, state: &mut AppState) -> bool {
    if platform_dpi::is_per_monitor_aware() {
        reset_main_dpi_compensation(state);
        return false;
    }
    let Some(rc) = main_window_bounds(hwnd) else {
        return false;
    };
    if rc.right <= rc.left || rc.bottom <= rc.top {
        return false;
    }
    let monitor_dpi = platform_dpi::monitor_dpi_for_window(hwnd).max(96);
    let work = platform_monitor::nearest_work_rect_for_window(hwnd);
    let Some(plan) = state
        .dpi_comp
        .resize_plan(rc, (&work).into(), monitor_dpi, 2)
    else {
        return false;
    };
    state.dpi_comp.set_applying(true);
    set_main_window_bounds(
        hwnd,
        UiRect::new(plan.x, plan.y, plan.x + plan.width, plan.y + plan.height),
    );
    state.dpi_comp.finish_resize(plan.monitor_dpi);
    refresh_main_window_layout_for_monitor(hwnd, state, Some(main_window_layout_dpi(hwnd)));
    true
}

unsafe fn sync_main_window_dpi(state: &mut AppState, dpi: u32) -> bool {
    let next = dpi.max(96);
    if state.ui_dpi == next {
        return false;
    }
    state.ui_dpi = next;
    true
}

pub(crate) unsafe fn refresh_main_window_layout_for_monitor(
    hwnd: HWND,
    state: &mut AppState,
    forced_dpi: Option<u32>,
) {
    let dpi = forced_dpi.unwrap_or_else(|| main_window_layout_dpi(hwnd));
    let _ = sync_main_window_dpi(state, dpi);
    refresh_search_font(state);
    layout_children(hwnd);
    repaint_main_window(hwnd, true);
}

unsafe fn refresh_main_window_layout_only(hwnd: HWND, state: &mut AppState) {
    refresh_main_window_layout_for_monitor(hwnd, state, None);
}

pub(crate) fn set_main_window_activation_policy(hwnd: HWND, allow_activation: bool) {
    WindowsMainWindowHost::new(Some(wnd_proc))
        .set_main_window_activation_policy(hwnd, allow_activation);
}

pub(crate) fn present_main_window(hwnd: HWND, mode: NativeMainWindowPresentMode) {
    WindowsMainWindowHost::new(Some(wnd_proc)).present_main_window(hwnd, mode);
}

pub(crate) fn hide_main_window(hwnd: HWND) {
    WindowsMainWindowHost::new(Some(wnd_proc)).hide_main_window(hwnd);
}

pub(super) fn destroy_main_window(hwnd: HWND) {
    WindowsMainWindowHost::new(Some(wnd_proc)).destroy_main_window(hwnd);
}

pub(crate) fn set_main_window_bounds(hwnd: HWND, bounds: UiRect) {
    WindowsMainWindowHost::new(Some(wnd_proc)).set_main_window_bounds(hwnd, bounds);
}

pub(super) fn capture_main_pointer(hwnd: HWND) {
    WindowsMainWindowHost::new(Some(wnd_proc)).capture_main_pointer(hwnd);
}

pub(super) fn release_main_pointer(hwnd: HWND) {
    WindowsMainWindowHost::new(Some(wnd_proc)).release_main_pointer(hwnd);
}

pub(super) fn begin_main_window_drag(hwnd: HWND) {
    WindowsMainWindowHost::new(Some(wnd_proc)).begin_main_window_drag(hwnd);
}
