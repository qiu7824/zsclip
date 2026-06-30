use super::prelude::*;
use crate::app_core::{
    NativeAppIconResource, NativeMainWindowHandles, NativeMainWindowHost,
    NativeMainWindowPresentMode, NativeMainWindowPresentation, NativeMainWindowRequest,
    NativeWindowOptions, UiRect,
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
        options: &NativeWindowOptions,
    ) -> HWND {
        let (ex_style, style) = win32_main_window_styles(role, options);
        let create_params = WindowCreateParams::new(role, options.min_size);
        platform_window::create_window_ex(
            ex_style,
            to_wide(role.class_name()).as_ptr(),
            title.as_ptr(),
            style,
            CW_USEDEFAULT,
            CW_USEDEFAULT,
            width,
            height,
            null_mut(),
            null_mut(),
            hinstance,
            &create_params as *const WindowCreateParams as _,
        )
    }
}

fn win32_main_window_styles(role: WindowRole, options: &NativeWindowOptions) -> (u32, u32) {
    let mut ex_style = 0;
    if !options.decorations {
        ex_style |= WS_EX_TOOLWINDOW;
    }
    if options.always_on_top {
        ex_style |= WS_EX_TOPMOST;
    }
    if matches!(role, WindowRole::Quick) {
        ex_style |= WS_EX_TOOLWINDOW | WS_EX_TOPMOST | WS_EX_NOACTIVATE;
    }

    let style = if options.decorations {
        let mut style = WS_OVERLAPPED | WS_CAPTION | WS_SYSMENU | WS_MINIMIZEBOX | WS_CLIPCHILDREN;
        if options.resizable {
            style |= WS_MAXIMIZEBOX | WS_THICKFRAME;
        }
        style
    } else {
        WS_POPUP | WS_CLIPCHILDREN
    };

    (ex_style, style)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn standard_options_map_to_decorated_resizable_win32_window() {
        let (ex_style, style) =
            win32_main_window_styles(WindowRole::Main, &NativeWindowOptions::standard());

        assert_eq!(ex_style & WS_EX_TOPMOST, 0);
        assert_eq!(ex_style & WS_EX_TOOLWINDOW, 0);
        assert_ne!(style & WS_CAPTION, 0);
        assert_ne!(style & WS_SYSMENU, 0);
        assert_ne!(style & WS_THICKFRAME, 0);
        assert_ne!(style & WS_MAXIMIZEBOX, 0);
    }

    #[test]
    fn tool_window_options_preserve_current_popup_topmost_shape() {
        let (ex_style, style) =
            win32_main_window_styles(WindowRole::Main, &NativeWindowOptions::tool_window());

        assert_ne!(ex_style & WS_EX_TOPMOST, 0);
        assert_ne!(ex_style & WS_EX_TOOLWINDOW, 0);
        assert_ne!(style & WS_POPUP, 0);
        assert_eq!(style & WS_CAPTION, 0);
        assert_eq!(style & WS_THICKFRAME, 0);
    }

    #[test]
    fn window_create_params_preserve_role_and_min_size_for_win32_create() {
        let params = WindowCreateParams::new(
            WindowRole::Main,
            Some(UiSize {
                width: 640,
                height: 420,
            }),
        );

        let decoded = WindowCreateParams::from_create_param(&params as *const _ as isize);
        assert_eq!(decoded.role, WindowRole::Main);
        assert_eq!(
            decoded.min_size,
            Some(UiSize {
                width: 640,
                height: 420
            })
        );
    }
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
            let main = self.create_window(
                WindowRole::Main,
                &title,
                width,
                height,
                hinstance,
                &request.options,
            );
            if main.is_null() {
                return NativeMainWindowPresentation::Failed;
            }

            let quick_options = NativeWindowOptions::tool_window();
            let quick = self.create_window(
                WindowRole::Quick,
                &title,
                width,
                height,
                hinstance,
                &quick_options,
            );
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
