use windows_sys::Win32::{
    Foundation::{HANDLE, HWND, POINT, RECT},
    Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateDIBSection,
        CreateFontW, CreateRectRgn, CreateSolidBrush, DeleteDC, DeleteObject, DrawTextW, EndPaint,
        FillRect, FrameRect, GetDC, GetDeviceCaps, GetStockObject, IntersectClipRect,
        InvalidateRect, OffsetViewportOrgEx, RedrawWindow, ReleaseDC, RestoreDC, RoundRect,
        SaveDC, SelectObject,
        SetBkColor, SetBkMode, SetBrushOrgEx, SetStretchBltMode, SetTextColor, StretchDIBits,
        BITMAPINFO, BITMAPINFOHEADER, BI_RGB, COLORONCOLOR, DIB_RGB_COLORS, HALFTONE, HBITMAP,
        HBRUSH, HDC, HFONT, HGDIOBJ, HRGN, NULL_PEN, PAINTSTRUCT, SRCCOPY,
    },
    UI::WindowsAndMessaging::{DrawIconEx, DI_NORMAL, HICON},
};

pub(crate) fn begin_paint(hwnd: HWND, ps: *mut PAINTSTRUCT) -> HDC {
    unsafe { BeginPaint(hwnd, ps) }
}

pub(crate) fn end_paint(hwnd: HWND, ps: *const PAINTSTRUCT) -> bool {
    unsafe { EndPaint(hwnd, ps) != 0 }
}

pub(crate) fn get_dc(hwnd: HWND) -> HDC {
    unsafe { GetDC(hwnd) }
}

pub(crate) fn release_dc(hwnd: HWND, dc: HDC) -> i32 {
    unsafe { ReleaseDC(hwnd, dc) }
}

pub(crate) fn create_solid_brush(color: u32) -> HBRUSH {
    unsafe { CreateSolidBrush(color) }
}

pub(crate) fn create_rect_rgn(left: i32, top: i32, right: i32, bottom: i32) -> HRGN {
    unsafe { CreateRectRgn(left, top, right, bottom) }
}

pub(crate) fn delete_object(obj: HGDIOBJ) -> bool {
    unsafe { DeleteObject(obj) != 0 }
}

pub(crate) fn fill_rect(dc: HDC, rect: *const RECT, brush: HBRUSH) -> i32 {
    unsafe { FillRect(dc, rect, brush) }
}

pub(crate) fn invalidate_rect(hwnd: HWND, rect: *const RECT, erase: i32) -> bool {
    unsafe { InvalidateRect(hwnd, rect, erase) != 0 }
}

pub(crate) fn frame_rect(dc: HDC, rect: *const RECT, brush: HBRUSH) -> i32 {
    unsafe { FrameRect(dc, rect, brush) }
}

pub(crate) fn create_compatible_dc(dc: HDC) -> HDC {
    unsafe { CreateCompatibleDC(dc) }
}

pub(crate) fn delete_dc(dc: HDC) -> bool {
    unsafe { DeleteDC(dc) != 0 }
}

pub(crate) fn create_compatible_bitmap(dc: HDC, width: i32, height: i32) -> HBITMAP {
    unsafe { CreateCompatibleBitmap(dc, width, height) }
}

/// A reusable, bounded native paint surface; release when its window is hidden.
#[derive(Default)]
pub(crate) struct PaintBuffer {
    dc: HDC,
    bitmap: HBITMAP,
    previous: HGDIOBJ,
    width: i32,
    height: i32,
    #[cfg(test)]
    pub(crate) allocations: usize,
}

impl PaintBuffer {
    pub(crate) fn ensure(&mut self, target: HDC, width: i32, height: i32) -> Option<HDC> {
        if target.is_null() || width <= 0 || height <= 0 || width > 16384 || height > 16384 { return None; }
        if !self.dc.is_null() && width <= self.width && height <= self.height { return Some(self.dc); }
        let capacity_w = ((width + 127) / 128) * 128;
        let capacity_h = ((height + 127) / 128) * 128;
        let dc = create_compatible_dc(target);
        if dc.is_null() { return None; }
        let bitmap = create_compatible_bitmap(target, capacity_w, capacity_h);
        if bitmap.is_null() { delete_dc(dc); return None; }
        let previous = select_object(dc, bitmap as _);
        if previous.is_null() || previous as isize == -1 {
            delete_object(bitmap as _); delete_dc(dc); return None;
        }
        self.clear();
        self.dc = dc; self.bitmap = bitmap; self.previous = previous;
        self.width = capacity_w; self.height = capacity_h;
        #[cfg(test)] { self.allocations += 1; }
        Some(dc)
    }
    pub(crate) fn clear(&mut self) {
        if !self.dc.is_null() {
            select_object(self.dc, self.previous);
            delete_object(self.bitmap as _);
            delete_dc(self.dc);
        }
        self.dc = core::ptr::null_mut(); self.bitmap = core::ptr::null_mut(); self.previous = core::ptr::null_mut();
        self.width = 0; self.height = 0;
    }
    #[cfg(test)]
    pub(crate) fn is_empty(&self) -> bool { self.dc.is_null() }
    #[cfg(test)]
    pub(crate) fn snapshot_rgba(&self,width:i32,height:i32)->Option<Vec<u8>> {
        if self.dc.is_null()||width<=0||height<=0||width>self.width||height>self.height{return None;}
        snapshot_dc_rgba(self.dc,width,height)
    }
}

#[cfg(test)]
pub(crate) fn snapshot_dc_rgba(source:HDC,width:i32,height:i32)->Option<Vec<u8>> {
        if source.is_null()||width<=0||height<=0{return None;}
        let dc=create_compatible_dc(source);if dc.is_null(){return None;}
        let (bitmap,bits)=create_top_down_32bpp_dib(dc,width,height);
        if bitmap.is_null()||bits.is_null(){if !bitmap.is_null(){delete_object(bitmap as _);}delete_dc(dc);return None;}
        let previous=select_object(dc,bitmap as _);
        let copied=copy_bits(dc,0,0,width,height,source,0,0);
        unsafe {windows_sys::Win32::Graphics::Gdi::GdiFlush();}
        let result=if copied {
            let mut rgba=unsafe{std::slice::from_raw_parts(bits as *const u8,width as usize*height as usize*4)}.to_vec();
            for pixel in rgba.chunks_exact_mut(4){pixel.swap(0,2);pixel[3]=255;}
            Some(rgba)
        }else{None};
        select_object(dc,previous);delete_object(bitmap as _);delete_dc(dc);result
}

impl Drop for PaintBuffer { fn drop(&mut self) { self.clear(); } }

pub(crate) fn create_dib_section(
    dc: HDC,
    info: *const BITMAPINFO,
    usage: u32,
    bits: *mut *mut core::ffi::c_void,
    section: HANDLE,
    offset: u32,
) -> HBITMAP {
    unsafe { CreateDIBSection(dc, info, usage, bits, section, offset) }
}

pub(crate) fn create_top_down_32bpp_dib(
    dc: HDC,
    width: i32,
    height: i32,
) -> (HBITMAP, *mut core::ffi::c_void) {
    let mut info: BITMAPINFO = unsafe { core::mem::zeroed() };
    info.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = width;
    info.bmiHeader.biHeight = -height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    let mut bits: *mut core::ffi::c_void = core::ptr::null_mut();
    let dib = create_dib_section(
        dc,
        &info,
        DIB_RGB_COLORS,
        &mut bits,
        core::ptr::null_mut(),
        0,
    );
    (dib, bits)
}

pub(crate) fn select_object(dc: HDC, obj: HGDIOBJ) -> HGDIOBJ {
    unsafe { SelectObject(dc, obj) }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn bit_blt(
    dst: HDC,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    src: HDC,
    src_x: i32,
    src_y: i32,
    rop: u32,
) -> bool {
    unsafe { BitBlt(dst, x, y, width, height, src, src_x, src_y, rop) != 0 }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn copy_bits(
    dst: HDC,
    x: i32,
    y: i32,
    width: i32,
    height: i32,
    src: HDC,
    src_x: i32,
    src_y: i32,
) -> bool {
    bit_blt(dst, x, y, width, height, src, src_x, src_y, SRCCOPY)
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn stretch_dibits(
    dc: HDC,
    x_dest: i32,
    y_dest: i32,
    dest_width: i32,
    dest_height: i32,
    x_src: i32,
    y_src: i32,
    src_width: i32,
    src_height: i32,
    bits: *const core::ffi::c_void,
    bitmap_info: *const BITMAPINFO,
    usage: u32,
    rop: u32,
) -> i32 {
    unsafe {
        StretchDIBits(
            dc,
            x_dest,
            y_dest,
            dest_width,
            dest_height,
            x_src,
            y_src,
            src_width,
            src_height,
            bits,
            bitmap_info,
            usage,
            rop,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn stretch_top_down_32bpp(
    dc: HDC,
    x_dest: i32,
    y_dest: i32,
    dest_width: i32,
    dest_height: i32,
    src_width: i32,
    src_height: i32,
    bgra_bits: &[u8],
) -> i32 {
    if dc.is_null()
        || dest_width <= 0
        || dest_height <= 0
        || src_width <= 0
        || src_height <= 0
        || bgra_bits.is_empty()
    {
        return 0;
    }
    let mut info: BITMAPINFO = unsafe { core::mem::zeroed() };
    info.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = src_width;
    info.bmiHeader.biHeight = -src_height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    set_stretch_blt_mode(dc, HALFTONE);
    set_brush_org_ex(dc, 0, 0, core::ptr::null_mut());
    stretch_dibits(
        dc,
        x_dest,
        y_dest,
        dest_width,
        dest_height,
        0,
        0,
        src_width,
        src_height,
        bgra_bits.as_ptr() as _,
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    )
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn stretch_top_down_32bpp_nearest(
    dc: HDC,
    x_dest: i32,
    y_dest: i32,
    dest_width: i32,
    dest_height: i32,
    src_width: i32,
    src_height: i32,
    bgra_bits: &[u8],
) -> i32 {
    if dc.is_null()
        || dest_width <= 0
        || dest_height <= 0
        || src_width <= 0
        || src_height <= 0
        || bgra_bits.is_empty()
    {
        return 0;
    }
    let mut info: BITMAPINFO = unsafe { core::mem::zeroed() };
    info.bmiHeader.biSize = core::mem::size_of::<BITMAPINFOHEADER>() as u32;
    info.bmiHeader.biWidth = src_width;
    info.bmiHeader.biHeight = -src_height;
    info.bmiHeader.biPlanes = 1;
    info.bmiHeader.biBitCount = 32;
    info.bmiHeader.biCompression = BI_RGB;
    set_stretch_blt_mode(dc, COLORONCOLOR);
    stretch_dibits(
        dc,
        x_dest,
        y_dest,
        dest_width,
        dest_height,
        0,
        0,
        src_width,
        src_height,
        bgra_bits.as_ptr() as _,
        &info,
        DIB_RGB_COLORS,
        SRCCOPY,
    )
}

pub(crate) fn set_stretch_blt_mode(dc: HDC, mode: i32) -> i32 {
    unsafe { SetStretchBltMode(dc, mode) }
}

pub(crate) fn set_brush_org_ex(dc: HDC, x: i32, y: i32, previous: *mut POINT) -> bool {
    unsafe { SetBrushOrgEx(dc, x, y, previous) != 0 }
}

pub(crate) fn get_stock_object(index: i32) -> HGDIOBJ {
    unsafe { GetStockObject(index) }
}

pub(crate) fn null_pen() -> HGDIOBJ {
    get_stock_object(NULL_PEN)
}

pub(crate) fn set_bk_mode(dc: HDC, mode: i32) -> i32 {
    unsafe { SetBkMode(dc, mode) }
}

pub(crate) fn set_bk_color(dc: HDC, color: u32) -> u32 {
    unsafe { SetBkColor(dc, color) }
}

pub(crate) fn set_text_color(dc: HDC, color: u32) -> u32 {
    unsafe { SetTextColor(dc, color) }
}

pub(crate) fn draw_text(
    dc: HDC,
    text: *const u16,
    count: i32,
    rect: *mut RECT,
    format: u32,
) -> i32 {
    unsafe { DrawTextW(dc, text, count, rect, format) }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn create_font_w(
    height: i32,
    width: i32,
    escapement: i32,
    orientation: i32,
    weight: i32,
    italic: u32,
    underline: u32,
    strike_out: u32,
    char_set: u32,
    output_precision: u32,
    clip_precision: u32,
    quality: u32,
    pitch_and_family: u32,
    face_name: *const u16,
) -> HFONT {
    unsafe {
        CreateFontW(
            height,
            width,
            escapement,
            orientation,
            weight,
            italic,
            underline,
            strike_out,
            char_set,
            output_precision,
            clip_precision,
            quality,
            pitch_and_family,
            face_name,
        )
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn round_rect(
    dc: HDC,
    left: i32,
    top: i32,
    right: i32,
    bottom: i32,
    ellipse_width: i32,
    ellipse_height: i32,
) -> bool {
    unsafe { RoundRect(dc, left, top, right, bottom, ellipse_width, ellipse_height) != 0 }
}

pub(crate) fn save_dc(dc: HDC) -> i32 {
    unsafe { SaveDC(dc) }
}

pub(crate) fn restore_dc(dc: HDC, saved_dc: i32) -> bool {
    unsafe { RestoreDC(dc, saved_dc) != 0 }
}

pub(crate) fn intersect_clip_rect(dc: HDC, left: i32, top: i32, right: i32, bottom: i32) -> i32 {
    unsafe { IntersectClipRect(dc, left, top, right, bottom) }
}

pub(crate) fn offset_viewport_org(dc: HDC, dx: i32, dy: i32) -> bool {
    unsafe { OffsetViewportOrgEx(dc, dx, dy, core::ptr::null_mut()) != 0 }
}

pub(crate) fn redraw_window(
    hwnd: HWND,
    rect: *const RECT,
    region: *mut core::ffi::c_void,
    flags: u32,
) -> bool {
    unsafe { RedrawWindow(hwnd, rect, region as _, flags) != 0 }
}

pub(crate) fn get_device_caps(dc: HDC, index: i32) -> i32 {
    unsafe { GetDeviceCaps(dc, index) }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draw_icon_ex(
    dc: HDC,
    x_left: i32,
    y_top: i32,
    icon: HICON,
    width: i32,
    height: i32,
    step_if_ani_cur: u32,
    flicker_free_brush: HBRUSH,
    flags: u32,
) -> bool {
    unsafe {
        DrawIconEx(
            dc,
            x_left,
            y_top,
            icon,
            width,
            height,
            step_if_ani_cur,
            flicker_free_brush,
            flags,
        ) != 0
    }
}

pub(crate) fn draw_icon_normal(
    dc: HDC,
    x_left: i32,
    y_top: i32,
    icon: HICON,
    width: i32,
    height: i32,
) -> bool {
    draw_icon_ex(
        dc,
        x_left,
        y_top,
        icon,
        width,
        height,
        0,
        core::ptr::null_mut(),
        DI_NORMAL,
    )
}
