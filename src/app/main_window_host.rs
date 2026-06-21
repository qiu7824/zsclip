use super::prelude::*;
use crate::app_core::{
    NativeAppIconResource, NativeMainWindowHandles, NativeMainWindowHost,
    NativeMainWindowPresentMode, NativeMainWindowPresentation, NativeMainWindowRequest, UiRect,
};
use crate::platform::appearance as platform_appearance;
use crate::platform::dpi as platform_dpi;
use crate::platform::gdi as platform_gdi;
use crate::platform::input as platform_input;
use crate::platform::string::to_wide;
use crate::platform::system_parameters as platform_system_parameters;
use crate::platform::window as platform_window;
use std::mem::{size_of, zeroed};
use windows_sys::Win32::Foundation::HINSTANCE;

#[derive(Clone, Copy)]
pub(super) struct WindowsMainWindowHost {
    window_proc: WNDPROC,
}

impl WindowsMainWindowHost {
    pub(super) const fn new(window_proc: WNDPROC) -> Self {
        Self { window_proc }
    }

    unsafe fn register_window_class(
        &self,
        role: WindowRole,
        hinstance: HINSTANCE,
        cursor: HCURSOR,
    ) -> bool {
        let class_name = to_wide(role.class_name());
        let wc = WNDCLASSEXW {
            cbSize: size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW | CS_DBLCLKS,
            lpfnWndProc: self.window_proc,
            hInstance: hinstance,
            hCursor: cursor,
            hbrBackground: null_mut(),
            lpszClassName: class_name.as_ptr(),
            ..zeroed()
        };
        platform_window::register_class_ex(&wc) != 0
    }

    unsafe fn create_window(
        &self,
        role: WindowRole,
        title: &[u16],
        width: i32,
        height: i32,
        hinstance: HINSTANCE,
    ) -> HWND {
        let ex_style = match role {
            WindowRole::Main => WS_EX_TOOLWINDOW | WS_EX_TOPMOST,
            WindowRole::Quick => WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE,
        };
        platform_window::create_window_ex(
            ex_style,
            to_wide(role.class_name()).as_ptr(),
            title.as_ptr(),
            WS_POPUP | WS_CLIPCHILDREN,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            null_mut(),
            null_mut(),
            hinstance,
            role as usize as _,
        )
    }
}

pub(super) fn repaint_main_window_area(hwnd: HWND, area: Option<UiRect>, erase: bool) -> bool {
    WindowsMainWindowHost::new(Some(wnd_proc)).request_main_window_area_repaint(hwnd, area, erase)
}

pub(super) fn repaint_main_window(hwnd: HWND, erase: bool) {
    let _ = repaint_main_window_area(hwnd, None, erase);
}

pub(super) fn main_window_layout_dpi(hwnd: HWND) -> u32 {
    WindowsMainWindowHost::new(Some(wnd_proc)).main_window_layout_dpi(hwnd)
}

pub(super) fn main_window_client_bounds(hwnd: HWND) -> Option<UiRect> {
    WindowsMainWindowHost::new(Some(wnd_proc)).main_window_client_bounds(hwnd)
}

pub(super) fn main_window_bounds(hwnd: HWND) -> Option<UiRect> {
    WindowsMainWindowHost::new(Some(wnd_proc)).main_window_bounds(hwnd)
}

pub(super) fn track_main_pointer_leave(hwnd: HWND) -> bool {
    WindowsMainWindowHost::new(Some(wnd_proc)).track_main_pointer_leave(hwnd)
}

unsafe fn apply_main_window_noactivate_mode(hwnd: HWND, enable: bool) {
    if hwnd.is_null() {
        return;
    }
    let ex_style = platform_window::window_ex_style(hwnd);
    let desired = if enable {
        ex_style | WS_EX_NOACTIVATE
    } else {
        ex_style & !WS_EX_NOACTIVATE
    };
    if desired == ex_style {
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            (*ptr).main_window_noactivate = enable;
        }
        refresh_low_level_input_hooks();
        return;
    }
    platform_window::set_window_ex_style(hwnd, desired);
    let flags = SWP_NOMOVE
        | SWP_NOSIZE
        | SWP_NOZORDER
        | SWP_FRAMECHANGED
        | if enable { SWP_NOACTIVATE } else { 0 };
    platform_window::set_pos(hwnd, null_mut(), 0, 0, 0, 0, flags);
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        (*ptr).main_window_noactivate = enable;
    }
    refresh_low_level_input_hooks();
}

impl NativeMainWindowHost for WindowsMainWindowHost {
    type Handle = HWND;
    type AppIcon = isize;

    fn create_main_windows(
        &mut self,
        request: NativeMainWindowRequest,
    ) -> NativeMainWindowPresentation<Self::Handle> {
        unsafe {
            let hinstance = platform_window::module_handle();
            if hinstance.is_null() {
                return NativeMainWindowPresentation::Failed;
            }

            let cursor = platform_window::arrow_cursor();
            if cursor.is_null() {
                return NativeMainWindowPresentation::Failed;
            }

            for role in [WindowRole::Main, WindowRole::Quick] {
                if !self.register_window_class(role, hinstance, cursor) {
                    return NativeMainWindowPresentation::Failed;
                }
            }

            let title = to_wide(&request.title);
            let width = request.size.width.max(1);
            let height = request.size.height.max(1);
            let main = self.create_window(WindowRole::Main, &title, width, height, hinstance);
            if main.is_null() {
                return NativeMainWindowPresentation::Failed;
            }

            let quick = self.create_window(WindowRole::Quick, &title, width, height, hinstance);
            if quick.is_null() {
                platform_window::destroy(main);
                return NativeMainWindowPresentation::Failed;
            }

            platform_window::set_visible(main, request.main_visible);
            platform_window::hide(quick);
            self.apply_main_window_appearance(main);
            self.apply_main_window_appearance(quick);
            NativeMainWindowPresentation::Created(NativeMainWindowHandles { main, quick })
        }
    }

    fn apply_main_window_appearance(&mut self, handle: Self::Handle) {
        platform_appearance::set_rounded_corners(handle);
        unsafe {
            platform_appearance::apply_dark_mode_to_window(handle);
        }
    }

    fn set_main_window_app_icon(
        &mut self,
        handle: Self::Handle,
        icon: NativeAppIconResource<Self::AppIcon>,
    ) {
        platform_window::send_message(handle, WM_SETICON, ICON_SMALL as WPARAM, icon.small);
        platform_window::send_message(handle, WM_SETICON, ICON_BIG as WPARAM, icon.big);
    }

    fn hide_main_window(&mut self, handle: Self::Handle) {
        platform_window::hide(handle);
    }

    fn present_main_window(&mut self, handle: Self::Handle, mode: NativeMainWindowPresentMode) {
        match mode {
            NativeMainWindowPresentMode::ActivateAndFocus => {
                platform_window::show(handle);
                platform_window::set_pos(
                    handle,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
                );
                platform_window::set_foreground(handle);
                platform_input::set_focus(handle);
            }
            NativeMainWindowPresentMode::NoActivate => {
                platform_window::show_no_activate(handle);
                platform_window::set_pos(
                    handle,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_SHOWWINDOW,
                );
            }
        }
    }

    fn set_main_window_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        platform_window::set_pos(
            handle,
            null_mut(),
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            SWP_NOZORDER | SWP_NOACTIVATE,
        );
    }

    fn activate_main_window(&mut self, handle: Self::Handle) {
        platform_window::show(handle);
        platform_window::set_pos(
            handle,
            HWND_TOPMOST,
            0,
            0,
            0,
            0,
            SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW,
        );
        let _ = platform_window::force_foreground(handle);
    }

    fn foreground_main_window(&mut self, handle: Self::Handle) {
        platform_window::set_foreground(handle);
    }

    fn restore_main_window(&mut self, handle: Self::Handle) {
        platform_window::restore(handle);
    }

    fn close_main_window(&mut self, handle: Self::Handle) {
        platform_window::close(handle);
    }

    fn set_main_window_activation_policy(&mut self, handle: Self::Handle, allow_activation: bool) {
        unsafe {
            apply_main_window_noactivate_mode(handle, !allow_activation);
        }
    }

    fn request_main_window_close(&mut self, handle: Self::Handle) {
        platform_window::send_message(handle, WM_CLOSE, 0, 0);
    }

    fn destroy_main_window(&mut self, handle: Self::Handle) {
        platform_window::destroy(handle);
    }

    fn capture_main_pointer(&mut self, handle: Self::Handle) {
        platform_input::set_capture(handle);
    }

    fn release_main_pointer(&mut self, _handle: Self::Handle) {
        platform_input::release_capture();
    }

    fn begin_main_window_drag(&mut self, handle: Self::Handle) {
        let _ = platform_window::force_foreground(handle);
        platform_input::release_capture();
        platform_window::send_message(
            handle,
            WM_SYSCOMMAND,
            (SC_MOVE as usize | HTCAPTION as usize) as WPARAM,
            0,
        );
    }

    fn track_main_pointer_leave(&mut self, handle: Self::Handle) -> bool {
        platform_input::track_mouse_leave_and_hover(
            handle,
            platform_system_parameters::mouse_hover_time_ms(),
        )
    }

    fn request_main_window_area_repaint(
        &mut self,
        handle: Self::Handle,
        area: Option<UiRect>,
        erase: bool,
    ) -> bool {
        let rect = area.map(RECT::from);
        platform_gdi::invalidate_rect(
            handle,
            rect.as_ref().map_or(null(), |rect| rect as *const RECT),
            erase as i32,
        )
    }

    fn main_window_layout_dpi(&mut self, handle: Self::Handle) -> u32 {
        unsafe { platform_dpi::layout_dpi_for_window(handle) }
    }

    fn main_window_client_bounds(&mut self, handle: Self::Handle) -> Option<UiRect> {
        platform_window::client_rect(handle).map(Into::into)
    }

    fn main_window_bounds(&mut self, handle: Self::Handle) -> Option<UiRect> {
        platform_window::window_rect(handle).map(Into::into)
    }
}
