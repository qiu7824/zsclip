use windows_sys::Win32::UI::{
    Shell::ShellExecuteW,
    WindowsAndMessaging::{CreateIconFromResourceEx, LR_DEFAULTCOLOR, SW_SHOWNORMAL},
};

use crate::app_core::NativeShellOpenHost;
use crate::platform::string::to_wide;

pub(crate) struct WindowsShellOpenHost;

impl WindowsShellOpenHost {
    pub(crate) const fn new() -> Self {
        Self
    }
}

fn open_path_with_shell_execute(path: &str) {
    let op = to_wide("open");
    let path = to_wide(path);
    unsafe {
        ShellExecuteW(
            core::ptr::null_mut(),
            op.as_ptr(),
            path.as_ptr(),
            core::ptr::null(),
            core::ptr::null(),
            SW_SHOWNORMAL,
        );
    }
}

impl NativeShellOpenHost for WindowsShellOpenHost {
    fn open_path(&self, path: &str) {
        open_path_with_shell_execute(path);
    }
}

pub(crate) fn create_icon_from_resource(bits: &[u8], width: i32, height: i32) -> Option<isize> {
    let handle = unsafe {
        CreateIconFromResourceEx(
            bits.as_ptr(),
            bits.len() as u32,
            1,
            0x00030000,
            width,
            height,
            LR_DEFAULTCOLOR,
        )
    };
    if handle.is_null() {
        None
    } else {
        Some(handle as isize)
    }
}
