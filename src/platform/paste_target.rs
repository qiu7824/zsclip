use std::mem::{size_of, zeroed};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM},
    UI::{
        Controls::EM_SETSEL,
        WindowsAndMessaging::{
            DLGC_HASSETSEL, DLGC_WANTARROWS, DLGC_WANTCHARS, DLGC_WANTTAB, GUITHREADINFO,
            WM_GETDLGCODE, WM_SETTEXT,
        },
    },
};

use crate::app_core::{
    NativeImeHost, NativePasteTargetHost, NativeWindowIdentityHost, PasteTargetFocusStatus,
    PasteTargetTextInputCapabilities,
};
use crate::platform::accessibility as platform_accessibility;
use crate::platform::ime::WindowsImeHost;
use crate::platform::window_identity::WindowsWindowIdentityHost;
use crate::platform::{
    input as platform_input, process as platform_process, string::to_wide,
    window as platform_window,
};

#[derive(Clone, Copy, Default)]
pub(crate) struct WindowsPasteTargetHost;

impl WindowsPasteTargetHost {
    pub(crate) const fn new() -> Self {
        Self
    }
}

fn is_word_process(process_name: &str) -> bool {
    let process = process_name.trim().to_ascii_lowercase();
    process == "winword.exe" || process == "winword" || process.contains("winword")
}

fn is_qq_wps_process(process_name: &str) -> bool {
    let process = process_name.trim().to_ascii_lowercase();
    matches!(
        process.as_str(),
        "qq.exe"
            | "qq"
            | "qqnt.exe"
            | "qqnt"
            | "tim.exe"
            | "tim"
            | "wps.exe"
            | "wps"
            | "wpp.exe"
            | "wpp"
            | "et.exe"
            | "et"
            | "kingsoftoffice.exe"
            | "kingsoftoffice"
    ) || process.contains("qq")
        || process.contains("wps")
        || process.contains("wpp")
}

fn is_telegram_process(process_name: &str) -> bool {
    let process = process_name.trim().to_ascii_lowercase();
    matches!(
        process.as_str(),
        "telegram.exe" | "telegram" | "telegramdesktop.exe" | "telegramdesktop"
    ) || process.contains("telegram")
}

fn is_word_document_class(class_name: &str) -> bool {
    let class_name = class_name.trim().to_ascii_lowercase();
    class_name.starts_with("_ww")
}

fn word_target_is_text_input_ready(process_name: &str, target_cls: &str, focus_cls: &str) -> bool {
    if !is_word_process(process_name) {
        return false;
    }
    if is_word_document_class(target_cls) || is_word_document_class(focus_cls) {
        return true;
    }
    target_cls.eq_ignore_ascii_case("opusapp")
        && (focus_cls.is_empty() || focus_cls.eq_ignore_ascii_case("opusapp"))
}

fn has_default_ime_window(focus: HWND) -> bool {
    WindowsImeHost::new().has_default_ime_window(focus)
}

fn has_accessible_caret(focus: HWND) -> bool {
    unsafe { platform_accessibility::caret_rect(focus).is_some() }
}

impl NativePasteTargetHost for WindowsPasteTargetHost {
    type Handle = HWND;

    fn force_paste_target_foreground(&mut self, target: Self::Handle) -> bool {
        platform_window::force_foreground(target)
    }

    fn restore_paste_target_focus(&mut self, target: Self::Handle, focus: Self::Handle) {
        unsafe {
            if target.is_null() || !platform_window::exists(focus) {
                return;
            }
            if platform_window::root_ancestor(focus) != target {
                return;
            }

            let current_thread = platform_process::current_thread_id();
            let target_thread = platform_window::window_thread_id(target);
            let focus_thread = platform_window::window_thread_id(focus);

            let mut info: GUITHREADINFO = zeroed();
            info.cbSize = size_of::<GUITHREADINFO>() as u32;
            if target_thread != 0 && platform_window::gui_thread_info(target_thread, &mut info) {
                let current_focus = info.hwndFocus;
                if platform_window::exists(current_focus)
                    && platform_window::root_ancestor(current_focus) == target
                    && current_focus != target
                {
                    return;
                }
            }

            let attach_target = target_thread != 0
                && target_thread != current_thread
                && platform_window::attach_thread_input(current_thread, target_thread, true);
            let attach_focus = focus_thread != 0
                && focus_thread != current_thread
                && focus_thread != target_thread
                && platform_window::attach_thread_input(current_thread, focus_thread, true);

            platform_input::set_focus(focus);

            if attach_focus {
                platform_window::attach_thread_input(current_thread, focus_thread, false);
            }
            if attach_target {
                platform_window::attach_thread_input(current_thread, target_thread, false);
            }
        }
    }

    fn set_paste_target_text(&mut self, target: Self::Handle, text: &str) -> bool {
        let wide = to_wide(text);
        let ok = platform_window::send_message(target, WM_SETTEXT, 0, wide.as_ptr() as LPARAM) != 0;
        if ok {
            let caret = text.encode_utf16().count() as isize;
            platform_window::send_message(target, EM_SETSEL, caret as usize, caret);
        }
        ok
    }

    fn paste_target_text_input_capabilities(
        &mut self,
        target: Self::Handle,
    ) -> PasteTargetTextInputCapabilities {
        let dlg_code = platform_window::send_message(target, WM_GETDLGCODE, 0, 0) as u32;
        PasteTargetTextInputCapabilities {
            has_selection: (dlg_code & DLGC_HASSETSEL) != 0,
            wants_chars: (dlg_code & DLGC_WANTCHARS) != 0,
            wants_tab: (dlg_code & DLGC_WANTTAB) != 0,
            wants_arrows: (dlg_code & DLGC_WANTARROWS) != 0,
        }
    }

    fn paste_target_focus_status(
        &mut self,
        target: Self::Handle,
        passthrough_focus: Self::Handle,
    ) -> PasteTargetFocusStatus {
        let target_thread = platform_window::window_thread_id(target);
        let mut info: GUITHREADINFO = unsafe { zeroed() };
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        if target_thread == 0 || !platform_window::gui_thread_info(target_thread, &mut info) {
            return PasteTargetFocusStatus::Unknown;
        }
        let focus = info.hwndFocus;
        if focus.is_null() {
            return PasteTargetFocusStatus::NoActiveFocus;
        }
        if platform_window::root_ancestor(focus) == target
            || focus == target
            || focus == passthrough_focus
        {
            PasteTargetFocusStatus::InsideTarget
        } else {
            PasteTargetFocusStatus::OutsideTarget
        }
    }

    fn paste_target_text_input_ready(&mut self, target: Self::Handle) -> bool {
        let identity_host = WindowsWindowIdentityHost::new();
        if !identity_host.exists(target) {
            return false;
        }

        let target_cls = identity_host.class_name(target).to_ascii_lowercase();
        let process_name = identity_host.process_name(target);
        let thread_id = platform_window::window_thread_id(target);
        if thread_id == 0 {
            if is_telegram_process(&process_name) {
                return true;
            }
            if is_qq_wps_process(&process_name) {
                return has_default_ime_window(target) || has_accessible_caret(target);
            }
            return word_target_is_text_input_ready(&process_name, &target_cls, "");
        }

        let mut info: GUITHREADINFO = unsafe { zeroed() };
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        if !platform_window::gui_thread_info(thread_id, &mut info) {
            if is_telegram_process(&process_name) {
                return true;
            }
            if is_qq_wps_process(&process_name) {
                return has_default_ime_window(target) || has_accessible_caret(target);
            }
            return word_target_is_text_input_ready(&process_name, &target_cls, "");
        }

        let focus = if !info.hwndFocus.is_null() {
            info.hwndFocus
        } else {
            target
        };
        let focus_cls = identity_host.class_name(focus).to_ascii_lowercase();
        let text_input_capabilities = self.paste_target_text_input_capabilities(focus);

        if is_telegram_process(&process_name) {
            return true;
        }

        if is_qq_wps_process(&process_name) {
            let has_focus_window = identity_host.exists(info.hwndFocus);
            if has_focus_window
                || has_default_ime_window(focus)
                || !info.hwndCaret.is_null()
                || has_accessible_caret(focus)
                || text_input_capabilities.accepts_text_input()
            {
                return true;
            }
        }

        if matches!(
            focus_cls.as_str(),
            "edit"
                | "richedit20w"
                | "richedit50w"
                | "scintilla"
                | "chrome_renderwidgethosthwnd"
                | "chrome_widgetwin_0"
                | "chrome_widgetwin_1"
                | "mozillawindowclass"
                | "windows.ui.composition.desktopwindowcontentbridge"
        ) {
            return true;
        }

        if matches!(
            target_cls.as_str(),
            "chrome_widgetwin_0"
                | "chrome_widgetwin_1"
                | "windows.ui.composition.desktopwindowcontentbridge"
        ) && (process_name.contains("codex") || process_name.contains("chatgpt"))
        {
            return true;
        }

        if word_target_is_text_input_ready(&process_name, &target_cls, &focus_cls) {
            return true;
        }
        if text_input_capabilities.accepts_text_input() {
            return true;
        }
        if has_default_ime_window(focus)
            && (process_name.contains("codex")
                || process_name.contains("chatgpt")
                || process_name.contains("cursor")
                || process_name.contains("code"))
        {
            return true;
        }
        if has_accessible_caret(focus) {
            return true;
        }
        !info.hwndCaret.is_null()
    }

    fn send_paste_shortcut(&mut self, _target: Self::Handle) -> bool {
        platform_input::send_ctrl_v();
        true
    }
}

#[cfg(test)]
mod tests {
    use super::is_telegram_process;

    #[test]
    fn telegram_desktop_process_names_are_recognized() {
        assert!(is_telegram_process("Telegram.exe"));
        assert!(is_telegram_process("TelegramDesktop.exe"));
        assert!(!is_telegram_process("notepad.exe"));
    }
}
