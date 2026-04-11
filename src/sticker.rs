use std::cmp::{max, min};
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::sync::OnceLock;

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush,
        DeleteDC, DeleteObject, EndPaint, FillRect, FrameRect, PatBlt, PAINTSTRUCT, SelectObject,
        StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY, WHITE_BRUSH,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{
        Input::KeyboardAndMouse::{
            GetKeyState, ReleaseCapture, SetCapture, VK_CONTROL, VK_DOWN, VK_ESCAPE,
            VK_OEM_MINUS, VK_OEM_PLUS, VK_UP,
        },
        WindowsAndMessaging::*,
    },
};

use crate::{
    app::{ensure_item_image_bytes, image_input_for_ocr, ClipItem},
    i18n::tr,
    ui::{draw_round_rect, draw_text, draw_text_ex, rgb, Theme},
    win_system_ui::{apply_window_corner_preference, get_x_lparam, get_y_lparam, to_wide},
};

#[link(name = "user32")]
unsafe extern "system" {
    fn TrackMouseEvent(lpeventtrack: *mut TRACKMOUSEEVENT) -> i32;
    fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: i32) -> i32;
    fn OpenClipboard(hwnd_new_owner: HWND) -> i32;
    fn CloseClipboard() -> i32;
    fn EmptyClipboard() -> i32;
    fn SetClipboardData(u_format: u32, h_mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(u_flags: u32, dw_bytes: usize) -> *mut core::ffi::c_void;
    fn GlobalFree(h_mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn GlobalLock(h_mem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn GlobalUnlock(h_mem: *mut core::ffi::c_void) -> i32;
}

const CF_UNICODETEXT: u32 = 13;
const GMEM_MOVEABLE_FLAG: u32 = 0x0002;
const DSTINVERT: u32 = 0x00550009;

const TME_LEAVE: u32 = 0x00000002;
const WM_MOUSELEAVE_MSG: u32 = 0x02A3;
const STICKER_CLASS: &str = "ZsClipSticker";
const STICKER_BAR_H: i32 = 36;
const STICKER_BTN_W: i32 = 28;
const STICKER_BTN_H: i32 = 24;
const STICKER_BTN_GAP: i32 = 6;
const STICKER_ZOOM_IN_KEY: u32 = VK_OEM_PLUS as u32;
const STICKER_ZOOM_OUT_KEY: u32 = VK_OEM_MINUS as u32;
const WM_STICKER_IMAGE_READY: u32 = WM_APP + 52;
const WM_STICKER_OCR_READY: u32 = WM_APP + 53;

#[repr(C)]
struct TRACKMOUSEEVENT {
    cb_size: u32,
    dw_flags: u32,
    hwnd_track: HWND,
    dw_hover_time: u32,
}

struct StickerImageResult {
    image: Option<(Vec<u8>, usize, usize)>,
}

struct StickerOcrResult {
    lines: Vec<crate::shell::OcrLine>,
    plain_text: String,
    error: bool,
    error_message: String,
}

struct StickerData {
    width: i32,
    height: i32,
    bgra: Vec<u8>,
    zoom_pct: i32,
    hover_btn: i32,
    down_btn: i32,
    loading: bool,
    // OCR support
    item: ClipItem,
    ocr_enabled: bool,
    ocr_provider: String,
    ocr_cloud_url: String,
    ocr_cloud_token: String,
    ocr_wechat_dir: String,
    ocr_loading: bool,
    ocr_error: bool,
    ocr_error_message: String,
    // Structured OCR results
    ocr_lines: Vec<crate::shell::OcrLine>,
    ocr_plain_text: String,
    // Selection state
    ocr_sel_anchor: i32,   // -1 = none
    ocr_sel_cursor: i32,   // -1 = none
    ocr_hover_line: i32,   // -1 = none
    ocr_mouse_selecting: bool,
}

impl StickerData {
    fn has_ocr_result(&self) -> bool {
        !self.ocr_plain_text.is_empty() || !self.ocr_lines.is_empty()
    }
}

// Maps image coordinates to window client coordinates
struct ImgToScreen {
    dx: i32,
    dy: i32,
    scale_x: f32,
    scale_y: f32,
}

impl ImgToScreen {
    fn from_data(data: &StickerData, client_rc: &RECT) -> Option<Self> {
        if data.bgra.is_empty() || data.width <= 0 || data.height <= 0 {
            return None;
        }
        let content = RECT {
            left: 12,
            top: STICKER_BAR_H + 2,
            right: client_rc.right - 12,
            bottom: client_rc.bottom - 12,
        };
        let avail_w = max(1, content.right - content.left);
        let avail_h = max(1, content.bottom - content.top);
        let zoom = data.zoom_pct.clamp(20, 400) as f32 / 100.0;
        let iw = max(1, (data.width as f32 * zoom).round() as i32);
        let ih = max(1, (data.height as f32 * zoom).round() as i32);
        let scale = (avail_w as f32 / iw as f32).min(avail_h as f32 / ih as f32).min(1.0);
        let dw = max(1, (iw as f32 * scale).round() as i32);
        let dh = max(1, (ih as f32 * scale).round() as i32);
        Some(ImgToScreen {
            dx: content.left + (avail_w - dw) / 2,
            dy: content.top + (avail_h - dh) / 2,
            scale_x: dw as f32 / data.width as f32,
            scale_y: dh as f32 / data.height as f32,
        })
    }

    fn line_rect(&self, line: &crate::shell::OcrLine) -> RECT {
        RECT {
            left: self.dx + (line.left as f32 * self.scale_x) as i32,
            top: self.dy + (line.top as f32 * self.scale_y) as i32,
            right: self.dx + ((line.left + line.width) as f32 * self.scale_x) as i32,
            bottom: self.dy + ((line.top + line.height) as f32 * self.scale_y) as i32,
        }
    }

    fn hit_test(&self, lines: &[crate::shell::OcrLine], x: i32, y: i32) -> i32 {
        for (i, line) in lines.iter().enumerate() {
            let rc = self.line_rect(line);
            if x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom {
                return i as i32;
            }
        }
        -1
    }
}

fn line_is_selected(anchor: i32, cursor: i32, idx: i32) -> bool {
    if anchor < 0 || cursor < 0 {
        return false;
    }
    let lo = anchor.min(cursor);
    let hi = anchor.max(cursor);
    idx >= lo && idx <= hi
}

fn get_ocr_copy_text(data: &StickerData) -> String {
    if data.ocr_lines.is_empty() {
        return data.ocr_plain_text.clone();
    }
    let lo;
    let hi;
    if data.ocr_sel_anchor >= 0 && data.ocr_sel_cursor >= 0 {
        lo = data.ocr_sel_anchor.min(data.ocr_sel_cursor) as usize;
        hi = data.ocr_sel_anchor.max(data.ocr_sel_cursor) as usize;
    } else {
        lo = 0;
        hi = data.ocr_lines.len().saturating_sub(1);
    }
    let hi = hi.min(data.ocr_lines.len().saturating_sub(1));
    if lo > hi {
        return String::new();
    }
    data.ocr_lines[lo..=hi]
        .iter()
        .map(|l| l.text.as_str())
        .collect::<Vec<_>>()
        .join("\n")
}

fn sticker_btn_rect(hwnd: HWND, index: i32) -> RECT {
    let mut rc: RECT = unsafe { zeroed() };
    unsafe { GetClientRect(hwnd, &mut rc); }
    let right_pad = 10;
    let top = 6;
    let right = rc.right - right_pad - index * (STICKER_BTN_W + STICKER_BTN_GAP);
    RECT { left: right - STICKER_BTN_W, top, right, bottom: top + STICKER_BTN_H }
}

fn sticker_hit_btn(hwnd: HWND, x: i32, y: i32) -> i32 {
    for idx in 0..5i32 {
        let rc = sticker_btn_rect(hwnd, idx);
        if x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom { return idx + 1; }
    }
    0
}

unsafe fn sticker_apply_zoom(hwnd: HWND, data: &StickerData) {
    let zoom = data.zoom_pct.clamp(20, 400);
    let w = max(180, min(960, data.width * zoom / 100 + 24));
    let h = max(120, min(960, data.height * zoom / 100 + STICKER_BAR_H + 24));
    SetWindowPos(hwnd, null_mut(), 0, 0, w, h, SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE);
}

unsafe fn sticker_track_mouse(hwnd: HWND) {
    let mut tme = TRACKMOUSEEVENT {
        cb_size: size_of::<TRACKMOUSEEVENT>() as u32,
        dw_flags: TME_LEAVE,
        hwnd_track: hwnd,
        dw_hover_time: 0,
    };
    TrackMouseEvent(&mut tme);
}

unsafe fn copy_text_to_clipboard(hwnd: HWND, text: &str) {
    let wide: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    let byte_len = wide.len() * 2;
    let hmem = GlobalAlloc(GMEM_MOVEABLE_FLAG, byte_len);
    if hmem.is_null() { return; }
    let ptr = GlobalLock(hmem);
    if ptr.is_null() {
        GlobalFree(hmem);
        return;
    }
    std::ptr::copy_nonoverlapping(wide.as_ptr(), ptr as *mut u16, wide.len());
    GlobalUnlock(hmem);
    if OpenClipboard(hwnd) != 0 {
        EmptyClipboard();
        SetClipboardData(CF_UNICODETEXT, hmem);
        CloseClipboard();
    } else {
        GlobalFree(hmem);
    }
}

unsafe fn sticker_spawn_ocr(
    hwnd: HWND,
    provider: String,
    cloud_url: String,
    cloud_token: String,
    wechat_dir: String,
    item: ClipItem,
) {
    let hwnd_val = hwnd as isize;
    std::thread::spawn(move || {
        let result: Result<StickerOcrResult, String> = match provider.as_str() {
            "baidu" => image_input_for_ocr(&item)
                .ok_or_else(|| {
                    tr("当前记录没有可识别的图片文件", "No recognizable image file").to_string()
                })
                .and_then(|input| {
                    let res = std::fs::read(&input.path)
                        .map_err(|e| e.to_string())
                        .and_then(|bytes| {
                            crate::shell::run_baidu_ocr_api_lines(&cloud_url, &cloud_token, &bytes)
                        });
                    if input.delete_after {
                        let _ = std::fs::remove_file(&input.path);
                    }
                    res.map(|lines| {
                        let plain_text = lines
                            .iter()
                            .map(|l| l.text.as_str())
                            .collect::<Vec<_>>()
                            .join("\n");
                        StickerOcrResult {
                            lines,
                            plain_text,
                            error: false,
                            error_message: String::new(),
                        }
                    })
                }),
            "winocr" => image_input_for_ocr(&item)
                .ok_or_else(|| {
                    tr("当前记录没有可识别的图片文件", "No recognizable image file").to_string()
                })
                .and_then(|input| {
                    let res = crate::shell::run_winocr_dll_ocr(&input.path, &wechat_dir);
                    if input.delete_after {
                        let _ = std::fs::remove_file(&input.path);
                    }
                    res.map(|text| StickerOcrResult {
                        lines: vec![],
                        plain_text: text,
                        error: false,
                        error_message: String::new(),
                    })
                }),
            _ => Err(
                tr(
                    "请先在设置-插件中启用图片 OCR",
                    "Please enable Image OCR in Settings > Plugins first",
                )
                .to_string(),
            ),
        };
        let payload = Box::new(match result {
            Ok(r) => r,
            Err(err) => StickerOcrResult {
                lines: vec![],
                plain_text: String::new(),
                error: true,
                error_message: err,
            },
        });
        unsafe {
            let hwnd = hwnd_val as HWND;
            if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                let _ = PostMessageW(hwnd, WM_STICKER_OCR_READY, 0, Box::into_raw(payload) as LPARAM);
            }
        }
    });
}

unsafe extern "system" fn sticker_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe fn sticker_cancel_ocr_selection(hwnd: HWND, data: &mut StickerData, invalidate: bool) {
        data.ocr_mouse_selecting = false;
        data.ocr_sel_anchor = -1;
        data.ocr_sel_cursor = -1;
        if ReleaseCapture() == 0 {
            let _ = hwnd;
        }
        if invalidate {
            InvalidateRect(hwnd, null(), 0);
        }
    }

    match msg {
        WM_NCCREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            apply_window_corner_preference(hwnd);
            1
        }
        WM_MOUSELEAVE_MSG => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() {
                (*ptr).hover_btn = 0;
                (*ptr).down_btn = 0;
                (*ptr).ocr_hover_line = -1;
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_MOUSEMOVE => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() {
                let x = get_x_lparam(lparam);
                let y = get_y_lparam(lparam);
                let hover = sticker_hit_btn(hwnd, x, y);
                let mut changed = false;
                if (*ptr).hover_btn != hover {
                    (*ptr).hover_btn = hover;
                    changed = true;
                }
                // Update OCR hover / selection cursor
                if !(*ptr).ocr_lines.is_empty() {
                    let mut rc_client: RECT = zeroed();
                    GetClientRect(hwnd, &mut rc_client);
                    if let Some(ims) = ImgToScreen::from_data(&*ptr, &rc_client) {
                        let hit_line = ims.hit_test(&(*ptr).ocr_lines, x, y);
                        if (*ptr).ocr_mouse_selecting {
                            if hit_line >= 0 && (*ptr).ocr_sel_cursor != hit_line {
                                (*ptr).ocr_sel_cursor = hit_line;
                                changed = true;
                            }
                        } else if (*ptr).ocr_hover_line != hit_line {
                            (*ptr).ocr_hover_line = hit_line;
                            changed = true;
                        }
                    }
                }
                if changed { InvalidateRect(hwnd, null(), 0); }
                sticker_track_mouse(hwnd);
            }
            0
        }
        WM_LBUTTONDOWN => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            if !ptr.is_null() {
                let hit = sticker_hit_btn(hwnd, x, y);
                let active = match hit {
                    1 | 2 | 3 => true,
                    4 => (*ptr).ocr_enabled,
                    5 => (*ptr).has_ocr_result(),
                    _ => false,
                };
                if active {
                    (*ptr).down_btn = hit;
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                // OCR text selection in image area
                if !(*ptr).ocr_lines.is_empty() && hit == 0 {
                    let mut rc_client: RECT = zeroed();
                    GetClientRect(hwnd, &mut rc_client);
                    if let Some(ims) = ImgToScreen::from_data(&*ptr, &rc_client) {
                        let hit_line = ims.hit_test(&(*ptr).ocr_lines, x, y);
                        if hit_line >= 0 {
                            (*ptr).ocr_sel_anchor = hit_line;
                            (*ptr).ocr_sel_cursor = hit_line;
                            (*ptr).ocr_mouse_selecting = true;
                            SetCapture(hwnd);
                            InvalidateRect(hwnd, null(), 0);
                            return 0;
                        } else if (*ptr).ocr_sel_anchor >= 0 {
                            // Click in image but not on a line — clear selection
                            (*ptr).ocr_sel_anchor = -1;
                            (*ptr).ocr_sel_cursor = -1;
                            InvalidateRect(hwnd, null(), 0);
                        }
                    }
                }
            }
            if y <= STICKER_BAR_H {
                SendMessageW(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as WPARAM, 0);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONUP => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() {
                // End selection drag
                if (*ptr).ocr_mouse_selecting {
                    sticker_cancel_ocr_selection(hwnd, &mut *ptr, true);
                    return 0;
                }
                let x = get_x_lparam(lparam);
                let y = get_y_lparam(lparam);
                let hit = sticker_hit_btn(hwnd, x, y);
                let down = (*ptr).down_btn;
                (*ptr).down_btn = 0;
                if hit != 0 && hit == down {
                    match hit {
                        1 => { DestroyWindow(hwnd); }
                        2 => {
                            (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10);
                            sticker_apply_zoom(hwnd, &*ptr);
                        }
                        3 => {
                            (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10);
                            sticker_apply_zoom(hwnd, &*ptr);
                        }
                        4 => {
                            if (*ptr).ocr_enabled && !(*ptr).ocr_loading {
                                (*ptr).ocr_loading = true;
                                (*ptr).ocr_lines = Vec::new();
                                (*ptr).ocr_plain_text = String::new();
                                (*ptr).ocr_error = false;
                                (*ptr).ocr_sel_anchor = -1;
                                (*ptr).ocr_sel_cursor = -1;
                                InvalidateRect(hwnd, null(), 0);
                                let d = &*ptr;
                                sticker_spawn_ocr(
                                    hwnd,
                                    d.ocr_provider.clone(),
                                    d.ocr_cloud_url.clone(),
                                    d.ocr_cloud_token.clone(),
                                    d.ocr_wechat_dir.clone(),
                                    d.item.clone(),
                                );
                            }
                        }
                        5 => {
                            let text = get_ocr_copy_text(&*ptr);
                            if !text.is_empty() {
                                copy_text_to_clipboard(hwnd, &text);
                            }
                        }
                        _ => {}
                    }
                }
                InvalidateRect(hwnd, null(), 0);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_CAPTURECHANGED | WM_CANCELMODE => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() && (*ptr).ocr_mouse_selecting {
                sticker_cancel_ocr_selection(hwnd, &mut *ptr, true);
            }
            0
        }
        WM_MOUSEWHEEL => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() {
                let delta = ((wparam >> 16) & 0xffff) as i16 as i32;
                if delta > 0 {
                    (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10);
                } else {
                    (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10);
                }
                sticker_apply_zoom(hwnd, &*ptr);
                InvalidateRect(hwnd, null(), 0);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_RBUTTONUP => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() && (*ptr).ocr_mouse_selecting {
                sticker_cancel_ocr_selection(hwnd, &mut *ptr, false);
            }
            DestroyWindow(hwnd);
            0
        }
        WM_STICKER_IMAGE_READY => {
            let payload_ptr = lparam as *mut StickerImageResult;
            if payload_ptr.is_null() { return 0; }
            let payload = Box::from_raw(payload_ptr);
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() {
                let data = &mut *ptr;
                data.loading = false;
                if let Some((mut bytes, width, height)) = payload.image {
                    for px in bytes.chunks_exact_mut(4) { px.swap(0, 2); }
                    data.width = width as i32;
                    data.height = height as i32;
                    data.bgra = bytes;
                    sticker_apply_zoom(hwnd, data);
                }
                InvalidateRect(hwnd, null(), 0);
            }
            0
        }
        WM_STICKER_OCR_READY => {
            if lparam == 0 { return 0; }
            let payload = Box::from_raw(lparam as *mut StickerOcrResult);
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() {
                (*ptr).ocr_loading = false;
                (*ptr).ocr_error = payload.error;
                (*ptr).ocr_error_message = payload.error_message;
                (*ptr).ocr_lines = payload.lines;
                (*ptr).ocr_plain_text = payload.plain_text;
                (*ptr).ocr_sel_anchor = -1;
                (*ptr).ocr_sel_cursor = -1;
                InvalidateRect(hwnd, null(), 0);
                if (*ptr).ocr_error && !(*ptr).ocr_error_message.trim().is_empty() {
                    MessageBoxW(
                        hwnd,
                        to_wide(&(*ptr).ocr_error_message).as_ptr(),
                        to_wide(tr("图片转文字", "Image OCR")).as_ptr(),
                        MB_OK | MB_ICONERROR,
                    );
                }
            }
            0
        }
        WM_KEYDOWN => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if (wparam as u32) == VK_ESCAPE as u32 { DestroyWindow(hwnd); return 0; }
            if !ptr.is_null() {
                if (wparam as u32) == STICKER_ZOOM_IN_KEY || (wparam as u32) == VK_UP as u32 {
                    (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10);
                    sticker_apply_zoom(hwnd, &*ptr);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                if (wparam as u32) == STICKER_ZOOM_OUT_KEY || (wparam as u32) == VK_DOWN as u32 {
                    (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10);
                    sticker_apply_zoom(hwnd, &*ptr);
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
                }
                // Ctrl+C / Ctrl+A for OCR selection
                let ctrl = (GetKeyState(VK_CONTROL as i32) as u16) & 0x8000 != 0;
                if ctrl {
                    match wparam as u32 {
                        0x43 => {
                            // Ctrl+C: copy selected lines (or all)
                            let text = get_ocr_copy_text(&*ptr);
                            if !text.is_empty() {
                                copy_text_to_clipboard(hwnd, &text);
                            }
                            return 0;
                        }
                        0x41 => {
                            // Ctrl+A: select all OCR lines
                            if !(*ptr).ocr_lines.is_empty() {
                                (*ptr).ocr_sel_anchor = 0;
                                (*ptr).ocr_sel_cursor = (*ptr).ocr_lines.len() as i32 - 1;
                                InvalidateRect(hwnd, null(), 0);
                            }
                            return 0;
                        }
                        _ => {}
                    }
                }
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_NCHITTEST => {
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            let mut rc: RECT = zeroed();
            GetWindowRect(hwnd, &mut rc);
            let grip = 8;
            if x >= rc.right - grip && y >= rc.bottom - grip { return HTBOTTOMRIGHT as isize; }
            if x <= rc.left + grip && y >= rc.bottom - grip { return HTBOTTOMLEFT as isize; }
            if x >= rc.right - grip && y <= rc.top + grip { return HTTOPRIGHT as isize; }
            if x <= rc.left + grip && y <= rc.top + grip { return HTTOPLEFT as isize; }
            if x <= rc.left + grip { return HTLEFT as isize; }
            if x >= rc.right - grip { return HTRIGHT as isize; }
            if y <= rc.top + grip { return HTTOP as isize; }
            if y >= rc.bottom - grip { return HTBOTTOM as isize; }
            let cx = x - rc.left;
            let cy = y - rc.top;
            if cy <= STICKER_BAR_H && sticker_hit_btn(hwnd, cx, cy) == 0 { return HTCAPTION as isize; }
            HTCLIENT as isize
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if ptr.is_null() { return DefWindowProcW(hwnd, msg, wparam, lparam); }
            let data = &mut *ptr;
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            let mut rc: RECT = zeroed();
            GetClientRect(hwnd, &mut rc);
            let th = Theme::default();
            let memdc = CreateCompatibleDC(hdc);
            let membmp = CreateCompatibleBitmap(hdc, rc.right - rc.left, rc.bottom - rc.top);
            let oldbmp = SelectObject(memdc, membmp as _);
            let bg = CreateSolidBrush(th.bg);
            FillRect(memdc, &rc, bg); DeleteObject(bg as _);
            draw_round_rect(memdc as _, &rc, th.surface, th.stroke, 8);
            let bar = RECT { left: 1, top: 1, right: rc.right - 1, bottom: STICKER_BAR_H };
            draw_round_rect(memdc as _, &bar, th.surface2, th.surface2, 8);
            draw_text_ex(memdc as _, tr("贴图", "Sticker"), &RECT{ left: 14, top: 8, right: 120, bottom: 30 }, th.text, 13, true, false, "Segoe UI");

            // Base buttons: ×(0), +(1), −(2)
            let base_labels = ["×", "+", "−"];
            for idx in 0..3i32 {
                let brc = sticker_btn_rect(hwnd, idx);
                let hover = data.hover_btn == idx + 1;
                let down = data.down_btn == idx + 1;
                let fill = if idx == 0 && hover { th.close_hover } else if down { th.button_pressed } else if hover { th.button_hover } else { th.button_bg };
                let stroke = if idx == 0 && hover { th.close_hover } else { th.control_stroke };
                draw_round_rect(memdc as _, &brc, fill, stroke, 4);
                let txt = if idx == 0 && hover { rgb(255, 255, 255) } else { th.text };
                draw_text(memdc as _, base_labels[idx as usize], &brc, txt, 14, false, true);
            }

            // OCR button (index 3): shown when OCR is enabled
            if data.ocr_enabled {
                let brc = sticker_btn_rect(hwnd, 3);
                let hover = data.hover_btn == 4;
                let down = data.down_btn == 4;
                let is_error = data.ocr_error && !data.ocr_loading;
                let fill = if is_error && hover { rgb(180, 50, 50) }
                           else if is_error { rgb(100, 30, 30) }
                           else if down { th.button_pressed }
                           else if hover { th.button_hover }
                           else { th.button_bg };
                draw_round_rect(memdc as _, &brc, fill, th.control_stroke, 4);
                let label = if data.ocr_loading { "…" } else if is_error { "!" } else { "文" };
                let txt_color = if is_error { rgb(255, 150, 150) }
                                else if data.ocr_loading { th.text_muted }
                                else { th.text };
                draw_text(memdc as _, label, &brc, txt_color, 12, false, true);
            }

            // Copy button (index 4): shown when OCR has results
            if data.has_ocr_result() {
                let brc = sticker_btn_rect(hwnd, 4);
                let hover = data.hover_btn == 5;
                let down = data.down_btn == 5;
                let fill = if down { th.button_pressed } else if hover { th.button_hover } else { th.button_bg };
                draw_round_rect(memdc as _, &brc, fill, th.control_stroke, 4);
                draw_text(memdc as _, "复", &brc, th.text, 12, false, true);
            }

            // Image content area
            let content = RECT { left: 12, top: STICKER_BAR_H + 2, right: rc.right - 12, bottom: rc.bottom - 12 };
            if data.loading || data.bgra.is_empty() {
                draw_text_ex(
                    memdc as _,
                    tr("正在加载预览…", "Loading preview..."),
                    &content,
                    th.text_muted,
                    12,
                    false,
                    true,
                    "Segoe UI",
                );
            } else {
                let avail_w = max(1, content.right - content.left);
                let avail_h = max(1, content.bottom - content.top);
                let zoom = data.zoom_pct.clamp(20, 400) as f32 / 100.0;
                let iw = max(1, (data.width as f32 * zoom).round() as i32);
                let ih = max(1, (data.height as f32 * zoom).round() as i32);
                let scale = (avail_w as f32 / iw as f32).min(avail_h as f32 / ih as f32).min(1.0);
                let dw = max(1, (iw as f32 * scale).round() as i32);
                let dh = max(1, (ih as f32 * scale).round() as i32);
                let dx = content.left + (avail_w - dw) / 2;
                let dy = content.top + (avail_h - dh) / 2;
                let mut bmi: BITMAPINFO = zeroed();
                bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                bmi.bmiHeader.biWidth = data.width;
                bmi.bmiHeader.biHeight = -data.height;
                bmi.bmiHeader.biPlanes = 1;
                bmi.bmiHeader.biBitCount = 32;
                bmi.bmiHeader.biCompression = BI_RGB;
                StretchDIBits(memdc, dx, dy, dw, dh, 0, 0, data.width, data.height, data.bgra.as_ptr() as _, &bmi, DIB_RGB_COLORS, SRCCOPY);

                // Draw OCR text region overlays
                if !data.ocr_lines.is_empty() {
                    let client_rc = rc;
                    if let Some(ims) = ImgToScreen::from_data(data, &client_rc) {
                        let sel_anchor = data.ocr_sel_anchor;
                        let sel_cursor = data.ocr_sel_cursor;
                        let hover_line = data.ocr_hover_line;
                        for (i, line) in data.ocr_lines.iter().enumerate() {
                            let lrc = ims.line_rect(line);
                            let selected = line_is_selected(sel_anchor, sel_cursor, i as i32);
                            let hovering = hover_line == i as i32 && !selected;
                            if selected {
                                // Invert pixels to indicate selection
                                PatBlt(memdc, lrc.left, lrc.top, lrc.right - lrc.left, lrc.bottom - lrc.top, DSTINVERT);
                            } else if hovering {
                                // Blue border for hover hint
                                let border_brush = CreateSolidBrush(rgb(80, 140, 255));
                                FrameRect(memdc, &lrc, border_brush);
                                DeleteObject(border_brush as _);
                            }
                        }
                    }
                }
            }
            BitBlt(hdc, 0, 0, rc.right - rc.left, rc.bottom - rc.top, memdc, 0, 0, SRCCOPY);
            SelectObject(memdc, oldbmp); DeleteObject(membmp as _); DeleteDC(memdc);
            EndPaint(hwnd, &ps);
            0
        }
        WM_NCDESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() {
                if (*ptr).ocr_mouse_selecting {
                    sticker_cancel_ocr_selection(hwnd, &mut *ptr, false);
                }
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_sticker_class() {
    static DONE: OnceLock<()> = OnceLock::new();
    if DONE.get().is_some() { return; }
    let hinstance = GetModuleHandleW(null());
    let class_name = to_wide(STICKER_CLASS);
    let mut wc: WNDCLASSW = zeroed();
    wc.lpfnWndProc = Some(sticker_wnd_proc);
    wc.hInstance = hinstance;
    wc.lpszClassName = class_name.as_ptr();
    wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
    wc.hbrBackground = (WHITE_BRUSH as usize) as _;
    RegisterClassW(&wc);
    let _ = DONE.set(());
}

pub(crate) unsafe fn show_image_sticker(item: &ClipItem, settings: &crate::app::state::AppSettings) {
    ensure_sticker_class();
    let ocr_enabled = settings.image_ocr_provider != "off";
    let data = Box::new(StickerData {
        width: item.image_width.max(1) as i32,
        height: item.image_height.max(1) as i32,
        bgra: Vec::new(),
        zoom_pct: 100,
        hover_btn: 0,
        down_btn: 0,
        loading: true,
        item: item.clone(),
        ocr_enabled,
        ocr_provider: settings.image_ocr_provider.clone(),
        ocr_cloud_url: settings.image_ocr_cloud_url.clone(),
        ocr_cloud_token: settings.image_ocr_cloud_token.clone(),
        ocr_wechat_dir: settings.image_ocr_wechat_dir.clone(),
        ocr_loading: false,
        ocr_error: false,
        ocr_error_message: String::new(),
        ocr_lines: Vec::new(),
        ocr_plain_text: String::new(),
        ocr_sel_anchor: -1,
        ocr_sel_cursor: -1,
        ocr_hover_line: -1,
        ocr_mouse_selecting: false,
    });
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    let w = min(760, max(260, data.width + 24));
    let h = min(760, max(180, data.height + STICKER_BAR_H + 24));
    let hwnd = CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        to_wide(STICKER_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_POPUP | WS_VISIBLE | WS_THICKFRAME,
        pt.x + 16, pt.y + 16, w, h,
        null_mut(), null_mut(), GetModuleHandleW(null()),
        Box::into_raw(data) as _,
    );
    if !hwnd.is_null() {
        ShowWindow(hwnd, SW_SHOW);
        InvalidateRect(hwnd, null(), 0);
        let hwnd_raw = hwnd as isize;
        let item = item.clone();
        std::thread::spawn(move || {
            let payload = Box::new(StickerImageResult {
                image: ensure_item_image_bytes(&item),
            });
            unsafe {
                let hwnd = hwnd_raw as HWND;
                if !hwnd.is_null() && IsWindow(hwnd) != 0 {
                    let _ = PostMessageW(hwnd, WM_STICKER_IMAGE_READY, 0, Box::into_raw(payload) as LPARAM);
                }
            }
        });
    }
}
