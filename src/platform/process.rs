use std::path::Path;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut core::ffi::c_void;
    fn GetCurrentProcessId() -> u32;
    fn GetCurrentThreadId() -> u32;
    fn GetLastError() -> u32;
    fn CreateMutexW(
        lp_attributes: *const core::ffi::c_void,
        b_initial_owner: i32,
        lp_name: *const u16,
    ) -> *mut core::ffi::c_void;
    fn OpenProcess(
        dwdesiredaccess: u32,
        binherithandle: i32,
        dwprocessid: u32,
    ) -> *mut core::ffi::c_void;
    fn QueryFullProcessImageNameW(
        hprocess: *mut core::ffi::c_void,
        dwflags: u32,
        lpexename: *mut u16,
        lpdwsize: *mut u32,
    ) -> i32;
    fn CloseHandle(hobject: *mut core::ffi::c_void) -> i32;
}

#[link(name = "psapi")]
unsafe extern "system" {
    fn EmptyWorkingSet(hprocess: *mut core::ffi::c_void) -> i32;
}

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const ERROR_ALREADY_EXISTS: u32 = 183;

pub(crate) fn current_process_id() -> u32 {
    unsafe { GetCurrentProcessId() }
}

pub(crate) fn current_thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

pub(crate) fn create_named_mutex(name: &str) -> (isize, bool) {
    let name: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let handle = CreateMutexW(core::ptr::null(), 0, name.as_ptr());
        (handle as isize, GetLastError() == ERROR_ALREADY_EXISTS)
    }
}

pub(crate) fn process_image_name(pid: u32) -> String {
    if pid == 0 {
        return String::new();
    }
    unsafe {
        let handle = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if handle.is_null() {
            return String::new();
        }
        let mut size: u32 = 512;
        let mut buf = vec![0u16; size as usize + 1];
        let ok = QueryFullProcessImageNameW(handle, 0, buf.as_mut_ptr(), &mut size);
        let _ = CloseHandle(handle);
        if ok == 0 || size == 0 {
            return String::new();
        }
        let path = String::from_utf16_lossy(&buf[..size as usize]);
        Path::new(&path)
            .file_name()
            .map(|name| name.to_string_lossy().to_lowercase())
            .unwrap_or_default()
    }
}

pub(crate) fn trim_current_working_set() {
    unsafe {
        let process = GetCurrentProcess();
        if !process.is_null() {
            let _ = EmptyWorkingSet(process);
        }
    }
}
