use std::cmp::{max, min};
use windows_sys::Win32::Foundation::POINT;

use crate::app::{AppSettings, WIN_H, WIN_W};

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

fn clamp_to_screen(x: i32, y: i32, sw: i32, sh: i32) -> (i32, i32) {
    (
        max(0, min(x, max(0, sw - WIN_W))),
        max(0, min(y, max(0, sh - WIN_H))),
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
    sw: i32,
    sh: i32,
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
        _ => ((sw - WIN_W) / 2, (sh - WIN_H) / 3),
    };
    clamp_to_screen(x, y, sw, sh)
}
