use windows_sys::Win32::{
    Foundation::HWND,
    System::DataExchange::{AddClipboardFormatListener, RemoveClipboardFormatListener},
};

pub(crate) fn register(hwnd: isize) -> bool {
    unsafe { AddClipboardFormatListener(hwnd as HWND) != 0 }
}

pub(crate) fn unregister(hwnd: isize) {
    unsafe {
        RemoveClipboardFormatListener(hwnd as HWND);
    }
}
