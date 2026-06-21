use std::{mem::zeroed, ptr::null};

use windows_sys::Win32::{
    Foundation::HWND,
    UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW,
    },
    UI::WindowsAndMessaging::{
        MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON,
    },
};

use crate::app_core::{StatusItemHost, StatusMenuEntry};

use super::{appearance, input, menu, string::to_wide, window};

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) unsafe fn add(
    hwnd: HWND,
    uid: u32,
    callback_message: u32,
    icon: isize,
    tip: &str,
) -> bool {
    let mut data: NOTIFYICONDATAW = zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = uid;
    data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
    data.uCallbackMessage = callback_message;
    data.hIcon = icon as _;

    let tip = wide_null(tip);
    let n = core::cmp::min(tip.len(), data.szTip.len());
    data.szTip[..n].copy_from_slice(&tip[..n]);

    Shell_NotifyIconW(NIM_ADD, &data) != 0
}

pub(crate) unsafe fn remove(hwnd: HWND, uid: u32) {
    let mut data: NOTIFYICONDATAW = zeroed();
    data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
    data.hWnd = hwnd;
    data.uID = uid;
    let _ = Shell_NotifyIconW(NIM_DELETE, &data);
}

pub(crate) struct WindowsStatusItemHost {
    owner: HWND,
    uid: u32,
    callback_message: u32,
    icon: isize,
}

impl WindowsStatusItemHost {
    pub(crate) const fn new(owner: HWND, uid: u32, callback_message: u32, icon: isize) -> Self {
        Self {
            owner,
            uid,
            callback_message,
            icon,
        }
    }
}

impl StatusItemHost for WindowsStatusItemHost {
    fn install(&mut self, tooltip: &str) -> bool {
        unsafe {
            add(
                self.owner,
                self.uid,
                self.callback_message,
                self.icon,
                tooltip,
            )
        }
    }

    fn remove(&mut self) {
        unsafe {
            remove(self.owner, self.uid);
        }
    }

    fn present_menu(&mut self, entries: &[StatusMenuEntry]) {
        let popup = menu::create_popup();
        if popup.is_null() {
            return;
        }
        unsafe {
            appearance::apply_theme_to_menu(popup as _);
        }
        for entry in entries {
            match entry {
                StatusMenuEntry::Command {
                    action,
                    label,
                    icon_name: _,
                } => {
                    menu::append_raw(
                        popup,
                        MF_STRING,
                        action.command_id(),
                        to_wide(label).as_ptr(),
                    );
                }
                StatusMenuEntry::Separator => {
                    menu::append_raw(popup, MF_SEPARATOR, 0, null());
                }
            }
        }

        let point = input::cursor_pos().unwrap_or_else(|| unsafe { zeroed() });
        window::set_foreground(self.owner);
        menu::track_popup_raw(
            popup,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
            point.x,
            point.y,
            0,
            self.owner,
            null(),
        );
        window::ping(self.owner);
        menu::destroy(popup);
    }
}
