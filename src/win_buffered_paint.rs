use std::ffi::c_void;
use std::ptr::null;
use std::sync::OnceLock;

use windows_sys::Win32::Foundation::RECT;

#[allow(non_camel_case_types)]
type HPAINTBUFFER = *mut c_void;

#[repr(C)]
#[allow(non_snake_case)]
struct BP_PAINTPARAMS {
    cbSize: u32,
    dwFlags: u32,
    prcExclude: *const RECT,
    pBlendFunction: *const c_void,
}

const BPBF_TOPDOWNDIB: u32 = 2;
static BUFFERED_PAINT_INIT: OnceLock<()> = OnceLock::new();

#[link(name = "uxtheme")]
unsafe extern "system" {
    fn BufferedPaintInit() -> i32;
    fn BeginBufferedPaint(
        hdcTarget: *mut c_void,
        prcTarget: *const RECT,
        dwFormat: u32,
        pPaintParams: *const BP_PAINTPARAMS,
        phdc: *mut *mut c_void,
    ) -> HPAINTBUFFER;
    fn EndBufferedPaint(hBufferedPaint: HPAINTBUFFER, fUpdateTarget: i32) -> i32;
}

unsafe fn ensure_buffered_paint() {
    BUFFERED_PAINT_INIT.get_or_init(|| {
        let _ = BufferedPaintInit();
    });
}

pub unsafe fn begin_buffered_paint(
    hdc_target: *mut c_void,
    rc: &RECT,
) -> Option<(HPAINTBUFFER, *mut c_void)> {
    ensure_buffered_paint();
    let mut paint_dc: *mut c_void = std::ptr::null_mut();
    let hbuf = BeginBufferedPaint(
        hdc_target,
        rc as *const RECT,
        BPBF_TOPDOWNDIB,
        null(),
        &mut paint_dc,
    );
    if hbuf.is_null() || paint_dc.is_null() {
        None
    } else {
        Some((hbuf, paint_dc))
    }
}

pub unsafe fn end_buffered_paint(hbuf: HPAINTBUFFER, update_target: bool) {
    if !hbuf.is_null() {
        let _ = EndBufferedPaint(hbuf, if update_target { 1 } else { 0 });
    }
}
