use super::prelude::*;

pub(super) unsafe fn handle_settings_general_selection(
    st: &mut SettingsWndState,
    control_id: isize,
    index: usize,
) {
    match control_id {
        IDC_SET_IMAGE_ROW_HEIGHT | IDC_SET_TEXT_ROW_HEIGHT | IDC_SET_FILE_ROW_HEIGHT => {
            let slot = (control_id - IDC_SET_IMAGE_ROW_HEIGHT) as usize;
            let current = [
                st.draft.image_row_height,
                st.draft.text_row_height,
                st.draft.file_row_height,
            ][slot];
            if let Some(value) = row_height_choices(slot, current).get(index).copied() {
                match slot {
                    0 => st.draft.image_row_height = value,
                    1 => st.draft.text_row_height = value,
                    _ => st.draft.file_row_height = value,
                }
                settings_set_text(st.row_height_edits[slot], &format!("{value} px"));
                repaint_settings_control(st.row_height_edits[slot]);
            }
        }
        IDC_SET_MAX => {
            let items = settings_dropdown_max_items_labels();
            if let Some(label) = items.get(index) {
                settings_set_text(st.cb_max, label);
                if let Some(max_items) = settings_dropdown_max_items_from_label_opt(label) {
                    st.draft.max_items = max_items;
                }
                repaint_settings_control(st.cb_max);
            }
        }
        IDC_SET_POSMODE => {
            let items = ["跟随鼠标", "固定位置", "上次位置"];
            if let Some(label) = items.get(index) {
                settings_set_text(st.cb_pos, label);
                repaint_settings_control(st.cb_pos);
                settings_sync_pos_fields_enabled(st);
            }
        }
        IDC_SET_PASTE_SOUND_KIND => {
            if let Some((key, _)) = PASTE_SOUND_OPTIONS.get(index) {
                st.draft.paste_success_sound_kind = (*key).to_string();
                settings_set_text(st.cb_paste_sound, &paste_sound_display(key));
                repaint_settings_control(st.cb_paste_sound);
                settings_sync_page_state(st, SettingsPage::General.index());
            }
        }
        _ => {}
    }
}
