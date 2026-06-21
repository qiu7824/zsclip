use super::prelude::*;
use crate::platform::gdi as platform_gdi;
use crate::win_system_ui::{settings_host_set_text, settings_host_set_visible_enabled};

pub(super) unsafe fn settings_set_text(hwnd: HWND, s: &str) {
    settings_host_set_text(hwnd, s);
}

pub(super) unsafe fn settings_show_enable(hwnd: HWND, visible: bool, enabled: bool) {
    if hwnd.is_null() {
        return;
    }
    settings_host_set_visible_enabled(hwnd, visible, enabled);
}

pub(super) unsafe fn settings_invalidate_page_ctrls(
    hwnd: HWND,
    st: &SettingsWndState,
    page: usize,
) {
    for reg in st.ui.page_regs(page) {
        if !reg.hwnd.is_null() {
            platform_gdi::invalidate_rect(reg.hwnd, null(), 1);
        }
    }
    if let Some(rc) = platform_window::client_rect(hwnd) {
        let viewport = settings_viewport_rect(&rc);
        platform_gdi::invalidate_rect(hwnd, &viewport, 0);
    }
}

pub(super) unsafe fn settings_refresh_theme_resources(st: &mut SettingsWndState) {
    if !st.bg_brush.is_null() {
        platform_gdi::delete_object(st.bg_brush as _);
    }
    if !st.surface_brush.is_null() {
        platform_gdi::delete_object(st.surface_brush as _);
    }
    if !st.control_brush.is_null() {
        platform_gdi::delete_object(st.control_brush as _);
    }
    if !st.nav_brush.is_null() {
        platform_gdi::delete_object(st.nav_brush as _);
    }
    let th = Theme::default();
    st.bg_brush = platform_gdi::create_solid_brush(th.bg) as _;
    st.surface_brush = platform_gdi::create_solid_brush(th.surface) as _;
    st.control_brush = platform_gdi::create_solid_brush(th.control_bg) as _;
    st.nav_brush = platform_gdi::create_solid_brush(th.nav_bg) as _;
}
