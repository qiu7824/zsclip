use super::prelude::*;

pub(super) fn settings_toggle_hotkey_get(st: &SettingsWndState, cid: isize) -> Option<bool> {
    match cid {
        6101 => Some(st.draft.hotkey_enabled),
        IDC_SET_PLAIN_HK_ENABLE => Some(st.draft.plain_paste_hotkey_enabled),
        IDC_SET_MOUSE_SIDE_ENABLE => Some(st.draft.mouse_side_button_enabled),
        _ => None,
    }
}

pub(super) fn settings_toggle_hotkey_flip(st: &mut SettingsWndState, cid: isize) -> bool {
    match cid {
        6101 => st.draft.hotkey_enabled = !st.draft.hotkey_enabled,
        IDC_SET_PLAIN_HK_ENABLE => {
            st.draft.plain_paste_hotkey_enabled = !st.draft.plain_paste_hotkey_enabled
        }
        IDC_SET_MOUSE_SIDE_ENABLE => {
            st.draft.mouse_side_button_enabled = !st.draft.mouse_side_button_enabled
        }
        _ => return false,
    }
    true
}
