use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        MessageBoxW, IDCANCEL, IDNO, IDYES, MB_ICONERROR, MB_ICONINFORMATION, MB_ICONQUESTION,
        MB_ICONWARNING, MB_OK, MB_YESNO, MB_YESNOCANCEL,
    },
};

use crate::app_core::{
    native_host_dialog_button_specs, NativeDialogButtons, NativeDialogHost, NativeDialogLevel,
    NativeDialogResponse, NativeHostDialogAction,
};

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

fn message_box(hwnd: HWND, text: &str, title: &str, flags: u32) -> i32 {
    let text = wide_null(text);
    let title = wide_null(title);
    unsafe { MessageBoxW(hwnd, text.as_ptr(), title.as_ptr(), flags) }
}

pub(crate) struct WindowsDialogHost;

impl WindowsDialogHost {
    pub(crate) const fn new() -> Self {
        Self
    }

    fn flags_for_level(level: NativeDialogLevel) -> u32 {
        match level {
            NativeDialogLevel::Info => MB_ICONINFORMATION,
            NativeDialogLevel::Warning => MB_ICONWARNING,
            NativeDialogLevel::Error => MB_ICONERROR,
            NativeDialogLevel::Question => MB_ICONQUESTION,
        }
    }

    fn flags_for_buttons(buttons: NativeDialogButtons) -> u32 {
        match buttons {
            NativeDialogButtons::YesNoCancel => MB_YESNOCANCEL,
            NativeDialogButtons::YesNo => MB_YESNO,
        }
    }

    fn response_from_result(result: i32) -> NativeDialogResponse {
        match result {
            IDYES => NativeDialogResponse::Yes,
            IDNO => NativeDialogResponse::No,
            IDCANCEL => NativeDialogResponse::Cancel,
            _ => NativeDialogResponse::Cancel,
        }
    }

    fn native_dialog_action_available(action: NativeHostDialogAction) -> bool {
        native_host_dialog_button_specs()
            .into_iter()
            .any(|spec| spec.action == action)
    }
}

impl NativeDialogHost for WindowsDialogHost {
    type Owner = HWND;

    fn show_message(
        &self,
        owner: Self::Owner,
        title: &str,
        message: &str,
        level: NativeDialogLevel,
    ) {
        if !Self::native_dialog_action_available(NativeHostDialogAction::ShowInfoMessage) {
            return;
        }
        message_box(owner, message, title, MB_OK | Self::flags_for_level(level));
    }

    fn confirm(
        &self,
        owner: Self::Owner,
        title: &str,
        message: &str,
        level: NativeDialogLevel,
        buttons: NativeDialogButtons,
    ) -> NativeDialogResponse {
        if !Self::native_dialog_action_available(NativeHostDialogAction::ConfirmQuestion) {
            return NativeDialogResponse::Cancel;
        }
        Self::response_from_result(message_box(
            owner,
            message,
            title,
            Self::flags_for_buttons(buttons) | Self::flags_for_level(level),
        ))
    }
}
