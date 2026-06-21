use super::prelude::*;
use crate::platform::string::to_wide;
use crate::win_ui_render::{DT_LEFT, DT_SINGLELINE, DT_VCENTER};

pub(super) unsafe fn draw_settings_source_link_item(
    dis: &DRAWITEMSTRUCT,
    text: &str,
    hover: bool,
    pressed: bool,
    th: Theme,
) -> bool {
    if dis.CtlID as isize != IDC_SET_OPEN_SOURCE {
        return false;
    }

    let hdc = dis.hDC;
    let rc = dis.rcItem;
    let text_color = if open_source_url().trim().is_empty() {
        th.text_muted
    } else if pressed {
        rgb(22, 78, 180)
    } else if hover {
        rgb(14, 111, 214)
    } else {
        rgb(24, 92, 189)
    };
    let font_px = platform_dpi::scale_for_window(dis.hwndItem, 14).max(12);
    let font = platform_gdi::create_font_w(
        -font_px,
        0,
        0,
        0,
        400,
        0,
        1,
        0,
        1,
        0,
        0,
        5,
        0,
        to_wide("Segoe UI").as_ptr(),
    ) as *mut core::ffi::c_void;
    let old_font = if !font.is_null() {
        platform_gdi::select_object(hdc, font)
    } else {
        null_mut()
    };
    platform_gdi::set_bk_mode(hdc, 1);
    platform_gdi::set_text_color(hdc, text_color);
    let mut text_rc = rc;
    text_rc.left += if pressed { 5 } else { 4 };
    text_rc.top += if pressed { 1 } else { 0 };
    let text_w = to_wide(text);
    platform_gdi::draw_text(
        hdc,
        text_w.as_ptr(),
        -1,
        &mut text_rc,
        DT_LEFT | DT_VCENTER | DT_SINGLELINE,
    );
    if !old_font.is_null() {
        platform_gdi::select_object(hdc, old_font);
    }
    if !font.is_null() {
        platform_gdi::delete_object(font as _);
    }
    true
}
