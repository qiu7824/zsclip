use std::os::windows::ffi::OsStrExt;
use std::path::Path;

use windows_sys::Win32::{
    Foundation::FreeLibrary,
    System::LibraryLoader::{GetProcAddress, LoadLibraryW},
};

pub(crate) struct DynamicLibrary {
    handle: *mut core::ffi::c_void,
}

impl DynamicLibrary {
    pub(crate) fn load(path: &Path) -> Option<Self> {
        let wide: Vec<u16> = path
            .as_os_str()
            .encode_wide()
            .chain(std::iter::once(0))
            .collect();
        let handle = unsafe { LoadLibraryW(wide.as_ptr()) };
        if handle.is_null() {
            None
        } else {
            Some(Self { handle })
        }
    }

    pub(crate) unsafe fn symbol<T: Sized>(&self, names: &[&[u8]]) -> Option<T> {
        for name in names {
            let proc = GetProcAddress(self.handle, name.as_ptr());
            if let Some(proc) = proc {
                return Some(std::mem::transmute_copy(&proc));
            }
        }
        None
    }
}

impl Drop for DynamicLibrary {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe {
                FreeLibrary(self.handle);
            }
            self.handle = core::ptr::null_mut();
        }
    }
}
