use std::cmp::{max, min};
use windows_sys::Win32::Foundation::{POINT, RECT};

use crate::app::{AppSettings, WIN_H, WIN_W};
use crate::win_system_ui::{nearest_monitor_work_rect_for_point, nearest_monitor_rect_for_point};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MainWindowPosMode {
    Mouse,
    Fixed,
    Last,
    Center,
}

pub(crate) fn parse_main_window_pos_mode(mode: &str) -> MainWindowPosMode {
    match mode {
        "fixed" => MainWindowPosMode::Fixed,
        "last" => MainWindowPosMode::Last,
        "mouse" => MainWindowPosMode::Mouse,
        _ => MainWindowPosMode::Center,
    }
}

fn clamp_to_rect(x: i32, y: i32, rc: &RECT) -> (i32, i32) {
    (
        max(rc.left, min(x, max(rc.left, rc.right - WIN_W))),
        max(rc.top, min(y, max(rc.top, rc.bottom - WIN_H))),
    )
}

fn mouse_anchor(settings: &AppSettings, cursor: POINT) -> (i32, i32) {
    (
        cursor.x + settings.show_mouse_dx,
        cursor.y + settings.show_mouse_dy,
    )
}

pub(crate) fn resolve_main_window_position(
    settings: &AppSettings,
    by_hotkey: bool,
    _sw: i32,
    _sh: i32,
    cursor: POINT,
) -> (i32, i32) {
    let requested = parse_main_window_pos_mode(settings.show_pos_mode.as_str());
    let (x, y) = match requested {
        MainWindowPosMode::Fixed => (settings.show_fixed_x, settings.show_fixed_y),
        MainWindowPosMode::Last if settings.last_window_x >= 0 && settings.last_window_y >= 0 => {
            (settings.last_window_x, settings.last_window_y)
        }
        MainWindowPosMode::Mouse => mouse_anchor(settings, cursor),
        MainWindowPosMode::Last if by_hotkey => mouse_anchor(settings, cursor),
        MainWindowPosMode::Center if by_hotkey => mouse_anchor(settings, cursor),
        _ => {
            let work = unsafe { nearest_monitor_work_rect_for_point(cursor) };
            (
                work.left + ((work.right - work.left - WIN_W) / 2),
                work.top + ((work.bottom - work.top - WIN_H) / 3),
            )
        }
    };
    let anchor = POINT { x, y };
    let work = unsafe { nearest_monitor_work_rect_for_point(anchor) };
    let monitor = unsafe { nearest_monitor_rect_for_point(anchor) };
    let clamp_rect = RECT {
        left: max(work.left, monitor.left),
        top: max(work.top, monitor.top),
        right: min(work.right, monitor.right),
        bottom: min(work.bottom, monitor.bottom),
    };
    clamp_to_rect(x, y, &clamp_rect)
}
