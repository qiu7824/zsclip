use super::prelude::*;

#[derive(Clone, Copy)]
pub(super) enum SettingsControlColorRole {
    Static,
    Edit,
    List,
}

pub(super) unsafe fn settings_control_color(
    hwnd: HWND,
    wparam: WPARAM,
    role: SettingsControlColorRole,
) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    let hdc = wparam as *mut core::ffi::c_void;
    if st_ptr.is_null() {
        return 0;
    }
    let theme = Theme::default();
    platform_gdi::set_bk_mode(hdc, 2);
    match role {
        SettingsControlColorRole::Static | SettingsControlColorRole::List => {
            platform_gdi::set_bk_color(hdc, theme.surface);
            platform_gdi::set_text_color(hdc, theme.text);
            (*st_ptr).surface_brush as isize
        }
        SettingsControlColorRole::Edit => {
            platform_gdi::set_bk_color(hdc, theme.control_bg);
            platform_gdi::set_text_color(hdc, theme.text);
            (*st_ptr).control_brush as isize
        }
    }
}
