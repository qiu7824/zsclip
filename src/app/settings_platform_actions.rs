use super::prelude::*;

pub(super) unsafe fn execute_settings_platform_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    execute_settings_platform_hotkey_action(hwnd, st, action)
        || execute_settings_platform_general_action(hwnd, st, action)
        || execute_settings_platform_plugin_action(hwnd, st, action)
        || execute_settings_platform_about_action(hwnd, action)
        || execute_settings_platform_system_action(hwnd, action)
}
