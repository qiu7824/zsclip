use std::mem::{size_of, zeroed};

use windows_sys::Win32::{
    Foundation::{HWND, LPARAM},
    UI::{
        Controls::{EM_REPLACESEL, EM_SETSEL},
        WindowsAndMessaging::{
            DLGC_HASSETSEL, DLGC_WANTARROWS, DLGC_WANTCHARS, DLGC_WANTTAB, GUITHREADINFO, GetWindow, GW_OWNER,
            WM_GETDLGCODE, WM_PASTE, WM_SETTEXT,
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

    pub(crate) fn replace_paste_target_selection(&self, target: HWND, text: &str) -> bool {
        if !platform_window::exists(target)
            || !platform_window::class_name(target).eq_ignore_ascii_case("Edit")
            || text.contains('\0')
        {
            return false;
        }
        let wide = to_wide(text);
        // Explorer selects the basename while leaving the extension untouched.
        // Replacing that selection also preserves explicit partial selections
        // and gives the native edit control its normal undo behavior.
        platform_window::send_message_bounded(target, EM_REPLACESEL, 1, wide.as_ptr() as LPARAM)
            .is_some()
    }

    fn focus_and_caret(&self, target: HWND) -> [HWND; 2] {
        let thread_id = platform_window::window_thread_id(target);
        let mut info: GUITHREADINFO = unsafe { zeroed() };
        info.cbSize = size_of::<GUITHREADINFO>() as u32;
        if thread_id == 0 || !platform_window::gui_thread_info(thread_id, &mut info) {
            return [core::ptr::null_mut(); 2];
        }
        [info.hwndFocus, info.hwndCaret]
    }

    pub(crate) fn focused_control(&self, target: HWND) -> HWND {
        self.focus_and_caret(target)
            .into_iter()
            .find(|&control| {
                platform_window::exists(control)
                    && platform_window::root_ancestor(control) == target
            })
            .unwrap_or(core::ptr::null_mut())
    }

    pub(crate) fn explorer_rename_edit(&self, target: HWND, saved_focus: HWND) -> HWND {
        let identity = WindowsWindowIdentityHost::new();
        if !identity.exists(target)
            || !identity
                .process_name(target)
                .eq_ignore_ascii_case("explorer.exe")
            || !matches!(
                identity.class_name(target).as_str(),
                "CabinetWClass" | "ExploreWClass" | "Progman" | "WorkerW"
            )
        {
            return core::ptr::null_mut();
        }
        let [focus, caret] = self.focus_and_caret(target);
        for control in [focus, caret, saved_focus] {
            if !platform_window::exists(control)
                || !platform_window::is_visible(control)
                || platform_window::root_ancestor(control) != target
                || !platform_window::class_name(control).eq_ignore_ascii_case("Edit")
            {
                continue;
            }
            let mut ancestors = Vec::new();
            let mut parent = platform_window::parent(control);
            for _ in 0..12 {
                if parent.is_null() || parent == target {
                    break;
                }
                ancestors.push(platform_window::class_name(parent));
                parent = platform_window::parent(parent);
            }
            if explorer_rename_ancestor_classes(&ancestors) {
                return control;
            }
        }
        core::ptr::null_mut()
    }
}

fn explorer_rename_ancestor_classes(classes: &[String]) -> bool {
    let is_address_or_search = classes.iter().any(|class| {
        let class = class.to_ascii_lowercase();
        class.contains("combobox")
            || class.contains("address")
            || class.contains("breadcrumb")
            || class.contains("search")
    });
    !is_address_or_search
        && classes.iter().any(|class| {
            class.eq_ignore_ascii_case("DirectUIHWND")
                || class.eq_ignore_ascii_case("SysListView32")
        })
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

fn is_weixin_qt_main_frame(process_name: &str, target_class: &str, focus_class: &str) -> bool {
    if !matches!(process_name.trim().to_ascii_lowercase().as_str(), "weixin.exe" | "wechat.exe")
        || !target_class.eq_ignore_ascii_case(focus_class)
    {
        return false;
    }
    let class = target_class.to_ascii_lowercase();
    class.strip_prefix("qt")
        .and_then(|suffix| suffix.strip_suffix("qwindowicon"))
        .is_some_and(|version| !version.is_empty() && version.bytes().all(|ch| ch.is_ascii_digit()))
}

fn weixin_qt_main_frame_accepts_input(target: HWND, focus: HWND) -> bool {
    if focus != target
        || !platform_window::is_foreground(target)
        || !platform_window::is_root_window(target)
        || !unsafe { GetWindow(target, GW_OWNER) }.is_null()
        || !has_default_ime_window(target)
    {
        return false;
    }
    // This custom main frame exposes no native edit HWND or accessible caret.
    // Identify its real rendering child; other Qt applications and owned dialogs
    // must continue through the ordinary editor/caret checks.
    let pid = platform_window::window_process_id(target);
    platform_window::children_bottom_to_top(target).into_iter().any(|child| {
        platform_window::is_visible(child)
            && platform_window::window_process_id(child) == pid
            && platform_window::class_name(child) == "MMUIRenderSubWindowHW"
    })
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
    unsafe {
        platform_accessibility::caret_rect(focus).is_some()
            || platform_accessibility::has_recent_caret(focus)
    }
}

fn class_accepts_direct_paste_message(class_name: &str) -> bool {
    let class_name = class_name.trim().to_ascii_lowercase();
    class_name == "edit"
        || class_name.ends_with("edit")
        || class_name.contains("richedit")
        || class_name == "scintilla"
}

fn focused_direct_paste_control(target: HWND) -> HWND {
    let thread_id = platform_window::window_thread_id(target);
    if thread_id == 0 {
        return core::ptr::null_mut();
    }
    let mut info: GUITHREADINFO = unsafe { zeroed() };
    info.cbSize = size_of::<GUITHREADINFO>() as u32;
    if !platform_window::gui_thread_info(thread_id, &mut info) {
        return core::ptr::null_mut();
    }
    for candidate in [info.hwndFocus, info.hwndCaret] {
        if candidate.is_null() || platform_window::root_ancestor(candidate) != target {
            continue;
        }
        if class_accepts_direct_paste_message(&platform_window::class_name(candidate)) {
            return candidate;
        }
    }
    core::ptr::null_mut()
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
            if platform_window::is_hung(target) || platform_window::is_hung(focus) {
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
        let ok =
            platform_window::send_message_bounded(target, WM_SETTEXT, 0, wide.as_ptr() as LPARAM)
                .unwrap_or(0)
                != 0;
        if ok {
            let caret = text.encode_utf16().count() as isize;
            let _ = platform_window::send_message_bounded(target, EM_SETSEL, caret as usize, caret);
        }
        ok
    }

    fn paste_target_text_input_capabilities(
        &mut self,
        target: Self::Handle,
    ) -> PasteTargetTextInputCapabilities {
        let dlg_code =
            platform_window::send_message_bounded(target, WM_GETDLGCODE, 0, 0).unwrap_or(0) as u32;
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
        if !identity_host.exists(target) || platform_window::is_hung(target) {
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
        if !info.hwndFocus.is_null()
            && is_weixin_qt_main_frame(&process_name, &target_cls, &focus_cls)
            && weixin_qt_main_frame_accepts_input(target, focus)
        {
            return true;
        }
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

    fn send_paste_shortcut(&mut self, target: Self::Handle) -> bool {
        let control = focused_direct_paste_control(target);
        if !control.is_null() && platform_window::post_hwnd_message(control, WM_PASTE, 0, 0) {
            return true;
        }
        platform_input::send_ctrl_v()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        class_accepts_direct_paste_message, explorer_rename_ancestor_classes, is_telegram_process,
    };

    #[test]
    fn weixin_custom_frame_identity_excludes_other_apps_and_qt_popups() {
        use super::is_weixin_qt_main_frame as matches_frame;
        for process in ["Weixin.exe", "WeChat.exe"] {
            assert!(matches_frame(process, "Qt51514QWindowIcon", "Qt51514QWindowIcon"));
            assert!(matches_frame(process, "Qt681QWindowIcon", "Qt681QWindowIcon"));
        }
        for process in ["other.exe", "WeChatAppEx.exe", "fake-weixin.exe"] {
            assert!(!matches_frame(process, "Qt51514QWindowIcon", "Qt51514QWindowIcon"));
        }
        for class in ["QtQWindowIcon", "Qt5xQWindowIcon", "Qt51514QWindowToolSaveBits", "Edit"] {
            assert!(!matches_frame("Weixin.exe", class, class));
        }
        assert!(!matches_frame("Weixin.exe", "Qt51514QWindowIcon", "Qt51514QWindowToolSaveBits"));
    }

    #[test]
    #[ignore = "Read-only probe of an explicitly supplied, foreground Weixin HWND"]
    fn weixin_live_target_readiness_probe() {
        use super::*;
        let raw: usize = std::env::var("ZSCLIP_TEST_WEIXIN_HWND").expect("Provide the observed Weixin HWND")
            .parse().unwrap();
        let target = raw as HWND;
        let identity = WindowsWindowIdentityHost::new();
        let process = identity.process_name(target);
        assert!(matches!(process.to_ascii_lowercase().as_str(), "weixin.exe" | "wechat.exe"));
        let focus = WindowsPasteTargetHost::new().focused_control(target);
        let ready = WindowsPasteTargetHost::new().paste_target_text_input_ready(target);
        eprintln!("Weixin readiness: target={raw:x} class={} focus={:x} ready={ready}", identity.class_name(target), focus as usize);
        assert!(ready, "The observed foreground Weixin main frame was rejected");
    }

    #[test]
    fn direct_edit_paste_replaces_only_native_selection_and_supports_undo() {
        use super::*;
        use windows_sys::Win32::UI::Controls::EM_UNDO;
        use windows_sys::Win32::UI::WindowsAndMessaging::{
            CreateWindowExW, DestroyWindow, WS_POPUP,
        };
        let class = to_wide("EDIT");
        let initial = to_wide("报告😀.txt");
        let edit = unsafe {
            CreateWindowExW(
                0,
                class.as_ptr(),
                initial.as_ptr(),
                WS_POPUP,
                0,
                0,
                100,
                24,
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null_mut(),
                core::ptr::null(),
            )
        };
        assert!(!edit.is_null());
        struct WindowGuard(HWND);
        impl Drop for WindowGuard {
            fn drop(&mut self) {
                unsafe {
                    DestroyWindow(self.0);
                }
            }
        }
        let _guard = WindowGuard(edit);
        let host = WindowsPasteTargetHost::new();
        // A partial UTF-16 selection must preserve both unselected ends.
        platform_window::send_message(edit, EM_SETSEL, 0, 2);
        assert!(host.replace_paste_target_selection(edit, "合同"));
        assert_eq!(platform_window::text(edit), "合同😀.txt");
        platform_window::send_message(edit, EM_UNDO, 0, 0);
        assert_eq!(platform_window::text(edit), "报告😀.txt");
        // Empty selection inserts at the caret rather than replacing the field.
        platform_window::send_message(edit, EM_SETSEL, 2, 2);
        assert!(host.replace_paste_target_selection(edit, "甲"));
        assert_eq!(platform_window::text(edit), "报告甲😀.txt");
        platform_window::send_message(edit, EM_SETSEL, 0, -1);
        assert!(host.replace_paste_target_selection(edit, "新名称"));
        assert_eq!(platform_window::text(edit), "新名称");
    }

    #[test]
    fn explorer_replacement_is_limited_to_file_list_rename_controls() {
        let classes = |items: &[&str]| {
            items
                .iter()
                .map(|item| item.to_string())
                .collect::<Vec<_>>()
        };
        assert!(explorer_rename_ancestor_classes(&classes(&[
            "DirectUIHWND",
            "SHELLDLL_DefView"
        ])));
        assert!(explorer_rename_ancestor_classes(&classes(&[
            "SysListView32",
            "SHELLDLL_DefView"
        ])));
        assert!(!explorer_rename_ancestor_classes(&classes(&[
            "ComboBoxEx32",
            "DirectUIHWND"
        ])));
        assert!(!explorer_rename_ancestor_classes(&classes(&[
            "SearchBox",
            "DirectUIHWND"
        ])));
        assert!(!explorer_rename_ancestor_classes(&classes(&["Notepad"])));
    }

    #[test]
    fn telegram_desktop_process_names_are_recognized() {
        assert!(is_telegram_process("Telegram.exe"));
        assert!(is_telegram_process("TelegramDesktop.exe"));
        assert!(!is_telegram_process("notepad.exe"));
    }

    #[test]
    fn native_edit_classes_use_direct_paste_messages() {
        assert!(class_accepts_direct_paste_message("Edit"));
        assert!(class_accepts_direct_paste_message("TNewEdit"));
        assert!(class_accepts_direct_paste_message("RICHEDIT50W"));
        assert!(class_accepts_direct_paste_message("Scintilla"));
        assert!(!class_accepts_direct_paste_message(
            "Chrome_RenderWidgetHostHWND"
        ));
    }
}
