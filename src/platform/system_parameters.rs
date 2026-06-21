use std::sync::OnceLock;

use windows_sys::Win32::UI::WindowsAndMessaging::SystemParametersInfoW;

const SPI_GETMOUSEHOVERTIME: u32 = 0x0066;
const SPI_GETNONCLIENTMETRICS: u32 = 0x0029;

static SYSTEM_UI_FONT_FAMILY: OnceLock<String> = OnceLock::new();

#[repr(C)]
struct RawLogFontW {
    lf_height: i32,
    lf_width: i32,
    lf_escapement: i32,
    lf_orientation: i32,
    lf_weight: i32,
    lf_italic: u8,
    lf_underline: u8,
    lf_strike_out: u8,
    lf_char_set: u8,
    lf_out_precision: u8,
    lf_clip_precision: u8,
    lf_quality: u8,
    lf_pitch_and_family: u8,
    lf_face_name: [u16; 32],
}

#[repr(C)]
struct RawNonClientMetricsW {
    cb_size: u32,
    i_border_width: i32,
    i_scroll_width: i32,
    i_scroll_height: i32,
    i_caption_width: i32,
    i_caption_height: i32,
    lf_caption_font: RawLogFontW,
    i_sm_caption_width: i32,
    i_sm_caption_height: i32,
    lf_sm_caption_font: RawLogFontW,
    i_menu_width: i32,
    i_menu_height: i32,
    lf_menu_font: RawLogFontW,
    lf_status_font: RawLogFontW,
    lf_message_font: RawLogFontW,
    i_padded_border_width: i32,
}

const fn zeroed_logfont() -> RawLogFontW {
    RawLogFontW {
        lf_height: 0,
        lf_width: 0,
        lf_escapement: 0,
        lf_orientation: 0,
        lf_weight: 0,
        lf_italic: 0,
        lf_underline: 0,
        lf_strike_out: 0,
        lf_char_set: 0,
        lf_out_precision: 0,
        lf_clip_precision: 0,
        lf_quality: 0,
        lf_pitch_and_family: 0,
        lf_face_name: [0; 32],
    }
}

pub(crate) fn mouse_hover_time_ms() -> u32 {
    let mut hover_ms = 0u32;
    let ok = unsafe {
        SystemParametersInfoW(SPI_GETMOUSEHOVERTIME, 0, &mut hover_ms as *mut _ as _, 0) != 0
    };
    if ok && hover_ms > 0 {
        hover_ms
    } else {
        400
    }
}

pub(crate) fn system_ui_text_font_family() -> &'static str {
    SYSTEM_UI_FONT_FAMILY.get_or_init(|| {
        let mut metrics = RawNonClientMetricsW {
            cb_size: core::mem::size_of::<RawNonClientMetricsW>() as u32,
            i_border_width: 0,
            i_scroll_width: 0,
            i_scroll_height: 0,
            i_caption_width: 0,
            i_caption_height: 0,
            lf_caption_font: zeroed_logfont(),
            i_sm_caption_width: 0,
            i_sm_caption_height: 0,
            lf_sm_caption_font: zeroed_logfont(),
            i_menu_width: 0,
            i_menu_height: 0,
            lf_menu_font: zeroed_logfont(),
            lf_status_font: zeroed_logfont(),
            lf_message_font: zeroed_logfont(),
            i_padded_border_width: 0,
        };
        let ok = unsafe {
            SystemParametersInfoW(
                SPI_GETNONCLIENTMETRICS,
                metrics.cb_size,
                &mut metrics as *mut _ as _,
                0,
            ) != 0
        };
        if ok {
            let end = metrics
                .lf_message_font
                .lf_face_name
                .iter()
                .position(|ch| *ch == 0)
                .unwrap_or(metrics.lf_message_font.lf_face_name.len());
            let face = String::from_utf16_lossy(&metrics.lf_message_font.lf_face_name[..end])
                .trim()
                .to_string();
            if !face.is_empty() {
                return face;
            }
        }
        "Segoe UI".to_string()
    })
}
