use std::collections::HashMap;
use std::ffi::c_void;
use std::sync::{Mutex, OnceLock};

pub(crate) use crate::app_core::{
    settings_timer_task_for_id, SettingsComponentKind, SettingsTimerIds, SettingsTimerTask,
};
use crate::i18n::translate;
use crate::platform::gdi as platform_gdi;
use crate::platform::string::to_wide;
use crate::platform::system_parameters as platform_system_parameters;
pub use crate::settings_model::SettingsPage;
pub use crate::settings_render::{
    draw_settings_chrome, draw_settings_content, draw_settings_nav_item, draw_settings_scrollbar,
    draw_settings_viewport_mask,
};
pub use crate::settings_ui_host::{
    create_settings_button, create_settings_component, create_settings_dropdown_button,
    create_settings_edit, create_settings_fonts, create_settings_label, create_settings_label_auto,
    create_settings_listbox, create_settings_password_edit, create_settings_small_button,
    create_settings_toggle_plain, draw_settings_button_component, draw_settings_toggle_component,
    draw_text_wide_centered, get_ctrl_text_wide, set_settings_font, settings_child_visible,
    settings_dropdown_index_for_max_items, settings_dropdown_index_for_pos_mode,
    settings_dropdown_label_for_max_items, settings_dropdown_label_for_pos_mode,
    settings_dropdown_max_items_from_label, settings_dropdown_max_items_from_label_opt,
    settings_dropdown_max_items_labels, settings_dropdown_pos_mode_from_label,
    settings_viewport_mask_rect, settings_viewport_rect, SettingsCtrlReg, SettingsUiRegistry,
};
pub(crate) use crate::settings_ui_host::{
    set_settings_viewport_child_visible, settings_action_for_control, settings_command_for_control,
    settings_dropdown_popup_bounds, settings_host_control_at_point, settings_host_exists,
    settings_host_request_repaint, settings_host_screen_bounds, settings_host_set_bounds,
    settings_host_set_enabled, settings_host_set_text, settings_host_set_visible,
    settings_host_set_visible_enabled, settings_host_text, settings_page_to_sync_after_toggle,
    settings_viewport_child_control_bounds, settings_window_bounds, settings_window_client_bounds,
    settings_window_client_to_screen, settings_window_host_event_from_message,
    settings_window_layout_dpi, settings_window_request_area_repaint,
    settings_window_track_pointer_leave, sync_settings_viewport_child_bounds,
};
use crate::win_ui_render::{
    DT_CENTER, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_VCENTER, DT_WORDBREAK,
};
use windows_sys::Win32::{
    Foundation::RECT,
    Graphics::Gdi::{DEFAULT_GUI_FONT, LOGPIXELSY},
};

static SCALED_FONT_CACHE: OnceLock<Mutex<HashMap<(String, i32, i32, i32), isize>>> =
    OnceLock::new();
pub(crate) fn resolve_ui_font_family(family: &str) -> &str {
    match family {
        "" => system_ui_text_font_family(),
        "Segoe UI Variable Text" => system_ui_text_font_family(),
        "Segoe UI Variable Display" => system_ui_text_font_family(),
        "Segoe Fluent Icons" => "Segoe MDL2 Assets",
        other => other,
    }
}

thread_local! {
    static PAINT_DPI_OVERRIDE: std::cell::Cell<u32> = std::cell::Cell::new(0);
}

pub(crate) fn set_paint_dpi_override(dpi: u32) {
    PAINT_DPI_OVERRIDE.with(|cell| cell.set(dpi));
}

pub(crate) fn clear_paint_dpi_override() {
    PAINT_DPI_OVERRIDE.with(|cell| cell.set(0));
}

fn paint_dpi_override() -> u32 {
    PAINT_DPI_OVERRIDE.with(|cell| cell.get())
}

pub(crate) unsafe fn create_scaled_font_for_hdc(
    hdc: *mut c_void,
    family: &str,
    size: i32,
    weight: i32,
) -> *mut c_void {
    let font_name = resolve_ui_font_family(family);
    let hdc_dpi = platform_gdi::get_device_caps(hdc as _, LOGPIXELSY as i32).max(96) as u32;
    let override_dpi = paint_dpi_override();
    let dpi = if override_dpi > hdc_dpi {
        override_dpi as i32
    } else {
        hdc_dpi as i32
    };
    let scaled_size = ((size.max(1) * dpi) + 48) / 96;
    let key = (font_name.to_string(), dpi, scaled_size, weight);
    let cache = SCALED_FONT_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    if let Ok(mut fonts) = cache.lock() {
        if let Some(font) = fonts.get(&key) {
            return *font as _;
        }
        let font = platform_gdi::create_font_w(
            -scaled_size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            to_wide(font_name).as_ptr(),
        ) as isize;
        if font != 0 {
            fonts.insert(key, font);
        }
        font as _
    } else {
        platform_gdi::create_font_w(
            -scaled_size,
            0,
            0,
            0,
            weight,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            to_wide(font_name).as_ptr(),
        ) as _
    }
}

pub(crate) unsafe fn create_font_px(family: &str, pixel_size: i32, weight: i32) -> *mut c_void {
    platform_gdi::create_font_w(
        -pixel_size.max(1),
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        1,
        0,
        0,
        5,
        0,
        to_wide(resolve_ui_font_family(family)).as_ptr(),
    ) as _
}

pub(crate) unsafe fn draw_translated_text_line(
    hdc: *mut c_void,
    text: &str,
    rc: &mut RECT,
    color: u32,
    size: i32,
    weight: i32,
    center: bool,
    family: &str,
    transparent_mode: i32,
    flags_extra: u32,
) {
    let translated = translate(text);
    platform_gdi::set_bk_mode(hdc as _, transparent_mode);
    platform_gdi::set_text_color(hdc as _, color);
    let font = create_scaled_font_for_hdc(hdc, family, size, weight);
    let font = if font.is_null() {
        platform_gdi::get_stock_object(DEFAULT_GUI_FONT) as *mut c_void
    } else {
        font
    };
    let old = platform_gdi::select_object(hdc as _, font as _);
    let flags = (if center { DT_CENTER } else { DT_LEFT })
        | DT_VCENTER
        | DT_SINGLELINE
        | DT_END_ELLIPSIS
        | flags_extra;
    platform_gdi::draw_text(
        hdc as _,
        to_wide(translated.as_ref()).as_ptr(),
        -1,
        rc,
        flags,
    );
    platform_gdi::select_object(hdc as _, old);
}

pub(crate) unsafe fn draw_translated_text_block(
    hdc: *mut c_void,
    text: &str,
    rc: &mut RECT,
    color: u32,
    size: i32,
    weight: i32,
    family: &str,
    transparent_mode: i32,
    flags_extra: u32,
) {
    let translated = translate(text);
    platform_gdi::set_bk_mode(hdc as _, transparent_mode);
    platform_gdi::set_text_color(hdc as _, color);
    let font = create_scaled_font_for_hdc(hdc, family, size, weight);
    let font = if font.is_null() {
        platform_gdi::get_stock_object(DEFAULT_GUI_FONT) as *mut c_void
    } else {
        font
    };
    let old = platform_gdi::select_object(hdc as _, font as _);
    let flags = DT_LEFT | DT_WORDBREAK | DT_NOPREFIX | flags_extra;
    platform_gdi::draw_text(
        hdc as _,
        to_wide(translated.as_ref()).as_ptr(),
        -1,
        rc,
        flags,
    );
    platform_gdi::select_object(hdc as _, old);
}

pub(crate) fn system_ui_text_font_family() -> &'static str {
    platform_system_parameters::system_ui_text_font_family()
}

pub(crate) fn get_x_lparam(lp: isize) -> i32 {
    (lp as i16) as i32
}

pub(crate) fn get_y_lparam(lp: isize) -> i32 {
    ((lp >> 16) as i16) as i32
}
