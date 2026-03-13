
use std::ffi::c_void;
use std::sync::OnceLock;

#[repr(C)]
struct GdiplusStartupInput {
    gdiplus_version: u32,
    debug_event_callback: *const c_void,
    suppress_background_thread: i32,
    suppress_external_codecs: i32,
}

const SMOOTHING_MODE_ANTI_ALIAS: i32 = 4;
const UNIT_PIXEL: i32 = 2;
const FILL_MODE_ALTERNATE: i32 = 0;

#[link(name = "gdiplus")]
unsafe extern "system" {
    fn GdiplusStartup(token: *mut usize, input: *const GdiplusStartupInput, output: *mut c_void) -> i32;
    fn GdipCreateFromHDC(hdc: *mut c_void, graphics: *mut *mut c_void) -> i32;
    fn GdipDeleteGraphics(graphics: *mut c_void) -> i32;
    fn GdipSetSmoothingMode(graphics: *mut c_void, smoothing_mode: i32) -> i32;
    fn GdipCreateSolidFill(color: u32, brush: *mut *mut c_void) -> i32;
    fn GdipDeleteBrush(brush: *mut c_void) -> i32;
    fn GdipCreatePen1(color: u32, width: f32, unit: i32, pen: *mut *mut c_void) -> i32;
    fn GdipDeletePen(pen: *mut c_void) -> i32;
    fn GdipCreatePath(fill_mode: i32, path: *mut *mut c_void) -> i32;
    fn GdipDeletePath(path: *mut c_void) -> i32;
    fn GdipAddPathArcI(path: *mut c_void, x: i32, y: i32, width: i32, height: i32, start_angle: f32, sweep_angle: f32) -> i32;
    fn GdipClosePathFigure(path: *mut c_void) -> i32;
    fn GdipFillPath(graphics: *mut c_void, brush: *mut c_void, path: *mut c_void) -> i32;
    fn GdipDrawPath(graphics: *mut c_void, pen: *mut c_void, path: *mut c_void) -> i32;
}

static GDIP_TOKEN: OnceLock<Option<usize>> = OnceLock::new();

fn ensure_startup() -> Option<usize> {
    *GDIP_TOKEN.get_or_init(|| unsafe {
        let mut token = 0usize;
        let input = GdiplusStartupInput {
            gdiplus_version: 1,
            debug_event_callback: std::ptr::null(),
            suppress_background_thread: 0,
            suppress_external_codecs: 0,
        };
        let ok = GdiplusStartup(&mut token, &input, std::ptr::null_mut()) == 0;
        if ok { Some(token) } else { None }
    })
}

#[inline]
fn colorref_to_argb(color: u32) -> u32 {
    let r = color & 0xFF;
    let g = (color >> 8) & 0xFF;
    let b = (color >> 16) & 0xFF;
    0xFF00_0000 | (r << 16) | (g << 8) | b
}

unsafe fn build_round_path(left: i32, top: i32, right: i32, bottom: i32, radius: i32) -> *mut c_void {
    let mut path = std::ptr::null_mut();
    if GdipCreatePath(FILL_MODE_ALTERNATE, &mut path) != 0 || path.is_null() {
        return std::ptr::null_mut();
    }
    let w = right - left;
    let h = bottom - top;
    let r = radius.min(w / 2).min(h / 2).max(1);
    let d = r * 2;
    let ok =
        GdipAddPathArcI(path, left, top, d, d, 180.0, 90.0) == 0 &&
        GdipAddPathArcI(path, right - d, top, d, d, 270.0, 90.0) == 0 &&
        GdipAddPathArcI(path, right - d, bottom - d, d, d, 0.0, 90.0) == 0 &&
        GdipAddPathArcI(path, left, bottom - d, d, d, 90.0, 90.0) == 0 &&
        GdipClosePathFigure(path) == 0;
    if !ok {
        let _ = GdipDeletePath(path);
        return std::ptr::null_mut();
    }
    path
}

pub unsafe fn draw_round_rect(hdc: *mut c_void, left: i32, top: i32, right: i32, bottom: i32, fill: u32, border: u32, radius: i32) -> bool {
    if ensure_startup().is_none() { return false; }
    if right <= left || bottom <= top { return true; }
    let mut graphics = std::ptr::null_mut();
    if GdipCreateFromHDC(hdc, &mut graphics) != 0 || graphics.is_null() { return false; }
    let _ = GdipSetSmoothingMode(graphics, SMOOTHING_MODE_ANTI_ALIAS);
    let path = build_round_path(left, top, right, bottom, radius.max(1));
    if path.is_null() {
        let _ = GdipDeleteGraphics(graphics);
        return false;
    }
    let mut ok = true;
    let mut brush = std::ptr::null_mut();
    if GdipCreateSolidFill(colorref_to_argb(fill), &mut brush) == 0 && !brush.is_null() {
        ok &= GdipFillPath(graphics, brush, path) == 0;
        let _ = GdipDeleteBrush(brush);
    } else {
        ok = false;
    }
    if border != 0 && border != fill {
        let mut pen = std::ptr::null_mut();
        if GdipCreatePen1(colorref_to_argb(border), 1.0, UNIT_PIXEL, &mut pen) == 0 && !pen.is_null() {
            ok &= GdipDrawPath(graphics, pen, path) == 0;
            let _ = GdipDeletePen(pen);
        }
    }
    let _ = GdipDeletePath(path);
    let _ = GdipDeleteGraphics(graphics);
    ok
}
