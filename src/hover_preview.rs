use std::io::Read;
use std::mem::zeroed;
use std::ptr::{null, null_mut};
use std::sync::OnceLock;

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM},
    Graphics::Gdi::PAINTSTRUCT,
    UI::WindowsAndMessaging::*,
};

use crate::{
    app::{ensure_item_image_bytes, rich_text_preview_text, ClipItem, ClipKind},
    i18n::tr,
    platform::{
        appearance as platform_appearance, gdi as platform_gdi, monitor as platform_monitor,
        string::to_wide,
        window::{self as platform_window, post_boxed_message},
    },
    ui::{draw_round_rect, draw_text_block, draw_text_ex, rgba_to_opaque_bgra_on_bg},
    win_native_style::Theme,
};

const HOVER_PREVIEW_CLASS: &str = "ZsClipHoverPreview";
const PREVIEW_W_TEXT: i32 = 420;
const PREVIEW_H_TEXT: i32 = 220;
const PREVIEW_W_IMAGE: i32 = 520;
const PREVIEW_H_IMAGE: i32 = 360;
// 392x164 的正文区使用 12px 字体时约有 10 行空间，正文保留 9 行以容纳截断提示。
const PREVIEW_TEXT_MAX_LINES: usize = 9;
const PREVIEW_TEXT_MAX_CHARS: usize = 420;
const PREVIEW_FILE_MAX_ITEMS: usize = 8;
const MARKDOWN_PREVIEW_MAX_BYTES: u64 = 32 * 1024;
const WM_HOVER_IMAGE_READY: u32 = WM_APP + 41;

struct HoverPreviewImageResult {
    item_id: i64,
    app_data_generation: u64,
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

impl HoverPreviewData {
    fn release_cached_content(&mut self) {
        self.item_id = 0;
        self.header.clear();
        self.header.shrink_to_fit();
        self.body.clear();
        self.body.shrink_to_fit();
        self.image = None;
        self.image_width = 0;
        self.image_height = 0;
        self.loading_item_id = 0;
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PreviewUpdatePlan {
    KeepVisible,
    ShowCached,
    ReplaceContent,
}

fn preview_update_plan(
    visible: bool,
    same_content: bool,
    same_geometry: bool,
) -> PreviewUpdatePlan {
    if visible && same_content && same_geometry {
        PreviewUpdatePlan::KeepVisible
    } else if same_content {
        PreviewUpdatePlan::ShowCached
    } else {
        PreviewUpdatePlan::ReplaceContent
    }
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
            platform_window::set_user_data(hwnd, cs.lpCreateParams as isize);
            platform_appearance::set_rounded_corners(hwnd);
            1
        }
        WM_PAINT => {
            let ptr = platform_window::user_data(hwnd) as *mut HoverPreviewData;
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = platform_gdi::begin_paint(hwnd, &mut ps);
            if !hdc.is_null() && !ptr.is_null() {
                let th = Theme::default();
                let data = &*ptr;
                let rc = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
                let bg = platform_gdi::create_solid_brush(th.surface);
                platform_gdi::fill_rect(hdc, &rc, bg);
                platform_gdi::delete_object(bg as _);
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
                    let bgra = rgba_to_opaque_bgra_on_bg(bytes, th.surface);
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

                    platform_gdi::stretch_top_down_32bpp(
                        hdc,
                        dx,
                        dy,
                        dw,
                        dh,
                        *width as i32,
                        *height as i32,
                        &bgra,
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
            platform_gdi::end_paint(hwnd, &ps);
            0
        }
        WM_NCHITTEST => HTTRANSPARENT as LRESULT,
        WM_HOVER_IMAGE_READY => {
            let payload_ptr = lparam as *mut HoverPreviewImageResult;
            if payload_ptr.is_null() {
                return 0;
            }
            let payload = Box::from_raw(payload_ptr);
            let ptr = platform_window::user_data(hwnd) as *mut HoverPreviewData;
            if !ptr.is_null() {
                let data = &mut *ptr;
                if data.item_id == payload.item_id
                    && payload.app_data_generation
                        == crate::db_runtime::current_app_data_generation()
                {
                    data.image = payload.image;
                    data.loading_item_id = 0;
                    platform_gdi::invalidate_rect(hwnd, null(), 0);
                }
            }
            0
        }
        WM_NCDESTROY => {
            let ptr = platform_window::user_data(hwnd) as *mut HoverPreviewData;
            if !ptr.is_null() {
                drop(Box::from_raw(ptr));
                platform_window::set_user_data(hwnd, 0);
            }
            0
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}

unsafe fn ensure_preview_class() {
    let hinstance = platform_window::module_handle();
    let cname = to_wide(HOVER_PREVIEW_CLASS);
    let mut wc: WNDCLASSEXW = zeroed();
    wc.cbSize = size_of::<WNDCLASSEXW>() as u32;
    wc.lpfnWndProc = Some(preview_wnd_proc);
    wc.hInstance = hinstance;
    wc.hCursor = platform_window::arrow_cursor();
    wc.hbrBackground = null_mut();
    wc.lpszClassName = cname.as_ptr();
    platform_window::register_class_ex(&wc);
}

unsafe fn create_preview_window() -> HWND {
    ensure_preview_class();
    platform_window::create_window_ex(
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
        platform_window::module_handle(),
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
    let mut truncated = false;

    let mut source_lines = text.lines().peekable();
    while let Some(line) = source_lines.next() {
        if lines >= max_lines || chars >= max_chars {
            truncated = true;
            break;
        }
        let remaining = max_chars.saturating_sub(chars);
        let chunk: String = line.chars().take(remaining).collect();
        let chunk_chars = chunk.chars().count();
        chars += chunk_chars;
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&chunk);
        lines += 1;
        if chunk_chars < line.chars().count() || source_lines.peek().is_some() && lines >= max_lines
        {
            truncated = true;
            break;
        }
    }

    if out.is_empty() {
        return String::new();
    }
    if truncated {
        out.push_str(" ......");
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

fn markdown_file_preview_text(paths: &[String]) -> Option<String> {
    if paths.len() != 1 {
        return None;
    }
    let path = std::path::Path::new(&paths[0]);
    let ext = path.extension()?.to_str()?.to_ascii_lowercase();
    if !matches!(ext.as_str(), "md" | "markdown") {
        return None;
    }
    let mut file = std::fs::File::open(path).ok()?;
    let mut text = String::new();
    file.by_ref()
        .take(MARKDOWN_PREVIEW_MAX_BYTES)
        .read_to_string(&mut text)
        .ok()?;
    let preview = limit_preview_text(&text, PREVIEW_TEXT_MAX_LINES, PREVIEW_TEXT_MAX_CHARS);
    (!preview.is_empty()).then_some(preview)
}

pub(crate) unsafe fn hide_hover_preview() {
    let Some(raw) = HOVER_HWND.get() else {
        return;
    };
    let hwnd = *raw as HWND;
    if platform_window::exists(hwnd) {
        platform_window::hide(hwnd);
    }
}

pub(crate) unsafe fn release_hover_preview_memory() {
    let Some(raw) = HOVER_HWND.get() else {
        return;
    };
    let hwnd = *raw as HWND;
    if !platform_window::exists(hwnd) {
        return;
    }
    let ptr = platform_window::user_data(hwnd) as *mut HoverPreviewData;
    if !ptr.is_null() {
        (*ptr).release_cached_content();
    }
}

fn rect_contains_point(x: i32, y: i32, w: i32, h: i32, point_x: i32, point_y: i32) -> bool {
    point_x >= x && point_x < x + w && point_y >= y && point_y < y + h
}

fn preview_origin_near_cursor(
    cursor_x: i32,
    cursor_y: i32,
    width: i32,
    height: i32,
    work_area: RECT,
) -> (i32, i32) {
    const GAP_X: i32 = 16;
    const GAP_Y: i32 = 22;
    let candidates = [
        (cursor_x + GAP_X, cursor_y + GAP_Y),
        (cursor_x - GAP_X - width, cursor_y + GAP_Y),
        (cursor_x + GAP_X, cursor_y - GAP_Y - height),
        (cursor_x - GAP_X - width, cursor_y - GAP_Y - height),
    ];
    for (x, y) in candidates {
        if x >= work_area.left
            && y >= work_area.top
            && x + width <= work_area.right
            && y + height <= work_area.bottom
        {
            return (x, y);
        }
    }

    let max_x = (work_area.right - width).max(work_area.left);
    let max_y = (work_area.bottom - height).max(work_area.top);
    for (x, y) in candidates {
        let x = x.clamp(work_area.left, max_x);
        let y = y.clamp(work_area.top, max_y);
        if !rect_contains_point(x, y, width, height, cursor_x, cursor_y) {
            return (x, y);
        }
    }
    (
        (cursor_x + GAP_X).clamp(work_area.left, max_x),
        (cursor_y + GAP_Y).clamp(work_area.top, max_y),
    )
}

fn spawn_hover_image_load(hwnd: HWND, item: ClipItem) -> bool {
    let hwnd_raw = hwnd as isize;
    let generation = crate::db_runtime::current_app_data_generation();
    crate::image_preview_jobs::submit(move || {
        let image = std::panic::catch_unwind(|| {
            crate::db_runtime::with_shared_app_data_generation(generation, || {
                ensure_item_image_bytes(&item).and_then(|(bytes, width, height)| {
                    crate::app::data::build_image_thumbnail_rgba(&bytes, width, height, 1024)
                        .map(|image| (image.bytes, image.width, image.height))
                })
            })
            .flatten()
        })
        .ok()
        .flatten();
        let payload = Box::new(HoverPreviewImageResult {
            item_id: item.id,
            app_data_generation: generation,
            image,
        });
        unsafe {
            let _ = post_boxed_message(hwnd_raw, WM_HOVER_IMAGE_READY, 0, payload);
        }
    })
}

pub(crate) unsafe fn show_hover_preview(item: &ClipItem, cursor_x: i32, cursor_y: i32) {
    let hwnd = preview_hwnd();
    if !platform_window::exists(hwnd) {
        return;
    }
    let ptr = platform_window::user_data(hwnd) as *mut HoverPreviewData;
    if ptr.is_null() {
        return;
    }

    let markdown_file_preview = if item.kind == ClipKind::Files {
        item.file_paths
            .as_ref()
            .and_then(|paths| markdown_file_preview_text(paths))
    } else {
        None
    };
    let header = match item.kind {
        ClipKind::Image => tr("图片预览", "Image Preview").to_string(),
        ClipKind::Files if markdown_file_preview.is_some() => {
            tr("Markdown 预览", "Markdown Preview").to_string()
        }
        ClipKind::Files => tr("文件预览", "File Preview").to_string(),
        ClipKind::Phrase => tr("短语预览", "Phrase Preview").to_string(),
        ClipKind::Text if item.rich_text_html.is_some() => {
            tr("富文本预览", "Rich Text Preview").to_string()
        }
        ClipKind::Text => tr("文本预览", "Text Preview").to_string(),
    };
    let body = match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            if let Some(html) = item.rich_text_html.as_deref() {
                let text = rich_text_preview_text(
                    html,
                    item.text.as_deref().unwrap_or(item.preview.as_str()),
                    PREVIEW_TEXT_MAX_LINES + 1,
                    PREVIEW_TEXT_MAX_CHARS + 1,
                );
                limit_preview_text(&text, PREVIEW_TEXT_MAX_LINES, PREVIEW_TEXT_MAX_CHARS)
            } else {
                limit_preview_text(
                    item.text.as_deref().unwrap_or(item.preview.as_str()),
                    PREVIEW_TEXT_MAX_LINES,
                    PREVIEW_TEXT_MAX_CHARS,
                )
            }
        }
        ClipKind::Files => markdown_file_preview.unwrap_or_else(|| {
            item.file_paths
                .as_ref()
                .map(|paths| limit_file_preview(paths, PREVIEW_FILE_MAX_ITEMS))
                .unwrap_or_else(|| item.preview.clone())
        }),
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
    let wa = platform_monitor::nearest_work_rect_for_point(POINT {
        x: cursor_x,
        y: cursor_y,
    });
    let (x, y) = preview_origin_near_cursor(cursor_x, cursor_y, w, h, wa);

    let data = &mut *ptr;
    let same_image_shape = image_shape == Some((data.image_width, data.image_height));
    let same_content =
        data.item_id == item.id && data.header == header && data.body == body && same_image_shape;
    let same_geometry =
        data.last_x == x && data.last_y == y && data.last_w == w && data.last_h == h;
    let visible = platform_window::is_visible(hwnd);

    match preview_update_plan(visible, same_content, same_geometry) {
        PreviewUpdatePlan::KeepVisible => return,
        PreviewUpdatePlan::ShowCached => {
            data.last_x = x;
            data.last_y = y;
            data.last_w = w;
            data.last_h = h;
            platform_window::set_pos(
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
        PreviewUpdatePlan::ReplaceContent => {}
    }

    let image = if item.kind == ClipKind::Image {
        if data.loading_item_id != item.id {
            data.loading_item_id = item.id;
            if !spawn_hover_image_load(hwnd, crate::app::data::clip_item_to_summary(item)) {
                data.loading_item_id = 0;
            }
        }
        None
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

    platform_window::set_pos(
        hwnd,
        HWND_TOPMOST,
        x,
        y,
        w,
        h,
        SWP_NOACTIVATE | SWP_SHOWWINDOW,
    );
    if !same_content {
        platform_gdi::invalidate_rect(hwnd, null(), 0);
    }
}

#[cfg(test)]
mod tests {
    use super::{
        limit_preview_text, preview_origin_near_cursor, preview_update_plan, rect_contains_point,
        PreviewUpdatePlan, PREVIEW_TEXT_MAX_CHARS, PREVIEW_TEXT_MAX_LINES,
    };
    use windows_sys::Win32::Foundation::RECT;

    #[test]
    fn text_preview_capacity_matches_text_window() {
        let source = (1..=12)
            .map(|line| format!("line {line}"))
            .collect::<Vec<_>>()
            .join("\n");
        let preview = limit_preview_text(&source, PREVIEW_TEXT_MAX_LINES, PREVIEW_TEXT_MAX_CHARS);

        assert!(preview.contains("line 9"));
        assert!(!preview.contains("line 10"));
        assert!(preview.ends_with("......"));
    }

    #[test]
    fn preview_placement_keeps_stationary_cursor_outside_at_monitor_edges() {
        let work_area = RECT {
            left: 0,
            top: 0,
            right: 1920,
            bottom: 1080,
        };
        for (width, height) in [(420, 220), (520, 360)] {
            for (cursor_x, cursor_y) in [(0, 0), (1919, 0), (0, 1079), (1919, 1079)] {
                let (x, y) =
                    preview_origin_near_cursor(cursor_x, cursor_y, width, height, work_area);
                assert!(!rect_contains_point(
                    x, y, width, height, cursor_x, cursor_y
                ));
            }
        }
    }

    #[test]
    fn hidden_same_item_reuses_cached_preview_without_replacing_image() {
        assert_eq!(
            preview_update_plan(false, true, true),
            PreviewUpdatePlan::ShowCached
        );
        assert_eq!(
            preview_update_plan(true, true, true),
            PreviewUpdatePlan::KeepVisible
        );
        assert_eq!(
            preview_update_plan(false, false, true),
            PreviewUpdatePlan::ReplaceContent
        );
    }

    #[test]
    fn complete_multiline_preview_does_not_add_false_ellipsis() {
        let preview = limit_preview_text(
            "first line\nsecond line",
            PREVIEW_TEXT_MAX_LINES,
            PREVIEW_TEXT_MAX_CHARS,
        );

        assert_eq!(preview, "first line\nsecond line");
    }

    #[test]
    fn text_preview_uses_full_420_character_capacity() {
        let source = "测试abc123".repeat(80);
        let preview = limit_preview_text(&source, PREVIEW_TEXT_MAX_LINES, PREVIEW_TEXT_MAX_CHARS);
        let content = preview.strip_suffix(" ......").unwrap();

        assert_eq!(content.chars().count(), PREVIEW_TEXT_MAX_CHARS);
    }
}
