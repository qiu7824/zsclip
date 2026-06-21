use super::prelude::*;

pub(super) fn copy_text_to_clipboard_in_background(text: String) {
    std::thread::spawn(move || {
        for attempt in 0..5 {
            if platform_clipboard::WindowsClipboardHost::write_text_ignored_by_monitors(&text) {
                return;
            }
            if attempt < 4 {
                std::thread::sleep(std::time::Duration::from_millis(40));
            }
        }
    });
}

pub(super) fn show_native_dialog_message(
    hwnd: HWND,
    title: &str,
    message: &str,
    level: NativeDialogLevel,
) {
    platform_dialog::WindowsDialogHost::new().show_message(hwnd, title, message, level);
}

pub(super) fn confirm_native_dialog(
    hwnd: HWND,
    title: &str,
    message: &str,
    level: NativeDialogLevel,
    buttons: NativeDialogButtons,
) -> NativeDialogResponse {
    platform_dialog::WindowsDialogHost::new().confirm(hwnd, title, message, level, buttons)
}
