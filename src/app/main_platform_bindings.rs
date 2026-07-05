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
    match id {
        HOTKEY_ID => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                (*ptr).plain_text_paste_mode = false;
            }
            toggle_window_visibility_hotkey(hwnd);
        }
        HOTKEY_ID_PLAIN => {
            let ptr = get_state_ptr(hwnd);
            if ptr.is_null() {
                return;
            }
            let state = &mut *ptr;
            prepare_plain_paste_hotkey_target(hwnd, state);
            state.plain_text_paste_mode = true;
            paste_selected(hwnd, state);
            if state.plain_text_paste_mode {
                state.paste_target_override = null_mut();
                clear_hotkey_passthrough_state(state);
            }
            state.plain_text_paste_mode = false;
        }
        _ => {}
    }
}

unsafe fn prepare_plain_paste_hotkey_target(hwnd: HWND, state: &mut AppState) {
    state.paste_target_override = null_mut();
    clear_hotkey_passthrough_state(state);

    let Some((target, focus)) = foreground_focus_snapshot_for_plain_paste(hwnd, state) else {
        return;
    };
    state.paste_target_override = target;
    state.hotkey_passthrough_active = true;
    state.hotkey_passthrough_target = target;
    state.hotkey_passthrough_focus = focus;
}

unsafe fn foreground_focus_snapshot_for_plain_paste(
    hwnd: HWND,
    state: &AppState,
) -> Option<(HWND, HWND)> {
    let foreground = WindowsWindowIdentityHost::new().foreground_handle();
    if foreground.is_null() {
        return None;
    }
    let target = platform_window::root_ancestor(foreground);
    if !is_viable_paste_window(target, hwnd, paste_target_skip_classes(&state.settings)) {
        return None;
    }

    let thread_id = platform_window::window_thread_id(target);
    if thread_id == 0 {
        return Some((target, null_mut()));
    }

    let mut info: GUITHREADINFO = zeroed();
    info.cbSize = std::mem::size_of::<GUITHREADINFO>() as u32;
    if !platform_window::gui_thread_info(thread_id, &mut info) {
        return Some((target, null_mut()));
    }

    let focus = if !info.hwndFocus.is_null() {
        info.hwndFocus
    } else {
        info.hwndCaret
    };
    if !focus.is_null() && platform_window::root_ancestor(focus) == target {
        Some((target, focus))
    } else {
        Some((target, null_mut()))
    }
}
