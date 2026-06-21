use crate::app_core::{
    native_host_edit_text_button_specs, KeyState as UiKeyState, NativeDialogButtons,
    NativeDialogHost, NativeDialogLevel, NativeDialogResponse, NativeEditTextDialogHost,
    NativeEditTextDialogRequest, NativeEditTextDialogResult, NativeEditTextSaveHandler,
    NativeHostEditTextAction, Size as UiSize, UiEvent,
};
use crate::i18n::{tr, translate};
use crate::platform::{
    appearance as platform_appearance, dialog as platform_dialog, dpi as platform_dpi,
    gdi as platform_gdi, hotkey, input as platform_input, monitor as platform_monitor,
    string::to_wide, ui_event as platform_ui_event, window as platform_window,
};
use crate::ui::draw_round_rect;
use crate::win_native_style::{rgb, ui_text_font_family, Theme};
use crate::win_system_ui::{draw_text_wide_centered, get_ctrl_text_wide};
use std::cmp::max;
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

const IDC_EDIT_TEXTAREA: usize = 9010;
const IDC_EDIT_LINENO: usize = 9011;
const IDC_EDIT_SAVE: usize = 9012;
const IDC_EDIT_CANCEL: usize = 9013;
const EDIT_DLG_CLASS: &str = "ZsClipEditDlg";
const EN_CHANGE_CODE: u16 = 0x0300;
const EM_SETSEL: u32 = 0x00B1;
const EM_GETLINECOUNT: u32 = 0x00BA;
const EM_GETFIRSTVISIBLELINE: u32 = 0x00CE;
const EN_VSCROLL: u16 = 0x0602;

struct EditDialogData {
    saved: bool,
    dirty: bool,
    loading_text: bool,
    original_text: String,
    pending_save: Option<String>,
    close_after_save: bool,
    last_w: i32,
    last_h: i32,
    ui_font: *mut core::ffi::c_void,
    btn_font: *mut core::ffi::c_void,
    surface_brush: *mut core::ffi::c_void,
    control_brush: *mut core::ffi::c_void,
    gutter_brush: *mut core::ffi::c_void,
}

unsafe fn refresh_theme(data: &mut EditDialogData) {
    if !data.surface_brush.is_null() {
        platform_gdi::delete_object(data.surface_brush as _);
    }
    if !data.control_brush.is_null() {
        platform_gdi::delete_object(data.control_brush as _);
    }
    if !data.gutter_brush.is_null() {
        platform_gdi::delete_object(data.gutter_brush as _);
    }
    let theme = Theme::default();
    data.surface_brush = platform_gdi::create_solid_brush(theme.surface) as _;
    data.control_brush = platform_gdi::create_solid_brush(theme.control_bg) as _;
    data.gutter_brush = platform_gdi::create_solid_brush(if platform_appearance::is_dark_mode() {
        rgb(38, 42, 48)
    } else {
        rgb(246, 248, 250)
    }) as _;
}

unsafe fn sync_line_numbers(line_number: HWND, text_area: HWND) {
    let line_count = platform_window::send_message(text_area, EM_GETLINECOUNT, 0, 0) as i32;
    let first_visible =
        platform_window::send_message(text_area, EM_GETFIRSTVISIBLELINE, 0, 0) as i32;
    let rect = platform_window::client_rect(text_area).unwrap_or_else(|| zeroed());
    let visible_lines = (rect.bottom - rect.top) / 16 + 2;
    let end = (first_visible + visible_lines).min(line_count);
    let mut lines = String::new();
    for line in first_visible..end {
        lines.push_str(&format!("{}\r\n", line + 1));
    }
    platform_window::set_text(line_number, &lines);
}

unsafe fn current_text(hwnd: HWND) -> String {
    let edit = platform_window::child(hwnd, IDC_EDIT_TEXTAREA as i32);
    if edit.is_null() {
        return String::new();
    }
    platform_window::text(edit)
        .replace("\r\n", "\n")
        .replace('\r', "\n")
}

unsafe fn mark_dirty(hwnd: HWND, data: &mut EditDialogData) {
    if !data.loading_text {
        data.dirty = current_text(hwnd) != data.original_text;
    }
}

unsafe fn queue_save(hwnd: HWND, data: &mut EditDialogData, close_after_save: bool) {
    data.pending_save = Some(current_text(hwnd));
    data.close_after_save = close_after_save;
}

fn native_edit_text_save_declared() -> bool {
    native_host_edit_text_button_specs()
        .into_iter()
        .any(|spec| spec.action == NativeHostEditTextAction::Save)
}

fn native_edit_text_cancel_declared() -> bool {
    native_host_edit_text_button_specs()
        .into_iter()
        .any(|spec| spec.action == NativeHostEditTextAction::Cancel)
}

unsafe fn confirm_close(hwnd: HWND, data: &mut EditDialogData) -> bool {
    if !data.dirty {
        return true;
    }
    match platform_dialog::WindowsDialogHost::new().confirm(
        hwnd,
        tr("编辑记录", "Edit Record"),
        tr(
            "当前修改尚未保存，是否先保存？",
            "You have unsaved changes. Save before closing?",
        ),
        NativeDialogLevel::Warning,
        NativeDialogButtons::YesNoCancel,
    ) {
        NativeDialogResponse::Yes => {
            queue_save(hwnd, data, true);
            false
        }
        NativeDialogResponse::No => true,
        NativeDialogResponse::Cancel => false,
    }
}

unsafe fn dispatch_ui_event(hwnd: HWND, event: UiEvent) -> Option<LRESULT> {
    match event {
        UiEvent::ControlCommand {
            control_id,
            notification,
        } => {
            let data_ptr = platform_window::user_data(hwnd) as *mut EditDialogData;
            if data_ptr.is_null() {
                return Some(0);
            }
            let data = &mut *data_ptr;
            let control_id = control_id as usize;
            if control_id == IDC_EDIT_TEXTAREA && notification == EN_VSCROLL {
                let edit = platform_window::child(hwnd, IDC_EDIT_TEXTAREA as i32);
                let line_number = platform_window::child(hwnd, IDC_EDIT_LINENO as i32);
                sync_line_numbers(line_number, edit);
            }
            if control_id == IDC_EDIT_TEXTAREA && notification == EN_CHANGE_CODE {
                let edit = platform_window::child(hwnd, IDC_EDIT_TEXTAREA as i32);
                let line_number = platform_window::child(hwnd, IDC_EDIT_LINENO as i32);
                sync_line_numbers(line_number, edit);
                mark_dirty(hwnd, data);
            }
            if control_id == IDC_EDIT_SAVE && native_edit_text_save_declared() {
                queue_save(hwnd, data, true);
            } else if control_id == IDC_EDIT_CANCEL && native_edit_text_cancel_declared() {
                platform_window::close(hwnd);
            }
            Some(0)
        }
        UiEvent::WindowSize { size, .. } => {
            let width = size.width;
            let height = size.height;
            let gutter_width = 44;
            let edit_height = height - 56;
            let line_number = platform_window::child(hwnd, IDC_EDIT_LINENO as i32);
            let edit = platform_window::child(hwnd, IDC_EDIT_TEXTAREA as i32);
            let cancel = platform_window::child(hwnd, IDC_EDIT_CANCEL as i32);
            let save = platform_window::child(hwnd, IDC_EDIT_SAVE as i32);
            if !line_number.is_null() {
                platform_window::set_pos(
                    line_number,
                    null_mut(),
                    0,
                    0,
                    gutter_width,
                    edit_height,
                    SWP_NOMOVE | SWP_NOZORDER,
                );
            }
            if !edit.is_null() {
                platform_window::set_pos(
                    edit,
                    null_mut(),
                    gutter_width,
                    0,
                    width - gutter_width,
                    edit_height,
                    SWP_NOZORDER,
                );
            }
            if !cancel.is_null() {
                platform_window::set_pos(
                    cancel,
                    null_mut(),
                    width - 210,
                    height - 44,
                    90,
                    30,
                    SWP_NOZORDER,
                );
            }
            if !save.is_null() {
                platform_window::set_pos(
                    save,
                    null_mut(),
                    width - 110,
                    height - 44,
                    90,
                    30,
                    SWP_NOZORDER,
                );
            }
            let data = platform_window::user_data(hwnd) as *mut EditDialogData;
            if !data.is_null() {
                if let Some(rect) = platform_window::window_rect(hwnd) {
                    (*data).last_w = rect.right - rect.left;
                    (*data).last_h = rect.bottom - rect.top;
                }
            }
            Some(0)
        }
        UiEvent::Key {
            code,
            state: UiKeyState::Down,
            ..
        } => {
            if hotkey::is_escape_vk(code) {
                platform_window::close(hwnd);
            } else if code == 'S' as u32 && hotkey::control_pressed() {
                platform_window::send_message(hwnd, WM_COMMAND, IDC_EDIT_SAVE, 0);
            }
            Some(0)
        }
        UiEvent::ThemeChanged | UiEvent::SystemMetricsChanged => {
            let data = platform_window::user_data(hwnd) as *mut EditDialogData;
            if !data.is_null() {
                refresh_theme(&mut *data);
                platform_gdi::invalidate_rect(hwnd, null(), 1);
            }
            Some(0)
        }
        UiEvent::CloseRequested => {
            let data = platform_window::user_data(hwnd) as *mut EditDialogData;
            if data.is_null() || confirm_close(hwnd, &mut *data) {
                platform_window::destroy(hwnd);
            }
            Some(0)
        }
        _ => None,
    }
}

pub(crate) fn edit_dialog_host_event_from_message(
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> Option<UiEvent> {
    platform_ui_event::from_window_message(msg, wparam, lparam)
}

unsafe extern "system" fn edit_dialog_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if let Some(event) = edit_dialog_host_event_from_message(msg, wparam, lparam) {
        if let Some(result) = dispatch_ui_event(hwnd, event) {
            return result;
        }
    }

    match msg {
        WM_CREATE => {
            let create = &*(lparam as *const CREATESTRUCTW);
            let data = create.lpCreateParams as *mut EditDialogData;
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
            data.btn_font = platform_gdi::create_font_w(
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

            let rect = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
            let width = rect.right;
            let height = rect.bottom;
            let gutter_width = 44;
            let edit_height = height - 56;
            let line_number = platform_window::create_window_ex(
                0,
                to_wide("EDIT").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD | WS_VISIBLE | ES_MULTILINE as u32 | ES_READONLY as u32 | ES_RIGHT as u32,
                0,
                0,
                gutter_width,
                edit_height,
                hwnd,
                IDC_EDIT_LINENO as _,
                module,
                null(),
            );
            platform_window::send_message(line_number, WM_SETFONT, data.ui_font as usize, 1);

            let edit = platform_window::create_window_ex(
                0,
                to_wide("EDIT").as_ptr(),
                to_wide("").as_ptr(),
                WS_CHILD
                    | WS_VISIBLE
                    | WS_VSCROLL
                    | WS_TABSTOP
                    | ES_MULTILINE as u32
                    | ES_AUTOVSCROLL as u32
                    | ES_WANTRETURN as u32
                    | ES_NOHIDESEL as u32,
                gutter_width,
                0,
                width - gutter_width,
                edit_height,
                hwnd,
                IDC_EDIT_TEXTAREA as _,
                module,
                null(),
            );
            platform_window::send_message(edit, WM_SETFONT, data.ui_font as usize, 1);
            platform_appearance::set_window_theme(edit, "Explorer");
            data.loading_text = true;
            platform_window::set_text(edit, &data.original_text.replace('\n', "\r\n"));
            data.loading_text = false;
            data.dirty = false;
            platform_window::send_message(edit, EM_SETSEL, 0, 0);
            platform_input::set_focus(edit);

            let cancel = platform_window::create_window_ex(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(translate("取消").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW as u32,
                width - 210,
                height - 44,
                90,
                30,
                hwnd,
                IDC_EDIT_CANCEL as _,
                module,
                null(),
            );
            platform_window::send_message(cancel, WM_SETFONT, data.btn_font as usize, 1);
            let save = platform_window::create_window_ex(
                0,
                to_wide("BUTTON").as_ptr(),
                to_wide(translate("保存").as_ref()).as_ptr(),
                WS_CHILD | WS_VISIBLE | WS_TABSTOP | BS_OWNERDRAW as u32,
                width - 110,
                height - 44,
                90,
                30,
                hwnd,
                IDC_EDIT_SAVE as _,
                module,
                null(),
            );
            platform_window::send_message(save, WM_SETFONT, data.btn_font as usize, 1);
            platform_appearance::set_rounded_corners(hwnd);
            sync_line_numbers(line_number, edit);
            0
        }
        WM_PAINT => {
            let mut paint: PAINTSTRUCT = zeroed();
            let hdc = platform_gdi::begin_paint(hwnd, &mut paint);
            let theme = Theme::default();
            let rect = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
            let background = platform_gdi::create_solid_brush(theme.surface);
            platform_gdi::fill_rect(hdc, &rect, background);
            platform_gdi::delete_object(background as _);
            let separator = platform_gdi::create_solid_brush(theme.stroke);
            let separator_rect = RECT {
                left: 0,
                top: rect.bottom - 56,
                right: rect.right,
                bottom: rect.bottom - 55,
            };
            platform_gdi::fill_rect(hdc, &separator_rect, separator);
            platform_gdi::delete_object(separator as _);
            platform_gdi::end_paint(hwnd, &paint);
            0
        }
        WM_CTLCOLORSTATIC => {
            let hdc = wparam as *mut core::ffi::c_void;
            let data = platform_window::user_data(hwnd) as *mut EditDialogData;
            let theme = Theme::default();
            platform_gdi::set_bk_mode(hdc, 1);
            platform_gdi::set_bk_color(hdc, theme.surface);
            platform_gdi::set_text_color(hdc, rgb(140, 148, 160));
            if data.is_null() {
                0
            } else {
                (*data).surface_brush as isize
            }
        }
        WM_CTLCOLOREDIT => {
            let hdc = wparam as *mut core::ffi::c_void;
            let child = lparam as HWND;
            let data = platform_window::user_data(hwnd) as *mut EditDialogData;
            let theme = Theme::default();
            if data.is_null() {
                return 0;
            }
            if GetDlgCtrlID(child) as usize == IDC_EDIT_LINENO {
                platform_gdi::set_bk_color(
                    hdc,
                    if platform_appearance::is_dark_mode() {
                        rgb(38, 42, 48)
                    } else {
                        rgb(246, 248, 250)
                    },
                );
                platform_gdi::set_text_color(hdc, rgb(140, 148, 160));
                (*data).gutter_brush as isize
            } else {
                platform_gdi::set_bk_color(hdc, theme.control_bg);
                platform_gdi::set_text_color(hdc, theme.text);
                (*data).control_brush as isize
            }
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
            if draw.CtlID as usize == IDC_EDIT_SAVE {
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
        WM_VSCROLL => {
            let edit = platform_window::child(hwnd, IDC_EDIT_TEXTAREA as i32);
            let line_number = platform_window::child(hwnd, IDC_EDIT_LINENO as i32);
            if !edit.is_null() && !line_number.is_null() {
                sync_line_numbers(line_number, edit);
            }
            platform_window::default_window_proc(hwnd, msg, wparam, lparam)
        }
        WM_NCDESTROY => {
            let data = platform_window::user_data(hwnd) as *mut EditDialogData;
            if !data.is_null() {
                if !(*data).ui_font.is_null()
                    && (*data).ui_font != platform_gdi::get_stock_object(DEFAULT_GUI_FONT)
                {
                    platform_gdi::delete_object((*data).ui_font as _);
                }
                if !(*data).btn_font.is_null()
                    && (*data).btn_font != platform_gdi::get_stock_object(DEFAULT_GUI_FONT)
                {
                    platform_gdi::delete_object((*data).btn_font as _);
                }
                if !(*data).surface_brush.is_null() {
                    platform_gdi::delete_object((*data).surface_brush as _);
                }
                if !(*data).control_brush.is_null() {
                    platform_gdi::delete_object((*data).control_brush as _);
                }
                if !(*data).gutter_brush.is_null() {
                    platform_gdi::delete_object((*data).gutter_brush as _);
                }
                platform_window::set_user_data(hwnd, 0);
            }
            0
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}

unsafe fn process_pending_save(
    hwnd: HWND,
    data: &mut EditDialogData,
    save_handler: &mut dyn NativeEditTextSaveHandler,
) {
    let Some(text) = data.pending_save.take() else {
        return;
    };
    match save_handler.save_text(&text) {
        Ok(()) => {
            data.saved = true;
            data.dirty = false;
            data.original_text = text;
            if data.close_after_save {
                platform_window::close(hwnd);
            }
        }
        Err(message) => {
            data.close_after_save = false;
            platform_dialog::WindowsDialogHost::new().show_message(
                hwnd,
                tr("编辑记录", "Edit Record"),
                if message.trim().is_empty() {
                    tr("保存失败，请稍后重试。", "Save failed. Please try again.")
                } else {
                    &message
                },
                NativeDialogLevel::Error,
            );
        }
    }
}

unsafe fn open_dialog(
    parent: HWND,
    request: NativeEditTextDialogRequest<'_>,
    save_handler: &mut dyn NativeEditTextSaveHandler,
) -> NativeEditTextDialogResult {
    let module = platform_window::module_handle();
    let class_name = to_wide(EDIT_DLG_CLASS);
    let mut window_class: WNDCLASSEXW = zeroed();
    window_class.cbSize = size_of::<WNDCLASSEXW>() as u32;
    window_class.lpfnWndProc = Some(edit_dialog_proc);
    window_class.hInstance = module;
    window_class.lpszClassName = class_name.as_ptr();
    window_class.hbrBackground = null_mut();
    window_class.style = CS_HREDRAW | CS_VREDRAW;
    let _ = platform_window::register_class_ex(&window_class);

    let parent_rect = platform_window::window_rect(parent).unwrap_or_else(|| zeroed());
    let parent_width = parent_rect.right - parent_rect.left;
    let parent_height = parent_rect.bottom - parent_rect.top;
    let work = platform_monitor::nearest_work_rect_for_window(parent);
    let margin = platform_dpi::scale_for_window(parent, 32);
    let max_width = (work.right - work.left - margin).max(640);
    let max_height = (work.bottom - work.top - margin).max(500);
    let width = request
        .initial_size
        .map(|size| size.width)
        .filter(|width| *width > 0)
        .unwrap_or_else(|| (parent_width * 3).max(640))
        .min(max_width);
    let height = request
        .initial_size
        .map(|size| size.height)
        .filter(|height| *height > 0)
        .unwrap_or_else(|| (parent_height * 4 / 3).max(500))
        .min(max_height);
    let x = max(work.left, work.left + (work.right - work.left - width) / 2);
    let y = max(work.top, work.top + (work.bottom - work.top - height) / 2);

    let data = Box::new(EditDialogData {
        saved: false,
        dirty: false,
        loading_text: false,
        original_text: request
            .initial_text
            .replace("\r\n", "\n")
            .replace('\r', "\n"),
        pending_save: None,
        close_after_save: false,
        last_w: width,
        last_h: height,
        ui_font: null_mut(),
        btn_font: null_mut(),
        surface_brush: null_mut(),
        control_brush: null_mut(),
        gutter_brush: null_mut(),
    });
    let data_ptr = Box::into_raw(data);
    let title = to_wide(request.title);
    let hwnd = platform_window::create_window_ex(
        WS_EX_DLGMODALFRAME | WS_EX_TOPMOST,
        class_name.as_ptr(),
        title.as_ptr(),
        WS_OVERLAPPEDWINDOW | WS_VISIBLE | WS_CLIPCHILDREN,
        x,
        y,
        width,
        height,
        parent,
        null_mut(),
        module,
        data_ptr as _,
    );
    if hwnd.is_null() {
        drop(Box::from_raw(data_ptr));
        return NativeEditTextDialogResult::default();
    }

    platform_window::set_enabled(parent, false);
    let mut message: MSG = zeroed();
    loop {
        let result = platform_window::get_message(&mut message);
        if result == 0 || result == -1 {
            break;
        }
        let root = platform_window::root_ancestor(message.hwnd);
        if root == hwnd
            && message.message == WM_KEYDOWN
            && message.wParam == 'S' as usize
            && hotkey::control_pressed()
        {
            platform_window::send_message(hwnd, WM_COMMAND, IDC_EDIT_SAVE, 0);
        } else if root == hwnd
            && message.message == WM_KEYDOWN
            && hotkey::is_escape_vk(message.wParam as u32)
        {
            platform_window::close(hwnd);
        } else if IsDialogMessageW(hwnd, &message) == 0 {
            platform_window::translate_message(&message);
            platform_window::dispatch_message(&message);
        }

        if platform_window::exists(hwnd) {
            process_pending_save(hwnd, &mut *data_ptr, save_handler);
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
    let data = Box::from_raw(data_ptr);
    NativeEditTextDialogResult {
        saved: data.saved,
        final_size: Some(UiSize {
            width: data.last_w,
            height: data.last_h,
        }),
    }
}

pub(crate) struct WindowsEditTextDialogHost;

impl WindowsEditTextDialogHost {
    pub(crate) const fn new() -> Self {
        Self
    }
}

impl NativeEditTextDialogHost for WindowsEditTextDialogHost {
    type Owner = HWND;

    fn open_edit_text(
        &self,
        owner: Self::Owner,
        request: NativeEditTextDialogRequest<'_>,
        save_handler: &mut dyn NativeEditTextSaveHandler,
    ) -> NativeEditTextDialogResult {
        unsafe { open_dialog(owner, request, save_handler) }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn windows_edit_text_dialog_is_owned_by_dedicated_host_module() {
        let source = include_str!("windows_edit_text_dialog.rs");
        let production = source
            .split("\n#[cfg(test)]\nmod tests")
            .next()
            .unwrap_or(source);

        assert!(production.contains("pub(crate) struct WindowsEditTextDialogHost"));
        assert!(production.contains("impl NativeEditTextDialogHost for WindowsEditTextDialogHost"));
        assert!(production.contains("unsafe extern \"system\" fn edit_dialog_proc"));
        assert!(production.contains("save_handler.save_text(&text)"));
        assert!(!production.contains("db_update_item_text"));
        assert!(!production.contains("with_db("));
        assert!(!production.contains("load_settings("));
    }
}
