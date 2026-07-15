use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    UI::{
        Input::KeyboardAndMouse::{
            keybd_event, GetAsyncKeyState, ReleaseCapture, SendInput, SetCapture, SetFocus, INPUT,
            INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_BACK, VK_CONTROL, VK_LBUTTON,
            VK_MBUTTON, VK_MENU, VK_RBUTTON, VK_SHIFT, VK_V,
        },
        WindowsAndMessaging::GetCursorPos,
    },
};

#[link(name = "user32")]
unsafe extern "system" {
    fn TrackMouseEvent(lpeventtrack: *mut TRACKMOUSEEVENT) -> i32;
}

#[link(name = "imm32")]
unsafe extern "system" {
    fn ImmGetDefaultIMEWnd(hwnd: HWND) -> HWND;
}

#[repr(C)]
struct TRACKMOUSEEVENT {
    cb_size: u32,
    dw_flags: u32,
    hwnd_track: HWND,
    dw_hover_time: u32,
}

const TME_LEAVE: u32 = 0x00000002;
const TME_HOVER: u32 = 0x00000001;

pub(crate) fn is_key_down(vk: u32) -> bool {
    unsafe { (GetAsyncKeyState(vk as i32) as u16 & 0x8000) != 0 }
}

pub(crate) fn primary_mouse_button_down() -> bool {
    is_key_down(VK_LBUTTON as u32)
}

pub(crate) fn any_mouse_button_down() -> bool {
    primary_mouse_button_down() || is_key_down(VK_RBUTTON as u32) || is_key_down(VK_MBUTTON as u32)
}

pub(crate) fn cursor_pos() -> Option<POINT> {
    let mut pt = POINT { x: 0, y: 0 };
    if unsafe { GetCursorPos(&mut pt) } != 0 {
        Some(pt)
    } else {
        None
    }
}

pub(crate) fn default_ime_window(hwnd: HWND) -> HWND {
    if hwnd.is_null() {
        return core::ptr::null_mut();
    }
    unsafe { ImmGetDefaultIMEWnd(hwnd) }
}

pub(crate) fn track_mouse_leave_and_hover(hwnd: HWND, hover_time_ms: u32) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let mut tme = TRACKMOUSEEVENT {
        cb_size: core::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dw_flags: TME_LEAVE | TME_HOVER,
        hwnd_track: hwnd,
        dw_hover_time: hover_time_ms,
    };
    unsafe { TrackMouseEvent(&mut tme) != 0 }
}

pub(crate) fn track_mouse_leave(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let mut tme = TRACKMOUSEEVENT {
        cb_size: core::mem::size_of::<TRACKMOUSEEVENT>() as u32,
        dw_flags: TME_LEAVE,
        hwnd_track: hwnd,
        dw_hover_time: 0,
    };
    unsafe { TrackMouseEvent(&mut tme) != 0 }
}

pub(crate) fn set_capture(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        SetCapture(hwnd);
    }
}

pub(crate) fn release_capture() {
    unsafe {
        ReleaseCapture();
    }
}

pub(crate) fn set_focus(hwnd: HWND) {
    if hwnd.is_null() {
        return;
    }
    unsafe {
        SetFocus(hwnd);
    }
}

fn key_input(vk: u16, flags: u32) -> INPUT {
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 {
            ki: KEYBDINPUT {
                wVk: vk,
                wScan: 0,
                dwFlags: flags,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    }
}

pub(crate) fn key_down(vk: u8) {
    unsafe {
        keybd_event(vk, 0, 0, 0);
    }
}

pub(crate) fn key_up(vk: u8) {
    unsafe {
        keybd_event(vk, 0, KEYEVENTF_KEYUP, 0);
    }
}

pub(crate) fn tap_key(vk: u8) {
    key_down(vk);
    key_up(vk);
}

pub(crate) fn send_ctrl_v() {
    let shift_down = is_key_down(VK_SHIFT as u32);
    let mut inputs = Vec::with_capacity(if shift_down { 6 } else { 4 });
    if shift_down {
        inputs.push(key_input(VK_SHIFT as u16, KEYEVENTF_KEYUP));
    }
    inputs.push(key_input(VK_CONTROL as u16, 0));
    inputs.push(key_input(VK_V as u16, 0));
    inputs.push(key_input(VK_V as u16, KEYEVENTF_KEYUP));
    inputs.push(key_input(VK_CONTROL as u16, KEYEVENTF_KEYUP));
    if shift_down {
        inputs.push(key_input(VK_SHIFT as u16, 0));
    }
    unsafe {
        let _ = SendInput(
            inputs.len() as u32,
            inputs.as_mut_ptr(),
            core::mem::size_of::<INPUT>() as i32,
        );
    }
}

pub(crate) fn send_backspace_times(count: u8) {
    for _ in 0..count {
        tap_key(VK_BACK as u8);
    }
}

pub(crate) fn send_alt_tap() {
    tap_key(VK_MENU as u8);
}
