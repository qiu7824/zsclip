#[link(name = "kernel32")]
unsafe extern "system" {
    fn GlobalAlloc(uflags: u32, dwbytes: usize) -> *mut core::ffi::c_void;
    fn GlobalLock(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn GlobalUnlock(hmem: *mut core::ffi::c_void) -> i32;
    fn GlobalFree(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
    fn GlobalSize(hmem: *mut core::ffi::c_void) -> usize;
}

pub(crate) fn global_alloc(flags: u32, bytes: usize) -> *mut core::ffi::c_void {
    unsafe { GlobalAlloc(flags, bytes) }
}

pub(crate) fn global_lock(handle: *mut core::ffi::c_void) -> *mut core::ffi::c_void {
    unsafe { GlobalLock(handle) }
}

pub(crate) fn global_unlock(handle: *mut core::ffi::c_void) {
    unsafe {
        GlobalUnlock(handle);
    }
}

pub(crate) fn global_free(handle: *mut core::ffi::c_void) {
    unsafe {
        GlobalFree(handle);
    }
}

pub(crate) fn global_size(handle: *mut core::ffi::c_void) -> usize {
    unsafe { GlobalSize(handle) }
}
