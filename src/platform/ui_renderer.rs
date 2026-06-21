use crate::app_core::{
    Color, HorizontalAlign, Rect, Renderer, Size, TextLayout, TextRun, TextStyle, TextWeight,
    TextWrap, VerticalAlign,
};
use crate::platform::{gdi as platform_gdi, string::to_wide};
use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{HDC, HGDIOBJ},
};

const TRANSPARENT: i32 = 1;
const DT_LEFT: u32 = 0x0000;
const DT_CENTER: u32 = 0x0001;
const DT_RIGHT: u32 = 0x0002;
const DT_VCENTER: u32 = 0x0004;
const DT_BOTTOM: u32 = 0x0008;
const DT_WORDBREAK: u32 = 0x0010;
const DT_SINGLELINE: u32 = 0x0020;
const DT_CALCRECT: u32 = 0x0400;
const DT_NOPREFIX: u32 = 0x0800;
const DT_END_ELLIPSIS: u32 = 0x0000_8000;

pub(crate) struct GdiRenderer {
    dc: HDC,
    clip_stack: Vec<i32>,
}

impl GdiRenderer {
    #[allow(dead_code)]
    pub(crate) fn new(dc: HDC) -> Self {
        Self {
            dc,
            clip_stack: Vec::new(),
        }
    }
}

impl Drop for GdiRenderer {
    fn drop(&mut self) {
        while let Some(saved) = self.clip_stack.pop() {
            platform_gdi::restore_dc(self.dc, saved);
        }
    }
}

impl Renderer for GdiRenderer {
    fn fill_rect(&mut self, rect: Rect, color: Color) {
        let rect = to_win_rect(rect);
        let brush = platform_gdi::create_solid_brush(to_colorref(color));
        platform_gdi::fill_rect(self.dc, &rect, brush);
        platform_gdi::delete_object(brush as HGDIOBJ);
    }

    fn stroke_rect(&mut self, rect: Rect, color: Color, width: i32) {
        let mut rect = to_win_rect(rect);
        let brush = platform_gdi::create_solid_brush(to_colorref(color));
        for _ in 0..width.max(1) {
            if rect.right <= rect.left || rect.bottom <= rect.top {
                break;
            }
            platform_gdi::frame_rect(self.dc, &rect, brush);
            rect.left += 1;
            rect.top += 1;
            rect.right -= 1;
            rect.bottom -= 1;
        }
        platform_gdi::delete_object(brush as HGDIOBJ);
    }

    fn draw_text(&mut self, run: &TextRun, style: &TextStyle) {
        if run.text.is_empty() {
            return;
        }
        with_font(self.dc, style, |dc| {
            let mut rect = to_win_rect(run.bounds);
            platform_gdi::set_bk_mode(dc, TRANSPARENT);
            platform_gdi::set_text_color(dc, to_colorref(style.color));
            let text = to_wide(&run.text);
            platform_gdi::draw_text(dc, text.as_ptr(), -1, &mut rect, text_flags(style, false));
        });
    }

    fn push_clip(&mut self, rect: Rect) {
        let saved = platform_gdi::save_dc(self.dc);
        if saved != 0 {
            let rect = to_win_rect(rect);
            platform_gdi::intersect_clip_rect(
                self.dc,
                rect.left,
                rect.top,
                rect.right,
                rect.bottom,
            );
            self.clip_stack.push(saved);
        }
    }

    fn pop_clip(&mut self) {
        if let Some(saved) = self.clip_stack.pop() {
            platform_gdi::restore_dc(self.dc, saved);
        }
    }
}

pub(crate) struct GdiTextLayout {
    dc: HDC,
}

impl GdiTextLayout {
    #[allow(dead_code)]
    pub(crate) fn new(dc: HDC) -> Self {
        Self { dc }
    }
}

impl TextLayout for GdiTextLayout {
    fn measure(&self, text: &str, style: &TextStyle, max_width: i32) -> Size {
        if text.is_empty() {
            return Size {
                width: 0,
                height: 0,
            };
        }
        let mut measured = Size {
            width: 0,
            height: 0,
        };
        with_font(self.dc, style, |dc| {
            let mut rect = RECT {
                left: 0,
                top: 0,
                right: if max_width > 0 { max_width } else { 32_767 },
                bottom: 0,
            };
            let wide = to_wide(text);
            platform_gdi::draw_text(dc, wide.as_ptr(), -1, &mut rect, text_flags(style, true));
            measured = Size {
                width: (rect.right - rect.left).max(0),
                height: (rect.bottom - rect.top).max(0),
            };
        });
        measured
    }

    fn layout_runs(&self, text: &str, _style: &TextStyle, bounds: Rect) -> Vec<TextRun> {
        if text.is_empty() {
            Vec::new()
        } else {
            vec![TextRun {
                text: text.to_string(),
                bounds,
            }]
        }
    }
}

fn with_font<R>(dc: HDC, style: &TextStyle, f: impl FnOnce(HDC) -> R) -> R {
    let family = to_wide(&style.font_family);
    let font = platform_gdi::create_font_w(
        -style.size.round().max(1.0) as i32,
        0,
        0,
        0,
        font_weight(style.weight),
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        family.as_ptr(),
    );
    let old = if font.is_null() {
        core::ptr::null_mut()
    } else {
        platform_gdi::select_object(dc, font as HGDIOBJ)
    };
    let result = f(dc);
    if !old.is_null() {
        platform_gdi::select_object(dc, old);
    }
    if !font.is_null() {
        platform_gdi::delete_object(font as HGDIOBJ);
    }
    result
}

fn font_weight(weight: TextWeight) -> i32 {
    match weight {
        TextWeight::Regular => 400,
        TextWeight::Medium => 500,
        TextWeight::Bold => 700,
    }
}

fn text_flags(style: &TextStyle, measure: bool) -> u32 {
    let mut flags = DT_NOPREFIX;
    flags |= match style.horizontal_align {
        HorizontalAlign::Start => DT_LEFT,
        HorizontalAlign::Center => DT_CENTER,
        HorizontalAlign::End => DT_RIGHT,
    };
    flags |= match style.wrap {
        TextWrap::NoWrap => DT_SINGLELINE,
        TextWrap::Word => DT_WORDBREAK,
    };
    if style.wrap == TextWrap::NoWrap {
        flags |= match style.vertical_align {
            VerticalAlign::Start => 0,
            VerticalAlign::Center => DT_VCENTER,
            VerticalAlign::End => DT_BOTTOM,
        };
        if style.ellipsis {
            flags |= DT_END_ELLIPSIS;
        }
    }
    if measure {
        flags |= DT_CALCRECT;
    }
    flags
}

#[cfg(test)]
mod render_contract_tests {
    #[test]
    fn windows_gdi_renderer_covers_required_render_primitives() {
        let source = include_str!("ui_renderer.rs");

        assert!(source.contains("impl Renderer for GdiRenderer"));
        assert!(source.contains("fn fill_rect(&mut self, rect: Rect, color: Color)"));
        assert!(source.contains("fn stroke_rect(&mut self, rect: Rect, color: Color, width: i32)"));
        assert!(source.contains("fn draw_text(&mut self, run: &TextRun, style: &TextStyle)"));
        assert!(source.contains("fn push_clip(&mut self, rect: Rect)"));
        assert!(source.contains("fn pop_clip(&mut self)"));
        assert!(source.contains("impl TextLayout for GdiTextLayout"));
        assert!(source.contains("fn measure(&self, text: &str, style: &TextStyle, max_width: i32)"));
        assert!(
            source.contains("fn layout_runs(&self, text: &str, _style: &TextStyle, bounds: Rect)")
        );
    }
}

fn to_win_rect(rect: Rect) -> RECT {
    RECT {
        left: rect.x,
        top: rect.y,
        right: rect.x + rect.width.max(0),
        bottom: rect.y + rect.height.max(0),
    }
}

#[allow(dead_code)]
pub(crate) fn rect_from_win(rect: RECT) -> Rect {
    Rect {
        x: rect.left,
        y: rect.top,
        width: (rect.right - rect.left).max(0),
        height: (rect.bottom - rect.top).max(0),
    }
}

fn to_colorref(color: Color) -> u32 {
    (color.r as u32) | ((color.g as u32) << 8) | ((color.b as u32) << 16)
}

#[allow(dead_code)]
pub(crate) fn color_from_colorref(color: u32) -> Color {
    Color {
        r: (color & 0xff) as u8,
        g: ((color >> 8) & 0xff) as u8,
        b: ((color >> 16) & 0xff) as u8,
        a: 255,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> TextStyle {
        TextStyle::line(
            "Segoe UI",
            14.0,
            Color {
                r: 1,
                g: 2,
                b: 3,
                a: 255,
            },
        )
    }

    #[test]
    fn color_and_rect_conversion_match_gdi_contract() {
        assert_eq!(
            to_colorref(Color {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 0x44,
            }),
            0x0033_2211
        );
        let win = to_win_rect(Rect {
            x: 10,
            y: 20,
            width: 30,
            height: 40,
        });
        assert_eq!((win.left, win.top, win.right, win.bottom), (10, 20, 40, 60));
        assert_eq!(
            rect_from_win(win),
            Rect {
                x: 10,
                y: 20,
                width: 30,
                height: 40,
            }
        );
        assert_eq!(
            color_from_colorref(0x0033_2211),
            Color {
                r: 0x11,
                g: 0x22,
                b: 0x33,
                a: 255,
            }
        );
    }

    #[test]
    fn text_flags_follow_style_protocol() {
        let mut value = style();
        value.horizontal_align = HorizontalAlign::Center;
        value.vertical_align = VerticalAlign::Center;
        assert_eq!(
            text_flags(&value, false),
            DT_NOPREFIX | DT_CENTER | DT_SINGLELINE | DT_VCENTER | DT_END_ELLIPSIS
        );

        value.wrap = TextWrap::Word;
        value.ellipsis = false;
        assert_eq!(
            text_flags(&value, true),
            DT_NOPREFIX | DT_CENTER | DT_WORDBREAK | DT_CALCRECT
        );
    }

    #[test]
    fn font_weights_are_stable() {
        assert_eq!(font_weight(TextWeight::Regular), 400);
        assert_eq!(font_weight(TextWeight::Medium), 500);
        assert_eq!(font_weight(TextWeight::Bold), 700);
    }
}
