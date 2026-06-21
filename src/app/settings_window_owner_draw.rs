use super::prelude::*;

pub(super) unsafe fn draw_settings_window_item(hwnd: HWND, lparam: LPARAM) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return 0;
    }
    let st = &mut *st_ptr;
    let dis = &*(lparam as *const DRAWITEMSTRUCT);
    let rc0 = dis.rcItem;
    let w = (rc0.right - rc0.left).max(1);
    let h = (rc0.bottom - rc0.top).max(1);
    let memdc = platform_gdi::create_compatible_dc(dis.hDC);
    let bmp = platform_gdi::create_compatible_bitmap(dis.hDC, w, h);
    let oldbmp = platform_gdi::select_object(memdc, bmp as _);
    let th = Theme::default();
    let bg_fill = if is_settings_surface_control(dis.CtlID as isize) {
        th.surface
    } else {
        th.bg
    };
    let bg = platform_gdi::create_solid_brush(bg_fill);
    let local = RECT {
        left: 0,
        top: 0,
        right: w,
        bottom: h,
    };
    platform_gdi::fill_rect(memdc, &local, bg);
    platform_gdi::delete_object(bg as _);
    let mut dis2 = *dis;
    dis2.hDC = memdc;
    dis2.rcItem = local;
    settings_draw_button_item(st, &dis2);
    platform_gdi::copy_bits(dis.hDC, rc0.left, rc0.top, w, h, memdc, 0, 0);
    platform_gdi::select_object(memdc, oldbmp);
    platform_gdi::delete_object(bmp as _);
    platform_gdi::delete_dc(memdc);
    1
}
