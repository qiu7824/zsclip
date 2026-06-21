use super::prelude::*;

pub(super) fn window_rect_or_empty(hwnd: HWND) -> RECT {
    platform_window::window_rect(hwnd).unwrap_or(RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    })
}

pub(super) unsafe fn resize_settings_window_for_dpi_transition(
    hwnd: HWND,
    old_dpi: u32,
    new_dpi: u32,
) {
    let Some(rc) = settings_window_bounds(hwnd).map(RECT::from) else {
        return;
    };
    set_settings_ui_dpi(new_dpi);
    let work = platform_monitor::nearest_work_rect_for_window(hwnd);
    let Some(plan) = settings_window_dpi_transition_plan(
        (&rc).into(),
        (&work).into(),
        settings_scale(12),
        settings_scale(720),
        settings_scale(480),
        old_dpi,
        new_dpi,
    ) else {
        return;
    };
    set_settings_window_bounds(
        hwnd,
        UiRect::new(plan.x, plan.y, plan.x + plan.width, plan.y + plan.height),
    );
}

pub(super) unsafe fn ensure_settings_window_in_work_area(hwnd: HWND) {
    let Some(rc) = settings_window_bounds(hwnd).map(RECT::from) else {
        return;
    };
    let work = platform_monitor::nearest_work_rect_for_window(hwnd);
    let Some(plan) = settings_window_fit_plan(
        (&rc).into(),
        (&work).into(),
        settings_scale(12),
        settings_scale(720),
        settings_scale(480),
    ) else {
        return;
    };
    set_settings_window_bounds(
        hwnd,
        UiRect::new(plan.x, plan.y, plan.x + plan.width, plan.y + plan.height),
    );
}

pub(super) unsafe fn reset_settings_dpi_compensation(st: &mut SettingsWndState) {
    st.dpi_comp.reset();
}

pub(super) unsafe fn update_settings_dpi_compensation_base(hwnd: HWND, st: &mut SettingsWndState) {
    if platform_dpi::is_per_monitor_aware() || st.dpi_comp.is_applying() {
        return;
    }
    let Some(rc) = settings_window_bounds(hwnd).map(RECT::from) else {
        return;
    };
    if rc.right <= rc.left || rc.bottom <= rc.top {
        return;
    }
    st.dpi_comp.set_base(
        rc.right - rc.left,
        rc.bottom - rc.top,
        platform_dpi::monitor_dpi_for_window(hwnd),
    );
}

pub(super) unsafe fn apply_settings_system_dpi_compensation(
    hwnd: HWND,
    st: &mut SettingsWndState,
) -> bool {
    if platform_dpi::is_per_monitor_aware() {
        reset_settings_dpi_compensation(st);
        return false;
    }
    let Some(rc) = settings_window_bounds(hwnd).map(RECT::from) else {
        return false;
    };
    if rc.right <= rc.left || rc.bottom <= rc.top {
        return false;
    }
    let monitor_dpi = platform_dpi::monitor_dpi_for_window(hwnd).max(96);
    let work = platform_monitor::nearest_work_rect_for_window(hwnd);
    let pad = settings_scale(12);
    let bounds = RECT {
        left: work.left + pad,
        top: work.top + pad,
        right: work.right - pad,
        bottom: work.bottom - pad,
    };
    let Some(plan) = st
        .dpi_comp
        .resize_plan((&rc).into(), (&bounds).into(), monitor_dpi, 2)
    else {
        return false;
    };
    st.dpi_comp.set_applying(true);
    set_settings_window_bounds(
        hwnd,
        UiRect::new(plan.x, plan.y, plan.x + plan.width, plan.y + plan.height),
    );
    st.dpi_comp.finish_resize(plan.monitor_dpi);
    true
}
