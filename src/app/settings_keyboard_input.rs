use super::prelude::*;

pub(super) unsafe fn handle_settings_key_down(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
    code: u32,
) -> LRESULT {
    let st_ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if st_ptr.is_null() {
        return 0;
    }
    let st = &mut *st_ptr;
    if !st.hotkey_recording {
        return platform_window::default_window_proc(hwnd, msg, wparam, lparam);
    }
    if hotkey::is_escape_vk(code) {
        settings_set_hotkey_recording(st, false);
        return 0;
    }
    if hotkey::is_modifier_vk(code) {
        return 0;
    }
    if let Some(key_label) = hotkey::key_label_from_vk(code) {
        if let Some(mod_label) = hotkey::modifier_label_from_pressed_state() {
            settings_set_text(st.cb_hk_mod, &mod_label);
            settings_set_text(st.cb_hk_key, key_label);
            settings_set_hotkey_recording(st, false);
        } else {
            settings_set_text(
                st.lb_hk_preview,
                tr("请按修饰键 + 按键", "Press modifier + key"),
            );
            repaint_settings_control(st.lb_hk_preview);
        }
        return 0;
    }
    0
}
