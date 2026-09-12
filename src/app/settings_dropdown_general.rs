use super::prelude::*;

pub(super) unsafe fn open_settings_general_dropdown(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
) -> bool {
    match control_id {
        IDC_SET_IMAGE_ROW_HEIGHT | IDC_SET_TEXT_ROW_HEIGHT | IDC_SET_FILE_ROW_HEIGHT => {
            let slot = (control_id - IDC_SET_IMAGE_ROW_HEIGHT) as usize;
            let values = row_height_choices(
                slot,
                [
                    st.draft.image_row_height,
                    st.draft.text_row_height,
                    st.draft.file_row_height,
                ][slot],
            );
            let labels = values.iter().map(|v| format!("{v} px")).collect::<Vec<_>>();
            let refs = labels.iter().map(String::as_str).collect::<Vec<_>>();
            let selected = labels
                .iter()
                .position(|label| *label == settings_host_text(st.row_height_edits[slot]))
                .unwrap_or(0);
            let rc = settings_control_screen_rect_or_empty(st.row_height_edits[slot]);
            st.dropdown_popup =
                present_settings_dropdown_popup(hwnd, control_id, &rc, &refs, selected, 150);
            true
        }
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

pub(super) fn row_height_choices(slot: usize, current: i32) -> Vec<i32> {
    let mut values = if slot == 0 {
        vec![80, 100, 120, 132, 160, 200, 240, 280, 320]
    } else {
        vec![32, 36, 40, 44, 48, 56, 64, 80, 100, 120, 160]
    };
    let value = current.clamp(
        if slot == 0 { 80 } else { 32 },
        if slot == 0 { 320 } else { 160 },
    );
    if !values.contains(&value) {
        values.push(value);
        values.sort_unstable();
    }
    values
}
