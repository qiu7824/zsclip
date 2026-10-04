use super::prelude::*;
use crate::win_system_ui::draw_settings_list_item;
use windows_sys::Win32::UI::Controls::{ODS_DISABLED, ODT_LISTBOX};

const LB_GETTEXT_MSG: u32 = 0x0189;
const LB_GETTEXTLEN_MSG: u32 = 0x018A;

unsafe fn settings_list_item_text(list: HWND, index: u32) -> String {
    let len = platform_window::send_message(list, LB_GETTEXTLEN_MSG, index as usize, 0);
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let copied = platform_window::send_message(
        list,
        LB_GETTEXT_MSG,
        index as usize,
        buf.as_mut_ptr() as isize,
    );
    String::from_utf16_lossy(&buf[..copied.clamp(0, len) as usize])
}

unsafe fn draw_settings_list_window_item(dis: &DRAWITEMSTRUCT) -> LRESULT {
    let th = Theme::default();
    let text = if dis.itemID == u32::MAX {
        String::new()
    } else {
        settings_list_item_text(dis.hwndItem, dis.itemID)
    };
    draw_settings_list_item(
        dis.hDC as _,
        &dis.rcItem,
        &text,
        dis.itemID != u32::MAX && (dis.itemState & ODS_SELECTED) != 0,
        (dis.itemState & ODS_DISABLED) != 0,
        th,
    );
    1
}

pub(super) unsafe fn draw_settings_window_item(hwnd: HWND, lparam: LPARAM) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return 0;
    }
    let st = &mut *st_ptr;
    let dis = &*(lparam as *const DRAWITEMSTRUCT);
    if dis.CtlType == ODT_LISTBOX {
        return draw_settings_list_window_item(dis);
    }
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
