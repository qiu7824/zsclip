use super::prelude::*;

pub(super) unsafe fn handle_settings_destroy(hwnd: HWND) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if !st_ptr.is_null() {
        cancel_settings_scroll_drag(hwnd, &mut *st_ptr);
        timer::stop(hwnd, ID_TIMER_SETTINGS_SCROLLBAR);
        timer::stop(hwnd, ID_TIMER_SETTINGS_SAVE_HINT);
        timer::stop(hwnd, ID_TIMER_SETTINGS_DPI_FIT);
        let parent = (*st_ptr).parent_hwnd;
        if !(*st_ptr).dropdown_popup.is_null() {
            if settings_dropdown_popup_exists((*st_ptr).dropdown_popup) {
                destroy_settings_dropdown_popup((*st_ptr).dropdown_popup);
            }
            (*st_ptr).dropdown_popup = null_mut();
        }
        if !(*st_ptr).nav_font.is_null() {
            platform_gdi::delete_object((*st_ptr).nav_font as _);
        }
        if !(*st_ptr).ui_font.is_null()
            && (*st_ptr).ui_font != platform_gdi::get_stock_object(DEFAULT_GUI_FONT)
        {
            platform_gdi::delete_object((*st_ptr).ui_font as _);
        }
        if !(*st_ptr).title_font.is_null()
            && (*st_ptr).title_font != platform_gdi::get_stock_object(DEFAULT_GUI_FONT)
        {
            platform_gdi::delete_object((*st_ptr).title_font as _);
        }
        if !(*st_ptr).bg_brush.is_null() {
            platform_gdi::delete_object((*st_ptr).bg_brush as _);
        }
        if !(*st_ptr).surface_brush.is_null() {
            platform_gdi::delete_object((*st_ptr).surface_brush as _);
        }
        if !(*st_ptr).control_brush.is_null() {
            platform_gdi::delete_object((*st_ptr).control_brush as _);
        }
        if !(*st_ptr).nav_brush.is_null() {
            platform_gdi::delete_object((*st_ptr).nav_brush as _);
        }
        drop(Box::from_raw(st_ptr));
        platform_window::set_user_data(hwnd, 0);
        let pst = get_state_ptr(parent);
        if !pst.is_null() {
            (*pst).settings_hwnd = null_mut();
        }
        refresh_low_level_input_hooks();
    }
    0
}
