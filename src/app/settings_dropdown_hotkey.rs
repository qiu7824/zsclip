use super::prelude::*;

pub(super) unsafe fn open_settings_hotkey_dropdown(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
) -> bool {
    match control_id {
        IDC_SET_HOTKEY_MOD => {
            let rc = settings_control_screen_rect_or_empty(st.cb_hk_mod);
            let current = HOTKEY_MOD_OPTIONS
                .iter()
                .position(|x| *x == settings_host_text(st.cb_hk_mod))
                .unwrap_or(0);
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_HOTKEY_MOD,
                &rc,
                &HOTKEY_MOD_OPTIONS,
                current,
                200,
            );
            true
        }
        IDC_SET_HOTKEY_KEY => {
            let rc = settings_control_screen_rect_or_empty(st.cb_hk_key);
            let current = HOTKEY_KEY_OPTIONS
                .iter()
                .position(|x| *x == settings_host_text(st.cb_hk_key))
                .unwrap_or(21);
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_HOTKEY_KEY,
                &rc,
                &HOTKEY_KEY_OPTIONS,
                current,
                220,
            );
            true
        }
        IDC_SET_PLAIN_HK_MOD => {
            let rc = settings_control_screen_rect_or_empty(st.cb_plain_hk_mod);
            let current = HOTKEY_MOD_OPTIONS
                .iter()
                .position(|x| *x == settings_host_text(st.cb_plain_hk_mod))
                .unwrap_or(5);
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_PLAIN_HK_MOD,
                &rc,
                &HOTKEY_MOD_OPTIONS,
                current,
                220,
            );
            true
        }
        IDC_SET_PLAIN_HK_KEY => {
            let rc = settings_control_screen_rect_or_empty(st.cb_plain_hk_key);
            let current = HOTKEY_KEY_OPTIONS
                .iter()
                .position(|x| *x == settings_host_text(st.cb_plain_hk_key))
                .unwrap_or(21);
            st.dropdown_popup = present_settings_dropdown_popup(
                hwnd,
                IDC_SET_PLAIN_HK_KEY,
                &rc,
                &HOTKEY_KEY_OPTIONS,
                current,
                220,
            );
            true
        }
        _ => false,
    }
}
