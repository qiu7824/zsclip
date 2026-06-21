use super::prelude::*;

pub(super) fn settings_control_screen_rect_or_empty(hwnd: HWND) -> RECT {
    settings_host_screen_bounds(hwnd)
        .map(RECT::from)
        .unwrap_or(RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        })
}

pub(super) unsafe fn close_settings_dropdown_popup(st: &mut SettingsWndState) {
    if !st.dropdown_popup.is_null() {
        if settings_dropdown_popup_exists(st.dropdown_popup) {
            destroy_settings_dropdown_popup(st.dropdown_popup);
        }
        st.dropdown_popup = null_mut();
    }
}

pub(super) fn present_settings_dropdown_popup(
    owner: HWND,
    control_id: isize,
    anchor: &RECT,
    items: &[&str],
    selected: usize,
    width: i32,
) -> HWND {
    let mut host = WindowsSettingsDropdownHost;
    let request = NativeSettingsDropdownRequest {
        owner,
        control_id,
        anchor: (*anchor).into(),
        items: items.iter().map(|item| (*item).to_string()).collect(),
        selected,
        width,
    };
    match host.present_settings_dropdown(request) {
        NativeSettingsDropdownPresentation::Created(handle) => handle,
        NativeSettingsDropdownPresentation::Failed => null_mut(),
    }
}

pub(super) fn destroy_settings_dropdown_popup(handle: HWND) {
    let mut host = WindowsSettingsDropdownHost;
    host.destroy_settings_dropdown(handle);
}

pub(super) fn settings_dropdown_popup_exists(handle: HWND) -> bool {
    !handle.is_null() && WindowsWindowIdentityHost::new().exists(handle)
}

pub(super) fn repaint_settings_control(hwnd: HWND) {
    let _ = settings_host_request_repaint(hwnd);
}
