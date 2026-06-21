use super::main_startup_integrations::taskbar_created_message;
use crate::app_core::{
    ApplicationEvent, ImagePasteReadyResult, ImageThumbReadyResult, MainAsyncEvent,
    NativeWindowToken, TextOperationReadyResult, UiEvent,
};
use crate::platform::ui_event as platform_ui_event;
use windows_sys::Win32::UI::WindowsAndMessaging::WM_APP;

pub(super) const WM_VV_SHOW: u32 = WM_APP + 20;
pub(super) const WM_VV_HIDE: u32 = WM_APP + 21;
pub(super) const WM_VV_SELECT: u32 = WM_APP + 22;
pub(super) const WM_ITEMS_PAGE_READY: u32 = WM_APP + 30;
pub(super) const WM_UPDATE_CHECK_READY: u32 = WM_APP + 31;
pub(super) const WM_CLOUD_SYNC_READY: u32 = WM_APP + 33;
pub(super) const WM_IMAGE_PASTE_READY: u32 = WM_APP + 34;
pub(super) const WM_IMAGE_OCR_READY: u32 = WM_APP + 35;
pub(super) const WM_TEXT_TRANSLATE_READY: u32 = WM_APP + 36;
pub(super) const WM_IMAGE_THUMB_READY: u32 = WM_APP + 37;
pub(crate) const WM_LAN_SYNC_READY: u32 = WM_APP + 38;
pub(super) const WM_STARTUP_DATA_RECONCILED: u32 = WM_APP + 39;
pub(crate) const WM_TRAYICON: u32 = WM_APP + 1;

pub(super) enum MainWindowHostEvent {
    Ui(UiEvent),
    Async(MainAsyncEvent),
}

pub(super) unsafe fn main_window_host_event_from_message(
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> Option<MainWindowHostEvent> {
    if is_main_async_result_message(msg) {
        return take_main_async_event_from_window_message(msg, lparam)
            .map(MainWindowHostEvent::Async);
    }
    main_application_event_from_window_message(msg, wparam, lparam)
        .or_else(|| platform_ui_event::from_window_message(msg, wparam, lparam))
        .map(MainWindowHostEvent::Ui)
}

pub(super) fn is_main_async_result_message(msg: u32) -> bool {
    matches!(
        msg,
        WM_IMAGE_PASTE_READY | WM_IMAGE_OCR_READY | WM_TEXT_TRANSLATE_READY | WM_IMAGE_THUMB_READY
    )
}

pub(super) unsafe fn take_main_async_event_from_window_message(
    msg: u32,
    lparam: isize,
) -> Option<MainAsyncEvent> {
    if lparam == 0 {
        return None;
    }
    match msg {
        WM_IMAGE_PASTE_READY => Some(MainAsyncEvent::ImagePaste(*Box::from_raw(
            lparam as *mut ImagePasteReadyResult,
        ))),
        WM_IMAGE_OCR_READY => Some(MainAsyncEvent::ImageOcr(*Box::from_raw(
            lparam as *mut TextOperationReadyResult,
        ))),
        WM_TEXT_TRANSLATE_READY => Some(MainAsyncEvent::TextTranslate(*Box::from_raw(
            lparam as *mut TextOperationReadyResult,
        ))),
        WM_IMAGE_THUMB_READY => Some(MainAsyncEvent::ImageThumbnail(*Box::from_raw(
            lparam as *mut ImageThumbReadyResult,
        ))),
        _ => None,
    }
}

pub(super) fn main_application_event_from_window_message(
    msg: u32,
    wparam: usize,
    lparam: isize,
) -> Option<UiEvent> {
    if msg == taskbar_created_message() {
        return Some(UiEvent::Application(
            ApplicationEvent::ShellIntegrationRestored,
        ));
    }
    let event = match msg {
        WM_LAN_SYNC_READY => ApplicationEvent::LanSyncReady,
        WM_VV_SHOW => ApplicationEvent::VvShowRequested {
            target: NativeWindowToken(wparam),
        },
        WM_VV_HIDE => ApplicationEvent::VvHideRequested,
        WM_VV_SELECT => ApplicationEvent::VvSelectRequested { index: wparam },
        WM_ITEMS_PAGE_READY => ApplicationEvent::ItemsPageReady,
        WM_STARTUP_DATA_RECONCILED => ApplicationEvent::StartupDataReconciled { deleted: wparam },
        WM_CLOUD_SYNC_READY => ApplicationEvent::CloudSyncReady,
        WM_UPDATE_CHECK_READY => ApplicationEvent::UpdateCheckReady,
        WM_TRAYICON => ApplicationEvent::TrayCallback {
            code: lparam as u32,
        },
        _ => return None,
    };
    Some(UiEvent::Application(event))
}
