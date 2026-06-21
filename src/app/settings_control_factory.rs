use super::prelude::*;
use crate::settings_model::SettingsToggleRowLayout;
use crate::win_system_ui::{
    create_settings_dropdown_button, create_settings_edit as host_create_settings_edit,
    create_settings_label as host_create_settings_label,
    create_settings_label_auto as host_create_settings_label_auto,
    create_settings_listbox as host_create_settings_listbox,
    create_settings_password_edit as host_create_settings_password_edit,
    create_settings_small_button, create_settings_toggle_plain,
};

pub(super) unsafe fn settings_create_label(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    host_create_settings_label(parent, text, x, y, w, h, font)
}

pub(super) unsafe fn settings_create_label_auto(
    parent: HWND,
    text: &str,
    x: i32,
    y: i32,
    w: i32,
    min_h: i32,
    font: *mut core::ffi::c_void,
) -> (HWND, i32) {
    host_create_settings_label_auto(parent, text, x, y, w, min_h, font)
}

pub(super) unsafe fn settings_create_edit(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    host_create_settings_edit(parent, text, id, x, y, w, font)
}

pub(super) unsafe fn settings_create_password_edit(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    host_create_settings_password_edit(parent, text, id, x, y, w, font)
}

pub(super) unsafe fn settings_create_listbox(
    parent: HWND,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    host_create_settings_listbox(parent, id, x, y, w, h, font)
}

pub(super) unsafe fn settings_create_small_btn(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    create_settings_small_button(parent, text, id, x, y, w, font)
}

pub(super) unsafe fn settings_create_dropdown_btn(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut core::ffi::c_void,
) -> HWND {
    create_settings_dropdown_button(parent, text, id, x, y, w, font)
}

pub(super) unsafe fn settings_create_toggle_plain(
    parent: HWND,
    text: &str,
    id: isize,
    x: i32,
    y: i32,
    w: i32,
    font: *mut core::ffi::c_void,
) -> (HWND, HWND, SettingsToggleRowLayout) {
    create_settings_toggle_plain(parent, text, id, x, y, w, font)
}
