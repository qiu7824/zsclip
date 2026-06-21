use std::ffi::c_void;
use std::path::{Path, PathBuf};
use std::ptr::{null, null_mut};
use std::sync::atomic::{AtomicU32, Ordering};

use windows_sys::Win32::{
    System::Ole::{DoDragDrop, OleInitialize, OleUninitialize, DROPEFFECT, DROPEFFECT_COPY},
    UI::Shell::{ILClone, ILCreateFromPathW, ILFindLastID, ILFree, SHCreateDataObject},
};

use crate::platform::com::{release_raw, RawIUnknownVtbl};
use crate::platform::string::to_wide;

const MK_LBUTTON_FLAG: u32 = 0x0001;
const S_OK_HR: windows_sys::core::HRESULT = 0;
const E_NOINTERFACE_HR: windows_sys::core::HRESULT = 0x80004002u32 as i32;
const E_POINTER_HR: windows_sys::core::HRESULT = 0x80004003u32 as i32;
const DRAGDROP_S_DROP_HR: windows_sys::core::HRESULT = 0x00040100;
const DRAGDROP_S_CANCEL_HR: windows_sys::core::HRESULT = 0x00040101;
const DRAGDROP_S_USEDEFAULTCURSORS_HR: windows_sys::core::HRESULT = 0x00040102;
const RPC_E_CHANGED_MODE_HR: windows_sys::core::HRESULT = 0x80010106u32 as i32;

const IID_IUNKNOWN_RAW: windows_sys::core::GUID =
    windows_sys::core::GUID::from_u128(0x00000000_0000_0000_c000_000000000046);
const IID_IDROPSOURCE_RAW: windows_sys::core::GUID =
    windows_sys::core::GUID::from_u128(0x00000121_0000_0000_c000_000000000046);
const IID_IDATAOBJECT_RAW: windows_sys::core::GUID =
    windows_sys::core::GUID::from_u128(0x0000010e_0000_0000_c000_000000000046);

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
    *out = null_mut();
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

unsafe fn create_drop_source() -> *mut c_void {
    Box::into_raw(Box::new(SimpleDropSource {
        vtbl: &DROP_SOURCE_VTBL,
        refs: AtomicU32::new(1),
    })) as *mut c_void
}

unsafe fn create_shell_data_object(paths: &[PathBuf]) -> Option<*mut c_void> {
    if paths.is_empty() {
        return None;
    }
    let parent = paths.first()?.parent()?.to_path_buf();
    if paths.iter().any(|p| p.parent() != Some(parent.as_path())) {
        return None;
    }

    let parent_wide = to_wide(path_to_text(&parent).as_ref());
    let parent_pidl = ILCreateFromPathW(parent_wide.as_ptr());
    if parent_pidl.is_null() {
        return None;
    }

    let mut child_pidls: Vec<*mut windows_sys::Win32::UI::Shell::Common::ITEMIDLIST> = Vec::new();
    for path in paths {
        let wide = to_wide(path_to_text(path).as_ref());
        let abs_pidl = ILCreateFromPathW(wide.as_ptr());
        if abs_pidl.is_null() {
            continue;
        }
        let child = ILClone(ILFindLastID(abs_pidl));
        ILFree(abs_pidl);
        if !child.is_null() {
            child_pidls.push(child);
        }
    }

    if child_pidls.is_empty() {
        ILFree(parent_pidl);
        return None;
    }

    let mut data_obj: *mut c_void = null_mut();
    let hr = SHCreateDataObject(
        parent_pidl,
        child_pidls.len() as u32,
        child_pidls.as_ptr() as *const *const windows_sys::Win32::UI::Shell::Common::ITEMIDLIST,
        null_mut(),
        &IID_IDATAOBJECT_RAW,
        &mut data_obj,
    );
    for child in child_pidls {
        ILFree(child);
    }
    ILFree(parent_pidl);
    if hr >= 0 && !data_obj.is_null() {
        Some(data_obj)
    } else {
        None
    }
}

fn path_to_text(path: &Path) -> std::borrow::Cow<'_, str> {
    path.to_string_lossy()
}

pub(crate) unsafe fn begin_file_drag(paths: &[PathBuf]) -> bool {
    let init_hr = OleInitialize(null());
    if init_hr < 0 && init_hr != RPC_E_CHANGED_MODE_HR {
        return false;
    }
    let Some(data_obj) = create_shell_data_object(paths) else {
        if init_hr >= 0 {
            OleUninitialize();
        }
        return false;
    };
    let drop_source = create_drop_source();
    if drop_source.is_null() {
        release_raw(data_obj);
        if init_hr >= 0 {
            OleUninitialize();
        }
        return false;
    }

    let mut effect: DROPEFFECT = 0;
    let drag_hr = DoDragDrop(data_obj, drop_source, DROPEFFECT_COPY, &mut effect);
    release_raw(data_obj);
    release_raw(drop_source);
    if init_hr >= 0 {
        OleUninitialize();
    }
    let _ = effect;
    drag_hr >= 0
}
