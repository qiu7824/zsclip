use crate::app_core::{
    NativeTransientWindowHost, NativeTransientWindowPresentation, NativeTransientWindowRequest,
    UiRect,
};
use crate::platform::string::to_wide;
use crate::platform::window as platform_window;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{
        HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, WNDCLASSEXW, WNDPROC, WS_EX_NOACTIVATE,
        WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    },
};

#[derive(Clone, Copy)]
pub(super) struct WindowsTransientWindowHost {
    class_name: &'static str,
    window_proc: WNDPROC,
}

impl WindowsTransientWindowHost {
    pub(super) const fn new(class_name: &'static str, window_proc: WNDPROC) -> Self {
        Self {
            class_name,
            window_proc,
        }
    }

    unsafe fn register_transient_class(&self) {
        let class_name = to_wide(self.class_name);
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            lpfnWndProc: self.window_proc,
            hInstance: platform_window::module_handle(),
            hCursor: platform_window::arrow_cursor(),
            hbrBackground: null_mut(),
            lpszClassName: class_name.as_ptr(),
            ..zeroed()
        };
        platform_window::register_class_ex(&wc);
    }
}

impl NativeTransientWindowHost for WindowsTransientWindowHost {
    type Handle = HWND;
    type Owner = HWND;

    fn create_transient_window(
        &mut self,
        request: NativeTransientWindowRequest<Self::Owner>,
    ) -> NativeTransientWindowPresentation<Self::Handle> {
        unsafe {
            let hinstance = platform_window::module_handle();
            if hinstance.is_null() || self.window_proc.is_none() || self.class_name.is_empty() {
                return NativeTransientWindowPresentation::Failed;
            }
            self.register_transient_class();
            let class_name = to_wide(self.class_name);
            let handle = platform_window::create_window_ex(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                to_wide("").as_ptr(),
                WS_POPUP,
                request.bounds.left,
                request.bounds.top,
                request.bounds.right - request.bounds.left,
                request.bounds.bottom - request.bounds.top,
                null_mut(),
                null_mut(),
                hinstance,
                request.owner as _,
            );
            if handle.is_null() {
                NativeTransientWindowPresentation::Failed
            } else {
                NativeTransientWindowPresentation::Created(handle)
            }
        }
    }

    fn present_transient_window(&mut self, handle: Self::Handle, bounds: UiRect) {
        platform_window::set_pos(
            handle,
            HWND_TOPMOST,
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            SWP_NOACTIVATE | SWP_SHOWWINDOW,
        );
    }

    fn hide_transient_window(&mut self, handle: Self::Handle) {
        platform_window::hide(handle);
    }

    fn destroy_transient_window(&mut self, handle: Self::Handle) {
        platform_window::destroy(handle);
    }
}
