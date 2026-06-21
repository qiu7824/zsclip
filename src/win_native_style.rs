use crate::app_core::{
    Color, ColorRole, NativeControlFamily, NativeControlMapper, NativeStyleResolver,
    SemanticTextStyle, SettingsComponentKind, TextRole, TextStyle,
};
use crate::platform::appearance;

pub fn ui_text_font_family() -> &'static str {
    appearance::system_ui_text_font_family()
}

pub fn ui_display_font_family() -> &'static str {
    appearance::system_ui_text_font_family()
}

pub fn ui_icon_font_family() -> &'static str {
    "Segoe MDL2 Assets"
}

#[derive(Clone, Copy)]
pub struct Theme {
    pub accent: u32,
    pub accent_hover: u32,
    pub accent_pressed: u32,
    pub bg: u32,
    pub nav_bg: u32,
    pub nav_sel_fill: u32,
    pub surface: u32,
    pub surface2: u32,
    pub stroke: u32,
    pub text: u32,
    pub text_muted: u32,
    pub item_hover: u32,
    pub item_selected: u32,
    pub control_bg: u32,
    pub control_stroke: u32,
    pub button_bg: u32,
    pub button_hover: u32,
    pub button_pressed: u32,
    pub close_hover: u32,
}

impl Default for Theme {
    fn default() -> Self {
        let accent = appearance::system_accent();
        let dark = appearance::is_dark_mode();
        let accent_r = (accent & 0xFF) as i32;
        let accent_g = ((accent >> 8) & 0xFF) as i32;
        let accent_b = ((accent >> 16) & 0xFF) as i32;
        let accent_hover = rgb(
            ((accent_r as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
            ((accent_g as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
            ((accent_b as f32 * 0.9 + 255.0 * 0.1) as i32).min(255) as u8,
        );
        let accent_pressed = rgb(
            ((accent_r as f32 * 0.82) as i32).min(255) as u8,
            ((accent_g as f32 * 0.82) as i32).min(255) as u8,
            ((accent_b as f32 * 0.82) as i32).min(255) as u8,
        );

        if dark {
            Self {
                accent,
                accent_hover,
                accent_pressed,
                bg: rgb(32, 32, 32),
                nav_bg: rgb(40, 40, 40),
                nav_sel_fill: rgb(58, 58, 58),
                surface: rgb(44, 44, 44),
                surface2: rgb(50, 50, 50),
                stroke: rgb(60, 60, 60),
                text: rgb(255, 255, 255),
                text_muted: rgb(162, 162, 162),
                item_hover: rgb(54, 54, 54),
                item_selected: mix(accent, rgb(44, 44, 44), 0.75),
                control_bg: rgb(58, 58, 58),
                control_stroke: rgb(80, 80, 80),
                button_bg: rgb(58, 58, 58),
                button_hover: rgb(68, 68, 68),
                button_pressed: rgb(50, 50, 50),
                close_hover: rgb(196, 43, 28),
            }
        } else {
            Self {
                accent,
                accent_hover,
                accent_pressed,
                bg: rgb(243, 243, 243),
                nav_bg: rgb(243, 243, 243),
                nav_sel_fill: rgb(255, 255, 255),
                surface: rgb(255, 255, 255),
                surface2: rgb(250, 250, 250),
                stroke: rgb(229, 229, 229),
                text: rgb(28, 28, 28),
                text_muted: rgb(96, 96, 96),
                item_hover: rgb(249, 249, 249),
                item_selected: mix(accent, rgb(255, 255, 255), 0.85),
                control_bg: rgb(255, 255, 255),
                control_stroke: rgb(204, 204, 204),
                button_bg: rgb(255, 255, 255),
                button_hover: rgb(249, 249, 249),
                button_pressed: rgb(238, 238, 238),
                close_hover: rgb(196, 43, 28),
            }
        }
    }
}

pub fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

fn mix(a: u32, b: u32, t: f32) -> u32 {
    let ar = (a & 0xFF) as f32;
    let ag = ((a >> 8) & 0xFF) as f32;
    let ab = ((a >> 16) & 0xFF) as f32;
    let br = (b & 0xFF) as f32;
    let bg = ((b >> 8) & 0xFF) as f32;
    let bb = ((b >> 16) & 0xFF) as f32;
    rgb(
        (ar + (br - ar) * t).round() as u8,
        (ag + (bg - ag) * t).round() as u8,
        (ab + (bb - ab) * t).round() as u8,
    )
}

#[derive(Clone, Copy)]
pub(crate) struct WindowsNativeStyleResolver {
    theme: Theme,
    scale: f32,
}

impl WindowsNativeStyleResolver {
    pub(crate) fn new(theme: Theme, scale: f32) -> Self {
        Self {
            theme,
            scale: scale.max(0.1),
        }
    }

    fn font_family(self, role: TextRole) -> String {
        match role {
            TextRole::Monospace => "Consolas".to_string(),
            TextRole::Title => ui_display_font_family().to_string(),
            TextRole::Body | TextRole::Caption | TextRole::Button => {
                ui_text_font_family().to_string()
            }
        }
    }

    fn font_size(self, role: TextRole) -> f32 {
        let base = match role {
            TextRole::Title => 20.0,
            TextRole::Caption => 12.0,
            TextRole::Monospace => 13.0,
            TextRole::Body | TextRole::Button => 14.0,
        };
        base * self.scale
    }

    fn color(self, role: ColorRole) -> Color {
        colorref_to_core_color(match role {
            ColorRole::PrimaryText => self.theme.text,
            ColorRole::SecondaryText => self.theme.text_muted,
            ColorRole::Accent => self.theme.accent,
            ColorRole::Surface => self.theme.surface,
            ColorRole::Control => self.theme.control_bg,
            ColorRole::Danger => self.theme.close_hover,
        })
    }
}

impl Default for WindowsNativeStyleResolver {
    fn default() -> Self {
        Self::new(Theme::default(), 1.0)
    }
}

impl NativeStyleResolver for WindowsNativeStyleResolver {
    fn resolve_text_style(&self, style: SemanticTextStyle) -> TextStyle {
        TextStyle {
            font_family: self.font_family(style.role),
            size: self.font_size(style.role),
            weight: style.weight,
            color: self.color(style.color),
            horizontal_align: style.horizontal_align,
            vertical_align: style.vertical_align,
            wrap: style.wrap,
            ellipsis: style.ellipsis,
        }
    }
}

fn colorref_to_core_color(value: u32) -> Color {
    Color {
        r: (value & 0xFF) as u8,
        g: ((value >> 8) & 0xFF) as u8,
        b: ((value >> 16) & 0xFF) as u8,
        a: 255,
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct WindowsNativeControlMapper;

impl NativeControlMapper for WindowsNativeControlMapper {
    type ClassName = &'static str;

    fn class_name(&self, kind: SettingsComponentKind) -> Self::ClassName {
        match kind.family() {
            NativeControlFamily::StaticText => "STATIC",
            NativeControlFamily::TextInput => "EDIT",
            NativeControlFamily::Action => "BUTTON",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app_core::{
        ColorRole, HorizontalAlign, NativeControlMapper, NativeStyleResolver, SemanticTextStyle,
        TextRole, TextWeight, TextWrap, VerticalAlign,
    };

    fn test_theme() -> Theme {
        Theme {
            accent: rgb(10, 20, 30),
            accent_hover: rgb(11, 21, 31),
            accent_pressed: rgb(12, 22, 32),
            bg: rgb(1, 2, 3),
            nav_bg: rgb(4, 5, 6),
            nav_sel_fill: rgb(7, 8, 9),
            surface: rgb(40, 50, 60),
            surface2: rgb(41, 51, 61),
            stroke: rgb(42, 52, 62),
            text: rgb(70, 80, 90),
            text_muted: rgb(71, 81, 91),
            item_hover: rgb(72, 82, 92),
            item_selected: rgb(73, 83, 93),
            control_bg: rgb(100, 110, 120),
            control_stroke: rgb(101, 111, 121),
            button_bg: rgb(102, 112, 122),
            button_hover: rgb(103, 113, 123),
            button_pressed: rgb(104, 114, 124),
            close_hover: rgb(200, 10, 20),
        }
    }

    #[test]
    fn windows_native_style_resolver_maps_semantic_roles_to_host_theme() {
        let resolver = WindowsNativeStyleResolver::new(test_theme(), 2.0);
        let semantic = SemanticTextStyle {
            role: TextRole::Title,
            color: ColorRole::Accent,
            weight: TextWeight::Bold,
            horizontal_align: HorizontalAlign::Center,
            vertical_align: VerticalAlign::Start,
            wrap: TextWrap::Word,
            ellipsis: false,
        };

        let style = resolver.resolve_text_style(semantic);

        assert_eq!(style.font_family, ui_display_font_family());
        assert_eq!(style.size, 40.0);
        assert_eq!(style.weight, TextWeight::Bold);
        assert_eq!(
            style.color,
            Color {
                r: 10,
                g: 20,
                b: 30,
                a: 255
            }
        );
        assert_eq!(style.horizontal_align, HorizontalAlign::Center);
        assert_eq!(style.vertical_align, VerticalAlign::Start);
        assert_eq!(style.wrap, TextWrap::Word);
        assert!(!style.ellipsis);
    }

    #[test]
    fn windows_native_style_resolver_keeps_monospace_role_platform_local() {
        let resolver = WindowsNativeStyleResolver::new(test_theme(), 1.0);
        let style = resolver.resolve_text_style(SemanticTextStyle {
            role: TextRole::Monospace,
            color: ColorRole::SecondaryText,
            ..SemanticTextStyle::body()
        });

        assert_eq!(style.font_family, "Consolas");
        assert_eq!(style.size, 13.0);
        assert_eq!(
            style.color,
            Color {
                r: 71,
                g: 81,
                b: 91,
                a: 255
            }
        );
    }

    #[test]
    fn windows_native_control_mapper_keeps_win32_classes_platform_local() {
        let mapper = WindowsNativeControlMapper;

        assert_eq!(mapper.class_name(SettingsComponentKind::Label), "STATIC");
        assert_eq!(mapper.class_name(SettingsComponentKind::TextInput), "EDIT");
        assert_eq!(mapper.class_name(SettingsComponentKind::Toggle), "BUTTON");
        assert_eq!(mapper.class_name(SettingsComponentKind::Dropdown), "BUTTON");
        assert_eq!(mapper.class_name(SettingsComponentKind::Button), "BUTTON");
        assert_eq!(
            mapper.class_name(SettingsComponentKind::AccentButton),
            "BUTTON"
        );
    }

    #[test]
    fn windows_native_style_host_covers_required_operations() {
        let source = include_str!("win_native_style.rs");

        assert!(source.contains("impl NativeStyleResolver for WindowsNativeStyleResolver"));
        assert!(source.contains("fn resolve_text_style(&self, style: SemanticTextStyle)"));
        assert!(source.contains("impl NativeControlMapper for WindowsNativeControlMapper"));
        assert!(source.contains("fn class_name(&self, kind: SettingsComponentKind)"));
    }
}
