use std::ffi::c_void;
use std::sync::atomic::{AtomicU32, Ordering};

pub use crate::settings_registry::{SettingsCtrlReg, SettingsPage, SettingsUiRegistry};
pub use crate::settings_render::{
    draw_settings_nav_item, draw_settings_page_cards, draw_settings_page_content, nav_divider_x,
    settings_title_rect_win,
};
pub use crate::settings_ui_host::{
    create_settings_component, create_settings_edit, create_settings_label, create_settings_label_auto,
    create_settings_listbox, create_settings_password_edit, draw_settings_button_component,
    draw_settings_toggle_component, settings_child_visible, settings_dropdown_index_for_max_items,
    settings_dropdown_index_for_pos_mode, settings_dropdown_label_for_max_items,
    settings_dropdown_label_for_pos_mode, settings_dropdown_max_items_from_label,
    settings_dropdown_pos_mode_from_label, settings_safe_paint_rect, settings_viewport_mask_rect,
    settings_viewport_rect, show_settings_dropdown_popup, SettingsComponentKind,
    WM_SETTINGS_DROPDOWN_SELECTED,
};
use windows_sys::Win32::{
    Foundation::HWND,
    System::Ole::DROPEFFECT,
    UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW},
};

use crate::ui::is_dark_mode;
use crate::win_system_params::{
    DRAGDROP_S_CANCEL_HR, DRAGDROP_S_DROP_HR, DRAGDROP_S_USEDEFAULTCURSORS_HR,
    E_NOINTERFACE_HR, E_POINTER_HR, IID_IDROPSOURCE_RAW, IID_IUNKNOWN_RAW, MK_LBUTTON_FLAG,
    S_OK_HR,
};

#[link(name = "dwmapi")]
unsafe extern "system" {
    fn DwmSetWindowAttribute(
        hwnd: HWND,
        dwattribute: u32,
        pvattribute: *const c_void,
        cbattribute: u32,
    ) -> i32;
}

pub(crate) fn to_wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(std::iter::once(0)).collect()
}

pub(crate) unsafe fn init_dark_mode_for_process() {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    let lib = LoadLibraryW(to_wide("uxtheme.dll").as_ptr());
    if lib.is_null() {
        return;
    }

    type FnSetMode = unsafe extern "system" fn(i32) -> i32;
    if let Some(f) = core::mem::transmute::<_, Option<FnSetMode>>(GetProcAddress(lib, 135usize as _))
    {
        f(if is_dark_mode() { 2 } else { 0 });
    }

    type FnFlush = unsafe extern "system" fn();
    if let Some(f) = core::mem::transmute::<_, Option<FnFlush>>(GetProcAddress(lib, 136usize as _))
    {
        f();
    }

    FreeLibrary(lib);
}

pub(crate) unsafe fn apply_dark_mode_to_window(hwnd: HWND) {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    if is_dark_mode() {
        let val: u32 = 1;
        DwmSetWindowAttribute(hwnd, 20, &val as *const u32 as _, 4);
    }

    let lib = LoadLibraryW(to_wide("uxtheme.dll").as_ptr());
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

pub(crate) unsafe fn apply_theme_to_menu(hmenu: *mut c_void) {
    use windows_sys::Win32::Foundation::FreeLibrary;
    use windows_sys::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};

    if hmenu.is_null() {
        return;
    }

    let lib = LoadLibraryW(to_wide("uxtheme.dll").as_ptr());
    if lib.is_null() {
        return;
    }

    type FnFlush = unsafe extern "system" fn();
    if let Some(f) = core::mem::transmute::<_, Option<FnFlush>>(GetProcAddress(lib, 136usize as _))
    {
        f();
    }

    FreeLibrary(lib);
}

pub(crate) unsafe fn apply_window_corner_preference(hwnd: HWND) {
    const DWMWA_WINDOW_CORNER_PREFERENCE: u32 = 33;
    const DWMWCP_ROUND: u32 = 2;

    let pref: u32 = DWMWCP_ROUND;
    let _ = DwmSetWindowAttribute(
        hwnd,
        DWMWA_WINDOW_CORNER_PREFERENCE,
        &pref as *const _ as *const c_void,
        core::mem::size_of::<u32>() as u32,
    );
}

pub(crate) unsafe fn get_window_text(hwnd: HWND) -> String {
    let len = GetWindowTextLengthW(hwnd);
    let mut buf = vec![0u16; (len as usize) + 1];
    if !buf.is_empty() {
        GetWindowTextW(hwnd, buf.as_mut_ptr(), buf.len() as i32);
    }
    String::from_utf16_lossy(&buf)
        .trim_end_matches('\0')
        .to_string()
}

pub(crate) fn get_x_lparam(lp: isize) -> i32 {
    (lp as i16) as i32
}

pub(crate) fn get_y_lparam(lp: isize) -> i32 {
    ((lp >> 16) as i16) as i32
}

#[repr(C)]
pub(crate) struct RawIUnknown {
    pub(crate) vtbl: *const RawIUnknownVtbl,
}

#[repr(C)]
pub(crate) struct RawIUnknownVtbl {
    pub(crate) query_interface: unsafe extern "system" fn(
        *mut c_void,
        *const windows_sys::core::GUID,
        *mut *mut c_void,
    ) -> windows_sys::core::HRESULT,
    pub(crate) add_ref: unsafe extern "system" fn(*mut c_void) -> u32,
    pub(crate) release: unsafe extern "system" fn(*mut c_void) -> u32,
}

#[repr(C)]
struct RawDropSourceVtbl {
    base: RawIUnknownVtbl,
    query_continue_drag:
        unsafe extern "system" fn(*mut c_void, i32, u32) -> windows_sys::core::HRESULT,
    give_feedback: unsafe extern "system" fn(*mut c_void, DROPEFFECT) -> windows_sys::core::HRESULT,
}

#[repr(C)]
struct SimpleDropSource {
    vtbl: *const RawDropSourceVtbl,
    refs: AtomicU32,
}

fn guid_eq(a: &windows_sys::core::GUID, b: &windows_sys::core::GUID) -> bool {
    a.data1 == b.data1 && a.data2 == b.data2 && a.data3 == b.data3 && a.data4 == b.data4
}

unsafe extern "system" fn drop_source_query_interface(
    this: *mut c_void,
    riid: *const windows_sys::core::GUID,
    out: *mut *mut c_void,
) -> windows_sys::core::HRESULT {
    if out.is_null() {
        return E_POINTER_HR;
    }
    *out = std::ptr::null_mut();
    if riid.is_null() {
        return E_NOINTERFACE_HR;
    }
    if guid_eq(&*riid, &IID_IUNKNOWN_RAW) || guid_eq(&*riid, &IID_IDROPSOURCE_RAW) {
        *out = this;
        drop_source_add_ref(this);
        S_OK_HR
    } else {
        E_NOINTERFACE_HR
    }
}

unsafe extern "system" fn drop_source_add_ref(this: *mut c_void) -> u32 {
    let obj = this as *mut SimpleDropSource;
    (*obj).refs.fetch_add(1, Ordering::Relaxed) + 1
}

unsafe extern "system" fn drop_source_release(this: *mut c_void) -> u32 {
    let obj = this as *mut SimpleDropSource;
    let remain = (*obj).refs.fetch_sub(1, Ordering::Release) - 1;
    if remain == 0 {
        std::sync::atomic::fence(Ordering::Acquire);
        drop(Box::from_raw(obj));
    }
    remain
}

unsafe extern "system" fn drop_source_query_continue_drag(
    _this: *mut c_void,
    escape_pressed: i32,
    key_state: u32,
) -> windows_sys::core::HRESULT {
    if escape_pressed != 0 {
        return DRAGDROP_S_CANCEL_HR;
    }
    if key_state & MK_LBUTTON_FLAG == 0 {
        return DRAGDROP_S_DROP_HR;
    }
    S_OK_HR
}

unsafe extern "system" fn drop_source_give_feedback(
    _this: *mut c_void,
    _effect: DROPEFFECT,
) -> windows_sys::core::HRESULT {
    DRAGDROP_S_USEDEFAULTCURSORS_HR
}

static DROP_SOURCE_VTBL: RawDropSourceVtbl = RawDropSourceVtbl {
    base: RawIUnknownVtbl {
        query_interface: drop_source_query_interface,
        add_ref: drop_source_add_ref,
        release: drop_source_release,
    },
    query_continue_drag: drop_source_query_continue_drag,
    give_feedback: drop_source_give_feedback,
};

pub(crate) unsafe fn release_raw_com(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let unk = ptr as *mut RawIUnknown;
    ((*(*unk).vtbl).release)(ptr);
}

pub(crate) unsafe fn create_drop_source() -> *mut c_void {
    Box::into_raw(Box::new(SimpleDropSource {
        vtbl: &DROP_SOURCE_VTBL,
        refs: AtomicU32::new(1),
    })) as *mut c_void
}
