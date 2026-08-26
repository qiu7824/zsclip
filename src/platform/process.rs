use std::path::{Component, Path, Prefix};

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetCurrentProcess() -> *mut core::ffi::c_void;
    fn GetCurrentProcessId() -> u32;
    fn GetCurrentThreadId() -> u32;
    fn GetLastError() -> u32;
    fn GetDriveTypeW(lprootpathname: *const u16) -> u32;
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
    fn WaitForSingleObject(hhandle: *mut core::ffi::c_void, dwmilliseconds: u32) -> u32;
    fn QueryFullProcessImageNameW(
        hprocess: *mut core::ffi::c_void,
        dwflags: u32,
        lpexename: *mut u16,
        lpdwsize: *mut u32,
    ) -> i32;
    fn CloseHandle(hobject: *mut core::ffi::c_void) -> i32;
}

#[link(name = "advapi32")]
unsafe extern "system" {
    fn OpenProcessToken(
        process_handle: *mut core::ffi::c_void,
        desired_access: u32,
        token_handle: *mut *mut core::ffi::c_void,
    ) -> i32;
    fn GetTokenInformation(
        token_handle: *mut core::ffi::c_void,
        token_information_class: i32,
        token_information: *mut core::ffi::c_void,
        token_information_length: u32,
        return_length: *mut u32,
    ) -> i32;
}

#[link(name = "psapi")]
unsafe extern "system" {
    fn EmptyWorkingSet(hprocess: *mut core::ffi::c_void) -> i32;
}

const PROCESS_QUERY_LIMITED_INFORMATION: u32 = 0x1000;
const SYNCHRONIZE: u32 = 0x0010_0000;
const WAIT_OBJECT_0: u32 = 0;
const INFINITE: u32 = u32::MAX;
const ERROR_ALREADY_EXISTS: u32 = 183;
const DRIVE_FIXED: u32 = 3;
const TOKEN_QUERY: u32 = 0x0008;
const TOKEN_ELEVATION_CLASS: i32 = 20;

#[repr(C)]
struct TokenElevation {
    token_is_elevated: u32,
}

fn process_handle_is_elevated(process: *mut core::ffi::c_void) -> Option<bool> {
    if process.is_null() {
        return None;
    }
    unsafe {
        let mut token = core::ptr::null_mut();
        if OpenProcessToken(process, TOKEN_QUERY, &mut token) == 0 || token.is_null() {
            return None;
        }
        let mut elevation = TokenElevation {
            token_is_elevated: 0,
        };
        let mut returned = 0u32;
        let ok = GetTokenInformation(
            token,
            TOKEN_ELEVATION_CLASS,
            &mut elevation as *mut TokenElevation as *mut core::ffi::c_void,
            core::mem::size_of::<TokenElevation>() as u32,
            &mut returned,
        ) != 0;
        let _ = CloseHandle(token);
        ok.then_some(elevation.token_is_elevated != 0)
    }
}

fn elevation_boundary_blocks_input(current: Option<bool>, target: Option<bool>) -> bool {
    matches!((current, target), (Some(false), Some(true)))
}

pub(crate) fn current_process_id() -> u32 {
    unsafe { GetCurrentProcessId() }
}

pub(crate) fn current_thread_id() -> u32 {
    unsafe { GetCurrentThreadId() }
}

pub(crate) fn path_is_on_fixed_drive(path: &Path) -> bool {
    let drive_letter = match path.components().next() {
        Some(Component::Prefix(prefix)) => match prefix.kind() {
            Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
            _ => return false,
        },
        _ => return false,
    };
    let root = [drive_letter as u16, b':' as u16, b'\\' as u16, 0];
    unsafe { GetDriveTypeW(root.as_ptr()) == DRIVE_FIXED }
}

pub(crate) fn wait_for_process_exit(pid: u32) -> bool {
    if pid == 0 || pid == current_process_id() {
        return false;
    }
    unsafe {
        let handle = OpenProcess(SYNCHRONIZE, 0, pid);
        if handle.is_null() {
            return true;
        }
        let result = WaitForSingleObject(handle, INFINITE);
        let _ = CloseHandle(handle);
        result == WAIT_OBJECT_0
    }
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

pub(crate) fn current_process_is_elevated() -> Option<bool> {
    unsafe { process_handle_is_elevated(GetCurrentProcess()) }
}

pub(crate) fn process_is_elevated(pid: u32) -> Option<bool> {
    if pid == 0 {
        return None;
    }
    if pid == current_process_id() {
        return current_process_is_elevated();
    }
    unsafe {
        let process = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
        if process.is_null() {
            return None;
        }
        let elevated = process_handle_is_elevated(process);
        let _ = CloseHandle(process);
        elevated
    }
}

pub(crate) fn process_has_higher_elevation(pid: u32) -> bool {
    elevation_boundary_blocks_input(current_process_is_elevated(), process_is_elevated(pid))
}

pub(crate) fn trim_current_working_set() -> bool {
    unsafe {
        let process = GetCurrentProcess();
        !process.is_null() && EmptyWorkingSet(process) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixed_drive_probe_rejects_remote_unc_paths() {
        let system_drive = std::env::var("SystemDrive").unwrap_or_else(|_| "C:".to_string());
        assert!(path_is_on_fixed_drive(Path::new(&format!(
            "{}\\",
            system_drive.trim_end_matches('\\')
        ))));
        assert!(!path_is_on_fixed_drive(Path::new(
            r"\\server\redirected-drive"
        )));
    }

    #[test]
    fn elevation_boundary_only_blocks_lower_to_higher_input() {
        assert!(elevation_boundary_blocks_input(Some(false), Some(true)));
        assert!(!elevation_boundary_blocks_input(Some(true), Some(false)));
        assert!(!elevation_boundary_blocks_input(Some(false), Some(false)));
        assert!(!elevation_boundary_blocks_input(None, Some(true)));
    }
}
