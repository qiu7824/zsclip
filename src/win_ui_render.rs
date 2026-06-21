use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::platform::gdi as platform_gdi;
use crate::platform::gdiplus;
use crate::win_native_style::ui_text_font_family;
use crate::win_system_ui::{draw_translated_text_block, draw_translated_text_line};

use windows_sys::Win32::Foundation::RECT;

pub(crate) const DT_LEFT: u32 = 0x0000;
pub(crate) const DT_CENTER: u32 = 0x0001;
pub(crate) const DT_VCENTER: u32 = 0x0004;
pub(crate) const DT_WORDBREAK: u32 = 0x0010;
pub(crate) const DT_SINGLELINE: u32 = 0x0020;
pub(crate) const DT_END_ELLIPSIS: u32 = 0x0000_8000;
pub(crate) const DT_NOPREFIX: u32 = 0x0000_0800;
pub(crate) const TRANSPARENT: i32 = 1;

static DARK_ICON_CACHE: OnceLock<Mutex<HashMap<(isize, i32, i32, u8), Vec<u32>>>> = OnceLock::new();

pub(crate) fn release_idle_memory() {
    if let Some(cache) = DARK_ICON_CACHE.get() {
        if let Ok(mut cache) = cache.lock() {
            cache.clear();
            cache.shrink_to_fit();
        }
    }
}

pub unsafe fn draw_round_rect(
    hdc: *mut core::ffi::c_void,
    rc: &RECT,
    fill: u32,
    border: u32,
    radius: i32,
) {
    if gdiplus::draw_round_rect(
        hdc,
        rc.left,
        rc.top,
        rc.right,
        rc.bottom,
        fill,
        border,
        radius.max(1),
    ) {
        return;
    }
    let er = (radius.max(1)) * 2;
    if border != 0 && border != fill {
        let outer_pen = platform_gdi::null_pen();
        let outer_br = platform_gdi::create_solid_brush(border);
        let old_pen = platform_gdi::select_object(hdc, outer_pen as _);
        let old_br = platform_gdi::select_object(hdc, outer_br as _);
        platform_gdi::round_rect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
        platform_gdi::select_object(hdc, old_pen);
        platform_gdi::select_object(hdc, old_br);
        platform_gdi::delete_object(outer_br as _);

        let inner = RECT {
            left: rc.left + 1,
            top: rc.top + 1,
            right: rc.right - 1,
            bottom: rc.bottom - 1,
        };
        if inner.right > inner.left && inner.bottom > inner.top {
            let inner_br = platform_gdi::create_solid_brush(fill);
            let old_pen2 = platform_gdi::select_object(hdc, outer_pen as _);
            let old_br2 = platform_gdi::select_object(hdc, inner_br as _);
            let inner_r = (radius - 1).max(1) * 2;
            platform_gdi::round_rect(
                hdc,
                inner.left,
                inner.top,
                inner.right,
                inner.bottom,
                inner_r,
                inner_r,
            );
            platform_gdi::select_object(hdc, old_pen2);
            platform_gdi::select_object(hdc, old_br2);
            platform_gdi::delete_object(inner_br as _);
        }
    } else {
        let pen = platform_gdi::null_pen();
        let brush = platform_gdi::create_solid_brush(fill);
        let old_pen = platform_gdi::select_object(hdc, pen as _);
        let old_br = platform_gdi::select_object(hdc, brush as _);
        platform_gdi::round_rect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
        platform_gdi::select_object(hdc, old_pen);
        platform_gdi::select_object(hdc, old_br);
        platform_gdi::delete_object(brush as _);
    }
}

pub unsafe fn draw_round_fill(hdc: *mut core::ffi::c_void, rc: &RECT, fill: u32, radius: i32) {
    if gdiplus::draw_round_rect(
        hdc,
        rc.left,
        rc.top,
        rc.right,
        rc.bottom,
        fill,
        fill,
        radius.max(1),
    ) {
        return;
    }
    let er = (radius.max(1)) * 2;
    let pen = platform_gdi::null_pen();
    let brush = platform_gdi::create_solid_brush(fill);
    let old_pen = platform_gdi::select_object(hdc, pen as _);
    let old_br = platform_gdi::select_object(hdc, brush as _);
    platform_gdi::round_rect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
    platform_gdi::select_object(hdc, old_pen);
    platform_gdi::select_object(hdc, old_br);
    platform_gdi::delete_object(brush as _);
}

pub unsafe fn draw_text(
    hdc: *mut core::ffi::c_void,
    text: &str,
    rc: &RECT,
    color: u32,
    size: i32,
    bold: bool,
    center: bool,
) {
    draw_text_ex(
        hdc,
        text,
        rc,
        color,
        size,
        bold,
        center,
        ui_text_font_family(),
    );
}

pub unsafe fn draw_text_block(
    hdc: *mut core::ffi::c_void,
    text: &str,
    rc: &RECT,
    color: u32,
    size: i32,
    bold: bool,
) {
    draw_text_block_ex(hdc, text, rc, color, size, bold, ui_text_font_family());
}

pub unsafe fn draw_text_ex(
    hdc: *mut core::ffi::c_void,
    text: &str,
    rc: &RECT,
    color: u32,
    size: i32,
    bold: bool,
    center: bool,
    family: &str,
) {
    let weight = if bold { 700 } else { 400 };
    let mut rc2 = *rc;
    draw_translated_text_line(
        hdc,
        text,
        &mut rc2,
        color,
        size,
        weight,
        center,
        family,
        TRANSPARENT,
        0,
    );
}

pub unsafe fn draw_text_block_ex(
    hdc: *mut core::ffi::c_void,
    text: &str,
    rc: &RECT,
    color: u32,
    size: i32,
    bold: bool,
    family: &str,
) {
    let weight = if bold { 700 } else { 400 };
    let mut rc2 = *rc;
    draw_translated_text_block(
        hdc,
        text,
        &mut rc2,
        color,
        size,
        weight,
        family,
        TRANSPARENT,
        0,
    );
}

/// 在深色模式绘制图标时，将深色图标转换为浅色版本。
pub unsafe fn draw_icon_tinted_soft(
    hdc: *mut core::ffi::c_void,
    x: i32,
    y: i32,
    icon: isize,
    w: i32,
    h: i32,
    dark: bool,
    soften: u8,
) {
    if icon == 0 {
        return;
    }
    if !dark {
        platform_gdi::draw_icon_normal(hdc, x, y, icon as _, w, h);
        return;
    }
    let n = (w * h) as usize;
    let derived = {
        let cache = DARK_ICON_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
        let mut cache = match cache.lock() {
            Ok(cache) => cache,
            Err(_) => {
                platform_gdi::draw_icon_normal(hdc, x, y, icon as _, w, h);
                return;
            }
        };
        if let Some(cached) = cache.get(&(icon, w, h, soften)) {
            cached.clone()
        } else {
            let make_dib = |bg: u32| -> (*mut core::ffi::c_void, *mut core::ffi::c_void, *mut u32) {
                let dc = platform_gdi::create_compatible_dc(hdc);
                let (dib, ptr) = platform_gdi::create_top_down_32bpp_dib(dc, w, h);
                if dib.is_null() || ptr.is_null() {
                    platform_gdi::delete_dc(dc);
                    return (
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                        core::ptr::null_mut(),
                    );
                }
                platform_gdi::select_object(dc, dib as _);
                let br = platform_gdi::create_solid_brush(bg);
                let rc = RECT {
                    left: 0,
                    top: 0,
                    right: w,
                    bottom: h,
                };
                platform_gdi::fill_rect(dc, &rc, br);
                platform_gdi::delete_object(br as _);
                platform_gdi::draw_icon_normal(dc, 0, 0, icon as _, w, h);
                (dc, dib, ptr as *mut u32)
            };

            let (dc_w, dib_w, px_w) = make_dib(0x00FFFFFFu32);
            let (dc_b, dib_b, px_b) = make_dib(0x00000000u32);
            if dc_w.is_null() || dc_b.is_null() {
                if !dc_w.is_null() {
                    platform_gdi::delete_dc(dc_w);
                }
                if !dc_b.is_null() {
                    platform_gdi::delete_dc(dc_b);
                }
                platform_gdi::draw_icon_normal(hdc, x, y, icon as _, w, h);
                return;
            }

            let src_w = core::slice::from_raw_parts(px_w, n);
            let src_b = core::slice::from_raw_parts(px_b, n);
            let mut derived = vec![0u32; n];
            for i in 0..n {
                let w_px = src_w[i];
                let b_px = src_b[i];
                let wr = ((w_px >> 16) & 0xFF) as i32;
                let wg = ((w_px >> 8) & 0xFF) as i32;
                let wb = (w_px & 0xFF) as i32;
                let br = ((b_px >> 16) & 0xFF) as i32;
                let bg = ((b_px >> 8) & 0xFF) as i32;
                let bb = (b_px & 0xFF) as i32;
                let ar = 255 - (wr - br).clamp(0, 255);
                let ag = 255 - (wg - bg).clamp(0, 255);
                let ab = 255 - (wb - bb).clamp(0, 255);
                let alpha = ((ar + ag + ab) / 3).clamp(0, 255) as u32;
                if alpha < 8 {
                    continue;
                }
                let icon_r = ((br * 255 / ar.max(1)) as u32).min(255);
                let icon_g = ((bg * 255 / ag.max(1)) as u32).min(255);
                let icon_b = ((bb * 255 / ab.max(1)) as u32).min(255);
                let lum = (icon_r * 299 + icon_g * 587 + icon_b * 114) / 1000;
                let (mut out_r, mut out_g, mut out_b) = if lum < 80 {
                    (255u32, 255u32, 255u32)
                } else if lum < 200 {
                    let bright = (255 - lum + 180).min(255);
                    (bright, bright, bright)
                } else {
                    (icon_r, icon_g, icon_b)
                };
                if soften > 0 {
                    let k = soften as u32;
                    let base = 32u32;
                    out_r = ((out_r * (255 - k)) + (base * k)) / 255;
                    out_g = ((out_g * (255 - k)) + (base * k)) / 255;
                    out_b = ((out_b * (255 - k)) + (base * k)) / 255;
                }
                derived[i] = (alpha << 24) | (out_r << 16) | (out_g << 8) | out_b;
            }
            platform_gdi::delete_object(dib_w as _);
            platform_gdi::delete_object(dib_b as _);
            platform_gdi::delete_dc(dc_w);
            platform_gdi::delete_dc(dc_b);
            cache.insert((icon, w, h, soften), derived.clone());
            derived
        }
    };

    let dc_bg = platform_gdi::create_compatible_dc(hdc);
    let (dib_bg, px_bg_ptr) = platform_gdi::create_top_down_32bpp_dib(dc_bg, w, h);
    platform_gdi::select_object(dc_bg, dib_bg as _);
    platform_gdi::copy_bits(dc_bg, 0, 0, w, h, hdc, x, y);
    let src_bg = if !dib_bg.is_null() && !px_bg_ptr.is_null() {
        core::slice::from_raw_parts(px_bg_ptr as *const u32, n)
    } else {
        &[] as &[u32]
    };

    let dc_out = platform_gdi::create_compatible_dc(hdc);
    let (dib_out, px_out_ptr) = platform_gdi::create_top_down_32bpp_dib(dc_out, w, h);
    if dib_out.is_null() || px_out_ptr.is_null() {
        if !dib_bg.is_null() {
            platform_gdi::delete_object(dib_bg as _);
        }
        platform_gdi::delete_dc(dc_bg);
        platform_gdi::delete_dc(dc_out);
        platform_gdi::draw_icon_normal(hdc, x, y, icon as _, w, h);
        return;
    }
    platform_gdi::select_object(dc_out, dib_out as _);
    let dst = core::slice::from_raw_parts_mut(px_out_ptr as *mut u32, n);

    let blend = |fg: u32, bg: u32, a: u32| -> u32 { (fg * a + bg * (255 - a)) / 255 };
    for i in 0..n {
        let fg = derived[i];
        let alpha = (fg >> 24) & 0xFF;
        let (bg_r, bg_g, bg_b) = if i < src_bg.len() {
            let bg_px = src_bg[i];
            (
                ((bg_px >> 16) & 0xFF) as u32,
                ((bg_px >> 8) & 0xFF) as u32,
                (bg_px & 0xFF) as u32,
            )
        } else {
            (32, 32, 32)
        };
        let out_r = (fg >> 16) & 0xFF;
        let out_g = (fg >> 8) & 0xFF;
        let out_b = fg & 0xFF;
        let final_r = blend(out_r, bg_r, alpha);
        let final_g = blend(out_g, bg_g, alpha);
        let final_b = blend(out_b, bg_b, alpha);
        dst[i] = (final_r << 16) | (final_g << 8) | final_b;
    }

    platform_gdi::copy_bits(hdc, x, y, w, h, dc_out, 0, 0);

    if !dib_bg.is_null() {
        platform_gdi::delete_object(dib_bg as _);
    }
    platform_gdi::delete_object(dib_out as _);
    platform_gdi::delete_dc(dc_bg);
    platform_gdi::delete_dc(dc_out);
}
