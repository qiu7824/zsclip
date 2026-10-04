use super::prelude::*;
use crate::win_system_ui::settings_host_text;

pub(super) unsafe fn settings_collect_general_to_draft(st: &mut SettingsWndState) {
    if !st.cb_max.is_null() {
        if let Some(value) = settings_dropdown_max_items_from_label_opt(&settings_host_text(st.cb_max)) {
            st.draft.max_items = value;
        }
    }
    for (handle, value) in [(st.ed_dx, &mut st.draft.show_mouse_dx),
        (st.ed_dy, &mut st.draft.show_mouse_dy), (st.ed_fx, &mut st.draft.show_fixed_x),
        (st.ed_fy, &mut st.draft.show_fixed_y)] {
        if !handle.is_null() {
            if let Ok(parsed) = settings_host_text(handle).trim().parse::<i32>() { *value = parsed; }
        }
    }
    if !st.cb_pos.is_null() {
        st.draft.show_pos_mode = settings_dropdown_pos_mode_from_label(&settings_host_text(st.cb_pos));
    }
    if !st.cb_paste_sound.is_null() {
        st.draft.paste_success_sound_kind = paste_sound_key_from_display(&settings_host_text(st.cb_paste_sound)).to_string();
    }
    if !st.ed_skip_class_names.is_null() {
        st.draft.paste_target_skip_class_names = settings_host_text(st.ed_skip_class_names).trim().to_string();
    }
}
