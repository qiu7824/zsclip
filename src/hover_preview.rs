use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use std::sync::OnceLock;

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::{
        BeginPaint, CreateSolidBrush, DeleteObject, EndPaint, FillRect, InvalidateRect,
        StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS, PAINTSTRUCT, SRCCOPY,
    },
    System::LibraryLoader::GetModuleHandleW,
    UI::WindowsAndMessaging::*,
};

use crate::{
    app::{ensure_item_image_bytes, post_boxed_message, ClipItem, ClipKind},
    i18n::tr,
    ui::{draw_round_rect, draw_text_block, draw_text_ex, rgba_to_bgra, Theme},
    win_system_ui::{apply_window_corner_preference, nearest_monitor_work_rect_for_point, to_wide},
};

const HOVER_PREVIEW_CLASS: &str = "ZsClipHoverPreview";
const PREVIEW_W_TEXT: i32 = 420;
const PREVIEW_H_TEXT: i32 = 220;
const PREVIEW_W_IMAGE: i32 = 520;
const PREVIEW_H_IMAGE: i32 = 360;
const WM_HOVER_IMAGE_READY: u32 = WM_APP + 41;

struct HoverPreviewImageResult {
    item_id: i64,
    image: Option<(Vec<u8>, usize, usize)>,
}

struct HoverPreviewData {
    item_id: i64,
    header: String,
    body: String,
    image: Option<(Vec<u8>, usize, usize)>,
    image_width: usize,
    image_height: usize,
    loading_item_id: i64,
    last_x: i32,
    last_y: i32,
    last_w: i32,
    last_h: i32,
}

static HOVER_HWND: OnceLock<isize> = OnceLock::new();

unsafe extern "system" fn preview_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, cs.lpCreateParams as isize);
            apply_window_corner_preference(hwnd);
            1
        }
        WM_PAINT => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HoverPreviewData;
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            if !hdc.is_null() && !ptr.is_null() {
                let th = Theme::default();
                let data = &*ptr;
                let mut rc: RECT = zeroed();
                GetClientRect(hwnd, &mut rc);
                let bg = CreateSolidBrush(th.surface);
                FillRect(hdc, &rc, bg);
                DeleteObject(bg as _);
                draw_round_rect(hdc as _, &rc, th.surface, th.stroke, 10);

                let header_rc = RECT {
                    left: 14,
                    top: 10,
                    right: rc.right - 14,
                    bottom: 34,
                };
                draw_text_ex(
                    hdc as _,
                    &data.header,
                    &header_rc,
                    th.text_muted,
                    12,
                    true,
                    false,
                    "Segoe UI Variable Text",
                );

                if let Some((bytes, width, height)) = &data.image {
                    let bgra = rgba_to_bgra(bytes);
                    let content = RECT {
                        left: 12,
                        top: 40,
                        right: rc.right - 12,
                        bottom: rc.bottom - 12,
                    };
                    let avail_w = (content.right - content.left).max(1);
                    let avail_h = (content.bottom - content.top).max(1);
                    let scale = (avail_w as f32 / *width as f32)
                        .min(avail_h as f32 / *height as f32)
                        .min(1.0);
                    let dw = ((*width as f32) * scale).max(1.0) as i32;
                    let dh = ((*height as f32) * scale).max(1.0) as i32;
                    let dx = content.left + (avail_w - dw) / 2;
                    let dy = content.top + (avail_h - dh) / 2;

                    let mut bmi: BITMAPINFO = zeroed();
                    bmi.bmiHeader.biSize = size_of::<BITMAPINFOHEADER>() as u32;
                    bmi.bmiHeader.biWidth = *width as i32;
                    bmi.bmiHeader.biHeight = -(*height as i32);
                    bmi.bmiHeader.biPlanes = 1;
                    bmi.bmiHeader.biBitCount = 32;
                    bmi.bmiHeader.biCompression = BI_RGB;

                    StretchDIBits(
                        hdc,
                        dx,
                        dy,
                        dw,
                        dh,
                        0,
                        0,
                        *width as i32,
                        *height as i32,
                        bgra.as_ptr() as _,
                        &bmi,
                        DIB_RGB_COLORS,
                        SRCCOPY,
                    );
                } else if !data.body.is_empty() {
                    let body_rc = RECT {
                        left: 14,
                        top: 42,
                        right: rc.right - 14,
                        bottom: rc.bottom - 14,
                    };
                    draw_text_block(hdc as _, &data.body, &body_rc, th.text, 12, false);
                } else {
                    let body_rc = RECT {
                        left: 14,
                        top: 42,
                        right: rc.right - 14,
                        bottom: rc.bottom - 14,
                    };
                    draw_text_block(
                        hdc as _,
                        tr("正在加载预览…", "Loading preview..."),
                        &body_rc,
                        th.text_muted,
                        12,
                        false,
                    );
                }
            }
            EndPaint(hwnd, &ps);
            0
        }
        WM_NCHITTEST => HTTRANSPARENT as LRESULT,
        WM_HOVER_IMAGE_READY => {
            let payload_ptr = lparam as *mut HoverPreviewImageResult;
            if payload_ptr.is_null() {
                return 0;
            }
            let payload = Box::from_raw(payload_ptr);
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HoverPreviewData;
            if !ptr.is_null() {
                let data = &mut *ptr;
                if data.item_id == payload.item_id {
                    data.image = payload.image;
                    data.loading_item_id = 0;
                    InvalidateRect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_NCDESTROY => {
            let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HoverPreviewData;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            }
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_preview_class() {
    let hinstance = GetModuleHandleW(null());
    let cname = to_wide(HOVER_PREVIEW_CLASS);
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(preview_wnd_proc);
    wc.hInstance = hinstance;
    wc.hCursor = LoadCursorW(null_mut(), IDC_ARROW);
    wc.hbrBackground = null_mut();
    wc.lpszClassName = cname.as_ptr();
    RegisterClassExW(&wc);
}

unsafe fn create_preview_window() -> HWND {
    ensure_preview_class();
    CreateWindowExW(
        WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
        to_wide(HOVER_PREVIEW_CLASS).as_ptr(),
        to_wide("").as_ptr(),
        WS_POPUP,
        0,
        0,
        PREVIEW_W_TEXT,
        PREVIEW_H_TEXT,
        null_mut(),
        null_mut(),
        GetModuleHandleW(null()),
        Box::into_raw(Box::new(HoverPreviewData {
            item_id: -1,
            header: String::new(),
            body: String::new(),
            image: None,
            image_width: 0,
            image_height: 0,
            loading_item_id: 0,
            last_x: i32::MIN,
            last_y: i32::MIN,
            last_w: 0,
            last_h: 0,
        })) as _,
    )
}

unsafe fn preview_hwnd() -> HWND {
    let raw = *HOVER_HWND.get_or_init(|| create_preview_window() as isize);
    raw as HWND
}

fn limit_preview_text(text: &str, max_lines: usize, max_chars: usize) -> String {
    let mut out = String::new();
    let mut chars = 0usize;
    let mut lines = 0usize;

    for line in text.lines() {
        if lines >= max_lines || chars >= max_chars {
            break;
        }
        let remaining = max_chars.saturating_sub(chars);
        let chunk: String = line.chars().take(remaining).collect();
        chars += chunk.chars().count();
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&chunk);
        lines += 1;
    }

    if out.is_empty() {
        return String::new();
    }
    if text.chars().count() > chars || text.lines().count() > lines {
        out.push_str("\n......");
    }
    out
}

fn limit_file_preview(paths: &[String], max_items: usize) -> String {
    let mut out = paths
        .iter()
        .take(max_items)
        .map(|path| {
            std::path::Path::new(path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or(path.as_str())
                .to_string()
        })
        .collect::<Vec<_>>()
        .join("\n");
    if paths.len() > max_items {
        out.push_str(&format!("\n......{} {}", tr("共", "Total"), paths.len()));
    }
    out
}

pub(crate) unsafe fn hide_hover_preview() {
    let hwnd = preview_hwnd();
    if !hwnd.is_null() && IsWindow(hwnd) != 0 {
        let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HoverPreviewData;
        if !ptr.is_null() {
            (*ptr).loading_item_id = 0;
        }
        ShowWindow(hwnd, SW_HIDE);
    }
}

fn spawn_hover_image_load(hwnd: HWND, item: ClipItem) {
    let hwnd_raw = hwnd as isize;
    std::thread::spawn(move || {
        let payload = Box::new(HoverPreviewImageResult {
            item_id: item.id,
            image: ensure_item_image_bytes(&item),
        });
        unsafe {
            let _ = post_boxed_message(hwnd_raw as HWND, WM_HOVER_IMAGE_READY, 0, payload);
        }
    });
}

pub(crate) unsafe fn show_hover_preview(item: &ClipItem, cursor_x: i32, cursor_y: i32) {
    let hwnd = preview_hwnd();
    if hwnd.is_null() || IsWindow(hwnd) == 0 {
        return;
    }
    let ptr = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut HoverPreviewData;
    if ptr.is_null() {
        return;
    }

    let header = match item.kind {
        ClipKind::Image => tr("图片预览", "Image Preview").to_string(),
        ClipKind::Files => tr("文件预览", "File Preview").to_string(),
        ClipKind::Phrase => tr("短语预览", "Phrase Preview").to_string(),
        ClipKind::Text => tr("文本预览", "Text Preview").to_string(),
    };
    let body = match item.kind {
        ClipKind::Text | ClipKind::Phrase => limit_preview_text(
            item.text.as_deref().unwrap_or(item.preview.as_str()),
            10,
            420,
        ),
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .map(|paths| limit_file_preview(paths, 8))
            .unwrap_or_else(|| item.preview.clone()),
        ClipKind::Image => String::new(),
    };
    let image_shape = if item.kind == ClipKind::Image {
        Some((item.image_width, item.image_height))
    } else {
        None
    };

    let (w, h) = if image_shape.is_some() {
        (PREVIEW_W_IMAGE, PREVIEW_H_IMAGE)
    } else {
        (PREVIEW_W_TEXT, PREVIEW_H_TEXT)
    };
    let wa = nearest_monitor_work_rect_for_point(POINT {
        x: cursor_x,
        y: cursor_y,
    });
    let mut x = cursor_x + 16;
    let mut y = cursor_y + 22;
    if x + w > wa.right {
        x = wa.right - w;
    }
    if y + h > wa.bottom {
        y = wa.bottom - h;
    }
    x = x.max(wa.left);
    y = y.max(wa.top);

    let data = &mut *ptr;
    let same_image_shape = image_shape == Some((data.image_width, data.image_height));
    let same_content =
        data.item_id == item.id && data.header == header && data.body == body && same_image_shape;
    let same_geometry =
        data.last_x == x && data.last_y == y && data.last_w == w && data.last_h == h;
    let visible = IsWindowVisible(hwnd) != 0;

    if visible && same_content && same_geometry {
        return;
    }

    if visible && same_content {
        data.last_x = x;
        data.last_y = y;
        data.last_w = w;
        data.last_h = h;
        SetWindowPos(
            hwnd,
            HWND_TOPMOST,
            x,
            y,
            w,
            h,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
        return;
    }

    let image = if item.kind == ClipKind::Image {
        if let Some(bytes) = item.image_bytes.as_ref() {
            Some((bytes.clone(), item.image_width, item.image_height))
        } else {
            if data.loading_item_id != item.id {
                data.loading_item_id = item.id;
                spawn_hover_image_load(hwnd, item.clone());
            }
            None
        }
    } else {
        data.loading_item_id = 0;
        None
    };

    data.item_id = item.id;
    data.header = header;
    data.body = body;
    data.image = image;
    data.image_width = image_shape.map(|shape| shape.0).unwrap_or(0);
    data.image_height = image_shape.map(|shape| shape.1).unwrap_or(0);
    data.last_x = x;
    data.last_y = y;
    data.last_w = w;
    data.last_h = h;

    SetWindowPos(
        hwnd,
        HWND_TOPMOST,
        x,
        y,
        w,
        h,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
    if !same_content {
        InvalidateRect(hwnd, null(), 0);
    }
}
