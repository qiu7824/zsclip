use crate::app_core::{NativeControlFamily, NativeControlMapper, SettingsComponentKind};
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
    use crate::app_core::NativeControlMapper;

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
    fn windows_native_control_mapper_covers_required_operation() {
        let source = include_str!("win_native_style.rs");

        assert!(source.contains("impl NativeControlMapper for WindowsNativeControlMapper"));
        assert!(source.contains("fn class_name(&self, kind: SettingsComponentKind)"));
    }
}
