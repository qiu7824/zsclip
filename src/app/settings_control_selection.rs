use super::prelude::*;

pub(super) unsafe fn handle_settings_control_selection(
    hwnd: HWND,
    control_id: isize,
    index: usize,
) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return 0;
    }
    let st = &mut *st_ptr;
    st.dropdown_popup = null_mut();
    match control_id {
        IDC_SET_MAX | IDC_SET_POSMODE | IDC_SET_PASTE_SOUND_KIND => {
            handle_settings_general_selection(st, control_id, index);
        }
        IDC_SET_CLOUD_INTERVAL | IDC_SET_MULTI_SYNC_MODE | IDC_SET_LAN_RECEIVE_MODE => {
            handle_settings_cloud_selection(hwnd, st, control_id, index);
        }
        IDC_SET_HOTKEY_MOD | IDC_SET_HOTKEY_KEY | IDC_SET_PLAIN_HK_MOD | IDC_SET_PLAIN_HK_KEY => {
            handle_settings_hotkey_selection(st, control_id, index);
        }
        IDC_SET_SEARCH_ENGINE
        | IDC_SET_OCR_PROVIDER
        | IDC_SET_TRANSLATE_PROVIDER
        | IDC_SET_TRANSLATE_TARGET => {
            handle_settings_plugin_selection(st, control_id, index);
        }
        IDC_SET_VV_SOURCE | IDC_SET_VV_GROUP => {
            handle_settings_group_selection(st, control_id, index);
        }
        _ => {}
    }
    0
}
