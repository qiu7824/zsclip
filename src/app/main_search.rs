use super::prelude::*;

pub(crate) unsafe fn layout_children(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let plan = state.layout().search_host_plan(state.search_on);
    let mut search_host = WindowsMainSearchControlHost::new();
    search_host.set_search_bounds(state.search_hwnd, plan.input_rect);
    search_host.set_search_visible(state.search_hwnd, plan.visible);
}

pub(super) unsafe fn refresh_search_font(state: &mut AppState) {
    let old_font = state.search_font;
    let request = NativeMainSearchStyleRequest {
        handle: state.search_hwnd,
        font_family: ui_text_font_family().to_string(),
        font_px: platform_dpi::scale_for_window(state.hwnd, 14),
        previous_resource: (!old_font.is_null()).then_some(old_font),
    };
    match WindowsMainSearchControlHost::new().apply_search_style(request) {
        NativeMainSearchStylePresentation::Applied(resource) => {
            state.search_font = resource.unwrap_or(null_mut());
        }
        NativeMainSearchStylePresentation::Failed => {
            state.search_font = null_mut();
        }
    }
}

pub(crate) unsafe fn reset_search_ui_state(state: &mut AppState) {
    let Some(plan) = state.list.search_reset_plan() else {
        return;
    };
    if !state.hwnd.is_null() && state.search_debounce_timer {
        stop_search_debounce_timer(state.hwnd, state);
    }
    state.list.apply_search_reset_plan(plan);
    WindowsMainSearchControlHost::new().set_search_text(state.search_hwnd, "");
    state.refilter();
    if !state.hwnd.is_null() {
        layout_children(state.hwnd);
        repaint_main_window(state.hwnd, true);
    }
}

pub(crate) unsafe fn prepare_search_ui_for_show(hwnd: HWND, state: &mut AppState) {
    if state.settings.persistent_search_box {
        state.search_on = true;
        layout_children(hwnd);
        repaint_main_window(hwnd, false);
    } else {
        reset_search_ui_state(state);
    }
}

pub(super) unsafe fn activate_window_for_search_input(hwnd: HWND, state: &mut AppState) {
    let mut main_window_host = WindowsMainWindowHost::new(Some(wnd_proc));
    main_window_host.set_main_window_activation_policy(hwnd, true);
    main_window_host.activate_main_window(hwnd);
    WindowsMainSearchControlHost::new().focus_search(state.search_hwnd);
}

pub(super) unsafe fn search_visibility_plan_for_request(
    state: &AppState,
    request: MainSearchVisibilityRequest,
) -> MainSearchVisibilityPlan {
    main_search_visibility_plan(MainSearchVisibilityInput {
        request,
        search_on: state.search_on,
        search_text_empty: state.search_text.is_empty(),
        persistent_search_box: state.settings.persistent_search_box,
        main_window_noactivate: state.main_window_noactivate,
        quick_window: state.role == WindowRole::Quick,
    })
}

pub(super) unsafe fn apply_search_visibility_plan(
    hwnd: HWND,
    state: &mut AppState,
    plan: MainSearchVisibilityPlan,
) {
    if plan.stop_debounce_timer && state.search_debounce_timer {
        stop_search_debounce_timer(hwnd, state);
    }
    state.search_on = plan.search_on;
    if plan.clear_search_text {
        state.search_text.clear();
        WindowsMainSearchControlHost::new().set_search_text(state.search_hwnd, "");
    }
    if plan.clear_selection {
        state.list.clear_selection();
    }
    if plan.refilter {
        state.refilter();
    }
    if plan.relayout {
        layout_children(hwnd);
    }
    if plan.activate_window {
        activate_window_for_search_input(hwnd, state);
    } else if matches!(
        plan.action,
        crate::app_core::MainSearchVisibilityAction::Open
    ) {
        WindowsMainSearchControlHost::new().focus_search(state.search_hwnd);
    }
    if plan.invalidate {
        repaint_main_window(hwnd, true);
    }
}

pub(super) unsafe fn close_search_ui(hwnd: HWND, state: &mut AppState) {
    let plan = search_visibility_plan_for_request(state, MainSearchVisibilityRequest::Close);
    apply_search_visibility_plan(hwnd, state, plan);
}

pub(super) unsafe fn handle_search_control_command(
    hwnd: HWND,
    state: &mut AppState,
    id: usize,
    notification: u16,
) -> bool {
    if id != IDC_SEARCH as usize || notification != EN_CHANGE_CODE {
        return false;
    }
    state.search_text = WindowsMainSearchControlHost::new().search_text(state.search_hwnd);
    start_flagged_timer(
        hwnd,
        ID_TIMER_SEARCH_DEBOUNCE,
        180,
        &mut state.search_debounce_timer,
    );
    true
}
