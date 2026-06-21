use super::prelude::*;
use crate::win_system_ui::settings_host_text;

pub(super) unsafe fn settings_collect_hotkey_to_draft(st: &mut SettingsWndState) {
    if st.ui.is_built(SettingsPage::Hotkey.index())
        && !st.cb_hk_mod.is_null()
        && !st.cb_hk_key.is_null()
    {
        st.draft.hotkey_mod = normalize_hotkey_mod(&settings_host_text(st.cb_hk_mod));
        st.draft.hotkey_key = normalize_hotkey_key(&settings_host_text(st.cb_hk_key));
        if !st.cb_plain_hk_mod.is_null() && !st.cb_plain_hk_key.is_null() {
            st.draft.plain_paste_hotkey_mod =
                normalize_hotkey_mod(&settings_host_text(st.cb_plain_hk_mod));
            st.draft.plain_paste_hotkey_key =
                normalize_hotkey_key(&settings_host_text(st.cb_plain_hk_key));
        }
    }
}
