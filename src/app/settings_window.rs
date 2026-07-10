use super::prelude::*;

pub(super) unsafe extern "system" fn settings_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if msg == WM_SETTINGS_SCROLL_FRAME {
        return handle_settings_scroll_frame(hwnd);
    }
    if let Some(event) = settings_window_host_event_from_message(msg, wparam, lparam) {
        if let Some(result) = dispatch_settings_ui_event(hwnd, msg, wparam, lparam, event) {
            return result;
        }
    }

    match msg {
        WM_CREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            let parent_hwnd = cs.lpCreateParams as HWND;
            let st = create_settings_window_state(hwnd, parent_hwnd);
            platform_window::set_user_data(hwnd, Box::into_raw(st) as isize);
            platform_appearance::set_rounded_corners(hwnd);
            platform_appearance::apply_dark_mode_to_window(hwnd);
            0
        }
        WM_DRAWITEM => draw_settings_window_item(hwnd, lparam),
        WM_CTLCOLORSTATIC => settings_control_color(hwnd, wparam, SettingsControlColorRole::Static),
        WM_CTLCOLOREDIT => settings_control_color(hwnd, wparam, SettingsControlColorRole::Edit),
        WM_CTLCOLORLISTBOX => settings_control_color(hwnd, wparam, SettingsControlColorRole::List),
        WM_ERASEBKGND => 1,
        WM_PAINT => {
            paint_settings_window(hwnd);
            0
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}
