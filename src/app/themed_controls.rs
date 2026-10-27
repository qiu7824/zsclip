//! Shared native button and choice painting for compact auxiliary windows.
use super::prelude::*;
use windows_sys::Win32::Graphics::Gdi::DT_RIGHT;
use windows_sys::Win32::UI::Controls::{
    ODS_DISABLED, ODS_FOCUS, ODS_SELECTED, ODT_COMBOBOX, ODT_LISTBOX,
};

/// List item data: the low 16 bits name a Segoe MDL2 glyph drawn before the
/// label, and this flag mutes the label (hidden entries).
pub(super) const LIST_ITEM_MUTED: usize = 1 << 20;
/// Separates a list label from the muted badge drawn at the row's right edge;
/// a control character so typed names (notes titles) never split.
pub(super) const LIST_BADGE_SEPARATOR: char = '\u{1F}';

pub(super) unsafe fn draw_item(
    item: &DRAWITEMSTRUCT,
    primary: bool,
    checked: Option<bool>,
) -> LRESULT {
    let theme = Theme::default();
    let dc = item.hDC;
    let rect = item.rcItem;
    let dpi = platform_dpi::layout_dpi_for_window(item.hwndItem).max(96) as i32;
    let _font_dpi = crate::win_system_ui::PaintDpiGuard::new(dpi as u32);
    let px = |n: i32| (n * dpi + 48) / 96;
    let disabled = item.itemState & ODS_DISABLED != 0;
    let pressed = item.itemState & ODS_SELECTED != 0;
    let text = if item.CtlType == ODT_COMBOBOX || item.CtlType == ODT_LISTBOX {
        if item.itemID == u32::MAX {
            platform_window::text(item.hwndItem)
        } else {
            let length = platform_window::send_message(
                item.hwndItem,
                if item.CtlType == ODT_LISTBOX {
                    LB_GETTEXTLEN
                } else {
                    CB_GETLBTEXTLEN
                },
                item.itemID as usize,
                0,
            );
            if !(0..=4096).contains(&length) {
                String::new()
            } else {
                let mut buffer = vec![0u16; length as usize + 1];
                let copied = platform_window::send_message(
                    item.hwndItem,
                    if item.CtlType == ODT_LISTBOX {
                        LB_GETTEXT
                    } else {
                        CB_GETLBTEXT
                    },
                    item.itemID as usize,
                    buffer.as_mut_ptr() as isize,
                );
                String::from_utf16_lossy(&buffer[..copied.clamp(0, length) as usize])
            }
        }
    } else {
        platform_window::text(item.hwndItem)
    };
    let mut label = RECT {
        left: rect.left + px(9),
        right: rect.right - px(9),
        ..rect
    };
    let mut color = if disabled {
        theme.text_muted
    } else {
        theme.text
    };
    let mut centered = true;
    let mut text = text.as_str();
    if let Some(on) = checked {
        let background = platform_gdi::create_solid_brush(theme.bg);
        platform_gdi::fill_rect(dc, &rect, background);
        platform_gdi::delete_object(background as _);
        let top = rect.top + (rect.bottom - rect.top - px(20)) / 2;
        let track = RECT {
            left: rect.left,
            top,
            right: rect.left + px(36),
            bottom: top + px(20),
        };
        draw_round_rect(
            dc,
            &track,
            if on { theme.accent } else { theme.control_bg },
            theme.control_stroke,
            px(10),
        );
        let left = track.left + if on { px(19) } else { px(3) };
        let knob = RECT {
            left,
            top: top + px(3),
            right: left + px(14),
            bottom: top + px(17),
        };
        draw_round_fill(
            dc,
            &knob,
            if disabled {
                theme.text_muted
            } else {
                rgb(245, 245, 245)
            },
            px(7),
        );
        label.left = track.right + px(10);
        centered = false;
    } else if item.CtlType == ODT_LISTBOX {
        let brush = platform_gdi::create_solid_brush(theme.surface);
        platform_gdi::fill_rect(dc, &rect, brush);
        platform_gdi::delete_object(brush as _);
        if pressed && item.itemID != u32::MAX {
            // Same selected-row treatment as the settings group list.
            let pill = RECT {
                left: rect.left + px(3),
                top: rect.top + px(1),
                right: rect.right - px(3),
                bottom: rect.bottom - px(1),
            };
            draw_round_fill(dc, &pill, theme.item_selected, px(4));
            if !disabled {
                let half = ((pill.bottom - pill.top) / 2 - px(6)).max(px(4));
                let cy = (pill.top + pill.bottom) / 2;
                let bar = RECT {
                    left: pill.left + px(2),
                    top: cy - half,
                    right: pill.left + px(5),
                    bottom: cy + half,
                };
                draw_round_fill(dc, &bar, theme.accent, px(2));
            }
        }
        if item.itemData & LIST_ITEM_MUTED != 0 {
            color = theme.text_muted;
        }
        label.left = rect.left + px(14);
        label.right = rect.right - px(10);
        if let Some(glyph) = char::from_u32((item.itemData & 0xFFFF) as u32).filter(|g| *g != '\0') {
            let icon = RECT {
                left: label.left,
                right: label.left + px(18),
                ..rect
            };
            draw_text_ex(dc, &glyph.to_string(), &icon, theme.text_muted, 14, false, true,
                "Segoe MDL2 Assets");
            label.left = icon.right + px(8);
        }
        if let Some((name, badge)) = text.split_once(LIST_BADGE_SEPARATOR) {
            if !badge.is_empty() {
                let mut badge_rect = label;
                crate::win_system_ui::draw_translated_text_line(dc as _, badge, &mut badge_rect,
                    theme.text_muted, 12, 400, false, ui_text_font_family(),
                    crate::win_ui_render::TRANSPARENT, DT_RIGHT);
                let badge_w = badge.chars().map(|c| if c.is_ascii() { 7 } else { 12 }).sum::<i32>();
                label.right -= px(badge_w + 10);
            }
            text = name;
        }
        centered = false;
    } else if item.CtlType == ODT_COMBOBOX {
        let brush = platform_gdi::create_solid_brush(if pressed {
            theme.item_selected
        } else {
            theme.control_bg
        });
        platform_gdi::fill_rect(dc, &rect, brush);
        platform_gdi::delete_object(brush as _);
        centered = false;
    } else {
        let background = platform_gdi::create_solid_brush(theme.bg);
        platform_gdi::fill_rect(dc, &rect, background);
        platform_gdi::delete_object(background as _);
        let body = RECT {
            left: rect.left + 1,
            top: rect.top + 1,
            right: rect.right - 1,
            bottom: rect.bottom - 1,
        };
        let fill = if disabled {
            theme.surface2
        } else if primary {
            theme.accent
        } else if pressed {
            theme.button_pressed
        } else {
            theme.control_bg
        };
        let stroke = if item.itemState & ODS_FOCUS != 0 {
            theme.accent
        } else {
            theme.control_stroke
        };
        draw_round_rect(dc, &body, fill, stroke, px(5));
        if primary && !disabled {
            color = rgb(255, 255, 255);
        }
    }
    // Logical size: the font cache scales it for the DC, so px() here would
    // enlarge labels twice on high-DPI displays.
    draw_text_ex(
        dc,
        text,
        &label,
        color,
        14,
        false,
        centered,
        ui_text_font_family(),
    );
    1
}
