use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HOOKPROC, KBDLLHOOKSTRUCT,
        MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
        WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_XBUTTONDOWN, WM_XBUTTONUP,
    },
};

use crate::platform::window as platform_window;

const LLKHF_LOWER_IL_INJECTED: u32 = 0x0000_0002;
const LLKHF_INJECTED: u32 = 0x0000_0010;
const LLMHF_LOWER_IL_INJECTED: u32 = 0x0000_0002;
const LLMHF_INJECTED: u32 = 0x0000_0001;
const XBUTTON1: u32 = 0x0001;
const XBUTTON2: u32 = 0x0002;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyboardHookEvent {
    pub(crate) vk_code: u32,
    flags: u32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MouseHookButton {
    Left,
    Right,
    Middle,
    X1,
    X2,
}

#[derive(Clone, Copy)]
pub(crate) struct MouseHookEvent {
    pub(crate) point: windows_sys::Win32::Foundation::POINT,
    pub(crate) button: MouseHookButton,
    pub(crate) button_down: bool,
    flags: u32,
}

impl KeyboardHookEvent {
    pub(crate) fn is_injected(self) -> bool {
        self.flags & LLKHF_INJECTED != 0
    }

    pub(crate) fn is_injected_or_lower_integrity(self) -> bool {
        self.flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED) != 0
    }
}

impl MouseHookEvent {
    pub(crate) fn is_injected_or_lower_integrity(self) -> bool {
        self.flags & (LLMHF_INJECTED | LLMHF_LOWER_IL_INJECTED) != 0
    }
}

pub(crate) unsafe fn keyboard_event(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<KeyboardHookEvent> {
    if code < 0 || (wparam as u32 != WM_KEYDOWN && wparam as u32 != WM_SYSKEYDOWN) {
        return None;
    }
    let data = &*(lparam as *const KBDLLHOOKSTRUCT);
    Some(KeyboardHookEvent {
        vk_code: data.vkCode,
        flags: data.flags,
    })
}

pub(crate) unsafe fn mouse_button_event(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<MouseHookEvent> {
    if code < 0
        || !matches!(
            wparam as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN | WM_XBUTTONUP
        )
    {
        return None;
    }
    let data = &*(lparam as *const MSLLHOOKSTRUCT);
    let button = match wparam as u32 {
        WM_LBUTTONDOWN => MouseHookButton::Left,
        WM_RBUTTONDOWN => MouseHookButton::Right,
        WM_MBUTTONDOWN => MouseHookButton::Middle,
        WM_XBUTTONDOWN | WM_XBUTTONUP => match (data.mouseData >> 16) & 0xffff {
            XBUTTON1 => MouseHookButton::X1,
            XBUTTON2 => MouseHookButton::X2,
            _ => return None,
        },
        _ => return None,
    };
    Some(MouseHookEvent {
        point: data.pt,
        button,
        button_down: wparam as u32 != WM_XBUTTONUP,
        flags: data.flags,
    })
}

pub(crate) fn call_next(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { CallNextHookEx(core::ptr::null_mut(), code, wparam, lparam) }
}

pub(crate) fn install_low_level_keyboard(proc: HOOKPROC) -> isize {
    unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, proc, platform_window::module_handle(), 0) as isize }
}

pub(crate) fn install_low_level_mouse(proc: HOOKPROC) -> isize {
    unsafe { SetWindowsHookExW(WH_MOUSE_LL, proc, platform_window::module_handle(), 0) as isize }
}

pub(crate) fn uninstall(handle: isize) -> bool {
    if handle == 0 {
        return true;
    }
    unsafe { UnhookWindowsHookEx(handle as _) != 0 }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows_sys::Win32::Foundation::POINT;
    use windows_sys::Win32::UI::WindowsAndMessaging::WM_MOUSEMOVE;

    #[test]
    fn mouse_hook_reports_button_down_at_event_point() {
        let data = MSLLHOOKSTRUCT {
            pt: POINT { x: -15, y: 42 },
            mouseData: 0,
            flags: 0,
            time: 0,
            dwExtraInfo: 0,
        };
        let event =
            unsafe { mouse_button_event(0, WM_LBUTTONDOWN as usize, &data as *const _ as isize) }
                .expect("left button down should be reported");

        assert_eq!((event.point.x, event.point.y), (-15, 42));
        assert_eq!(event.button, MouseHookButton::Left);
        assert!(event.button_down);
        assert!(unsafe {
            mouse_button_event(0, WM_MOUSEMOVE as usize, &data as *const _ as isize)
        }
        .is_none());
    }

    #[test]
    fn mouse_hook_distinguishes_side_buttons_and_injected_input() {
        let data = MSLLHOOKSTRUCT {
            pt: POINT { x: 10, y: 20 },
            mouseData: XBUTTON2 << 16,
            flags: LLMHF_INJECTED,
            time: 0,
            dwExtraInfo: 0,
        };
        let event =
            unsafe { mouse_button_event(0, WM_XBUTTONDOWN as usize, &data as *const _ as isize) }
                .expect("side button down should be reported");

        assert_eq!(event.button, MouseHookButton::X2);
        assert!(event.button_down);
        assert!(event.is_injected_or_lower_integrity());

        let released =
            unsafe { mouse_button_event(0, WM_XBUTTONUP as usize, &data as *const _ as isize) }
                .expect("side button release should be reported");
        assert!(!released.button_down);
    }
}
