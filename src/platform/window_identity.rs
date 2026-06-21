use windows_sys::Win32::Foundation::HWND;

use crate::app_core::NativeWindowIdentityHost;
use crate::platform::{process as platform_process, window as platform_window};

#[derive(Clone, Copy, Default)]
pub(crate) struct WindowsWindowIdentityHost;

impl WindowsWindowIdentityHost {
    pub(crate) const fn new() -> Self {
        Self
    }
}

impl NativeWindowIdentityHost for WindowsWindowIdentityHost {
    type Handle = HWND;

    fn process_name(&self, handle: Self::Handle) -> String {
        platform_process::process_image_name(platform_window::window_process_id(handle))
    }

    fn class_name(&self, handle: Self::Handle) -> String {
        platform_window::class_name(handle)
    }

    fn root_handle(&self, handle: Self::Handle) -> Self::Handle {
        if handle.is_null() {
            handle
        } else {
            platform_window::root_ancestor(handle)
        }
    }

    fn foreground_handle(&self) -> Self::Handle {
        platform_window::foreground()
    }

    fn exists(&self, handle: Self::Handle) -> bool {
        platform_window::exists(handle)
    }

    fn is_foreground(&self, handle: Self::Handle) -> bool {
        platform_window::is_foreground(handle)
    }

    fn is_current_process_window(&self, handle: Self::Handle) -> bool {
        !handle.is_null()
            && platform_window::window_process_id(handle) == platform_process::current_process_id()
    }
}
