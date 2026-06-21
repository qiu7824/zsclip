use windows_sys::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    UI::WindowsAndMessaging::{
        CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HOOKPROC, KBDLLHOOKSTRUCT,
        WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
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

pub(crate) fn call_next(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    unsafe { CallNextHookEx(core::ptr::null_mut(), code, wparam, lparam) }
}

pub(crate) fn install_low_level_keyboard(proc: HOOKPROC) -> isize {
    unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, proc, platform_window::module_handle(), 0) as isize }
}

pub(crate) fn uninstall(handle: isize) -> bool {
    if handle == 0 {
        return true;
    }
    unsafe { UnhookWindowsHookEx(handle as _) != 0 }
}
