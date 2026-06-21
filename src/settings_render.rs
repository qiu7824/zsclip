use windows_sys::Win32::Foundation::RECT;

use crate::platform::gdi as platform_gdi;
use crate::settings_model::{
    settings_chrome_paint_plan, settings_content_paint_plan, settings_nav_item_paint_plan,
    settings_scrollbar_paint_plan, settings_viewport_mask_paint_plan, SettingsChromeRenderPlan,
    SettingsContentRenderPlan, SettingsNavIconKind, SettingsNavItemRender, SettingsPaintCommand,
    SettingsPaintPlan, SettingsScrollbarRenderPlan, SettingsTextCommand, SettingsTextContent,
    SettingsTextFontRole, SettingsThemeRole,
};
use crate::ui::{draw_round_fill, draw_round_rect, draw_text_ex};
use crate::win_native_style::{rgb, Theme};

pub unsafe fn draw_settings_nav_item(
    hdc: *mut core::ffi::c_void,
    item: &SettingsNavItemRender,
    th: Theme,
) {
    let plan = settings_nav_item_paint_plan(item);
    draw_settings_paint_plan(hdc, plan, th);
}

unsafe fn draw_settings_paint_plan(
    hdc: *mut core::ffi::c_void,
    plan: SettingsPaintPlan,
    th: Theme,
) {
    for command in plan.paint_commands {
        draw_settings_paint_command(hdc, command, th);
    }
    for command in plan.text_commands {
        draw_settings_text_command(hdc, command, th);
    }
}

unsafe fn draw_settings_paint_command(
    hdc: *mut core::ffi::c_void,
    command: SettingsPaintCommand,
    th: Theme,
) {
    match command {
        SettingsPaintCommand::FillRect { rect, fill } => {
            let rc: RECT = rect.into();
            let br = platform_gdi::create_solid_brush(settings_theme_role_color(fill, th));
            platform_gdi::fill_rect(hdc, &rc, br);
            platform_gdi::delete_object(br as _);
        }
        SettingsPaintCommand::RoundRect {
            rect,
            fill,
            stroke,
            radius,
        } => {
            let rc: RECT = rect.into();
            draw_round_rect(
                hdc,
                &rc,
                settings_theme_role_color(fill, th),
                settings_theme_role_color(stroke, th),
                radius,
            );
        }
        SettingsPaintCommand::RoundFill { rect, fill, radius } => {
            let rc: RECT = rect.into();
            draw_round_fill(hdc, &rc, settings_theme_role_color(fill, th), radius);
        }
    }
}

unsafe fn draw_settings_text_command(
    hdc: *mut core::ffi::c_void,
    command: SettingsTextCommand,
    th: Theme,
) {
    let rc: RECT = command.rect.into();
    draw_text_ex(
        hdc,
        settings_text_content(command.content),
        &rc,
        settings_theme_role_color(command.color, th),
        command.size,
        command.bold,
        false,
        settings_text_font(command.font),
    );
}

fn settings_theme_role_color(role: SettingsThemeRole, th: Theme) -> u32 {
    match role {
        SettingsThemeRole::Background => th.bg,
        SettingsThemeRole::NavBackground => th.nav_bg,
        SettingsThemeRole::NavSelectedFill => th.nav_sel_fill,
        SettingsThemeRole::NavHoverFill => {
            if th.bg == rgb(32, 32, 32) {
                rgb(60, 60, 60)
            } else {
                rgb(237, 237, 237)
            }
        }
        SettingsThemeRole::Surface => th.surface,
        SettingsThemeRole::Accent => th.accent,
        SettingsThemeRole::Stroke => th.stroke,
        SettingsThemeRole::ScrollbarTrack => {
            if th.bg == rgb(32, 32, 32) {
                rgb(70, 70, 70)
            } else {
                rgb(200, 200, 200)
            }
        }
        SettingsThemeRole::ScrollbarThumb => {
            if th.bg == rgb(32, 32, 32) {
                rgb(120, 120, 120)
            } else {
                rgb(160, 160, 160)
            }
        }
        SettingsThemeRole::ScrollbarThumbDragging => th.accent,
        SettingsThemeRole::Text => th.text,
        SettingsThemeRole::TextMuted => th.text_muted,
        SettingsThemeRole::Danger => rgb(228, 60, 60),
    }
}

fn settings_text_content(content: SettingsTextContent) -> &'static str {
    match content {
        SettingsTextContent::Label(label) => label,
        SettingsTextContent::NavIcon(icon) => settings_nav_glyph(icon),
        SettingsTextContent::ChromeMenuIcon => "",
    }
}

fn settings_text_font(font: SettingsTextFontRole) -> &'static str {
    match font {
        SettingsTextFontRole::UiText => "Segoe UI Variable Text",
        SettingsTextFontRole::Display => "Segoe UI Variable Display",
        SettingsTextFontRole::FluentIcon => "Segoe Fluent Icons",
    }
}

pub unsafe fn draw_settings_chrome(
    hdc: *mut core::ffi::c_void,
    plan: &SettingsChromeRenderPlan,
    page_title: &'static str,
    th: Theme,
) {
    let paint_plan = settings_chrome_paint_plan(plan, page_title);
    draw_settings_paint_plan(hdc, paint_plan, th);
}

pub unsafe fn draw_settings_viewport_mask(
    hdc: *mut core::ffi::c_void,
    plan: &SettingsChromeRenderPlan,
    th: Theme,
) {
    let paint_plan = settings_viewport_mask_paint_plan(plan);
    draw_settings_paint_plan(hdc, paint_plan, th);
}

pub unsafe fn draw_settings_scrollbar(
    hdc: *mut core::ffi::c_void,
    plan: &SettingsScrollbarRenderPlan,
    th: Theme,
) {
    let paint_plan = settings_scrollbar_paint_plan(plan);
    draw_settings_paint_plan(hdc, paint_plan, th);
}

fn settings_nav_glyph(icon: SettingsNavIconKind) -> &'static str {
    match icon {
        SettingsNavIconKind::General => "",
        SettingsNavIconKind::Hotkey => "",
        SettingsNavIconKind::Plugin => "",
        SettingsNavIconKind::Group => "",
        SettingsNavIconKind::Sync => "",
        SettingsNavIconKind::About => "",
    }
}

pub unsafe fn draw_settings_content(
    hdc: *mut core::ffi::c_void,
    plan: &SettingsContentRenderPlan,
    th: Theme,
) {
    let paint_plan = settings_content_paint_plan(plan);
    draw_settings_paint_plan(hdc, paint_plan, th);
}

#[cfg(test)]
mod tests {
    #[test]
    fn settings_render_source_keeps_host_control_ids_out() {
        let source = include_str!("settings_render.rs");
        let forbidden = [
            format!("{}{}", "IDC_", "SET_"),
            format!("{}{}", "SETTINGS_", "CLASS"),
        ];
        for token in forbidden {
            assert!(!source.contains(&token), "{token}");
        }
    }
}
