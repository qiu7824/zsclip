use crate::i18n::tr;
use crate::platform::{dpi, gdi, monitor, string::to_wide, window};
use crate::win_native_style::{ui_text_font_family, Theme};
use std::mem::{size_of, zeroed};
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{SetBkColor, SetBkMode, SetTextColor, TRANSPARENT};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::{
    EnableWindow, GetFocus, IsWindowEnabled, SetFocus,
};
use windows_sys::Win32::UI::WindowsAndMessaging::*;

const CLASS: &str = "ZsClipUpdateDialog";
const ID_ACCEPT: usize = 1;
const ID_CANCEL: usize = 2;
const ID_TEXT: usize = 10;
const ID_MESSAGE: usize = 11;
const ID_ERROR: usize = 12;
const EM_LIMITTEXT: u32 = 0x00c5;

pub(super) struct UpdateDialogRequest<'a> {
    pub(super) title: &'a str,
    pub(super) message: &'a str,
    pub(super) text: &'a str,
    pub(super) editable: bool,
    pub(super) accept_label: Option<&'a str>,
}

struct DialogData {
    pending_accept: bool,
    accepted: bool,
    editable: bool,
    has_accept: bool,
    font: *mut core::ffi::c_void,
    surface: *mut core::ffi::c_void,
    control: *mut core::ffi::c_void,
}

impl Drop for DialogData {
    fn drop(&mut self) {
        for object in [self.font, self.surface, self.control] {
            if !object.is_null() {
                gdi::delete_object(object as _);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EnterAction {
    Accept,
    Cancel,
    Edit,
    Ignore,
}

fn enter_action(focused_id: usize, editable: bool, has_accept: bool) -> EnterAction {
    match focused_id {
        ID_CANCEL => EnterAction::Cancel,
        ID_ACCEPT if has_accept => EnterAction::Accept,
        ID_TEXT if editable => EnterAction::Edit,
        _ => EnterAction::Ignore,
    }
}

fn normalized_source(value: &str) -> Result<crate::update_feed::UpdateSource, String> {
    if value.chars().count() > 4096 || value.len() > 4096 {
        return Err(tr(
            "更新源地址不能超过 4096 个字符。",
            "The update source must not exceed 4096 characters.",
        )
        .into());
    }
    let source = crate::update_feed::UpdateSource {
        manifest_url: value.trim().to_string(),
    };
    crate::update_feed::validate_source(&source)?;
    Ok(source)
}

pub(super) fn source_from_text(value: &str) -> Result<crate::update_feed::UpdateSource, String> {
    normalized_source(value)
}

pub(super) fn bounded_notes(notes: &str) -> String {
    let text = notes
        .chars()
        .filter(|ch| *ch != '\0')
        .take(16_000)
        .collect::<String>();
    if text.trim().is_empty() {
        tr(
            "此版本未提供更新说明。",
            "No release notes were provided for this version.",
        )
        .into()
    } else {
        text.replace("\r\n", "\n")
            .replace('\r', "\n")
            .replace('\n', "\r\n")
    }
}

unsafe fn layout(hwnd: HWND) {
    let Some(rc) = window::client_rect(hwnd) else {
        return;
    };
    let s = |value| dpi::scale_for_window(hwnd, value);
    let margin = s(20);
    let width = (rc.right - margin * 2).max(s(100));
    let data = window::user_data(hwnd) as *mut DialogData;
    let font = if data.is_null() {
        null_mut()
    } else {
        (*data).font
    };
    let message_h = crate::settings_ui_host::settings_measure_text_height(
        hwnd,
        &window::text(window::child(hwnd, ID_MESSAGE as i32)),
        width,
        font,
        s(44),
    );
    let button_y = rc.bottom - margin - s(32);
    let error_y = button_y - s(46);
    let edit_y = margin + message_h + s(8);
    for (id, x, y, w, h) in [
        (ID_MESSAGE, margin, margin, width, message_h),
        (
            ID_TEXT,
            margin,
            edit_y,
            width,
            (error_y - edit_y - s(8)).max(s(32)),
        ),
        (ID_ERROR, margin, error_y, width, s(40)),
        (
            ID_CANCEL,
            rc.right - margin - s(104),
            button_y,
            s(104),
            s(32),
        ),
        (
            ID_ACCEPT,
            rc.right - margin - s(272),
            button_y,
            s(156),
            s(32),
        ),
    ] {
        let child = window::child(hwnd, id as i32);
        if !child.is_null() {
            window::move_window(child, x, y, w, h, true);
        }
    }
}

unsafe extern "system" fn dialog_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if msg == WM_NCCREATE {
        let create = &*(lp as *const CREATESTRUCTW);
        window::set_user_data(hwnd, create.lpCreateParams as isize);
    }
    let data = window::user_data(hwnd) as *mut DialogData;
    match msg {
        WM_COMMAND => {
            let id = wp & 0xffff;
            if id == ID_CANCEL {
                window::destroy(hwnd);
            } else if id == ID_ACCEPT && !data.is_null() && (*data).has_accept {
                (*data).pending_accept = true;
            }
            0
        }
        WM_CLOSE => {
            window::destroy(hwnd);
            0
        }
        WM_SIZE => {
            layout(hwnd);
            0
        }
        WM_GETMINMAXINFO => {
            let info = &mut *(lp as *mut MINMAXINFO);
            info.ptMinTrackSize.x = dpi::scale_for_window(hwnd, 480);
            info.ptMinTrackSize.y = dpi::scale_for_window(hwnd, 320);
            0
        }
        WM_CTLCOLORSTATIC | WM_CTLCOLOREDIT if !data.is_null() => {
            let theme = Theme::default();
            let is_edit = lp as HWND == window::child(hwnd, ID_TEXT as i32);
            SetTextColor(wp as _, theme.text);
            SetBkMode(wp as _, TRANSPARENT as i32);
            SetBkColor(
                wp as _,
                if is_edit {
                    theme.control_bg
                } else {
                    theme.surface
                },
            );
            if is_edit {
                (*data).control as isize
            } else {
                (*data).surface as isize
            }
        }
        WM_ERASEBKGND if !data.is_null() => {
            if let Some(rc) = window::client_rect(hwnd) {
                windows_sys::Win32::Graphics::Gdi::FillRect(wp as _, &rc, (*data).surface as _);
            }
            1
        }
        WM_NCDESTROY => {
            window::set_user_data(hwnd, 0);
            DefWindowProcW(hwnd, msg, wp, lp)
        }
        _ => DefWindowProcW(hwnd, msg, wp, lp),
    }
}

unsafe fn create_child(
    hwnd: HWND,
    class: &str,
    text: &str,
    style: u32,
    id: usize,
    font: *mut core::ffi::c_void,
) -> HWND {
    let child = window::create_window_ex(
        0,
        to_wide(class).as_ptr(),
        to_wide(text).as_ptr(),
        WS_CHILD | WS_VISIBLE | style,
        0,
        0,
        1,
        1,
        hwnd,
        id as _,
        window::module_handle(),
        null(),
    );
    window::send_message(child, WM_SETFONT, font as usize, 1);
    child
}

pub(super) unsafe fn show(
    owner: HWND,
    request: UpdateDialogRequest<'_>,
    on_accept: &mut dyn FnMut(&str) -> Result<(), String>,
) -> bool {
    let class = to_wide(CLASS);
    let wc = WNDCLASSEXW {
        cbSize: size_of::<WNDCLASSEXW>() as u32,
        lpfnWndProc: Some(dialog_proc),
        hInstance: window::module_handle(),
        hCursor: LoadCursorW(null_mut(), IDC_ARROW),
        lpszClassName: class.as_ptr(),
        ..zeroed()
    };
    window::register_class_ex(&wc);
    let theme = Theme::default();
    let mut data = Box::new(DialogData {
        pending_accept: false,
        accepted: false,
        editable: request.editable,
        has_accept: request.accept_label.is_some(),
        font: crate::win_system_ui::create_font_px(
            ui_text_font_family(),
            dpi::scale_for_window(owner, 14),
            400,
        ),
        surface: gdi::create_solid_brush(theme.surface) as _,
        control: gdi::create_solid_brush(theme.control_bg) as _,
    });
    let work = monitor::nearest_work_rect_for_window(owner);
    let width = dpi::scale_for_window(owner, 660).min(work.right - work.left);
    let height = dpi::scale_for_window(owner, if request.editable { 320 } else { 500 })
        .min(work.bottom - work.top);
    let bounds = window::window_rect(owner).unwrap_or(work);
    let x = (bounds.left + (bounds.right - bounds.left - width) / 2)
        .clamp(work.left, work.right - width);
    let y = (bounds.top + (bounds.bottom - bounds.top - height) / 2)
        .clamp(work.top, work.bottom - height);
    let hwnd = window::create_window_ex(
        WS_EX_DLGMODALFRAME,
        class.as_ptr(),
        to_wide(request.title).as_ptr(),
        WS_POPUP | WS_CAPTION | WS_SYSMENU | WS_THICKFRAME,
        x,
        y,
        width,
        height,
        owner,
        null_mut(),
        window::module_handle(),
        data.as_mut() as *mut DialogData as _,
    );
    if hwnd.is_null() {
        return false;
    }
    let message_label = create_child(
        hwnd,
        "STATIC",
        request.message,
        0x0080,
        ID_MESSAGE,
        data.font,
    );
    let text = if request.editable {
        request.text.replace('\0', "")
    } else {
        bounded_notes(request.text)
    };
    let edit = create_child(
        hwnd,
        "EDIT",
        &text,
        WS_TABSTOP
            | WS_BORDER
            | WS_VSCROLL
            | ES_MULTILINE as u32
            | ES_AUTOVSCROLL as u32
            | if request.editable {
                0
            } else {
                ES_READONLY as u32
            },
        ID_TEXT,
        data.font,
    );
    window::send_message(edit, EM_LIMITTEXT, 65_536, 0);
    let error_label = create_child(hwnd, "STATIC", "", 0x0080, ID_ERROR, data.font);
    let accept = if let Some(label) = request.accept_label {
        create_child(
            hwnd,
            "BUTTON",
            label,
            WS_TABSTOP | BS_PUSHBUTTON as u32,
            ID_ACCEPT,
            data.font,
        )
    } else {
        null_mut()
    };
    let cancel = create_child(
        hwnd,
        "BUTTON",
        if data.has_accept {
            tr("取消", "Cancel")
        } else {
            tr("关闭", "Close")
        },
        WS_TABSTOP | BS_PUSHBUTTON as u32,
        ID_CANCEL,
        data.font,
    );
    if edit.is_null()
        || cancel.is_null()
        || message_label.is_null()
        || error_label.is_null()
        || (data.has_accept && accept.is_null())
    {
        window::destroy(hwnd);
        return false;
    }
    layout(hwnd);
    let owner_enabled = IsWindowEnabled(owner) != 0;
    if owner_enabled {
        EnableWindow(owner, 0);
    }
    ShowWindow(hwnd, SW_SHOW);
    SetForegroundWindow(hwnd);
    SetFocus(if request.editable { edit } else { cancel });
    let mut message: MSG = zeroed();
    while IsWindow(hwnd) != 0 {
        let result = GetMessageW(&mut message, null_mut(), 0, 0);
        if result <= 0 {
            if result == 0 {
                PostQuitMessage(message.wParam as i32);
            }
            window::destroy(hwnd);
            break;
        }
        let in_dialog = message.hwnd == hwnd || IsChild(hwnd, message.hwnd) != 0;
        if in_dialog && message.message == WM_KEYDOWN && message.wParam == 0x1b {
            window::destroy(hwnd);
            continue;
        }
        if in_dialog && message.message == WM_KEYDOWN && message.wParam == 0x0d {
            match enter_action(
                GetDlgCtrlID(GetFocus()) as usize,
                data.editable,
                data.has_accept,
            ) {
                EnterAction::Cancel => {
                    window::destroy(hwnd);
                    continue;
                }
                EnterAction::Accept => data.pending_accept = true,
                EnterAction::Edit => {
                    TranslateMessage(&message);
                    DispatchMessageW(&message);
                }
                EnterAction::Ignore => {}
            }
        } else if IsDialogMessageW(hwnd, &message) == 0 {
            TranslateMessage(&message);
            DispatchMessageW(&message);
        }
        if IsWindow(hwnd) == 0 {
            break;
        }
        if data.pending_accept {
            data.pending_accept = false;
            let text = if data.editable {
                window::text(edit)
            } else {
                String::new()
            };
            // No borrowed window state crosses a callback or nested message pump.
            let result = on_accept(&text);
            if IsWindow(hwnd) == 0 {
                break;
            }
            match result {
                Ok(()) => {
                    data.accepted = true;
                    window::destroy(hwnd);
                }
                Err(error) => {
                    window::set_text(
                        window::child(hwnd, ID_ERROR as i32),
                        &error.chars().take(300).collect::<String>(),
                    );
                }
            }
        }
    }
    if owner_enabled && IsWindow(owner) != 0 {
        EnableWindow(owner, 1);
    }
    data.accepted
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confirmation_requires_the_accept_button_and_cancel_enter_stays_cancel() {
        assert_eq!(enter_action(ID_CANCEL, false, true), EnterAction::Cancel);
        assert_eq!(enter_action(ID_TEXT, false, true), EnterAction::Ignore);
        assert_eq!(enter_action(ID_TEXT, true, true), EnterAction::Edit);
        assert_eq!(enter_action(ID_ACCEPT, false, true), EnterAction::Accept);
        assert_eq!(enter_action(ID_ACCEPT, false, false), EnterAction::Ignore);
    }

    #[test]
    fn long_source_is_preserved_and_invalid_or_oversized_sources_are_rejected() {
        let url = format!("https://updates.example.com/{}.json", "a".repeat(1000));
        assert_eq!(source_from_text(&url).unwrap().manifest_url, url);
        assert!(source_from_text(&"a".repeat(4097)).is_err());
        assert!(source_from_text("http://updates.example.com/update.json").is_err());
        assert_eq!(source_from_text("").unwrap().manifest_url, "");
        assert!(bounded_notes(&"a".repeat(17_000)).len() == 16_000);
        assert!(!bounded_notes("a\0b").contains('\0'));
    }

    #[test]
    fn hidden_native_source_control_preserves_long_urls_and_notes_are_read_only() {
        unsafe {
            let parent = window::create_window_ex(
                0,
                to_wide("STATIC").as_ptr(),
                to_wide("Update control test").as_ptr(),
                WS_POPUP,
                0,
                0,
                660,
                500,
                null_mut(),
                null_mut(),
                window::module_handle(),
                null(),
            );
            assert!(!parent.is_null());
            let source = create_child(parent, "EDIT", "", ES_MULTILINE as u32, ID_TEXT, null_mut());
            window::send_message(source, EM_LIMITTEXT, 65_536, 0);
            let prefix = "https://updates.example.com/";
            let url = format!("{}{}", prefix, "a".repeat(4096 - prefix.len()));
            window::set_text(source, &url);
            assert_eq!(window::text(source), url);
            assert!(source_from_text(&window::text(source)).is_ok());
            let note_text = bounded_notes(&"n".repeat(17_000));
            let notes = create_child(
                parent,
                "EDIT",
                &note_text,
                ES_MULTILINE as u32 | ES_READONLY as u32 | WS_VSCROLL,
                20,
                null_mut(),
            );
            assert_eq!(window::text(notes), note_text);
            assert_ne!(window::window_style(notes) & ES_READONLY as u32, 0);
            assert!(!window::is_visible(parent));
            window::destroy(parent);
        }
    }
}
