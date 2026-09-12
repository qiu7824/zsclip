use std::collections::HashMap;
use std::sync::{Mutex, OnceLock};

use crate::platform::gdi as platform_gdi;
use crate::platform::gdiplus;
use crate::win_native_style::ui_text_font_family;
use crate::win_system_ui::{
    draw_translated_text_block, draw_translated_text_line, draw_translated_text_line_px,
};

use windows_sys::Win32::Foundation::RECT;

pub(crate) const DT_LEFT: u32 = 0x0000;
pub(crate) const DT_CENTER: u32 = 0x0001;
pub(crate) const DT_VCENTER: u32 = 0x0004;
pub(crate) const DT_WORDBREAK: u32 = 0x0010;
pub(crate) const DT_SINGLELINE: u32 = 0x0020;
pub(crate) const DT_END_ELLIPSIS: u32 = 0x0000_8000;
pub(crate) const DT_NOPREFIX: u32 = 0x0000_0800;
pub(crate) const TRANSPARENT: i32 = 1;

struct IconSurface {
    dc: *mut core::ffi::c_void,
    bitmap: *mut core::ffi::c_void,
    previous: *mut core::ffi::c_void,
    pixels: *mut u32,
}

impl IconSurface {
    unsafe fn new(hdc: *mut core::ffi::c_void, w: i32, h: i32) -> Option<Self> {
        let dc = platform_gdi::create_compatible_dc(hdc);
        if dc.is_null() {
            return None;
        }
        let (bitmap, pixels) = platform_gdi::create_top_down_32bpp_dib(dc, w, h);
        if bitmap.is_null() || pixels.is_null() {
            if !bitmap.is_null() {
                platform_gdi::delete_object(bitmap);
            }
            platform_gdi::delete_dc(dc);
            return None;
        }
        let previous = platform_gdi::select_object(dc, bitmap);
        if previous.is_null() || previous as isize == -1 {
            platform_gdi::delete_object(bitmap);
            platform_gdi::delete_dc(dc);
            return None;
        }
        Some(Self {
            dc,
            bitmap,
            previous,
            pixels: pixels as *mut u32,
        })
    }
}

impl Drop for IconSurface {
    fn drop(&mut self) {
        platform_gdi::select_object(self.dc, self.previous);
        platform_gdi::delete_object(self.bitmap);
        platform_gdi::delete_dc(self.dc);
    }
}

static DARK_ICON_CACHE: OnceLock<Mutex<HashMap<(isize, i32, i32, u8), Vec<u32>>>> = OnceLock::new();

#[cfg(test)]
mod resource_tests {
    use super::*;
    use windows_sys::Win32::{
        System::Threading::{GetCurrentProcess, GetGuiResources, GR_GDIOBJECTS},
        UI::WindowsAndMessaging::{LoadIconW, IDI_APPLICATION},
    };

    #[test]
    fn repeated_dark_icon_paint_releases_selected_bitmaps() {
        unsafe {
            let canvas = IconSurface::new(core::ptr::null_mut(), 64, 64).unwrap();
            let icon = LoadIconW(core::ptr::null_mut(), IDI_APPLICATION);
            assert!(!icon.is_null());
            draw_icon_tinted_soft(canvas.dc, 0, 0, icon as isize, 24, 24, true, 0);
            let before = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
            for _ in 0..1000 {
                draw_icon_tinted_soft(canvas.dc, 0, 0, icon as isize, 24, 24, true, 0);
            }
            let after = GetGuiResources(GetCurrentProcess(), GR_GDIOBJECTS);
            eprintln!("GDI handles before={before}, after={after}, iterations=1000");
            assert!(
                after <= before + 2,
                "GDI handles grew from {before} to {after}"
            );
        }
    }
}

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

pub unsafe fn draw_text_ex_px(
    hdc: *mut core::ffi::c_void,
    text: &str,
    rc: &RECT,
    color: u32,
    pixel_size: i32,
    bold: bool,
    center: bool,
    family: &str,
) {
    let weight = if bold { 700 } else { 400 };
    let mut rc2 = *rc;
    draw_translated_text_line_px(
        hdc,
        text,
        &mut rc2,
        color,
        pixel_size,
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
    if w <= 0 || h <= 0 || w > 512 || h > 512 {
        return;
    }
    let n = (w as usize) * (h as usize);
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
            let make_dib = |bg: u32| -> Option<IconSurface> {
                let surface = IconSurface::new(hdc, w, h)?;
                let br = platform_gdi::create_solid_brush(bg);
                platform_gdi::fill_rect(
                    surface.dc,
                    &RECT {
                        left: 0,
                        top: 0,
                        right: w,
                        bottom: h,
                    },
                    br,
                );
                platform_gdi::delete_object(br as _);
                platform_gdi::draw_icon_normal(surface.dc, 0, 0, icon as _, w, h);
                Some(surface)
            };
            let (Some(white), Some(black)) = (make_dib(0x00FFFFFF), make_dib(0)) else {
                platform_gdi::draw_icon_normal(hdc, x, y, icon as _, w, h);
                return;
            };
            let src_w = core::slice::from_raw_parts(white.pixels, n);
            let src_b = core::slice::from_raw_parts(black.pixels, n);
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
            if cache.len() >= 128
                || cache.values().map(|v| v.len() * 4).sum::<usize>() >= 4 * 1024 * 1024
            {
                cache.clear();
            }
            cache.insert((icon, w, h, soften), derived.clone());
            derived
        }
    };

    let (Some(background), Some(output)) =
        (IconSurface::new(hdc, w, h), IconSurface::new(hdc, w, h))
    else {
        platform_gdi::draw_icon_normal(hdc, x, y, icon as _, w, h);
        return;
    };
    platform_gdi::copy_bits(background.dc, 0, 0, w, h, hdc, x, y);
    let src_bg = core::slice::from_raw_parts(background.pixels, n);
    let dst = core::slice::from_raw_parts_mut(output.pixels, n);

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

    platform_gdi::copy_bits(hdc, x, y, w, h, output.dc, 0, 0);
}
