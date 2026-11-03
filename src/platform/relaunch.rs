//! Explicit, one-session UAC restart. No startup policy or security setting is changed.
use super::string::to_wide;
use std::ffi::{OsStr, OsString};
use windows_sys::Win32::{
    Foundation::HWND,
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::{
            MessageBoxW, IDYES, MB_DEFBUTTON2, MB_ICONQUESTION, MB_YESNO, SW_SHOWNORMAL,
        },
    },
};

const RESTART_ARG: &str = "--elevated-restart";

fn quote_argument(value: &OsStr) -> OsString {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    let mut output = vec![b'"' as u16];
    let mut slashes = 0;
    for ch in value.encode_wide() {
        if ch == b'\\' as u16 {
            slashes += 1;
            continue;
        }
        output.extend(std::iter::repeat_n(
            b'\\' as u16,
            if ch == b'"' as u16 {
                slashes * 2 + 1
            } else {
                slashes
            },
        ));
        output.push(ch);
        slashes = 0;
    }
    output.extend(std::iter::repeat_n(b'\\' as u16, slashes * 2));
    output.push(b'"' as u16);
    OsString::from_wide(&output)
}

pub(crate) fn confirm_session_restart(owner: HWND, title: &str, message: &str) -> bool {
    unsafe {
        MessageBoxW(
            owner,
            to_wide(message).as_ptr(),
            to_wide(title).as_ptr(),
            MB_YESNO | MB_ICONQUESTION | MB_DEFBUTTON2,
        ) == IDYES
    }
}

pub(crate) fn instance_mutex_name() -> String {
    if let Ok(value) = std::env::var("ZSCLIP_SESSION_MUTEX") {
        return value;
    }
    std::env::var_os("ZSCLIP_DATA_DIR")
        .map(|path| {
            format!(
                "Local\\ZSClipData-{:x}",
                md5::compute(path.to_string_lossy().as_bytes())
            )
        })
        .unwrap_or_else(|| "Global\\ZsClipSingleInstanceV2".to_string())
}

pub(crate) fn request_session_restart(
    owner: HWND,
    data_directory: &std::path::Path,
) -> Result<(), String> {
    use std::os::windows::ffi::OsStrExt;
    let executable = std::env::current_exe().map_err(|e| e.to_string())?;
    let mut parameters = OsString::from(format!(
        "{RESTART_ARG} {}",
        super::process::current_process_id()
    ));
    // Pin both the resolved profile and original instance identity. UAC can
    // change the working directory and environment; neither may select a new DB.
    let directory = std::fs::canonicalize(data_directory).map_err(|e| e.to_string())?;
    parameters.push(" ");
    parameters.push(quote_argument(directory.as_os_str()));
    parameters.push(" ");
    parameters.push(quote_argument(OsStr::new(&instance_mutex_name())));
    let executable: Vec<u16> = executable.as_os_str().encode_wide().chain([0]).collect();
    let parameters: Vec<u16> = parameters.encode_wide().chain([0]).collect();
    let result = unsafe {
        ShellExecuteW(
            owner,
            to_wide("runas").as_ptr(),
            executable.as_ptr(),
            parameters.as_ptr(),
            core::ptr::null(),
            SW_SHOWNORMAL,
        )
    } as isize;
    if result > 32 {
        Ok(())
    } else {
        Err(format!("Windows/UAC ({result})"))
    }
}

pub(crate) fn prepare_session_restart() -> bool {
    let mut args = std::env::args_os().skip(1);
    if args.next().as_deref() != Some(OsStr::new(RESTART_ARG)) {
        return true;
    }
    let Some(pid) = args
        .next()
        .and_then(|arg| arg.to_str().and_then(|s| s.parse().ok()))
    else {
        return false;
    };
    let Some(directory) = args.next() else {
        return false;
    };
    let Some(mutex) = args.next() else {
        return false;
    };
    if args.next().is_some()
        || !std::path::Path::new(&directory).is_absolute()
        || super::process::current_process_is_elevated() != Some(true)
        || !super::process::wait_for_process_exit_bounded(pid)
    {
        return false;
    }
    std::env::set_var("ZSCLIP_DATA_DIR", directory);
    std::env::set_var("ZSCLIP_SESSION_MUTEX", mutex);
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn profile_argument_round_trips_spaces_quotes_and_trailing_slashes() {
        assert_eq!(
            quote_argument(OsStr::new("C:\\My Data\\")),
            OsString::from("\"C:\\My Data\\\\\"")
        );
        assert_eq!(
            quote_argument(OsStr::new("a\"b")),
            OsString::from("\"a\\\"b\"")
        );
    }
}
