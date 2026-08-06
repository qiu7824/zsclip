use crate::app_core::{
    NativeTransientWindowHost, NativeTransientWindowPresentation, NativeTransientWindowRequest,
    UiRect,
};
use crate::platform::dpi as platform_dpi;
use crate::platform::string::to_wide;
use crate::platform::window as platform_window;
use std::mem::{size_of, zeroed};
use std::ptr::null_mut;
use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    UI::{
        HiDpi::AdjustWindowRectExForDpi,
        WindowsAndMessaging::{
            HWND_TOPMOST, SWP_NOACTIVATE, SWP_SHOWWINDOW, WNDCLASSEXW, WNDPROC, WS_EX_NOACTIVATE,
            WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP, WS_THICKFRAME,
        },
    },
};

const TRANSIENT_WINDOW_STYLE: u32 = WS_POPUP | WS_THICKFRAME;
const TRANSIENT_WINDOW_EX_STYLE: u32 = WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE;

fn transient_outer_bounds_for_client(bounds: UiRect) -> UiRect {
    let client_width = (bounds.right - bounds.left).max(1);
    let client_height = (bounds.bottom - bounds.top).max(1);
    let dpi = unsafe {
        platform_dpi::layout_dpi_for_point(POINT {
            x: bounds.left,
            y: bounds.top,
        })
        .max(96)
    };
    let mut adjusted = RECT {
        left: 0,
        top: 0,
        right: client_width,
        bottom: client_height,
    };
    let adjusted_ok = unsafe {
        AdjustWindowRectExForDpi(
            &mut adjusted,
            TRANSIENT_WINDOW_STYLE,
            0,
            TRANSIENT_WINDOW_EX_STYLE,
            dpi,
        ) != 0
    };
    if !adjusted_ok {
        return bounds;
    }
    UiRect::new(
        bounds.left + adjusted.left,
        bounds.top + adjusted.top,
        bounds.left + adjusted.right,
        bounds.top + adjusted.bottom,
    )
}

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
            let bounds = transient_outer_bounds_for_client(request.bounds);
            let handle = platform_window::create_window_ex(
                TRANSIENT_WINDOW_EX_STYLE,
                class_name.as_ptr(),
                to_wide("").as_ptr(),
                TRANSIENT_WINDOW_STYLE,
                bounds.left,
                bounds.top,
                bounds.right - bounds.left,
                bounds.bottom - bounds.top,
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
        let bounds = transient_outer_bounds_for_client(bounds);
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
