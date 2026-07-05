use super::prelude::*;

pub(super) unsafe fn settings_plugin_show_enable(
    st: &mut SettingsWndState,
    hwnd: HWND,
    visible: bool,
    enabled: bool,
) {
    if hwnd.is_null() {
        return;
    }
    st.ui.set_control_visible(hwnd, visible);
    settings_show_enable(hwnd, visible, enabled);
}

pub(super) unsafe fn settings_plugin_move_control(
    st: &mut SettingsWndState,
    hwnd: HWND,
    x: i32,
    y: i32,
    w: i32,
    h: i32,
) {
    if hwnd.is_null() {
        return;
    }
    st.ui.set_control_bounds(hwnd, x, y, w, h);
    let original = UiRect::new(x, y, x + w, y + h);
    let parent = platform_window::parent(hwnd);
    let bounds = if !st.viewport_hwnd.is_null() && parent == st.viewport_hwnd {
        let settings_hwnd = platform_window::root_ancestor(parent);
        if let Some(crc) = settings_window_client_bounds(settings_hwnd).map(RECT::from) {
            settings_viewport_child_control_bounds(
                original,
                st.content_scroll_y,
                settings_viewport_rect(&crc),
            )
        } else {
            UiRect::new(
                x,
                y - st.content_scroll_y,
                x + w,
                y - st.content_scroll_y + h,
            )
        }
    } else {
        UiRect::new(
            x,
            y - st.content_scroll_y,
            x + w,
            y - st.content_scroll_y + h,
        )
    };
    settings_host_set_bounds(hwnd, bounds);
}

pub(super) unsafe fn settings_plugin_move_toggle_row(
    st: &mut SettingsWndState,
    label: HWND,
    toggle: HWND,
    section: &SettingsFormSectionLayout,
    row: i32,
) {
    let layout = section.toggle_row_layout(row);
    settings_plugin_move_control(
        st,
        label,
        layout.label_rect.left,
        layout.label_rect.top,
        layout.label_rect.right - layout.label_rect.left,
        layout.label_rect.bottom - layout.label_rect.top,
    );
    settings_plugin_move_control(
        st,
        toggle,
        layout.toggle_rect.left,
        layout.toggle_rect.top,
        layout.toggle_rect.right - layout.toggle_rect.left,
        layout.toggle_rect.bottom - layout.toggle_rect.top,
    );
}

pub(super) unsafe fn settings_plugin_move_labeled_field(
    st: &mut SettingsWndState,
    label: HWND,
    field: HWND,
    section: &SettingsFormSectionLayout,
    row: i32,
    label_h: i32,
    field_w: i32,
    field_h: i32,
) {
    let layout = section.labeled_field_layout(row, label_h, field_w, field_h);
    settings_plugin_move_control(
        st,
        label,
        layout.label_rect.left,
        layout.label_rect.top,
        layout.label_rect.right - layout.label_rect.left,
        layout.label_rect.bottom - layout.label_rect.top,
    );
    settings_plugin_move_control(
        st,
        field,
        layout.field_rect.left,
        layout.field_rect.top,
        layout.field_rect.right - layout.field_rect.left,
        layout.field_rect.bottom - layout.field_rect.top,
    );
}
