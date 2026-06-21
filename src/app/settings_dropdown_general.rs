use super::prelude::*;

pub(super) unsafe fn open_settings_general_dropdown(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
) -> bool {
    match control_id {
        IDC_SET_MAX => {
            let rc = settings_control_screen_rect_or_empty(st.cb_max);
            let current = settings_dropdown_index_for_max_items(
                settings_dropdown_max_items_from_label(&settings_host_text(st.cb_max)),
            );
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_MAX,
                &rc,
                &settings_dropdown_max_items_labels(),
                current,
                180,
            );
            true
        }
        IDC_SET_POSMODE => {
            let rc = settings_control_screen_rect_or_empty(st.cb_pos);
            let current = settings_dropdown_index_for_pos_mode(
                &settings_dropdown_pos_mode_from_label(&settings_host_text(st.cb_pos)),
            );
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_POSMODE,
                &rc,
                &["跟随鼠标", "固定位置", "上次位置"],
                current,
                180,
            );
            true
        }
        IDC_SET_PASTE_SOUND_KIND => {
            let rc = settings_control_screen_rect_or_empty(st.cb_paste_sound);
            let current = PASTE_SOUND_OPTIONS
                .iter()
                .position(|(key, _)| {
                    paste_sound_display(key) == settings_host_text(st.cb_paste_sound)
                })
                .unwrap_or(0);
            let labels_owned: Vec<String> = PASTE_SOUND_OPTIONS
                .iter()
                .map(|(key, _)| paste_sound_display(key))
                .collect();
            let labels: Vec<&str> = labels_owned.iter().map(|s| s.as_str()).collect();
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_PASTE_SOUND_KIND,
                &rc,
                &labels,
                current,
                220,
            );
            true
        }
        _ => false,
    }
}
