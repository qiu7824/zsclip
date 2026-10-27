//! Shared native button and choice painting for compact auxiliary windows.
use super::prelude::*;
use windows_sys::Win32::UI::Controls::{
    ODS_DISABLED, ODS_FOCUS, ODS_SELECTED, ODT_COMBOBOX, ODT_LISTBOX,
};

pub(super) unsafe fn draw_item(
    item: &DRAWITEMSTRUCT,
    primary: bool,
    checked: Option<bool>,
) -> LRESULT {
    let theme = Theme::default();
    let dc = item.hDC;
    let rect = item.rcItem;
    let dpi = platform_dpi::layout_dpi_for_window(item.hwndItem).max(96) as i32;
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
    } else if item.CtlType == ODT_COMBOBOX || item.CtlType == ODT_LISTBOX {
        let brush = platform_gdi::create_solid_brush(if pressed {
            theme.item_selected
        } else {
            if item.CtlType == ODT_LISTBOX {
                theme.surface
            } else {
                theme.control_bg
            }
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
    draw_text_ex(
        dc,
        &text,
        &label,
        color,
        px(14),
        false,
        centered,
        ui_text_font_family(),
    );
    1
}
