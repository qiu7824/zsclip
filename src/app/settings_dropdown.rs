use super::prelude::*;

pub(super) unsafe fn open_settings_config_file(st: &SettingsWndState) {
    let pst = get_state_ptr(st.parent_hwnd);
    if !pst.is_null() {
        save_settings(&(*pst).settings);
    } else if !settings_file().exists() {
        save_settings(&AppSettings::default());
    }
    let path = settings_file();
    open_path_with_shell(path.to_string_lossy().as_ref());
}

pub(super) unsafe fn open_settings_dropdown_for_control(
    hwnd: HWND,
    st: &mut SettingsWndState,
    control_id: isize,
) -> bool {
    close_settings_dropdown_popup(st);
    match control_id {
        IDC_SET_MAX | IDC_SET_POSMODE | IDC_SET_PASTE_SOUND_KIND => {
            open_settings_general_dropdown(hwnd, st, control_id)
        }
        IDC_SET_CLOUD_INTERVAL | IDC_SET_MULTI_SYNC_MODE | IDC_SET_LAN_RECEIVE_MODE => {
            open_settings_cloud_dropdown(hwnd, st, control_id)
        }
        IDC_SET_HOTKEY_MOD | IDC_SET_HOTKEY_KEY | IDC_SET_PLAIN_HK_MOD | IDC_SET_PLAIN_HK_KEY => {
            open_settings_hotkey_dropdown(hwnd, st, control_id)
        }
        IDC_SET_VV_SOURCE | IDC_SET_VV_GROUP => open_settings_group_dropdown(hwnd, st, control_id),
        _ => settings_open_plugin_dropdown_for_control(hwnd, st, control_id),
    }
}
