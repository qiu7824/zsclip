use std::ffi::c_void;

use windows_sys::Win32::Foundation::{HWND, RECT};

use crate::platform::{com as platform_com, com::RawIUnknownVtbl, window as platform_window};

const S_OK_HR: i32 = 0;

#[link(name = "oleacc")]
unsafe extern "system" {
    fn AccessibleObjectFromWindow(
        hwnd: HWND,
        dw_id: i32,
        riid: *const windows_sys::core::GUID,
        ppv_object: *mut *mut c_void,
    ) -> i32;
}

#[repr(C)]
struct RawVariant {
    vt: u16,
    w_reserved1: u16,
    w_reserved2: u16,
    w_reserved3: u16,
    data: [u8; 16],
}

#[repr(C)]
struct RawIDispatchVtbl {
    base: RawIUnknownVtbl,
    get_type_info_count: usize,
    get_type_info: usize,
    get_ids_of_names: usize,
    invoke: usize,
}

#[repr(C)]
struct RawIAccessible {
    vtbl: *const RawIAccessibleVtbl,
}

#[repr(C)]
struct RawIAccessibleVtbl {
    base: RawIDispatchVtbl,
    get_acc_parent: usize,
    get_acc_child_count: usize,
    get_acc_child: usize,
    get_acc_name: usize,
    get_acc_value: usize,
    get_acc_description: usize,
    get_acc_role: usize,
    get_acc_state: usize,
    get_acc_help: usize,
    get_acc_help_topic: usize,
    get_acc_keyboard_shortcut: usize,
    get_acc_focus: usize,
    get_acc_selection: usize,
    get_acc_default_action: usize,
    acc_select: usize,
    acc_location: unsafe extern "system" fn(
        *mut c_void,
        *mut i32,
        *mut i32,
        *mut i32,
        *mut i32,
        RawVariant,
    ) -> i32,
    acc_navigate: usize,
    acc_hit_test: usize,
    acc_do_default_action: usize,
    put_acc_name: usize,
    put_acc_value: usize,
}

fn variant_child_self() -> RawVariant {
    RawVariant {
        vt: 3,
        w_reserved1: 0,
        w_reserved2: 0,
        w_reserved3: 0,
        data: [0; 16],
    }
}

pub(crate) unsafe fn caret_rect(hwnd: HWND) -> Option<RECT> {
    const OBJID_CARET_V: i32 = -8;
    const IID_IACCESSIBLE_RAW: windows_sys::core::GUID =
        windows_sys::core::GUID::from_u128(0x618736e0_3c3d_11cf_810c_00aa00389b71);

    if !platform_window::exists(hwnd) {
        return None;
    }

    let mut obj: *mut c_void = std::ptr::null_mut();
    let hr = AccessibleObjectFromWindow(hwnd, OBJID_CARET_V, &IID_IACCESSIBLE_RAW, &mut obj);
    if hr != S_OK_HR || obj.is_null() {
        return None;
    }

    let acc = obj as *mut RawIAccessible;
    let mut left = 0i32;
    let mut top = 0i32;
    let mut width = 0i32;
    let mut height = 0i32;
    let child = variant_child_self();
    let location_hr =
        ((*(*acc).vtbl).acc_location)(obj, &mut left, &mut top, &mut width, &mut height, child);
    platform_com::release_raw(obj);
    if location_hr != S_OK_HR || width <= 0 || height <= 0 {
        return None;
    }

    Some(RECT {
        left,
        top,
        right: left + width,
        bottom: top + height,
    })
}
