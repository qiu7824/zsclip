use super::prelude::*;

pub(super) unsafe fn execute_settings_sync_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    if execute_settings_webdav_sync_action(hwnd, st, action) {
        return true;
    }
    execute_settings_lan_sync_action(hwnd, st, action)
}
