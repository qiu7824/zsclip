use std::mem::zeroed;

use windows_sys::Win32::{
    Foundation::HWND,
    UI::Shell::{
        Shell_NotifyIconW, NIF_ICON, NIF_INFO, NIF_MESSAGE, NIF_REALTIME, NIF_TIP,
        NIIF_INFO, NIIF_NOSOUND, NIM_ADD, NIM_DELETE, NIM_MODIFY, NOTIFYICONDATAW,
    },
    UI::WindowsAndMessaging::{TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RIGHTBUTTON},
};

use crate::app_core::{NativePopupMenuEntry, StatusItemHost, StatusMenuEntry};

use super::{input, menu};

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

pub(crate) fn notify(hwnd: HWND, uid: u32, title: &str, message: &str) -> bool {
    unsafe {
        let mut data: NOTIFYICONDATAW = zeroed();
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = uid;
        data.uFlags = NIF_INFO | NIF_REALTIME;
        data.dwInfoFlags = NIIF_INFO | NIIF_NOSOUND;
        for (slot, ch) in data.szInfoTitle.iter_mut().take(63).zip(title.encode_utf16()) {
            *slot = ch;
        }
        for (slot, ch) in data.szInfo.iter_mut().take(255).zip(message.encode_utf16()) {
            *slot = ch;
        }
        Shell_NotifyIconW(NIM_MODIFY, &data) != 0
    }
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
        let entries = entries
            .iter()
            .map(|entry| match entry {
                StatusMenuEntry::Command {
                    action,
                    label,
                    icon_name: _,
                } => NativePopupMenuEntry::Command {
                    id: action.command_id(),
                    label: label.to_string(),
                    enabled: true,
                    checked: false,
                },
                StatusMenuEntry::Separator => NativePopupMenuEntry::Separator,
            })
            .collect::<Vec<_>>();
        let point = input::cursor_pos().unwrap_or_else(|| unsafe { zeroed() });
        // Without TPM_RETURNCMD the choice still arrives as WM_COMMAND.
        menu::present_themed_popup_menu(
            self.owner,
            point.x,
            point.y,
            TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_LEFTALIGN,
            &entries,
        );
    }
}
