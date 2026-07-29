use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HOOKPROC, KBDLLHOOKSTRUCT,
        MSLLHOOKSTRUCT, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_LBUTTONDOWN, WM_MBUTTONDOWN,
        WM_RBUTTONDOWN, WM_SYSKEYDOWN, WM_XBUTTONDOWN,
    },
};

use crate::platform::window as platform_window;

const LLKHF_LOWER_IL_INJECTED: u32 = 0x0000_0002;
const LLKHF_INJECTED: u32 = 0x0000_0010;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyboardHookEvent {
    pub(crate) vk_code: u32,
    flags: u32,
}

#[derive(Clone, Copy)]
pub(crate) struct MouseHookEvent {
    pub(crate) point: windows_sys::Win32::Foundation::POINT,
}

impl KeyboardHookEvent {
    pub(crate) fn is_injected(self) -> bool {
        self.flags & LLKHF_INJECTED != 0
    }

    pub(crate) fn is_injected_or_lower_integrity(self) -> bool {
        self.flags & (LLKHF_INJECTED | LLKHF_LOWER_IL_INJECTED) != 0
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

pub(crate) unsafe fn mouse_button_down_event(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> Option<MouseHookEvent> {
    if code < 0
        || !matches!(
            wparam as u32,
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN | WM_XBUTTONDOWN
        )
    {
        return None;
    }
    let data = &*(lparam as *const MSLLHOOKSTRUCT);
    Some(MouseHookEvent { point: data.pt })
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
        let event = unsafe {
            mouse_button_down_event(0, WM_LBUTTONDOWN as usize, &data as *const _ as isize)
        }
        .expect("left button down should be reported");

        assert_eq!((event.point.x, event.point.y), (-15, 42));
        assert!(unsafe {
            mouse_button_down_event(0, WM_MOUSEMOVE as usize, &data as *const _ as isize)
        }
        .is_none());
    }
}
