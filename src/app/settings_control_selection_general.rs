use super::prelude::*;

pub(super) unsafe fn handle_settings_general_selection(
    st: &mut SettingsWndState,
    control_id: isize,
    index: usize,
) {
    match control_id {
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
