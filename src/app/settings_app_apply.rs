use super::prelude::*;

pub(super) unsafe fn settings_apply_from_app(st: &mut SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    if pst.is_null() {
        return;
    }
    let app = &mut *pst;
    app.settings.auto_start = is_autostart_enabled();
    st.draft = app.settings.clone();
    st.vv_source_selected = normalize_source_tab(st.draft.vv_source_tab);
    st.vv_group_selected = st.draft.vv_group_id;
    st.group_view_tab = normalize_source_tab(app.tab_index);
    let s = &st.draft;
    settings_set_text(
        st.cb_max,
        settings_dropdown_label_for_max_items(s.max_items),
    );
    settings_set_text(st.ed_dx, &s.show_mouse_dx.to_string());
    settings_set_text(st.ed_dy, &s.show_mouse_dy.to_string());
    settings_set_text(st.ed_fx, &s.show_fixed_x.to_string());
    settings_set_text(st.ed_fy, &s.show_fixed_y.to_string());
    settings_set_text(
        st.cb_pos,
        settings_dropdown_label_for_pos_mode(&s.show_pos_mode),
    );
    if !st.cb_paste_sound.is_null() {
        settings_set_text(
            st.cb_paste_sound,
            &paste_sound_display(&s.paste_success_sound_kind),
        );
    }
    if !st.ed_skip_class_names.is_null() {
        settings_set_text(st.ed_skip_class_names, &s.paste_target_skip_class_names);
    }
    settings_sync_page_state(st, SettingsPage::General.index());
    if st.ui.is_built(SettingsPage::Hotkey.index()) {
        settings_sync_page_state(st, SettingsPage::Hotkey.index());
    }
    if st.ui.is_built(SettingsPage::Plugin.index()) {
        settings_sync_page_state(st, SettingsPage::Plugin.index());
    }
    if st.ui.is_built(SettingsPage::Group.index()) {
        settings_sync_page_state(st, SettingsPage::Group.index());
    }
    if st.ui.is_built(SettingsPage::Cloud.index()) {
        settings_sync_page_state(st, SettingsPage::Cloud.index());
    }
}
