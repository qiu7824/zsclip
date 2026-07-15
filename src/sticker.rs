use std::cmp::{max, min};
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{PAINTSTRUCT, WHITE_BRUSH},
    UI::{
        Input::KeyboardAndMouse::{VK_DOWN, VK_ESCAPE, VK_OEM_MINUS, VK_OEM_PLUS, VK_UP},
        WindowsAndMessaging::*,
    },
};

use crate::{
    app::{ensure_item_image_bytes, ClipItem},
    i18n::tr,
    platform::{
        appearance as platform_appearance, gdi as platform_gdi, input as platform_input,
        monitor as platform_monitor,
        string::to_wide,
        window::{self as platform_window, post_boxed_message},
    },
    ui::{draw_round_rect, draw_text, draw_text_ex, rgba_to_opaque_bgra_on_bg},
    win_native_style::{rgb, Theme},
    win_system_ui::{get_x_lparam, get_y_lparam},
};

const WM_MOUSELEAVE_MSG: u32 = 0x02A3;
const STICKER_CLASS: &str = "ZsClipSticker";
const STICKER_BAR_H: i32 = 36;
const STICKER_BTN_W: i32 = 28;
const STICKER_BTN_H: i32 = 24;
const STICKER_BTN_GAP: i32 = 6;
const STICKER_ZOOM_IN_KEY: u32 = VK_OEM_PLUS as u32;
const STICKER_ZOOM_OUT_KEY: u32 = VK_OEM_MINUS as u32;
const WM_STICKER_IMAGE_READY: u32 = WM_APP + 52;

struct StickerImageResult {
    image: Option<(Vec<u8>, usize, usize)>,
}

struct StickerData {
    settings_owner: HWND,
    width: i32,
    height: i32,
    bgra: Vec<u8>,
    zoom_pct: i32,
    hover_btn: i32,
    down_btn: i32,
    loading: bool,
}

fn sticker_btn_rect(hwnd: HWND, index: i32) -> RECT {
    let rc = platform_window::client_rect(hwnd).unwrap_or_else(|| unsafe { zeroed() });
    let right_pad = 10;
    let top = 6;
    let right = rc.right - right_pad - index * (STICKER_BTN_W + STICKER_BTN_GAP);
    RECT {
        left: right - STICKER_BTN_W,
        top,
        right,
        bottom: top + STICKER_BTN_H,
    }
}

fn sticker_hit_btn(hwnd: HWND, x: i32, y: i32) -> i32 {
    for idx in 0..3i32 {
        let rc = sticker_btn_rect(hwnd, idx);
        if x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom {
            return idx + 1;
        }
    }
    0
}

unsafe fn persist_sticker_layout(hwnd: HWND, data: &StickerData) {
    if let Some(rc) = platform_window::window_rect(hwnd) {
        let zoom_pct = data.zoom_pct.clamp(20, 400);
        crate::app::persist_sticker_layout(rc.left, rc.top, zoom_pct);
        let main_hwnd = crate::app::main_window_hwnd();
        for owner in [main_hwnd, data.settings_owner] {
            let state_ptr = crate::app::get_state_ptr(owner);
            if !state_ptr.is_null() {
                (*state_ptr).settings.sticker_x = rc.left;
                (*state_ptr).settings.sticker_y = rc.top;
                (*state_ptr).settings.sticker_zoom_pct = zoom_pct;
            }
        }
    }
}

unsafe fn sticker_apply_zoom(hwnd: HWND, data: &StickerData) {
    let zoom = data.zoom_pct.clamp(20, 400);
    let w = max(180, min(960, data.width * zoom / 100 + 24));
    let h = max(120, min(960, data.height * zoom / 100 + STICKER_BAR_H + 24));
    platform_window::set_pos(
        hwnd,
        null_mut(),
        0,
        0,
        w,
        h,
        SWP_NOMOVE | SWP_NOZORDER | SWP_NOACTIVATE,
    );
}

unsafe fn sticker_track_mouse(hwnd: HWND) {
    platform_input::track_mouse_leave(hwnd);
}

unsafe extern "system" fn sticker_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            platform_window::set_user_data(hwnd, cs.lpCreateParams as isize);
            platform_appearance::set_rounded_corners(hwnd);
            1
        }
        WM_MOUSELEAVE_MSG => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if !ptr.is_null() {
                (*ptr).hover_btn = 0;
                (*ptr).down_btn = 0;
                platform_gdi::invalidate_rect(hwnd, null(), 0);
            }
            0
        }
        WM_MOUSEMOVE => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if !ptr.is_null() {
                let x = get_x_lparam(lparam);
                let y = get_y_lparam(lparam);
                let hover = sticker_hit_btn(hwnd, x, y);
                let mut changed = false;
                if (*ptr).hover_btn != hover {
                    (*ptr).hover_btn = hover;
                    changed = true;
                }
                if changed {
                    platform_gdi::invalidate_rect(hwnd, null(), 0);
                }
                sticker_track_mouse(hwnd);
            }
            0
        }
        WM_LBUTTONDOWN => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            if !ptr.is_null() {
                let hit = sticker_hit_btn(hwnd, x, y);
                if matches!(hit, 1..=3) {
                    (*ptr).down_btn = hit;
                    platform_gdi::invalidate_rect(hwnd, null(), 0);
                    return 0;
                }
            }
            if y <= STICKER_BAR_H {
                platform_window::send_message(hwnd, WM_NCLBUTTONDOWN, HTCAPTION as WPARAM, 0);
                return 0;
            }
            platform_window::default_window_proc(hwnd, msg, wparam, lparam)
        }
        WM_LBUTTONUP => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if !ptr.is_null() {
                let x = get_x_lparam(lparam);
                let y = get_y_lparam(lparam);
                let hit = sticker_hit_btn(hwnd, x, y);
                let down = (*ptr).down_btn;
                (*ptr).down_btn = 0;
                if hit != 0 && hit == down {
                    match hit {
                        1 => {
                            platform_window::destroy(hwnd);
                        }
                        2 => {
                            (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10);
                            sticker_apply_zoom(hwnd, &*ptr);
                            persist_sticker_layout(hwnd, &*ptr);
                        }
                        3 => {
                            (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10);
                            sticker_apply_zoom(hwnd, &*ptr);
                            persist_sticker_layout(hwnd, &*ptr);
                        }
                        _ => {}
                    }
                }
                platform_gdi::invalidate_rect(hwnd, null(), 0);
                return 0;
            }
            platform_window::default_window_proc(hwnd, msg, wparam, lparam)
        }
        WM_MOUSEWHEEL => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if !ptr.is_null() {
                let delta = ((wparam >> 16) & 0xffff) as i16 as i32;
                if delta > 0 {
                    (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10);
                } else {
                    (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10);
                }
                sticker_apply_zoom(hwnd, &*ptr);
                persist_sticker_layout(hwnd, &*ptr);
                platform_gdi::invalidate_rect(hwnd, null(), 0);
                return 0;
            }
            platform_window::default_window_proc(hwnd, msg, wparam, lparam)
        }
        WM_RBUTTONUP => {
            platform_window::destroy(hwnd);
            0
        }
        WM_STICKER_IMAGE_READY => {
            let payload_ptr = lparam as *mut StickerImageResult;
            if payload_ptr.is_null() {
                return 0;
            }
            let payload = Box::from_raw(payload_ptr);
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if !ptr.is_null() {
                let data = &mut *ptr;
                data.loading = false;
                if let Some((bytes, width, height)) = payload.image {
                    let th = Theme::default();
                    data.width = width as i32;
                    data.height = height as i32;
                    data.bgra = rgba_to_opaque_bgra_on_bg(&bytes, th.surface);
                    sticker_apply_zoom(hwnd, data);
                }
                platform_gdi::invalidate_rect(hwnd, null(), 0);
            }
            0
        }
        WM_KEYDOWN => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if (wparam as u32) == VK_ESCAPE as u32 {
                platform_window::destroy(hwnd);
                return 0;
            }
            if !ptr.is_null() {
                if (wparam as u32) == STICKER_ZOOM_IN_KEY || (wparam as u32) == VK_UP as u32 {
                    (*ptr).zoom_pct = min(400, (*ptr).zoom_pct + 10);
                    sticker_apply_zoom(hwnd, &*ptr);
                    persist_sticker_layout(hwnd, &*ptr);
                    platform_gdi::invalidate_rect(hwnd, null(), 0);
                    return 0;
                }
                if (wparam as u32) == STICKER_ZOOM_OUT_KEY || (wparam as u32) == VK_DOWN as u32 {
                    (*ptr).zoom_pct = max(20, (*ptr).zoom_pct - 10);
                    sticker_apply_zoom(hwnd, &*ptr);
                    persist_sticker_layout(hwnd, &*ptr);
                    platform_gdi::invalidate_rect(hwnd, null(), 0);
                    return 0;
                }
            }
            platform_window::default_window_proc(hwnd, msg, wparam, lparam)
        }
        WM_EXITSIZEMOVE => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if !ptr.is_null() {
                persist_sticker_layout(hwnd, &*ptr);
            }
            0
        }
        WM_NCHITTEST => {
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            let rc = platform_window::window_rect(hwnd).unwrap_or_else(|| zeroed());
            let grip = 8;
            if x >= rc.right - grip && y >= rc.bottom - grip {
                return HTBOTTOMRIGHT as isize;
            }
            if x <= rc.left + grip && y >= rc.bottom - grip {
                return HTBOTTOMLEFT as isize;
            }
            if x >= rc.right - grip && y <= rc.top + grip {
                return HTTOPRIGHT as isize;
            }
            if x <= rc.left + grip && y <= rc.top + grip {
                return HTTOPLEFT as isize;
            }
            if x <= rc.left + grip {
                return HTLEFT as isize;
            }
            if x >= rc.right - grip {
                return HTRIGHT as isize;
            }
            if y <= rc.top + grip {
                return HTTOP as isize;
            }
            if y >= rc.bottom - grip {
                return HTBOTTOM as isize;
            }
            let cx = x - rc.left;
            let cy = y - rc.top;
            if cy <= STICKER_BAR_H && sticker_hit_btn(hwnd, cx, cy) == 0 {
                return HTCAPTION as isize;
            }
            HTCLIENT as isize
        }
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if ptr.is_null() {
                return platform_window::default_window_proc(hwnd, msg, wparam, lparam);
            }
            let data = &mut *ptr;
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = platform_gdi::begin_paint(hwnd, &mut ps);
            let rc = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
            let th = Theme::default();
            let memdc = platform_gdi::create_compatible_dc(hdc);
            let membmp =
                platform_gdi::create_compatible_bitmap(hdc, rc.right - rc.left, rc.bottom - rc.top);
            let oldbmp = platform_gdi::select_object(memdc, membmp as _);
            let bg = platform_gdi::create_solid_brush(th.bg);
            platform_gdi::fill_rect(memdc, &rc, bg);
            platform_gdi::delete_object(bg as _);
            draw_round_rect(memdc as _, &rc, th.surface, th.stroke, 8);
            let bar = RECT {
                left: 1,
                top: 1,
                right: rc.right - 1,
                bottom: STICKER_BAR_H,
            };
            draw_round_rect(memdc as _, &bar, th.surface2, th.surface2, 8);
            draw_text_ex(
                memdc as _,
                tr("贴图", "Sticker"),
                &RECT {
                    left: 14,
                    top: 8,
                    right: 120,
                    bottom: 30,
                },
                th.text,
                13,
                true,
                false,
                "Segoe UI",
            );

            // Base buttons: ×(0), +(1), −(2)
            let base_labels = ["×", "+", "−"];
            for idx in 0..3i32 {
                let brc = sticker_btn_rect(hwnd, idx);
                let hover = data.hover_btn == idx + 1;
                let down = data.down_btn == idx + 1;
                let fill = if idx == 0 && hover {
                    th.close_hover
                } else if down {
                    th.button_pressed
                } else if hover {
                    th.button_hover
                } else {
                    th.button_bg
                };
                let stroke = if idx == 0 && hover {
                    th.close_hover
                } else {
                    th.control_stroke
                };
                draw_round_rect(memdc as _, &brc, fill, stroke, 4);
                let txt = if idx == 0 && hover {
                    rgb(255, 255, 255)
                } else {
                    th.text
                };
                draw_text(
                    memdc as _,
                    base_labels[idx as usize],
                    &brc,
                    txt,
                    14,
                    false,
                    true,
                );
            }

            // Image content area
            let content = RECT {
                left: 12,
                top: STICKER_BAR_H + 2,
                right: rc.right - 12,
                bottom: rc.bottom - 12,
            };
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
                let scale = (avail_w as f32 / iw as f32)
                    .min(avail_h as f32 / ih as f32)
                    .min(1.0);
                let dw = max(1, (iw as f32 * scale).round() as i32);
                let dh = max(1, (ih as f32 * scale).round() as i32);
                let dx = content.left + (avail_w - dw) / 2;
                let dy = content.top + (avail_h - dh) / 2;
                platform_gdi::stretch_top_down_32bpp(
                    memdc,
                    dx,
                    dy,
                    dw,
                    dh,
                    data.width,
                    data.height,
                    &data.bgra,
                );
            }
            platform_gdi::copy_bits(
                hdc,
                0,
                0,
                rc.right - rc.left,
                rc.bottom - rc.top,
                memdc,
                0,
                0,
            );
            platform_gdi::select_object(memdc, oldbmp);
            platform_gdi::delete_object(membmp as _);
            platform_gdi::delete_dc(memdc);
            platform_gdi::end_paint(hwnd, &ps);
            0
        }
        WM_NCDESTROY => {
            let ptr = platform_window::user_data(hwnd) as *mut StickerData;
            if !ptr.is_null() {
                persist_sticker_layout(hwnd, &*ptr);
                drop(Box::from_raw(ptr));
                platform_window::set_user_data(hwnd, 0);
            }
            platform_window::default_window_proc(hwnd, msg, wparam, lparam)
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_sticker_class() {
    static DONE: OnceLock<()> = OnceLock::new();
    if DONE.get().is_some() {
        return;
    }
    let hinstance = platform_window::module_handle();
    let class_name = to_wide(STICKER_CLASS);
    let mut wc: WNDCLASSW = zeroed();
    wc.lpfnWndProc = Some(sticker_wnd_proc);
    wc.hInstance = hinstance;
    wc.lpszClassName = class_name.as_ptr();
    wc.hCursor = platform_window::arrow_cursor();
    wc.hbrBackground = (WHITE_BRUSH as usize) as _;
    platform_window::register_class(&wc);
    let _ = DONE.set(());
}

pub(crate) unsafe fn show_image_sticker(
    settings_owner: HWND,
    item: &ClipItem,
    settings: &crate::app::state::AppSettings,
) {
    ensure_sticker_class();
    let data = Box::new(StickerData {
        settings_owner,
        width: item.image_width.max(1) as i32,
        height: item.image_height.max(1) as i32,
        bgra: Vec::new(),
        zoom_pct: settings.sticker_zoom_pct.clamp(20, 400),
        hover_btn: 0,
        down_btn: 0,
        loading: true,
    });
    let pt = platform_input::cursor_pos().unwrap_or_else(|| zeroed());
    let zoom = data.zoom_pct as f32 / 100.0;
    let w = min(
        960,
        max(180, (data.width as f32 * zoom).round() as i32 + 24),
    );
    let h = min(
        960,
        max(
            120,
            (data.height as f32 * zoom).round() as i32 + STICKER_BAR_H + 24,
        ),
    );
    let mut x = if settings.sticker_x >= 0 {
        settings.sticker_x
    } else {
        pt.x + 16
    };
    let mut y = if settings.sticker_y >= 0 {
        settings.sticker_y
    } else {
        pt.y + 16
    };
    let work = platform_monitor::nearest_work_rect_for_point(POINT { x, y });
    x = x.clamp(work.left, (work.right - w).max(work.left));
    y = y.clamp(work.top, (work.bottom - h).max(work.top));
    let hwnd = platform_window::create_window_ex(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
        to_wide(STICKER_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_POPUP | WS_VISIBLE | WS_THICKFRAME,
        x,
        y,
        w,
        h,
        null_mut(),
        null_mut(),
        platform_window::module_handle(),
        Box::into_raw(data) as _,
    );
    if !hwnd.is_null() {
        platform_window::show(hwnd);
        platform_gdi::invalidate_rect(hwnd, null(), 0);
        let hwnd_raw = hwnd as isize;
        let item = item.clone();
        std::thread::spawn(move || {
            let payload = Box::new(StickerImageResult {
                image: ensure_item_image_bytes(&item),
            });
            unsafe {
                let _ = post_boxed_message(hwnd_raw, WM_STICKER_IMAGE_READY, 0, payload);
            }
        });
    }
}
