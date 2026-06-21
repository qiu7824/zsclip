use super::prelude::*;

pub(super) fn register_hotkey_for(hwnd: HWND, state: &mut AppState) {
    let plan = main_hotkey_registration_plan(MainHotkeyRegistrationInput {
        enabled: state.settings.hotkey_enabled,
        already_registered: state.hotkey_registered,
        mod_label: &state.settings.hotkey_mod,
        key_label: &state.settings.hotkey_key,
    });
    if plan.unregister_existing {
        hotkey::unregister(hwnd as isize, HOTKEY_ID);
        state.hotkey_registered = false;
    }
    let Some(spec) = plan.register else {
        return;
    };
    match hotkey::register(
        hwnd as isize,
        HOTKEY_ID,
        hotkey::mods_from_spec(spec.modifiers),
        hotkey::vk_from_spec(spec.key),
    ) {
        Ok(()) => {
            state.hotkey_registered = true;
            state.hotkey_conflict_notified = false;
        }
        Err(err) => {
            state.hotkey_registered = false;
            if err == hotkey::ERROR_HOTKEY_ALREADY_REGISTERED && !state.hotkey_conflict_notified {
                state.hotkey_conflict_notified = true;
                let hk =
                    hotkey_preview_text(&state.settings.hotkey_mod, &state.settings.hotkey_key)
                        .replace(tr("当前设置：", "Current setting: "), "");
                let message = format!(
                    "{} {} {}",
                    tr("快捷键", "Hotkey"),
                    hk,
                    tr("已被其他程序或系统占用，当前不会注册全局热键。请在设置-快捷键中改用其他组合。", "is already used by another app or the system. The global hotkey will not be registered. Please choose another combination in Settings > Hotkeys.")
                );
                show_native_dialog_message(
                    hwnd,
                    translate("快捷键冲突").as_ref(),
                    &message,
                    NativeDialogLevel::Warning,
                );
            }
        }
    }
}

pub(super) fn unregister_hotkey_for(hwnd: HWND, state: &mut AppState) {
    if state.hotkey_registered {
        hotkey::unregister(hwnd as isize, HOTKEY_ID);
        state.hotkey_registered = false;
    }
}

pub(super) fn register_plain_paste_hotkey_for(hwnd: HWND, state: &mut AppState) {
    let plan = main_hotkey_registration_plan(MainHotkeyRegistrationInput {
        enabled: state.settings.plain_paste_hotkey_enabled,
        already_registered: state.plain_paste_hotkey_registered,
        mod_label: &state.settings.plain_paste_hotkey_mod,
        key_label: &state.settings.plain_paste_hotkey_key,
    });
    if plan.unregister_existing {
        hotkey::unregister(hwnd as isize, HOTKEY_ID_PLAIN);
        state.plain_paste_hotkey_registered = false;
    }
    let Some(spec) = plan.register else {
        return;
    };
    state.plain_paste_hotkey_registered = hotkey::register(
        hwnd as isize,
        HOTKEY_ID_PLAIN,
        hotkey::mods_from_spec(spec.modifiers),
        hotkey::vk_from_spec(spec.key),
    )
    .is_ok();
}

pub(super) fn unregister_plain_paste_hotkey_for(hwnd: HWND, state: &mut AppState) {
    if state.plain_paste_hotkey_registered {
        hotkey::unregister(hwnd as isize, HOTKEY_ID_PLAIN);
        state.plain_paste_hotkey_registered = false;
    }
}

pub(super) fn register_clipboard_listener_for(hwnd: HWND, state: &mut AppState) {
    if state.clipboard_listener_registered || state.role != WindowRole::Main {
        return;
    }
    state.clipboard_listener_registered = clipboard_listener::register(hwnd as isize);
}

pub(super) fn unregister_clipboard_listener_for(hwnd: HWND, state: &mut AppState) {
    if state.clipboard_listener_registered {
        clipboard_listener::unregister(hwnd as isize);
        state.clipboard_listener_registered = false;
    }
}

pub(super) unsafe fn handle_global_hotkey(hwnd: HWND, id: i32) {
    let plain_text = match id {
        HOTKEY_ID => false,
        HOTKEY_ID_PLAIN => true,
        _ => return,
    };
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        (*ptr).plain_text_paste_mode = plain_text;
    }
    toggle_window_visibility_hotkey(hwnd);
}
