use super::{Rect, Size};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Color {
    pub(crate) r: u8,
    pub(crate) g: u8,
    pub(crate) b: u8,
    pub(crate) a: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextWeight {
    Regular,
    Medium,
    Bold,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum HorizontalAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum VerticalAlign {
    Start,
    Center,
    End,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextWrap {
    NoWrap,
    Word,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextRole {
    Body,
    Caption,
    Title,
    Button,
    Monospace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ColorRole {
    PrimaryText,
    SecondaryText,
    Accent,
    Surface,
    Control,
    Danger,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SemanticTextStyle {
    pub(crate) role: TextRole,
    pub(crate) color: ColorRole,
    pub(crate) weight: TextWeight,
    pub(crate) horizontal_align: HorizontalAlign,
    pub(crate) vertical_align: VerticalAlign,
    pub(crate) wrap: TextWrap,
    pub(crate) ellipsis: bool,
}

impl SemanticTextStyle {
    pub(crate) fn body() -> Self {
        Self {
            role: TextRole::Body,
            color: ColorRole::PrimaryText,
            weight: TextWeight::Regular,
            horizontal_align: HorizontalAlign::Start,
            vertical_align: VerticalAlign::Center,
            wrap: TextWrap::NoWrap,
            ellipsis: true,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TextStyle {
    pub(crate) font_family: String,
    pub(crate) size: f32,
    pub(crate) weight: TextWeight,
    pub(crate) color: Color,
    pub(crate) horizontal_align: HorizontalAlign,
    pub(crate) vertical_align: VerticalAlign,
    pub(crate) wrap: TextWrap,
    pub(crate) ellipsis: bool,
}

impl TextStyle {
    pub(crate) fn line(font_family: impl Into<String>, size: f32, color: Color) -> Self {
        Self {
            font_family: font_family.into(),
            size,
            weight: TextWeight::Regular,
            color,
            horizontal_align: HorizontalAlign::Start,
            vertical_align: VerticalAlign::Center,
            wrap: TextWrap::NoWrap,
            ellipsis: true,
        }
    }
}

pub(crate) trait NativeStyleResolver {
    fn resolve_text_style(&self, style: SemanticTextStyle) -> TextStyle;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NativeStyleHostOperation {
    ResolveTextStyle,
}

impl NativeStyleHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::ResolveTextStyle => "resolve_text_style",
        }
    }
}

pub(crate) const REQUIRED_NATIVE_STYLE_HOST_OPERATIONS: [NativeStyleHostOperation; 1] =
    [NativeStyleHostOperation::ResolveTextStyle];

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct TextRun {
    pub(crate) text: String,
    pub(crate) bounds: Rect,
}

pub(crate) trait TextLayout {
    fn measure(&self, text: &str, style: &TextStyle, max_width: i32) -> Size;
    fn layout_runs(&self, text: &str, style: &TextStyle, bounds: Rect) -> Vec<TextRun>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum TextLayoutHostOperation {
    Measure,
    LayoutRuns,
}

impl TextLayoutHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::Measure => "measure",
            Self::LayoutRuns => "layout_runs",
        }
    }
}

pub(crate) const REQUIRED_TEXT_LAYOUT_HOST_OPERATIONS: [TextLayoutHostOperation; 2] = [
    TextLayoutHostOperation::Measure,
    TextLayoutHostOperation::LayoutRuns,
];

pub(crate) trait Renderer {
    fn fill_rect(&mut self, rect: Rect, color: Color);
    fn stroke_rect(&mut self, rect: Rect, color: Color, width: i32);
    fn draw_text(&mut self, run: &TextRun, style: &TextStyle);
    fn push_clip(&mut self, rect: Rect);
    fn pop_clip(&mut self);
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum RendererHostOperation {
    FillRect,
    StrokeRect,
    DrawText,
    PushClip,
    PopClip,
}

impl RendererHostOperation {
    pub(crate) const fn operation_name(self) -> &'static str {
        match self {
            Self::FillRect => "fill_rect",
            Self::StrokeRect => "stroke_rect",
            Self::DrawText => "draw_text",
            Self::PushClip => "push_clip",
            Self::PopClip => "pop_clip",
        }
    }
}

pub(crate) const REQUIRED_RENDERER_HOST_OPERATIONS: [RendererHostOperation; 5] = [
    RendererHostOperation::FillRect,
    RendererHostOperation::StrokeRect,
    RendererHostOperation::DrawText,
    RendererHostOperation::PushClip,
    RendererHostOperation::PopClip,
];
