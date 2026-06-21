use std::ffi::c_void;

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

pub(crate) unsafe fn release_raw(ptr: *mut c_void) {
    if ptr.is_null() {
        return;
    }
    let unk = ptr as *mut RawIUnknown;
    ((*(*unk).vtbl).release)(ptr);
}
