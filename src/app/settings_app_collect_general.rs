use super::prelude::*;
use crate::win_system_ui::settings_host_text;

pub(super) unsafe fn settings_collect_general_to_draft(st: &mut SettingsWndState) {
    if st.ui.is_built(SettingsPage::General.index()) && !st.cb_max.is_null() {
        if let Some(max_items) =
            settings_dropdown_max_items_from_label_opt(&settings_host_text(st.cb_max))
        {
            st.draft.max_items = max_items;
        }
    }
    if st.ui.is_built(SettingsPage::General.index()) {
        let old = [
            st.draft.image_row_height,
            st.draft.text_row_height,
            st.draft.file_row_height,
        ];
        let mut values = old;
        for index in 0..3 {
            if !st.row_height_edits[index].is_null() {
                values[index] = settings_host_text(st.row_height_edits[index])
                    .trim()
                    .parse::<i32>()
                    .unwrap_or(old[index])
                    .clamp(
                        if index == 0 { 80 } else { 32 },
                        if index == 0 { 320 } else { 160 },
                    );
            }
        }
        st.draft.image_row_height = values[0];
        st.draft.text_row_height = values[1];
        st.draft.file_row_height = values[2];
    }
    st.draft.show_mouse_dx = settings_host_text(st.ed_dx)
        .parse::<i32>()
        .ok()
        .unwrap_or(12);
    st.draft.show_mouse_dy = settings_host_text(st.ed_dy)
        .parse::<i32>()
        .ok()
        .unwrap_or(12);
    st.draft.show_fixed_x = settings_host_text(st.ed_fx)
        .parse::<i32>()
        .ok()
        .unwrap_or(120);
    st.draft.show_fixed_y = settings_host_text(st.ed_fy)
        .parse::<i32>()
        .ok()
        .unwrap_or(120);
    st.draft.show_pos_mode = settings_dropdown_pos_mode_from_label(&settings_host_text(st.cb_pos));
    if st.ui.is_built(SettingsPage::General.index()) && !st.cb_paste_sound.is_null() {
        st.draft.paste_success_sound_kind =
            paste_sound_key_from_display(&settings_host_text(st.cb_paste_sound)).to_string();
    }
    if st.ui.is_built(SettingsPage::General.index()) && !st.ed_skip_class_names.is_null() {
        st.draft.paste_target_skip_class_names = settings_host_text(st.ed_skip_class_names)
            .trim()
            .to_string();
    }
}
