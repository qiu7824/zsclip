use super::prelude::*;

pub(super) unsafe fn handle_settings_hotkey_selection(
    st: &mut SettingsWndState,
    control_id: isize,
    index: usize,
) {
    match control_id {
        IDC_SET_HOTKEY_MOD => {
            if let Some(label) = HOTKEY_MOD_OPTIONS.get(index) {
                settings_set_text(st.cb_hk_mod, label);
                settings_set_text(
                    st.lb_hk_preview,
                    &hotkey_preview_text(label, &settings_host_text(st.cb_hk_key)),
                );
                repaint_settings_control(st.cb_hk_mod);
            }
        }
        IDC_SET_HOTKEY_KEY => {
            if let Some(label) = HOTKEY_KEY_OPTIONS.get(index) {
                settings_set_text(st.cb_hk_key, label);
                settings_set_text(
                    st.lb_hk_preview,
                    &hotkey_preview_text(&settings_host_text(st.cb_hk_mod), label),
                );
                repaint_settings_control(st.cb_hk_key);
            }
        }
        IDC_SET_PLAIN_HK_MOD => {
            if let Some(label) = HOTKEY_MOD_OPTIONS.get(index) {
                settings_set_text(st.cb_plain_hk_mod, label);
                settings_set_text(
                    st.lb_plain_hk_preview,
                    &hotkey_preview_text(label, &settings_host_text(st.cb_plain_hk_key)),
                );
                repaint_settings_control(st.cb_plain_hk_mod);
            }
        }
        IDC_SET_PLAIN_HK_KEY => {
            if let Some(label) = HOTKEY_KEY_OPTIONS.get(index) {
                settings_set_text(st.cb_plain_hk_key, label);
                settings_set_text(
                    st.lb_plain_hk_preview,
                    &hotkey_preview_text(&settings_host_text(st.cb_plain_hk_mod), label),
                );
                repaint_settings_control(st.cb_plain_hk_key);
            }
        }
        _ => {}
    }
}
