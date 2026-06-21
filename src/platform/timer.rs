use windows_sys::Win32::{
    Foundation::HWND,
    UI::WindowsAndMessaging::{KillTimer, SetTimer},
};

pub(crate) fn start(hwnd: HWND, timer_id: usize, period_ms: u32) {
    unsafe {
        SetTimer(hwnd, timer_id, period_ms, None);
    }
}

pub(crate) fn stop(hwnd: HWND, timer_id: usize) {
    unsafe {
        KillTimer(hwnd, timer_id);
    }
}

pub(crate) fn start_flagged(hwnd: HWND, timer_id: usize, period_ms: u32, flag: &mut bool) {
    *flag = true;
    start(hwnd, timer_id, period_ms);
}

pub(crate) fn stop_flagged(hwnd: HWND, timer_id: usize, flag: &mut bool) {
    stop(hwnd, timer_id);
    *flag = false;
}
