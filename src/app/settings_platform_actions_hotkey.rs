use super::prelude::*;

pub(super) unsafe fn execute_settings_platform_hotkey_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    match action {
        SettingsAction::ToggleHotkeyRecording => {
            let next = !st.hotkey_recording;
            settings_set_hotkey_recording(st, next);
            if next {
                focus_settings_window(hwnd);
            }
            true
        }
        _ => false,
    }
}
