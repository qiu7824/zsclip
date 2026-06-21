use std::ffi::c_void;

use windows_sys::Win32::{
    Foundation::{HWND, POINT, RECT},
    Graphics::Gdi::{MonitorFromPoint, MonitorFromWindow, MONITOR_DEFAULTTONEAREST},
    UI::WindowsAndMessaging::{SM_CXSCREEN, SM_CYSCREEN},
};

use crate::platform::window as platform_window;

pub(crate) type MonitorHandle = *mut c_void;

#[repr(C)]
struct RawMonitorInfo {
    cb_size: u32,
    rc_monitor: RECT,
    rc_work: RECT,
    dw_flags: u32,
}

#[link(name = "user32")]
unsafe extern "system" {
    fn GetMonitorInfoW(hmonitor: MonitorHandle, lpmi: *mut RawMonitorInfo) -> i32;
}

fn fallback_primary_rect() -> RECT {
    RECT {
        left: 0,
        top: 0,
        right: platform_window::system_metric(SM_CXSCREEN),
        bottom: platform_window::system_metric(SM_CYSCREEN),
    }
}

fn monitor_info(handle: MonitorHandle) -> Option<RawMonitorInfo> {
    if handle.is_null() {
        return None;
    }
    let mut info = RawMonitorInfo {
        cb_size: core::mem::size_of::<RawMonitorInfo>() as u32,
        rc_monitor: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        rc_work: RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        },
        dw_flags: 0,
    };
    if unsafe { GetMonitorInfoW(handle, &mut info) } != 0 {
        Some(info)
    } else {
        None
    }
}

pub(crate) fn nearest_handle_for_point(point: POINT) -> MonitorHandle {
    unsafe { MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST) }
}

pub(crate) fn nearest_handle_for_window(hwnd: HWND) -> MonitorHandle {
    unsafe { MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) }
}

pub(crate) fn nearest_work_rect_for_point(point: POINT) -> RECT {
    monitor_info(nearest_handle_for_point(point))
        .map(|info| info.rc_work)
        .unwrap_or_else(fallback_primary_rect)
}

pub(crate) fn nearest_work_rect_for_window(hwnd: HWND) -> RECT {
    monitor_info(nearest_handle_for_window(hwnd))
        .map(|info| info.rc_work)
        .unwrap_or_else(fallback_primary_rect)
}

pub(crate) fn nearest_rect_for_point(point: POINT) -> RECT {
    monitor_info(nearest_handle_for_point(point))
        .map(|info| info.rc_monitor)
        .unwrap_or_else(fallback_primary_rect)
}

pub(crate) fn nearest_rect_for_window(hwnd: HWND) -> RECT {
    monitor_info(nearest_handle_for_window(hwnd))
        .map(|info| info.rc_monitor)
        .unwrap_or_else(fallback_primary_rect)
}
