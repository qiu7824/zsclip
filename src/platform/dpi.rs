use std::ptr::null_mut;
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::{
    Foundation::{FreeLibrary, HWND, POINT},
    System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

use crate::platform::{gdi as platform_gdi, monitor as platform_monitor};

static DPI_AWARENESS_CACHE: AtomicU32 = AtomicU32::new(0);

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) unsafe fn init_process_awareness() {
    let user32 = LoadLibraryW(wide_null("user32.dll").as_ptr());
    if !user32.is_null() {
        type FnSetCtx = unsafe extern "system" fn(isize) -> i32;
        if let Some(f) = core::mem::transmute::<_, Option<FnSetCtx>>(GetProcAddress(
            user32,
            b"SetProcessDpiAwarenessContext\0".as_ptr(),
        )) {
            const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4isize;
            if f(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) != 0 {
                DPI_AWARENESS_CACHE.store(0, Ordering::Relaxed);
                FreeLibrary(user32);
                return;
            }
        }
        FreeLibrary(user32);
    }

    let shcore = LoadLibraryW(wide_null("shcore.dll").as_ptr());
    if !shcore.is_null() {
        type FnSetAwareness = unsafe extern "system" fn(i32) -> i32;
        if let Some(f) = core::mem::transmute::<_, Option<FnSetAwareness>>(GetProcAddress(
            shcore,
            b"SetProcessDpiAwareness\0".as_ptr(),
        )) {
            const PROCESS_PER_MONITOR_DPI_AWARE: i32 = 2;
            if f(PROCESS_PER_MONITOR_DPI_AWARE) == 0 {
                DPI_AWARENESS_CACHE.store(0, Ordering::Relaxed);
                FreeLibrary(shcore);
                return;
            }
        }
        FreeLibrary(shcore);
    }

    let user32 = LoadLibraryW(wide_null("user32.dll").as_ptr());
    if !user32.is_null() {
        type FnSetAware = unsafe extern "system" fn() -> i32;
        if let Some(f) = core::mem::transmute::<_, Option<FnSetAware>>(GetProcAddress(
            user32,
            b"SetProcessDPIAware\0".as_ptr(),
        )) {
            let _ = f();
            DPI_AWARENESS_CACHE.store(0, Ordering::Relaxed);
            FreeLibrary(user32);
            return;
        }
        FreeLibrary(user32);
    }
}

unsafe fn detect_per_monitor_awareness() -> bool {
    let user32 = LoadLibraryW(wide_null("user32.dll").as_ptr());
    if !user32.is_null() {
        type FnGetThreadCtx = unsafe extern "system" fn() -> isize;
        type FnAreCtxEqual = unsafe extern "system" fn(isize, isize) -> i32;
        type FnGetAwareness = unsafe extern "system" fn(isize) -> i32;
        let get_ctx = core::mem::transmute::<_, Option<FnGetThreadCtx>>(GetProcAddress(
            user32,
            b"GetThreadDpiAwarenessContext\0".as_ptr(),
        ));
        if let Some(get_ctx) = get_ctx {
            let ctx = get_ctx();
            if ctx != 0 {
                if let Some(eq) = core::mem::transmute::<_, Option<FnAreCtxEqual>>(GetProcAddress(
                    user32,
                    b"AreDpiAwarenessContextsEqual\0".as_ptr(),
                )) {
                    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE: isize = -3isize;
                    const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4isize;
                    if eq(ctx, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2) != 0
                        || eq(ctx, DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE) != 0
                    {
                        FreeLibrary(user32);
                        return true;
                    }
                }
                if let Some(get_awareness) = core::mem::transmute::<_, Option<FnGetAwareness>>(
                    GetProcAddress(user32, b"GetAwarenessFromDpiAwarenessContext\0".as_ptr()),
                ) {
                    const DPI_AWARENESS_PER_MONITOR_AWARE: i32 = 2;
                    if get_awareness(ctx) == DPI_AWARENESS_PER_MONITOR_AWARE {
                        FreeLibrary(user32);
                        return true;
                    }
                }
            }
        }
        FreeLibrary(user32);
    }

    let shcore = LoadLibraryW(wide_null("shcore.dll").as_ptr());
    if !shcore.is_null() {
        type FnGetProcessAwareness =
            unsafe extern "system" fn(*mut core::ffi::c_void, *mut i32) -> i32;
        if let Some(get_awareness) = core::mem::transmute::<_, Option<FnGetProcessAwareness>>(
            GetProcAddress(shcore, b"GetProcessDpiAwareness\0".as_ptr()),
        ) {
            let mut awareness = -1i32;
            const PROCESS_PER_MONITOR_DPI_AWARE: i32 = 2;
            if get_awareness(null_mut(), &mut awareness) == 0
                && awareness == PROCESS_PER_MONITOR_DPI_AWARE
            {
                FreeLibrary(shcore);
                return true;
            }
        }
        FreeLibrary(shcore);
    }
    false
}

pub(crate) unsafe fn is_per_monitor_aware() -> bool {
    match DPI_AWARENESS_CACHE.load(Ordering::Relaxed) {
        1 => false,
        2 => true,
        _ => {
            let per_monitor = detect_per_monitor_awareness();
            DPI_AWARENESS_CACHE.store(if per_monitor { 2 } else { 1 }, Ordering::Relaxed);
            per_monitor
        }
    }
}

pub(crate) unsafe fn window_dpi(hwnd: HWND) -> u32 {
    if !hwnd.is_null() {
        let user32 = LoadLibraryW(wide_null("user32.dll").as_ptr());
        if !user32.is_null() {
            type FnGetDpiForWindow = unsafe extern "system" fn(HWND) -> u32;
            type FnGetDpiForSystem = unsafe extern "system" fn() -> u32;
            if let Some(f) = core::mem::transmute::<_, Option<FnGetDpiForWindow>>(GetProcAddress(
                user32,
                b"GetDpiForWindow\0".as_ptr(),
            )) {
                let dpi = f(hwnd);
                if dpi != 0 {
                    FreeLibrary(user32);
                    return dpi;
                }
            }
            if let Some(f) = core::mem::transmute::<_, Option<FnGetDpiForSystem>>(GetProcAddress(
                user32,
                b"GetDpiForSystem\0".as_ptr(),
            )) {
                let dpi = f();
                if dpi != 0 {
                    FreeLibrary(user32);
                    return dpi;
                }
            }
            FreeLibrary(user32);
        }
    }

    let screen_dc = platform_gdi::get_dc(core::ptr::null_mut());
    if !screen_dc.is_null() {
        let dpi = platform_gdi::get_device_caps(screen_dc, 88);
        platform_gdi::release_dc(core::ptr::null_mut(), screen_dc);
        if dpi > 0 {
            return dpi as u32;
        }
    }
    96
}

unsafe fn monitor_dpi_for_handle(monitor: platform_monitor::MonitorHandle) -> Option<u32> {
    if monitor.is_null() {
        return None;
    }
    type FnSetThreadCtx = unsafe extern "system" fn(isize) -> isize;
    let mut user32 = null_mut();
    let mut restore_thread_ctx = 0isize;
    let mut set_thread_ctx: Option<FnSetThreadCtx> = None;
    if !is_per_monitor_aware() {
        user32 = LoadLibraryW(wide_null("user32.dll").as_ptr());
        if !user32.is_null() {
            set_thread_ctx = core::mem::transmute::<_, Option<FnSetThreadCtx>>(GetProcAddress(
                user32,
                b"SetThreadDpiAwarenessContext\0".as_ptr(),
            ));
            if let Some(set_ctx) = set_thread_ctx {
                const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2: isize = -4isize;
                const DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE: isize = -3isize;
                restore_thread_ctx = set_ctx(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE_V2);
                if restore_thread_ctx == 0 {
                    restore_thread_ctx = set_ctx(DPI_AWARENESS_CONTEXT_PER_MONITOR_AWARE);
                }
            }
        }
    }
    let shcore = LoadLibraryW(wide_null("shcore.dll").as_ptr());
    let mut result = None;
    if !shcore.is_null() {
        type FnGetDpiForMonitor = unsafe extern "system" fn(
            platform_monitor::MonitorHandle,
            i32,
            *mut u32,
            *mut u32,
        ) -> i32;
        if let Some(f) = core::mem::transmute::<_, Option<FnGetDpiForMonitor>>(GetProcAddress(
            shcore,
            b"GetDpiForMonitor\0".as_ptr(),
        )) {
            let mut dpi_x = 0u32;
            let mut dpi_y = 0u32;
            const MDT_EFFECTIVE_DPI: i32 = 0;
            if f(monitor, MDT_EFFECTIVE_DPI, &mut dpi_x, &mut dpi_y) == 0 && dpi_x != 0 {
                result = Some(dpi_x);
            }
        }
        FreeLibrary(shcore);
    }
    if restore_thread_ctx != 0 {
        if let Some(set_ctx) = set_thread_ctx {
            let _ = set_ctx(restore_thread_ctx);
        }
    }
    if !user32.is_null() {
        FreeLibrary(user32);
    }
    result
}

pub(crate) unsafe fn monitor_dpi_for_point(point: POINT) -> u32 {
    let monitor = platform_monitor::nearest_handle_for_point(point);
    monitor_dpi_for_handle(monitor).unwrap_or_else(|| window_dpi(null_mut()))
}

pub(crate) unsafe fn monitor_dpi_for_window(hwnd: HWND) -> u32 {
    if !hwnd.is_null() {
        let monitor = platform_monitor::nearest_handle_for_window(hwnd);
        if let Some(dpi) = monitor_dpi_for_handle(monitor) {
            return dpi;
        }
    }
    window_dpi(hwnd)
}

pub(crate) unsafe fn layout_dpi_for_point(point: POINT) -> u32 {
    if is_per_monitor_aware() {
        monitor_dpi_for_point(point)
    } else {
        window_dpi(null_mut())
    }
}

pub(crate) unsafe fn layout_dpi_for_window(hwnd: HWND) -> u32 {
    if is_per_monitor_aware() {
        monitor_dpi_for_window(hwnd)
    } else {
        window_dpi(hwnd)
    }
}

pub(crate) unsafe fn scale_for_window(hwnd: HWND, value: i32) -> i32 {
    let dpi = layout_dpi_for_window(hwnd).max(96) as i32;
    ((value * dpi) + 48) / 96
}
