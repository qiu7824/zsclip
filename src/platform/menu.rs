use windows_sys::Win32::{
    Foundation::{HWND, RECT},
    UI::WindowsAndMessaging::{
        AppendMenuW, CreatePopupMenu, DestroyMenu, TrackPopupMenu, HMENU, MF_CHECKED, MF_GRAYED,
        MF_POPUP, MF_SEPARATOR, MF_STRING, TPM_BOTTOMALIGN, TPM_LEFTALIGN, TPM_RETURNCMD,
        TPM_RIGHTBUTTON, TPM_TOPALIGN,
    },
};

use crate::app_core::{NativePopupMenuEntry, NativePopupMenuHost, NativePopupMenuPlacement};

use super::{appearance, string::to_wide, window};

pub(crate) fn create_popup() -> HMENU {
    unsafe { CreatePopupMenu() }
}

pub(crate) fn append_raw(menu: HMENU, flags: u32, id_or_submenu: usize, text: *const u16) -> bool {
    unsafe { AppendMenuW(menu, flags, id_or_submenu, text) != 0 }
}

pub(crate) fn track_popup_raw(
    menu: HMENU,
    flags: u32,
    x: i32,
    y: i32,
    reserved: i32,
    owner: HWND,
    rect: *const RECT,
) -> usize {
    unsafe { TrackPopupMenu(menu, flags, x, y, reserved, owner, rect) as usize }
}

pub(crate) fn destroy(menu: HMENU) {
    if menu.is_null() {
        return;
    }
    unsafe {
        DestroyMenu(menu);
    }
}

pub(crate) struct WindowsPopupMenuHost;

impl WindowsPopupMenuHost {
    pub(crate) const fn new() -> Self {
        Self
    }

    fn append_entries(menu: HMENU, entries: &[NativePopupMenuEntry]) {
        for entry in entries {
            match entry {
                NativePopupMenuEntry::Command {
                    id,
                    label,
                    enabled,
                    checked,
                } => {
                    let mut flags = MF_STRING;
                    if !enabled {
                        flags |= MF_GRAYED;
                    }
                    if *checked {
                        flags |= MF_CHECKED;
                    }
                    append_raw(menu, flags, *id, to_wide(label).as_ptr());
                }
                NativePopupMenuEntry::Submenu {
                    label,
                    enabled,
                    entries,
                } => {
                    let submenu = create_popup();
                    if submenu.is_null() {
                        continue;
                    }
                    unsafe {
                        appearance::apply_theme_to_menu(submenu as _);
                    }
                    Self::append_entries(submenu, entries);
                    let mut flags = MF_POPUP;
                    if !enabled {
                        flags |= MF_GRAYED;
                    }
                    append_raw(menu, flags, submenu as usize, to_wide(label).as_ptr());
                }
                NativePopupMenuEntry::Separator => {
                    append_raw(menu, MF_SEPARATOR, 0, core::ptr::null());
                }
            }
        }
    }
}

impl NativePopupMenuHost for WindowsPopupMenuHost {
    type Owner = HWND;

    fn present_popup_menu(
        &mut self,
        owner: Self::Owner,
        x: i32,
        y: i32,
        placement: NativePopupMenuPlacement,
        entries: &[NativePopupMenuEntry],
    ) -> usize {
        let menu = create_popup();
        if menu.is_null() {
            return 0;
        }
        unsafe {
            appearance::apply_theme_to_menu(menu as _);
        }
        Self::append_entries(menu, entries);
        window::set_foreground(owner);
        let align = match placement {
            NativePopupMenuPlacement::TopLeft => TPM_TOPALIGN,
            NativePopupMenuPlacement::BottomLeft => TPM_BOTTOMALIGN,
        };
        let cmd = track_popup_raw(
            menu,
            TPM_RIGHTBUTTON | align | TPM_LEFTALIGN | TPM_RETURNCMD,
            x,
            y,
            0,
            owner,
            core::ptr::null::<RECT>(),
        );
        window::ping(owner);
        destroy(menu);
        cmd
    }
}
