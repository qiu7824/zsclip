use std::ffi::c_void;

use windows_sys::Win32::{
    Foundation::{FreeLibrary, HWND, RECT},
    System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

const HKEY_CURRENT_USER: isize = -2147483647i32 as isize;
const KEY_READ: u32 = 0x20019;
const REG_DWORD: u32 = 4;

#[link(name = "advapi32")]
unsafe extern "system" {
    fn RegOpenKeyExW(
        hkey: isize,
        lpsubkey: *const u16,
        uloptions: u32,
        samdesired: u32,
        phkresult: *mut isize,
    ) -> i32;
    fn RegQueryValueExW(
        hkey: isize,
        lpvaluename: *const u16,
        lpreserved: *mut u32,
        lptype: *mut u32,
        lpdata: *mut u8,
        lpcbdata: *mut u32,
    ) -> i32;
    fn RegCloseKey(hkey: isize) -> i32;
}

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: HWND,
        dwattribute: u32,
        pvattribute: *const c_void,
        cbattribute: u32,
    ) -> i32;
    fn DwmGetWindowAttribute(
        hwnd: HWND,
        dwattribute: u32,
        pvattribute: *mut c_void,
        cbattribute: u32,
    ) -> i32;
    fn DwmGetColorizationColor(pcr_colorization: *mut u32, pf_opaque_blend: *mut i32) -> i32;
}

#[link(name = "uxtheme")]
unsafe extern "system" {
    fn SetWindowTheme(hwnd: HWND, pszsubappid: *const u16, pszsubidlist: *const u16) -> i32;
}

fn wide_null(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) fn system_ui_text_font_family() -> &'static str {
    crate::platform::system_parameters::system_ui_text_font_family()
}

fn rgb(r: u8, g: u8, b: u8) -> u32 {
    (r as u32) | ((g as u32) << 8) | ((b as u32) << 16)
}

pub(crate) fn system_accent() -> u32 {
    unsafe {
        let mut color = 0u32;
        let mut opaque = 0i32;
        if DwmGetColorizationColor(&mut color, &mut opaque) == 0 {
            let r = ((color >> 16) & 0xFF) as u8;
            let g = ((color >> 8) & 0xFF) as u8;
            let b = (color & 0xFF) as u8;
            return rgb(r, g, b);
        }
    }
    rgb(0, 120, 212)
}

pub(crate) fn is_dark_mode() -> bool {
    let key_path = wide_null("Software\\Microsoft\\Windows\\CurrentVersion\\Themes\\Personalize");
    let val_name = wide_null("AppsUseLightTheme");
    unsafe {
        let mut hkey = 0isize;
        if RegOpenKeyExW(HKEY_CURRENT_USER, key_path.as_ptr(), 0, KEY_READ, &mut hkey) != 0 {
            return false;
        }
        let mut data = 0u32;
        let mut data_size = 4u32;
        let mut reg_type = 0u32;
        let ret = RegQueryValueExW(
            hkey,
            val_name.as_ptr(),
            core::ptr::null_mut(),
            &mut reg_type,
            &mut data as *mut u32 as *mut u8,
            &mut data_size,
        );
        RegCloseKey(hkey);
        ret == 0 && reg_type == REG_DWORD && data == 0
    }
}

pub(crate) fn set_window_theme(hwnd: HWND, theme: &str) -> bool {
    if hwnd.is_null() {
        return false;
    }
    let theme = wide_null(theme);
    unsafe { SetWindowTheme(hwnd, theme.as_ptr(), core::ptr::null()) != 0 }
}

pub(crate) fn set_dwm_u32_attribute(hwnd: HWND, attribute: u32, value: u32) -> bool {
    if hwnd.is_null() {
        return false;
    }
    unsafe {
        DwmSetWindowAttribute(
            hwnd,
            attribute,
            &value as *const u32 as *const c_void,
            core::mem::size_of::<u32>() as u32,
        ) == 0
    }
}

pub(crate) fn set_dark_frame(hwnd: HWND, enabled: bool) {
    if enabled {
        let _ = set_dwm_u32_attribute(hwnd, 20, 1);
        let _ = set_dwm_u32_attribute(hwnd, 19, 1);
    }
}

pub(crate) fn set_rounded_corners(hwnd: HWND) {
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_ROUND: u32 = 2;
    let _ = set_dwm_u32_attribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND);
}

pub(crate) unsafe fn init_dark_mode_for_process() {
    let lib = LoadLibraryW(wide_null("uxtheme.dll").as_ptr());
    if lib.is_null() {
        return;
    }

    type FnSetMode = unsafe extern "system" fn(i32) -> i32;
    if let Some(f) =
        core::mem::transmute::<_, Option<FnSetMode>>(GetProcAddress(lib, 135usize as _))
    {
        f(if is_dark_mode() { 2 } else { 0 });
    }

    flush_menu_themes_raw(lib);
    FreeLibrary(lib);
}

pub(crate) unsafe fn apply_dark_mode_to_window(hwnd: HWND) {
    set_dark_frame(hwnd, is_dark_mode());

    let lib = LoadLibraryW(wide_null("uxtheme.dll").as_ptr());
    if lib.is_null() {
        return;
    }

    type FnAllow = unsafe extern "system" fn(HWND, i32) -> i32;
    if let Some(f) = core::mem::transmute::<_, Option<FnAllow>>(GetProcAddress(lib, 133usize as _))
    {
        f(hwnd, if is_dark_mode() { 1 } else { 0 });
    }

    FreeLibrary(lib);
}

unsafe fn flush_menu_themes_raw(lib: *mut core::ffi::c_void) {
    type FnFlush = unsafe extern "system" fn();
    if let Some(f) = core::mem::transmute::<_, Option<FnFlush>>(GetProcAddress(lib, 136usize as _))
    {
        f();
    }
}

pub(crate) unsafe fn apply_theme_to_menu(menu: *mut core::ffi::c_void) {
    if menu.is_null() {
        return;
    }

    let lib = LoadLibraryW(wide_null("uxtheme.dll").as_ptr());
    if lib.is_null() {
        return;
    }

    flush_menu_themes_raw(lib);
    FreeLibrary(lib);
}

pub(crate) fn extended_frame_bounds(hwnd: HWND) -> Option<RECT> {
    const DWMWA_EXTENDED_FRAME_BOUNDS: u32 = 9;
    if hwnd.is_null() {
        return None;
    }
    let mut rect = RECT {
        left: 0,
        top: 0,
        right: 0,
        bottom: 0,
    };
    let ok = unsafe {
        DwmGetWindowAttribute(
            hwnd,
            DWMWA_EXTENDED_FRAME_BOUNDS,
            &mut rect as *mut _ as *mut c_void,
            core::mem::size_of::<RECT>() as u32,
        ) == 0
    };
    if ok && rect.right > rect.left && rect.bottom > rect.top {
        Some(rect)
    } else {
        None
    }
}
