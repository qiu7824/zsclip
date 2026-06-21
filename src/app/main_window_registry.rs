use super::prelude::*;

#[derive(Default)]
struct WindowHosts {
    main: isize,
    quick: isize,
}

static WINDOW_HOSTS: OnceLock<Mutex<WindowHosts>> = OnceLock::new();

fn window_hosts() -> &'static Mutex<WindowHosts> {
    WINDOW_HOSTS.get_or_init(|| Mutex::new(WindowHosts::default()))
}

pub(super) fn set_window_host(role: WindowRole, hwnd: HWND) {
    if let Ok(mut hosts) = window_hosts().lock() {
        match role {
            WindowRole::Main => hosts.main = hwnd as isize,
            WindowRole::Quick => hosts.quick = hwnd as isize,
        }
    }
}

pub(super) fn clear_window_host(role: WindowRole, hwnd: HWND) {
    if let Ok(mut hosts) = window_hosts().lock() {
        let slot = match role {
            WindowRole::Main => &mut hosts.main,
            WindowRole::Quick => &mut hosts.quick,
        };
        if *slot == hwnd as isize {
            *slot = 0;
        }
    }
}

pub(super) fn window_host_hwnds() -> [HWND; 2] {
    window_hosts()
        .lock()
        .ok()
        .map(|hosts| [hosts.main as HWND, hosts.quick as HWND])
        .unwrap_or([null_mut(), null_mut()])
}

pub(super) fn window_host_hwnds_try() -> [HWND; 2] {
    window_hosts()
        .try_lock()
        .ok()
        .map(|hosts| [hosts.main as HWND, hosts.quick as HWND])
        .unwrap_or([null_mut(), null_mut()])
}

pub(super) unsafe fn set_ignore_clipboard_for_all_hosts(duration_ms: u64) {
    let until = Instant::now() + std::time::Duration::from_millis(duration_ms);
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() {
            continue;
        }
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            (*ptr).ignore_clipboard_until = Some(until);
        }
    }
}

pub(super) unsafe fn skip_next_clipboard_update_for_all_hosts() {
    for hwnd in window_host_hwnds() {
        if hwnd.is_null() {
            continue;
        }
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            (*ptr).skip_next_clipboard_update_once = true;
        }
    }
}

pub(super) fn is_app_window(hwnd: HWND) -> bool {
    if hwnd.is_null() {
        return false;
    }
    window_host_hwnds()
        .into_iter()
        .any(|host| !host.is_null() && host == hwnd)
}

pub(crate) fn main_window_hwnd() -> HWND {
    window_hosts()
        .lock()
        .ok()
        .map(|hosts| hosts.main as HWND)
        .unwrap_or(null_mut())
}

pub(crate) fn quick_window_hwnd() -> HWND {
    window_hosts()
        .lock()
        .ok()
        .map(|hosts| hosts.quick as HWND)
        .unwrap_or(null_mut())
}

pub(crate) unsafe fn get_state_ptr(hwnd: HWND) -> *mut AppState {
    platform_window::user_data(hwnd) as *mut AppState
}

pub(super) unsafe fn get_state_mut(hwnd: HWND) -> Option<&'static mut AppState> {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        None
    } else {
        Some(&mut *ptr)
    }
}
