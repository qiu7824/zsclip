use std::ffi::OsStr;
use std::iter::once;
use std::os::windows::ffi::OsStrExt;

use crate::gdiplus;

use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{
        BitBlt, CreateCompatibleDC, DeleteDC,
        CreateFontW, CreateSolidBrush, DeleteObject, DrawTextW, Ellipse, FillRect, GetStockObject, RoundRect, SelectObject, SetBkMode, SetTextColor, DEFAULT_GUI_FONT, NULL_PEN,
        SRCCOPY,
        CreateDIBSection, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS, BI_RGB,
    },
};

// Registry API — Win32_System_Registry feature not enabled, declare manually
const HKEY_CURRENT_USER: isize = -2147483647i32 as isize;
const KEY_READ: u32 = 0x20019;
const REG_DWORD: u32 = 4;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(hkey: isize, lpsubkey: *const u16, uloptions: u32, samdesired: u32, phkresult: *mut isize) -> i32;
    fn RegQueryValueExW(hkey: isize, lpvaluename: *const u16, lpreserved: *mut u32, lptype: *mut u32, lpdata: *mut u8, lpcbdata: *mut u32) -> i32;
    fn RegCloseKey(hkey: isize) -> i32;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmGetColorizationColor(pcr_colorization: *mut u32, pf_opaque_blend: *mut i32) -> i32;
}

pub const DT_LEFT: u32 = 0x0000;
pub const DT_CENTER: u32 = 0x0001;
pub const DT_VCENTER: u32 = 0x0004;
pub const DT_SINGLELINE: u32 = 0x0020;
pub const DT_END_ELLIPSIS: u32 = 0x00008000;
pub const TRANSPARENT: i32 = 1;

pub const SETTINGS_PAGES: [&str; 6] = ["常规", "快捷键", "插件", "分组", "云同步", "关于"];
pub const SETTINGS_NAV_GLYPHS: [&str; 6] = ["", "", "", "", "", ""];
pub const SETTINGS_W: i32 = 1100;
pub const SETTINGS_H: i32 = 740;
pub const SETTINGS_NAV_W: i32 = 236;
pub const SETTINGS_TOP_H: i32 = 84;
pub const SETTINGS_NAV_Y: i32 = 72;
pub const SETTINGS_CONTENT_X: i32 = SETTINGS_NAV_W + 28;
pub const SETTINGS_CONTENT_W: i32 = SETTINGS_W - SETTINGS_CONTENT_X - 28;
pub const SETTINGS_CONTENT_Y: i32 = SETTINGS_TOP_H;

#[derive(Clone, Copy)]
#[allow(dead_code)]
pub struct Theme {
    pub accent: u32,
    pub accent_hover: u32,
    pub accent_pressed: u32,
    pub bg: u32,
    pub nav_bg: u32,
    pub nav_sel_fill: u32,
    pub surface: u32,
    pub surface2: u32,
    pub stroke: u32,
    pub text: u32,
    pub text_muted: u32,
    pub text_dim: u32,
    pub item_hover: u32,
    pub item_selected: u32,
    pub control_bg: u32,
    pub control_stroke: u32,
    pub button_bg: u32,
    pub button_hover: u32,
    pub button_pressed: u32,
    pub close_hover: u32,
}

impl Default for Theme {
    fn default() -> Self {
        let accent = system_accent();
        let dark = is_dark_mode();
        let accent_r = (accent & 0xFF) as i32;
        let accent_g = ((accent >> 8) & 0xFF) as i32;
        let accent_b = ((accent >> 16) & 0xFF) as i32;
        let accent_hover = rgb(
            ((accent_r as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
            ((accent_g as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
            ((accent_b as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
        );
        let accent_pressed = rgb(
            ((accent_r as f32 * 0.82) as i32).min(255) as u8,
            ((accent_g as f32 * 0.82) as i32).min(255) as u8,
            ((accent_b as f32 * 0.82) as i32).min(255) as u8,
        );

        if dark {
            // WinUI3 Dark theme tokens
            Self {
                accent,
                accent_hover,
                accent_pressed,
                bg:             rgb(32, 32, 32),   // SolidBackgroundFillColorBase dark
                nav_bg:         rgb(40, 40, 40),
                nav_sel_fill:   rgb(58, 58, 58),
                surface:        rgb(44, 44, 44),   // LayerFillColorDefault dark
                surface2:       rgb(50, 50, 50),
                stroke:         rgb(60, 60, 60),   // CardStrokeColorDefault dark
                text:           rgb(255, 255, 255),// TextFillColorPrimary dark
                text_muted:     rgb(162, 162, 162),
                text_dim:       rgb(100, 100, 100),
                item_hover:     rgb(54, 54, 54),
                item_selected:  mix(accent, rgb(44, 44, 44), 0.75),
                control_bg:     rgb(58, 58, 58),   // ControlFillColorDefault dark
                control_stroke: rgb(80, 80, 80),
                button_bg:      rgb(58, 58, 58),
                button_hover:   rgb(68, 68, 68),
                button_pressed: rgb(50, 50, 50),
                close_hover:    rgb(196, 43, 28),
            }
        } else {
            // WinUI3 Light theme tokens
            Self {
                accent,
                accent_hover,
                accent_pressed,
                bg:             rgb(243, 243, 243), // SolidBackgroundFillColorBase
                nav_bg:         rgb(243, 243, 243),
                nav_sel_fill:   rgb(255, 255, 255),
                surface:        rgb(255, 255, 255),
                surface2:       rgb(250, 250, 250),
                stroke:         rgb(229, 229, 229),
                text:           rgb(28, 28, 28),
                text_muted:     rgb(96, 96, 96),
                text_dim:       rgb(160, 160, 160),
                item_hover:     rgb(249, 249, 249),
                item_selected:  mix(accent, rgb(255, 255, 255), 0.85),
                control_bg:     rgb(255, 255, 255),
                control_stroke: rgb(204, 204, 204),
                button_bg:      rgb(255, 255, 255),
                button_hover:   rgb(249, 249, 249),
                button_pressed: rgb(238, 238, 238),
                close_hover:    rgb(196, 43, 28),
            }
        }
    }
}

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ar = (a & 0xFF) as f32;
    let ag = ((a >> 8) & 0xFF) as f32;
    let ab = ((a >> 16) & 0xFF) as f32;
    let br = (b & 0xFF) as f32;
    let bg = ((b >> 8) & 0xFF) as f32;
    let bb = ((b >> 16) & 0xFF) as f32;
    rgb(
        (ar + (br - ar) * t).round() as u8,
        (ag + (bg - ag) * t).round() as u8,
        (ab + (bb - ab) * t).round() as u8,
    )
}

pub fn is_dark_mode() -> bool {
    unsafe {
        let key_path = OsStr::new("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize")
            .encode_wide().chain(once(0)).collect::<Vec<u16>>();
        let val_name = OsStr::new("AppsUseLightTheme")
            .encode_wide().chain(once(0)).collect::<Vec<u16>>();
        let mut hkey = 0isize;
        if RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return false;
        }
        let mut data = 0u32;
        let mut data_size = 4u32;
        let mut reg_type = 0u32;
        let ret = RegQueryValueExW(
            hkey, val_name.as_ptr(), std::ptr::null_mut(), &mut reg_type,
            &mut data as *mut u32 as *mut u8, &mut data_size,
        );
        RegCloseKey(hkey);
        if ret == 0 && reg_type == REG_DWORD {
            data == 0  // 0 = dark, 1 = light
        } else {
            false
        }
    }
}

fn system_accent() -> u32 {
    unsafe {
        let mut c = 0u32;
        let mut opaque = 0i32;
        if DwmGetColorizationColor(&mut c, &mut opaque) == 0 {
            let r = ((c >> 16) & 0xFF) as u8;
            let g = ((c >> 8) & 0xFF) as u8;
            let b = (c & 0xFF) as u8;
            return rgb(r, g, b);
        }
    }
    rgb(0, 120, 212)
}

fn to_wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(once(0)).collect()
}

pub fn settings_nav_item_rect(index: usize) -> RECT {
    let x = 10;
    let y = SETTINGS_NAV_Y + 8 + (index as i32) * 44;
    RECT { left: x, top: y, right: SETTINGS_NAV_W - 10, bottom: y + 36 }
}



#[allow(dead_code)]
pub unsafe fn draw_pill_fill(hdc: *mut core::ffi::c_void, rc: &RECT, fill: u32) {
    let w = rc.right - rc.left;
    let h = rc.bottom - rc.top;
    if w <= 1 || h <= 1 { return; }
    let radius = (h / 2).max(1);
    if gdiplus::draw_round_rect(hdc, rc.left, rc.top, rc.right, rc.bottom, fill, fill, radius) {
        return;
    }
    let d = h.max(2);
    let r = d / 2;
    let br = CreateSolidBrush(fill);
    let old_br = SelectObject(hdc, br as _);
    let pen = GetStockObject(NULL_PEN);
    let old_pen = SelectObject(hdc, pen as _);
    let mid = RECT { left: rc.left + r, top: rc.top, right: rc.right - r, bottom: rc.bottom };
    if mid.right > mid.left { FillRect(hdc, &mid, br); }
    Ellipse(hdc, rc.left, rc.top, rc.left + d, rc.bottom);
    Ellipse(hdc, rc.right - d, rc.top, rc.right, rc.bottom);
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_br);
    DeleteObject(br as _);
}

#[allow(dead_code)]
pub unsafe fn draw_pill(hdc: *mut core::ffi::c_void, rc: &RECT, fill: u32, border: u32) {
    if border != 0 && border != fill {
        draw_pill_fill(hdc, rc, border);
        let inner = RECT { left: rc.left + 1, top: rc.top + 1, right: rc.right - 1, bottom: rc.bottom - 1 };
        if inner.right > inner.left && inner.bottom > inner.top {
            draw_pill_fill(hdc, &inner, fill);
        }
    } else {
        draw_pill_fill(hdc, rc, fill);
    }
}

#[allow(dead_code)]
pub fn settings_card_rect(y: i32, h: i32) -> RECT {
    RECT {
        left: SETTINGS_CONTENT_X,
        top: SETTINGS_CONTENT_Y + y,
        right: SETTINGS_CONTENT_X + SETTINGS_CONTENT_W,
        bottom: SETTINGS_CONTENT_Y + y + h,
    }
}


pub unsafe fn draw_round_rect(hdc: *mut core::ffi::c_void, rc: &RECT, fill: u32, border: u32, radius: i32) {
    if gdiplus::draw_round_rect(hdc, rc.left, rc.top, rc.right, rc.bottom, fill, border, radius.max(1)) {
        return;
    }
    let er = (radius.max(1)) * 2;
    if border != 0 && border != fill {
        let outer_pen = GetStockObject(NULL_PEN);
        let outer_br = CreateSolidBrush(border);
        let old_pen = SelectObject(hdc, outer_pen as _);
        let old_br = SelectObject(hdc, outer_br as _);
        RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_br);
        DeleteObject(outer_br as _);

        let inner = RECT {
            left: rc.left + 1,
            top: rc.top + 1,
            right: rc.right - 1,
            bottom: rc.bottom - 1,
        };
        if inner.right > inner.left && inner.bottom > inner.top {
            let inner_br = CreateSolidBrush(fill);
            let old_pen2 = SelectObject(hdc, outer_pen as _);
            let old_br2 = SelectObject(hdc, inner_br as _);
            let inner_r = (radius - 1).max(1) * 2;
            RoundRect(hdc, inner.left, inner.top, inner.right, inner.bottom, inner_r, inner_r);
            SelectObject(hdc, old_pen2);
            SelectObject(hdc, old_br2);
            DeleteObject(inner_br as _);
        }
    } else {
        let pen = GetStockObject(NULL_PEN);
        let brush = CreateSolidBrush(fill);
        let old_pen = SelectObject(hdc, pen as _);
        let old_br = SelectObject(hdc, brush as _);
        RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
        SelectObject(hdc, old_pen);
        SelectObject(hdc, old_br);
        DeleteObject(brush as _);
    }
}

pub unsafe fn draw_round_fill(hdc: *mut core::ffi::c_void, rc: &RECT, fill: u32, radius: i32) {
    if gdiplus::draw_round_rect(hdc, rc.left, rc.top, rc.right, rc.bottom, fill, fill, radius.max(1)) {
        return;
    }
    let er = (radius.max(1)) * 2;
    let pen = GetStockObject(NULL_PEN);
    let brush = CreateSolidBrush(fill);
    let old_pen = SelectObject(hdc, pen as _);
    let old_br = SelectObject(hdc, brush as _);
    RoundRect(hdc, rc.left, rc.top, rc.right, rc.bottom, er, er);
    SelectObject(hdc, old_pen);
    SelectObject(hdc, old_br);
    DeleteObject(brush as _);
}

pub unsafe fn draw_main_segment_bar(
    hdc: *mut core::ffi::c_void,
    outer: &RECT,
    tab0: &RECT,
    tab1: &RECT,
    selected: i32,
    hover: i32,
    th: Theme,
) {
    draw_round_rect(hdc, outer, th.surface, th.stroke, 4);

    let mut sel_rc = *tab0;
    if selected == 1 { sel_rc = *tab1; }
    let inner_sel = RECT {
        left: sel_rc.left + 2,
        top: sel_rc.top + 2,
        right: sel_rc.right - 2,
        bottom: sel_rc.bottom - 2,
    };
    let selected_fill = if th.bg == rgb(255, 255, 255) { th.surface2 } else { th.nav_sel_fill };
    draw_round_rect(hdc, &inner_sel, selected_fill, th.stroke, 3);

    if hover == 0 && selected != 0 {
        let hr = RECT { left: tab0.left + 2, top: tab0.top + 2, right: tab0.right - 2, bottom: tab0.bottom - 2 };
        draw_round_fill(hdc, &hr, th.item_hover, 3);
    }
    if hover == 1 && selected != 1 {
        let hr = RECT { left: tab1.left + 2, top: tab1.top + 2, right: tab1.right - 2, bottom: tab1.bottom - 2 };
        draw_round_fill(hdc, &hr, th.item_hover, 3);
    }

    let t0c = if selected == 0 { th.text } else if hover == 0 { th.text } else { th.text_muted };
    let t1c = if selected == 1 { th.text } else if hover == 1 { th.text } else { th.text_muted };
    draw_text_ex(hdc, "复制记录", tab0, t0c, 11, false, true, "Segoe UI Variable Text");
    draw_text_ex(hdc, "常用短语", tab1, t1c, 11, false, true, "Segoe UI Variable Text");
}

pub unsafe fn draw_text(hdc: *mut core::ffi::c_void, text: &str, rc: &RECT, color: u32, size: i32, bold: bool, center: bool) {
    draw_text_ex(hdc, text, rc, color, size, bold, center, "Segoe UI Variable Text");
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
    SetBkMode(hdc, TRANSPARENT);
    SetTextColor(hdc, color);
    let weight = if bold { 700 } else { 400 };
    let font = CreateFontW(-size, 0, 0, 0, weight, 0, 0, 0, 1, 0, 0, 5, 0, to_wide(if family.is_empty() { "Segoe UI Variable Text" } else { family }).as_ptr());
    let font = if font.is_null() { GetStockObject(DEFAULT_GUI_FONT) } else { font };
    let old = SelectObject(hdc, font as _);
    let mut rc2 = *rc;
    let flags = (if center { DT_CENTER } else { DT_LEFT }) | DT_VCENTER | DT_SINGLELINE | DT_END_ELLIPSIS;
    DrawTextW(hdc, to_wide(text).as_ptr(), -1, &mut rc2, flags);
    SelectObject(hdc, old);
    if !font.is_null() && font != GetStockObject(DEFAULT_GUI_FONT) {
        DeleteObject(font as _);
    }
}

/// 在深色模式下绘制图标时，将黑色图标反色为白色。
/// 深色模式下把图标反色为白色版本（两次绘制提取真实像素）
pub unsafe fn draw_icon_tinted(
    hdc: *mut core::ffi::c_void,
    x: i32, y: i32,
    icon: isize,
    w: i32, h: i32,
    dark: bool,
) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL};
    if icon == 0 { return; }
    if !dark {
        DrawIconEx(hdc, x, y, icon as _, w, h, 0, std::ptr::null_mut(), DI_NORMAL);
        return;
    }

    // 两次绘制法提取带 alpha 的图标，再以灰/白色重绘到目标 DC
    // pass1: 白底 → 图标像素 = alpha*icon_color + (1-alpha)*255
    // pass2: 黑底 → 图标像素 = alpha*icon_color
    // alpha = 1 - (pass1 - pass2) / 255
    // icon_color = pass2 / alpha
    // 最终以亮灰色绘制到目标（保留 alpha）

    let make_dib = |bg: u32| -> (*mut core::ffi::c_void, *mut core::ffi::c_void, *mut u32) {
        let dc = CreateCompatibleDC(hdc);
        let mut bmi: BITMAPINFO = core::mem::zeroed();
        bmi.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
        bmi.bmiHeader.biWidth = w;
        bmi.bmiHeader.biHeight = -h;
        bmi.bmiHeader.biPlanes = 1;
        bmi.bmiHeader.biBitCount = 32;
        bmi.bmiHeader.biCompression = BI_RGB as u32;
        let mut ptr: *mut core::ffi::c_void = core::ptr::null_mut();
        let dib = CreateDIBSection(dc, &bmi, DIB_RGB_COLORS, &mut ptr, core::ptr::null_mut(), 0);
        if dib.is_null() || ptr.is_null() {
            DeleteDC(dc);
            return (core::ptr::null_mut(), core::ptr::null_mut(), core::ptr::null_mut());
        }
        SelectObject(dc, dib as _);
        // 填充背景色
        let br = CreateSolidBrush(bg);
        let rc = RECT { left: 0, top: 0, right: w, bottom: h };
        FillRect(dc, &rc, br);
        DeleteObject(br as _);
        DrawIconEx(dc, 0, 0, icon as _, w, h, 0, core::ptr::null_mut(), DI_NORMAL);
        (dc, dib as *mut core::ffi::c_void, ptr as *mut u32)
    };

    let (dc_w, dib_w, px_w) = make_dib(0x00FFFFFFu32); // 白底
    let (dc_b, dib_b, px_b) = make_dib(0x00000000u32); // 黑底

    if dc_w.is_null() || dc_b.is_null() {
        if !dc_w.is_null() { DeleteDC(dc_w); }
        if !dc_b.is_null() { DeleteDC(dc_b); }
        DrawIconEx(hdc, x, y, icon as _, w, h, 0, std::ptr::null_mut(), DI_NORMAL);
        return;
    }

    // 创建输出 DIB
    let dc_out = CreateCompatibleDC(hdc);
    let mut bmi_out: BITMAPINFO = core::mem::zeroed();
    bmi_out.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi_out.bmiHeader.biWidth = w;
    bmi_out.bmiHeader.biHeight = -h;
    bmi_out.bmiHeader.biPlanes = 1;
    bmi_out.bmiHeader.biBitCount = 32;
    bmi_out.bmiHeader.biCompression = BI_RGB as u32;
    let mut px_out_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
    let dib_out = CreateDIBSection(dc_out, &bmi_out, DIB_RGB_COLORS, &mut px_out_ptr, core::ptr::null_mut(), 0);
    if dib_out.is_null() || px_out_ptr.is_null() {
        DeleteDC(dc_w); DeleteDC(dc_b); DeleteDC(dc_out);
        DrawIconEx(hdc, x, y, icon as _, w, h, 0, std::ptr::null_mut(), DI_NORMAL);
        return;
    }
    SelectObject(dc_out, dib_out as _);

    let n = (w * h) as usize;
    let src_w = core::slice::from_raw_parts(px_w, n);
    let src_b = core::slice::from_raw_parts(px_b, n);
    let dst   = core::slice::from_raw_parts_mut(px_out_ptr as *mut u32, n);

    // 从目标 DC 读取背景色（用于 premix）
    // 获取目标 DC 当前对应区域的背景像素（读取当前 hdc 内容）
    let dc_bg = CreateCompatibleDC(hdc);
    let mut bmi_bg: BITMAPINFO = core::mem::zeroed();
    bmi_bg.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
    bmi_bg.bmiHeader.biWidth = w;
    bmi_bg.bmiHeader.biHeight = -h;
    bmi_bg.bmiHeader.biPlanes = 1;
    bmi_bg.bmiHeader.biBitCount = 32;
    bmi_bg.bmiHeader.biCompression = BI_RGB as u32;
    let mut px_bg_ptr: *mut core::ffi::c_void = core::ptr::null_mut();
    let dib_bg = CreateDIBSection(dc_bg, &bmi_bg, DIB_RGB_COLORS, &mut px_bg_ptr, core::ptr::null_mut(), 0);
    SelectObject(dc_bg, dib_bg as _);
    BitBlt(dc_bg, 0, 0, w, h, hdc, x, y, SRCCOPY);
    let src_bg = if !dib_bg.is_null() && !px_bg_ptr.is_null() {
        core::slice::from_raw_parts(px_bg_ptr as *const u32, n)
    } else {
        &[] as &[u32]
    };

    for i in 0..n {
        let w_px = src_w[i];
        let b_px = src_b[i];

        let wr = ((w_px >> 16) & 0xFF) as i32;
        let wg = ((w_px >> 8)  & 0xFF) as i32;
        let wb = ( w_px        & 0xFF) as i32;

        let br = ((b_px >> 16) & 0xFF) as i32;
        let bg = ((b_px >> 8)  & 0xFF) as i32;
        let bb = ( b_px        & 0xFF) as i32;

        // alpha（每通道）: diff = white - black = (1-a)*255, so a = 1 - diff/255
        let ar = 255 - (wr - br).clamp(0, 255);
        let ag = 255 - (wg - bg).clamp(0, 255);
        let ab = 255 - (wb - bb).clamp(0, 255);
        let alpha = ((ar + ag + ab) / 3) as u32;

        if alpha < 8 {
            // 完全透明：直接用背景色
            dst[i] = if i < src_bg.len() { src_bg[i] & 0x00FFFFFF } else { 0x00202020 };
            continue;
        }

        // 原图标颜色（从黑底版本恢复）
        let icon_r = ((br * 255 / ar.max(1)) as u32).min(255);
        let icon_g = ((bg * 255 / ag.max(1)) as u32).min(255);
        let icon_b = ((bb * 255 / ab.max(1)) as u32).min(255);

        // 图标亮度
        let lum = (icon_r * 299 + icon_g * 587 + icon_b * 114) / 1000;

        // 深色背景上：把暗色图标像素映射为亮灰/白色，彩色像素适当提亮
        let (out_r, out_g, out_b) = if lum < 80 {
            // 纯黑/深灰图标 → 纯白
            (255u32, 255u32, 255u32)
        } else if lum < 200 {
            // 中灰 → 亮灰
            let bright = (255 - lum + 180).min(255);
            (bright, bright, bright)
        } else {
            // 已是亮色/彩色 → 保留
            (icon_r, icon_g, icon_b)
        };

        // 与背景 alpha 合成
        let (bg_r, bg_g, bg_b) = if i < src_bg.len() {
            let bg_px = src_bg[i];
            (((bg_px >> 16) & 0xFF) as u32,
             ((bg_px >> 8)  & 0xFF) as u32,
             ( bg_px        & 0xFF) as u32)
        } else {
            (32, 32, 32)
        };

        let blend = |fg: u32, bg: u32, a: u32| -> u32 { (fg * a + bg * (255 - a)) / 255 };
        let final_r = blend(out_r, bg_r, alpha);
        let final_g = blend(out_g, bg_g, alpha);
        let final_b = blend(out_b, bg_b, alpha);

        dst[i] = (final_r << 16) | (final_g << 8) | final_b;
    }

    BitBlt(hdc, x, y, w, h, dc_out, 0, 0, SRCCOPY);

    DeleteObject(dib_w as _); DeleteDC(dc_w);
    DeleteObject(dib_b as _); DeleteDC(dc_b);
    DeleteObject(dib_out as _); DeleteDC(dc_out);
    if !dib_bg.is_null() { DeleteObject(dib_bg as _); }
    DeleteDC(dc_bg);
}
