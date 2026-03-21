use std::cmp::{max, min};
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::sync::OnceLock;

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateSolidBrush, DeleteDC, DeleteObject, EndPaint, FillRect, PAINTSTRUCT, SelectObject, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, SRCCOPY, WHITE_BRUSH,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::{Input::KeyboardAndMouse::{VK_DOWN, VK_ESCAPE, VK_OEM_MINUS, VK_OEM_PLUS, VK_UP}, WindowsAndMessaging::*},
};

use crate::{
    app::{ensure_item_image_bytes, ClipItem},
    i18n::tr,
    ui::{draw_round_rect, draw_text, draw_text_ex, rgb, Theme},
    win_system_ui::{apply_window_corner_preference, get_x_lparam, get_y_lparam, to_wide},
};

#[repr(C)]
struct TRACKMOUSEEVENT {
    cb_size: u32,
    dw_flags: u32,
    hwnd_track: HWND,
    dw_hover_time: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn TrackMouseEvent(lpeventtrack: *mut TRACKMOUSEEVENT) -> i32;
    fn InvalidateRect(hWnd: HWND, lpRect: *const RECT, bErase: i32) -> i32;
}

const TME_LEAVE: u32 = 0x00000002;
const WM_MOUSELEAVE_MSG: u32 = 0x02A3;
const STICKER_CLASS: &str = "ZsClipSticker";
const STICKER_BAR_H: i32 = 36;
const STICKER_BTN_W: i32 = 28;
const STICKER_BTN_H: i32 = 24;
const STICKER_BTN_GAP: i32 = 6;
const STICKER_ZOOM_IN_KEY: u32 = VK_OEM_PLUS as u32;
const STICKER_ZOOM_OUT_KEY: u32 = VK_OEM_MINUS as u32;

struct StickerData {
    width: i32,
    height: i32,
    bgra: Vec<u8>,
    zoom_pct: i32,
    hover_btn: i32,
    down_btn: i32,
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
    for idx in 0..3 {
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
    let mut tme = TRACKMOUSEEVENT { cb_size: size_of::<TRACKMOUSEEVENT>() as u32, dw_flags: TME_LEAVE, hwnd_track: hwnd, dw_hover_time: 0 };
    TrackMouseEvent(&mut tme);
}

unsafe extern "system" fn sticker_wnd_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
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
                if (*ptr).hover_btn != hover {
                    (*ptr).hover_btn = hover;
                    InvalidateRect(hwnd, null(), 0);
                }
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
                if hit != 0 {
                    (*ptr).down_btn = hit;
                    InvalidateRect(hwnd, null(), 0);
                    return 0;
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
                let x = get_x_lparam(lparam);
                let y = get_y_lparam(lparam);
                let hit = sticker_hit_btn(hwnd, x, y);
                let down = (*ptr).down_btn;
                (*ptr).down_btn = 0;
                if hit != 0 && hit == down {
                    match hit {
                        1 => { DestroyWindow(hwnd); }
                        2 => { (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10); sticker_apply_zoom(hwnd, &*ptr); }
                        3 => { (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10); sticker_apply_zoom(hwnd, &*ptr); }
                        _ => {}
                    }
                }
                InvalidateRect(hwnd, null(), 0);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEWHEEL => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() {
                let delta = ((wparam >> 16) & 0xffff) as i16 as i32;
                if delta > 0 { (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10); } else { (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10); }
                sticker_apply_zoom(hwnd, &*ptr);
                InvalidateRect(hwnd, null(), 0);
                return 0;
            }
            DefWindowProcW(hwnd, msg, wparam, lparam)
        }
        WM_RBUTTONUP => { DestroyWindow(hwnd); 0 }
        WM_KEYDOWN => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if (wparam as u32) == VK_ESCAPE as u32 { DestroyWindow(hwnd); return 0; }
            if !ptr.is_null() {
                if (wparam as u32) == STICKER_ZOOM_IN_KEY || (wparam as u32) == VK_UP as u32 { (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10); sticker_apply_zoom(hwnd, &*ptr); InvalidateRect(hwnd, null(), 0); return 0; }
                if (wparam as u32) == STICKER_ZOOM_OUT_KEY || (wparam as u32) == VK_DOWN as u32 { (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10); sticker_apply_zoom(hwnd, &*ptr); InvalidateRect(hwnd, null(), 0); return 0; }
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
            let cx = x - rc.left; let cy = y - rc.top;
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
            let labels = ["×", "+", "−"];
            for idx in 0..3 {
                let brc = sticker_btn_rect(hwnd, idx as i32);
                let hover = data.hover_btn == (idx as i32 + 1);
                let down = data.down_btn == (idx as i32 + 1);
                let fill = if idx == 0 && hover { th.close_hover } else if down { th.button_pressed } else if hover { th.button_hover } else { th.button_bg };
                let stroke = if idx == 0 && hover { th.close_hover } else { th.control_stroke };
                draw_round_rect(memdc as _, &brc, fill, stroke, 4);
                let txt = if idx == 0 && hover { rgb(255,255,255) } else { th.text };
                draw_text(memdc as _, labels[idx], &brc, txt, 14, false, true);
            }
            let content = RECT { left: 12, top: STICKER_BAR_H + 2, right: rc.right - 12, bottom: rc.bottom - 12 };
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
            BitBlt(hdc, 0, 0, rc.right - rc.left, rc.bottom - rc.top, memdc, 0, 0, SRCCOPY);
            SelectObject(memdc, oldbmp); DeleteObject(membmp as _); DeleteDC(memdc);
            EndPaint(hwnd, &ps);
            0
        }
        WM_NCDESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut StickerData;
            if !ptr.is_null() { drop(Box::from_raw(ptr)); SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0); }
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

pub(crate) unsafe fn show_image_sticker(item: &ClipItem) {
    let Some((bytes, width, height)) = ensure_item_image_bytes(item) else { return; };
    ensure_sticker_class();
    let mut bgra = bytes;
    for px in bgra.chunks_exact_mut(4) { px.swap(0,2); }
    let data = Box::new(StickerData { width: width as i32, height: height as i32, bgra, zoom_pct: 100, hover_btn: 0, down_btn: 0 });
    let mut pt: POINT = zeroed();
    GetCursorPos(&mut pt);
    let w = min(760, max(260, width as i32 + 24));
    let h = min(760, max(180, height as i32 + STICKER_BAR_H + 24));
    let hwnd = CreateWindowExW(WS_EX_TOPMOST | WS_EX_TOOLWINDOW, to_wide(STICKER_CLASS).as_ptr(), to_wide("").as_ptr(), WS_POPUP | WS_VISIBLE | WS_THICKFRAME, pt.x + 16, pt.y + 16, w, h, null_mut(), null_mut(), GetModuleHandleW(null()), Box::into_raw(data) as _);
    if !hwnd.is_null() { ShowWindow(hwnd, SW_SHOW); InvalidateRect(hwnd, null(), 0); }
}

