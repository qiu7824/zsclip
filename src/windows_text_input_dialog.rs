use crate::app_core::{
    KeyState as UiKeyState, NativeTextInputDialogHost, NativeTextInputDialogRequest, UiEvent,
};
use crate::i18n::translate;
use crate::platform::{
    appearance as platform_appearance, dpi as platform_dpi, gdi as platform_gdi, hotkey,
    input as platform_input, string::to_wide, ui_event as platform_ui_event,
    window as platform_window,
};
use crate::ui::draw_round_rect;
use crate::win_native_style::{rgb, ui_display_font_family, ui_text_font_family, Theme};
use crate::win_system_ui::{draw_text_wide_centered, get_ctrl_text_wide};
use crate::win_ui_render::{DT_LEFT, DT_SINGLELINE, DT_VCENTER};
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM},
    Graphics::Gdi::{DEFAULT_GUI_FONT, PAINTSTRUCT},
    UI::{
        Controls::{DRAWITEMSTRUCT, ODS_SELECTED},
        WindowsAndMessaging::*,
    },
};

const IDC_INPUT_EDIT: usize = 9001;
const IDC_INPUT_OK: usize = 9002;
const IDC_INPUT_CANCEL: usize = 9003;
const INPUT_DLG_CLASS: &str = "ZsClipInputDlg";
const EM_SETSEL: u32 = 0x00B1;

struct InputDialogData {
    result: Option<String>,
    initial: [u16; 256],
    title_w: Vec<u16>,
    label_w: Vec<u16>,
    ui_font: *mut core::ffi::c_void,
    surface_brush: *mut core::ffi::c_void,
    control_brush: *mut core::ffi::c_void,
}

unsafe fn refresh_theme(data: &mut InputDialogData) {
    if !data.surface_brush.is_null() {
        platform_gdi::delete_object(data.surface_brush as _);
    }
    if !data.control_brush.is_null() {
        platform_gdi::delete_object(data.control_brush as _);
    }
    let theme = Theme::default();
    data.surface_brush = platform_gdi::create_solid_brush(theme.surface) as _;
    data.control_brush = platform_gdi::create_solid_brush(theme.control_bg) as _;
}

unsafe fn dispatch_ui_event(hwnd: HWND, event: UiEvent) -> Option<LRESULT> {
    match event {
        UiEvent::ControlCommand { control_id, .. } => {
            let data_ptr = platform_window::user_data(hwnd) as *mut InputDialogData;
            if data_ptr.is_null() {
                return Some(0);
            }
            let data = &mut *data_ptr;
            if control_id as usize == IDC_INPUT_OK {
                let edit = platform_window::child(hwnd, IDC_INPUT_EDIT as i32);
                if !edit.is_null() {
                    let value = platform_window::text(edit).trim().to_string();
                    if !value.is_empty() {
                        data.result = Some(value);
                        platform_window::destroy(hwnd);
                    }
                }
            } else if control_id as usize == IDC_INPUT_CANCEL {
                platform_window::destroy(hwnd);
            }
            Some(0)
        }
        UiEvent::Key {
            code,
            state: UiKeyState::Down,
            ..
        } => {
            if hotkey::is_enter_vk(code) {
                platform_window::send_message(hwnd, WM_COMMAND, IDC_INPUT_OK, 0);
            } else if hotkey::is_escape_vk(code) {
                platform_window::destroy(hwnd);
            }
            Some(0)
        }
        UiEvent::ThemeChanged | UiEvent::SystemMetricsChanged => {
            let data_ptr = platform_window::user_data(hwnd) as *mut InputDialogData;
            if !data_ptr.is_null() {
                refresh_theme(&mut *data_ptr);
                platform_gdi::invalidate_rect(hwnd, null(), 1);
            }
            Some(0)
        }
        UiEvent::CloseRequested => {
            platform_window::destroy(hwnd);
            Some(0)
        }
        _ => None,
    }
}

pub(crate) fn input_dialog_host_event_from_message(
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> Option<UiEvent> {
    platform_ui_event::from_window_message(msg, wparam, lparam)
}

unsafe extern "system" fn input_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(event) = input_dialog_host_event_from_message(msg, wparam, lparam) {
        if let Some(result) = dispatch_ui_event(hwnd, event) {
            return result;
        }
    }

    match msg {
        WM_CREATE => {
            let create = &*(lparam as *const CREATESTRUCTW);
            let data = create.lpCreateParams as *mut InputDialogData;
            platform_window::set_user_data(hwnd, data as isize);

            let data = &mut *data;
            let module = platform_window::module_handle();
            data.ui_font = platform_gdi::create_font_w(
                -platform_dpi::scale_for_window(hwnd, 14),
                0,
                0,
                0,
                400,
                0,
                0,
                0,
                1,
                0,
                0,
                5,
                0,
                to_wide(ui_text_font_family()).as_ptr(),
            ) as _;
            refresh_theme(data);

            let label = platform_window::create_window_ex(
                0,
                to_wide("STATIC").as_ptr(),
                data.label_w.as_ptr(),
                WS_CHILD | WS_VISIBLE,
                20,
                58,
                320,
                22,
                hwnd,
                null_mut(),
                module,
                null(),
            );
            platform_window::send_message(label, WM_SETFONT, data.ui_font as usize, 1);

            let edit = platform_window::create_window_ex(
                WS_EX_CLIENTEDGE,
                to_wide("EDIT").as_ptr(),
                data.initial.as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                20,
                84,
                320,
                32,
                hwnd,
                IDC_INPUT_EDIT as _,
                module,
                null(),
            );
            platform_window::send_message(edit, WM_SETFONT, data.ui_font as usize, 1);
            platform_appearance::set_window_theme(edit, "Explorer");
            platform_window::send_message(edit, EM_SETSEL, 0, -1isize);
            platform_input::set_focus(edit);

            let cancel = platform_window::create_window_ex(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(translate("取消").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW as u32,
                148,
                132,
                88,
                30,
                hwnd,
                IDC_INPUT_CANCEL as _,
                module,
                null(),
            );
            platform_window::send_message(cancel, WM_SETFONT, data.ui_font as usize, 1);

            let save = platform_window::create_window_ex(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(translate("保存").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW as u32,
                248,
                132,
                88,
                30,
                hwnd,
                IDC_INPUT_OK as _,
                module,
                null(),
            );
            platform_window::send_message(save, WM_SETFONT, data.ui_font as usize, 1);
            platform_appearance::set_rounded_corners(hwnd);
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLORDLG => {
            let hdc = wparam as *mut core::ffi::c_void;
            let data = platform_window::user_data(hwnd) as *mut InputDialogData;
            let theme = Theme::default();
            platform_gdi::set_bk_mode(hdc, 1);
            platform_gdi::set_bk_color(hdc, theme.surface);
            platform_gdi::set_text_color(hdc, theme.text);
            if data.is_null() {
                0
            } else {
                (*data).surface_brush as isize
            }
        }
        WM_CTLCOLOREDIT => {
            let hdc = wparam as *mut core::ffi::c_void;
            let data = platform_window::user_data(hwnd) as *mut InputDialogData;
            let theme = Theme::default();
            platform_gdi::set_bk_color(hdc, theme.control_bg);
            platform_gdi::set_text_color(hdc, theme.text);
            if data.is_null() {
                0
            } else {
                (*data).control_brush as isize
            }
        }
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = zeroed();
            let hdc = platform_gdi::begin_paint(hwnd, &mut paint);
            let theme = Theme::default();
            let rect = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
            let background = platform_gdi::create_solid_brush(theme.surface);
            platform_gdi::fill_rect(hdc, &rect, background);
            platform_gdi::delete_object(background as _);

            let data = platform_window::user_data(hwnd) as *mut InputDialogData;
            if !data.is_null() {
                let title_rect = RECT {
                    left: 20,
                    top: 12,
                    right: rect.right - 20,
                    bottom: 46,
                };
                let title_font = platform_gdi::create_font_w(
                    -platform_dpi::scale_for_window(hwnd, 16),
                    0,
                    0,
                    0,
                    600,
                    0,
                    0,
                    0,
                    1,
                    0,
                    0,
                    5,
                    0,
                    to_wide(ui_display_font_family()).as_ptr(),
                );
                let old = platform_gdi::select_object(hdc, title_font);
                platform_gdi::set_bk_mode(hdc, 1);
                platform_gdi::set_text_color(hdc, theme.text);
                platform_gdi::draw_text(
                    hdc,
                    (*data).title_w.as_ptr(),
                    -1,
                    &title_rect as *const _ as *mut _,
                    DT_LEFT | DT_VCENTER | DT_SINGLELINE,
                );
                platform_gdi::select_object(hdc, old);
                platform_gdi::delete_object(title_font);
            }

            let separator = platform_gdi::create_solid_brush(theme.stroke);
            let separator_rect = RECT {
                left: 0,
                top: 47,
                right: rect.right,
                bottom: 48,
            };
            platform_gdi::fill_rect(hdc, &separator_rect, separator);
            platform_gdi::delete_object(separator as _);
            platform_gdi::end_paint(hwnd, &paint);
            0
        }
        WM_DRAWITEM => {
            let draw = &*(lparam as *const DRAWITEMSTRUCT);
            let theme = Theme::default();
            let pressed = draw.itemState & ODS_SELECTED != 0;
            let text = get_ctrl_text_wide(draw.hwndItem);
            let rect = RECT {
                left: draw.rcItem.left + 1,
                top: draw.rcItem.top + 1,
                right: draw.rcItem.right - 1,
                bottom: draw.rcItem.bottom - 1,
            };
            if draw.CtlID as usize == IDC_INPUT_OK {
                let fill = if pressed {
                    let red = (theme.accent & 0xFF) as i32;
                    let green = ((theme.accent >> 8) & 0xFF) as i32;
                    let blue = ((theme.accent >> 16) & 0xFF) as i32;
                    rgb(
                        (red - 18).max(0) as u8,
                        (green - 18).max(0) as u8,
                        (blue - 18).max(0) as u8,
                    )
                } else {
                    theme.accent
                };
                draw_round_rect(draw.hDC as _, &rect, fill, fill, 4);
                draw_text_wide_centered(
                    draw.hDC as _,
                    &text,
                    &rect,
                    rgb(255, 255, 255),
                    14,
                    "Segoe UI Variable Text",
                );
            } else {
                let fill = if pressed {
                    theme.button_pressed
                } else {
                    theme.button_bg
                };
                let border = if pressed {
                    rgb(180, 180, 180)
                } else {
                    rgb(196, 196, 196)
                };
                draw_round_rect(draw.hDC as _, &rect, fill, border, 4);
                draw_text_wide_centered(
                    draw.hDC as _,
                    &text,
                    &rect,
                    theme.text,
                    14,
                    "Segoe UI Variable Text",
                );
            }
            1
        }
        WM_NCDESTROY => {
            let data = platform_window::user_data(hwnd) as *mut InputDialogData;
            if !data.is_null() {
                if !(*data).ui_font.is_null()
                    && (*data).ui_font != platform_gdi::get_stock_object(DEFAULT_GUI_FONT)
                {
                    platform_gdi::delete_object((*data).ui_font as _);
                }
                if !(*data).surface_brush.is_null() {
                    platform_gdi::delete_object((*data).surface_brush as _);
                }
                if !(*data).control_brush.is_null() {
                    platform_gdi::delete_object((*data).control_brush as _);
                }
                platform_window::set_user_data(hwnd, 0);
            }
            0
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}

unsafe fn prompt_text(parent: HWND, title: &str, label: &str, initial: &str) -> Option<String> {
    let module = platform_window::module_handle();
    let class_name = to_wide(INPUT_DLG_CLASS);
    let mut window_class: WNDCLASSEXW = zeroed();
    window_class.cbSize = size_of::<WNDCLASSEXW>() as u32;
    window_class.lpfnWndProc = Some(input_dialog_proc);
    window_class.hInstance = module;
    window_class.lpszClassName = class_name.as_ptr();
    window_class.hbrBackground = null_mut();
    let _ = platform_window::register_class_ex(&window_class);

    let mut initial_buffer = [0u16; 256];
    let initial_wide: Vec<u16> = initial.encode_utf16().collect();
    let copy_len = initial_wide.len().min(initial_buffer.len() - 1);
    initial_buffer[..copy_len].copy_from_slice(&initial_wide[..copy_len]);

    let data = Box::new(InputDialogData {
        result: None,
        initial: initial_buffer,
        title_w: translate(title)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect(),
        label_w: translate(label)
            .encode_utf16()
            .chain(std::iter::once(0))
            .collect(),
        ui_font: null_mut(),
        surface_brush: null_mut(),
        control_brush: null_mut(),
    });
    let data_ptr = Box::into_raw(data);

    let (dialog_width, dialog_height) = (360, 180);
    let parent_rect = platform_window::window_rect(parent).unwrap_or_else(|| zeroed());
    let x = parent_rect.left + (parent_rect.right - parent_rect.left - dialog_width) / 2;
    let y = parent_rect.top + (parent_rect.bottom - parent_rect.top - dialog_height) / 2;
    let title_wide = to_wide(translate(title).as_ref());
    let hwnd = platform_window::create_window_ex(
        WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
        class_name.as_ptr(),
        title_wide.as_ptr(),
        WS_POPUP | WS_VISIBLE | WS_CLIPCHILDREN,
        x,
        y,
        dialog_width,
        dialog_height,
        parent,
        null_mut(),
        module,
        data_ptr as _,
    );
    if hwnd.is_null() {
        drop(Box::from_raw(data_ptr));
        return None;
    }

    platform_window::set_enabled(parent, false);
    let mut message: MSG = zeroed();
    loop {
        if platform_window::get_message(&mut message) == 0 {
            break;
        }
        if message.message == WM_KEYDOWN
            && (hotkey::is_enter_vk(message.wParam as u32)
                || hotkey::is_escape_vk(message.wParam as u32))
        {
            platform_window::send_message(hwnd, WM_KEYDOWN, message.wParam, message.lParam);
            continue;
        }
        if IsDialogMessageW(hwnd, &message) == 0 {
            platform_window::translate_message(&message);
            platform_window::dispatch_message(&message);
        }
        if !platform_window::exists(hwnd) {
            break;
        }
    }
    if platform_window::exists(hwnd) {
        platform_window::destroy(hwnd);
    }
    platform_window::set_enabled(parent, true);
    platform_window::set_foreground(parent);
    Box::from_raw(data_ptr).result
}

pub(crate) struct WindowsTextInputDialogHost;

impl WindowsTextInputDialogHost {
    pub(crate) const fn new() -> Self {
        Self
    }
}

impl NativeTextInputDialogHost for WindowsTextInputDialogHost {
    type Owner = HWND;

    fn prompt_text(
        &self,
        owner: Self::Owner,
        request: NativeTextInputDialogRequest<'_>,
    ) -> Option<String> {
        unsafe { prompt_text(owner, request.title, request.label, request.initial) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn windows_text_input_dialog_is_owned_by_dedicated_host_module() {
        let source = include_str!("windows_text_input_dialog.rs");

        assert!(source.contains("pub(crate) struct WindowsTextInputDialogHost"));
        assert!(source.contains("impl NativeTextInputDialogHost for WindowsTextInputDialogHost"));
        assert!(source.contains("unsafe extern \"system\" fn input_dialog_proc"));
        assert!(source.contains("fn input_dialog_host_event_from_message"));
    }
}
