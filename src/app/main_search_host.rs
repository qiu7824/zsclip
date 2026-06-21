use crate::app_core::{
    native_host_search_input_specs, NativeHostSearchControlAction, NativeMainSearchControlHost,
    NativeMainSearchControlPresentation, NativeMainSearchControlRequest,
    NativeMainSearchStylePresentation, NativeMainSearchStyleRequest, UiRect,
};
use crate::platform::gdi as platform_gdi;
use crate::platform::input as platform_input;
use crate::platform::string::to_wide;
use crate::platform::window as platform_window;
use std::ptr::null;
use windows_sys::Win32::{
    Foundation::{HWND, LPARAM, WPARAM},
    Graphics::Gdi::DEFAULT_GUI_FONT,
    UI::WindowsAndMessaging::{ES_AUTOHSCROLL, WM_SETFONT, WS_CHILD, WS_TABSTOP},
};

const MAIN_SEARCH_EM_SETMARGINS: u32 = 0x00D3;
const MAIN_SEARCH_EC_LEFTMARGIN: usize = 0x0001;
const MAIN_SEARCH_EC_RIGHTMARGIN: usize = 0x0002;

#[derive(Clone, Copy, Default)]
pub(super) struct WindowsMainSearchControlHost;

impl WindowsMainSearchControlHost {
    pub(super) const fn new() -> Self {
        Self
    }

    pub(super) fn search_control_request_from_native_spec(
        owner: HWND,
        id: i64,
        bounds: UiRect,
        visible: bool,
    ) -> NativeMainSearchControlRequest<HWND> {
        let spec = native_host_search_input_specs()
            .into_iter()
            .find(|spec| spec.id == "main.search")
            .expect("main search native component spec must exist");
        assert_eq!(spec.action, NativeHostSearchControlAction::UpdateText);
        NativeMainSearchControlRequest {
            owner,
            id,
            bounds,
            visible,
        }
    }
}

impl NativeMainSearchControlHost for WindowsMainSearchControlHost {
    type Owner = HWND;
    type Handle = HWND;
    type StyleResource = *mut core::ffi::c_void;

    fn create_search_control(
        &mut self,
        request: NativeMainSearchControlRequest<Self::Owner>,
    ) -> NativeMainSearchControlPresentation<Self::Handle> {
        let hinstance = platform_window::module_handle();
        if hinstance.is_null() {
            return NativeMainSearchControlPresentation::Failed;
        }
        let hwnd = platform_window::create_window_ex(
            0,
            to_wide("EDIT").as_ptr(),
            to_wide("").as_ptr(),
            WS_CHILD | WS_TABSTOP | (ES_AUTOHSCROLL as u32),
            request.bounds.left,
            request.bounds.top,
            request.bounds.right - request.bounds.left,
            request.bounds.bottom - request.bounds.top,
            request.owner,
            request.id as usize as _,
            hinstance,
            null(),
        );
        if hwnd.is_null() {
            return NativeMainSearchControlPresentation::Failed;
        }
        platform_window::set_visible(hwnd, request.visible);
        platform_window::send_message(
            hwnd,
            MAIN_SEARCH_EM_SETMARGINS,
            (MAIN_SEARCH_EC_LEFTMARGIN | MAIN_SEARCH_EC_RIGHTMARGIN) as WPARAM,
            0,
        );
        NativeMainSearchControlPresentation::Created(hwnd)
    }

    fn apply_search_style(
        &mut self,
        request: NativeMainSearchStyleRequest<Self::Handle, Self::StyleResource>,
    ) -> NativeMainSearchStylePresentation<Self::StyleResource> {
        let created = platform_gdi::create_font_w(
            -request.font_px.max(1),
            0,
            0,
            0,
            400,
            0,
            0,
            0,
            1,
            0,
            0,
            5,
            0,
            to_wide(&request.font_family).as_ptr(),
        ) as *mut core::ffi::c_void;
        let font: *mut core::ffi::c_void = if created.is_null() {
            platform_gdi::get_stock_object(DEFAULT_GUI_FONT) as _
        } else {
            created
        };
        platform_window::send_message(request.handle, WM_SETFONT, font as WPARAM, 1 as LPARAM);
        if let Some(old_font) = request.previous_resource {
            if !old_font.is_null() && old_font != platform_gdi::get_stock_object(DEFAULT_GUI_FONT) {
                platform_gdi::delete_object(old_font as _);
            }
        }
        NativeMainSearchStylePresentation::Applied((!created.is_null()).then_some(created))
    }

    fn release_search_style_resource(&mut self, resource: Self::StyleResource) {
        if !resource.is_null() && resource != platform_gdi::get_stock_object(DEFAULT_GUI_FONT) {
            platform_gdi::delete_object(resource as _);
        }
    }

    fn set_search_bounds(&mut self, handle: Self::Handle, bounds: UiRect) {
        platform_window::move_window(
            handle,
            bounds.left,
            bounds.top,
            bounds.right - bounds.left,
            bounds.bottom - bounds.top,
            true,
        );
    }

    fn set_search_visible(&mut self, handle: Self::Handle, visible: bool) {
        platform_window::set_visible(handle, visible);
    }

    fn search_text(&self, handle: Self::Handle) -> String {
        platform_window::text(handle)
    }

    fn set_search_text(&mut self, handle: Self::Handle, text: &str) {
        platform_window::set_text(handle, text);
    }

    fn focus_search(&mut self, handle: Self::Handle) {
        platform_input::set_focus(handle);
    }
}
