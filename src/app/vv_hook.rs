use super::prelude::*;

const VV_TRIGGER_TIMEOUT_MS: u128 = 300;

pub(super) unsafe fn window_process_name(hwnd: HWND) -> String {
    WindowsWindowIdentityHost::new().process_name(hwnd)
}

pub(super) unsafe fn send_escape_key() {
    platform_input::tap_key(hotkey::escape_key_u8());
}

unsafe fn vv_target_is_ignored(hwnd: HWND, main_hwnd: HWND) -> bool {
    if hwnd.is_null() || hwnd == main_hwnd {
        return true;
    }
    let popup = current_vv_popup_hwnd();
    if hwnd == popup {
        return true;
    }
    WindowsWindowIdentityHost::new().is_current_process_window(hwnd)
}

pub(super) unsafe fn vv_window_class_name(hwnd: HWND) -> String {
    WindowsWindowIdentityHost::new().class_name(hwnd)
}

pub(super) fn vv_is_qq_wps_process(process_name: &str) -> bool {
    let process = process_name.trim().to_ascii_lowercase();
    matches!(
        process.as_str(),
        "qq.exe"
            | "qq"
            | "qqnt.exe"
            | "qqnt"
            | "tim.exe"
            | "tim"
            | "wps.exe"
            | "wps"
            | "wpp.exe"
            | "wpp"
            | "et.exe"
            | "et"
    )
}

fn vv_is_qq_process(process_name: &str) -> bool {
    let process = process_name.trim().to_ascii_lowercase();
    matches!(
        process.as_str(),
        "qq.exe" | "qq" | "qqnt.exe" | "qqnt" | "tim.exe" | "tim"
    )
}

fn vv_is_browser_process(process_name: &str) -> bool {
    source_app_is_browser(process_name)
}

fn vv_is_browser_window_class(class_name: &str) -> bool {
    let class_name = class_name.trim().to_ascii_lowercase();
    matches!(
        class_name.as_str(),
        "chrome_renderwidgethosthwnd"
            | "chrome_widgetwin_0"
            | "chrome_widgetwin_1"
            | "mozillawindowclass"
            | "windows.ui.composition.desktopwindowcontentbridge"
    )
}

pub(super) fn vv_backspace_count_for_target_identity(
    process_name: &str,
    root_process_name: &str,
    target_class_name: &str,
    replaces_ime: bool,
) -> u8 {
    if replaces_ime
        || vv_is_qq_process(process_name)
        || vv_is_qq_process(root_process_name)
        || vv_is_browser_process(process_name)
        || vv_is_browser_process(root_process_name)
        || vv_is_browser_window_class(target_class_name)
    {
        0
    } else {
        2
    }
}

pub(super) fn vv_backspace_count_for_trigger_state(
    process_name: &str,
    root_process_name: &str,
    target_class_name: &str,
    replaces_ime: bool,
    trigger_text_visible: bool,
) -> u8 {
    if trigger_text_visible {
        2
    } else {
        vv_backspace_count_for_target_identity(
            process_name,
            root_process_name,
            target_class_name,
            replaces_ime,
        )
    }
}

pub(super) unsafe fn vv_backspace_count_for_target_window(
    target: HWND,
    replaces_ime: bool,
    trigger_text_visible: bool,
) -> u8 {
    let process_name = window_process_name(target);
    let root = WindowsWindowIdentityHost::new().root_handle(target);
    let root_process_name = if root.is_null() || root == target {
        String::new()
    } else {
        window_process_name(root)
    };
    let target_class_name = vv_window_class_name(target);
    vv_backspace_count_for_trigger_state(
        &process_name,
        &root_process_name,
        &target_class_name,
        replaces_ime,
        trigger_text_visible,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn english_vv_trigger_is_replaced_including_browser_targets() {
        assert_eq!(
            vv_backspace_count_for_trigger_state(
                "chrome.exe",
                "",
                "Chrome_WidgetWin_1",
                false,
                true,
            ),
            2
        );
        assert_eq!(
            vv_backspace_count_for_trigger_state("notepad.exe", "", "Edit", false, true),
            2
        );
    }

    #[test]
    fn native_ime_trigger_does_not_delete_existing_text() {
        assert_eq!(
            vv_backspace_count_for_trigger_state("notepad.exe", "", "Edit", true, false),
            0
        );
    }
}

unsafe fn vv_target_is_text_input_ready(target: HWND) -> bool {
    WindowsPasteTargetHost::new().paste_target_text_input_ready(target)
}

pub(super) unsafe fn vv_current_focus(target: HWND) -> HWND {
    WindowsTextCaretHost::new().focus_handle_for_target(target)
}

pub(super) unsafe fn vv_request_show(main: HWND, target: HWND, triggered_by_text: bool) {
    if !platform_window::exists(target) || platform_window::foreground() != target {
        return;
    }
    let focus = vv_current_focus(target);
    let Ok(mut hook) = vv_hook_state().lock() else {
        return;
    };
    let id = hook
        .session
        .begin(target as usize, focus as usize, triggered_by_text);
    hook.popup_active = false;
    platform_window::post_hwnd_message(main, WM_VV_SHOW, target as usize, id as isize);
}

pub(super) unsafe fn vv_paste_target_is_current(state: &AppState) -> bool {
    let Some((id, target, focus)) = state.vv_paste_guard else {
        return true;
    };
    platform_window::foreground() as isize == target
        && vv_current_focus(target as HWND) as isize == focus
        && vv_hook_state().lock().is_ok_and(|hook| {
            hook.session.matches(id, target as usize, focus as usize)
                && hook.session.phase == crate::app_core::vv_session::VvPhase::Selected
        })
}

pub(super) fn vv_finish_paste(state: &mut AppState) {
    if let Some((id, _, _)) = state.vv_paste_guard.take() {
        if let Ok(mut hook) = vv_hook_state().lock() {
            if hook.session.id == id {
                hook.session.cancel();
            }
        }
    }
}

pub(super) unsafe fn vv_cancel_for_pointer(point: POINT) {
    let under_pointer = platform_window::window_from_point(point);
    let menu_window = platform_window::class_name(under_pointer) == "#32768"
        || platform_window::class_name(platform_window::root_ancestor(under_pointer)) == "#32768";
    if platform_window::is_visible(current_vv_popup_hwnd())
        && platform_window::window_rect(current_vv_popup_hwnd())
            .is_some_and(|rect| platform_window::point_in_rect_screen(&point, &rect))
        || super::vv_preview::vv_preview_contains(point)
        || menu_window
    {
        return;
    }
    let Ok(mut hook) = vv_hook_state().try_lock() else {
        return;
    };
    if hook.session.active() || hook.session.phase == crate::app_core::vv_session::VvPhase::Selected
    {
        hook.session.cancel();
        hook.popup_active = false;
        platform_window::post_hwnd_message(
            hook.main_hwnd as HWND,
            WM_VV_HIDE,
            0,
            hook.session.id as isize,
        );
    }
    hook.last_was_v = false;
}

unsafe extern "system" fn vv_keyboard_hook_proc(
    code: i32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    use crate::app_core::vv_session::VvKeyAction;
    let Some(event) = platform_hook::keyboard_transition(code, wparam, lparam) else {
        return platform_hook::call_next(code, wparam, lparam);
    };
    if event.is_injected_or_lower_integrity() {
        return platform_hook::call_next(code, wparam, lparam);
    }
    let Ok(mut hook) = vv_hook_state().try_lock() else {
        return platform_hook::call_next(code, wparam, lparam);
    };
    if !hook.enabled || hook.main_hwnd == 0 {
        return platform_hook::call_next(code, wparam, lparam);
    }
    let main = hook.main_hwnd as HWND;
    let identity_host = WindowsWindowIdentityHost::new();
    let fg = identity_host.foreground_handle();
    let focus = vv_current_focus(fg);
    let menu_active = hook.popup_menu_active;
    let modifiers = hotkey::command_modifier_pressed()
        || hotkey::shift_pressed()
        || matches!(event.vk_code,0x10..=0x12|0x5b|0x5c|0xa0..=0xa5);
    let same = (menu_active && hotkey::is_escape_vk(event.vk_code))
        || hook.session.target == fg as usize && hook.session.focus == focus as usize;
    // Menus retain their own keyboard navigation; owned releases still go through the session.
    let result = if menu_active && event.down && !hotkey::is_escape_vk(event.vk_code) {
        None
    } else {
        Some(hook.session.key(event.vk_code, event.down, modifiers, same))
    };
    if let Some(result) = result {
        match result.action {
            VvKeyAction::Hide => {
                hook.popup_active = false;
                hook.last_was_v = false;
                platform_window::post_hwnd_message(main, WM_VV_HIDE, 0, hook.session.id as isize);
            }
            VvKeyAction::Select(index) => {
                hook.popup_active = false;
                hook.last_was_v = false;
                platform_window::post_hwnd_message(
                    main,
                    WM_VV_SELECT,
                    index,
                    hook.session.id as isize,
                );
            }
            VvKeyAction::Navigate(delta) => {
                platform_window::post_hwnd_message(
                    current_vv_popup_hwnd(),
                    WM_VV_PREVIEW_NAV,
                    delta as usize,
                    hook.session.id as isize,
                );
            }
            VvKeyAction::Scroll(delta) => {
                platform_window::post_hwnd_message(
                    current_vv_popup_hwnd(),
                    WM_VV_PREVIEW_SCROLL,
                    delta as usize,
                    hook.session.id as isize,
                );
            }
            VvKeyAction::None => {}
        }
        if result.consume {
            return 1;
        }
        if !event.down || result.repeat || result.action != VvKeyAction::None {
            return platform_hook::call_next(code, wparam, lparam);
        }
    } else {
        return platform_hook::call_next(code, wparam, lparam);
    }
    if modifiers || !identity_host.exists(fg) || vv_target_is_ignored(fg, main) {
        hook.last_was_v = false;
        hook.last_v_at = None;
        return platform_hook::call_next(code, wparam, lparam);
    }
    if event.vk_code == hook.trigger_vk {
        let timely = hook
            .last_v_at
            .is_some_and(|at| at.elapsed().as_millis() <= VV_TRIGGER_TIMEOUT_MS);
        if hook.last_was_v
            && hook.last_v_target == fg as isize
            && hook.last_v_focus == focus as isize
            && timely
        {
            hook.last_was_v = false;
            hook.last_v_at = None;
            if vv_target_is_text_input_ready(fg) {
                let id = hook.session.begin(fg as usize, focus as usize, true);
                platform_window::post_hwnd_message(main, WM_VV_SHOW, fg as usize, id as isize);
            }
        } else {
            let _ = vv_target_is_text_input_ready(fg);
            hook.last_was_v = true;
            hook.last_v_target = fg as isize;
            hook.last_v_focus = focus as isize;
            hook.last_v_at = Some(Instant::now());
        }
    } else {
        hook.last_was_v = false;
        hook.last_v_at = None;
    }
    platform_hook::call_next(code, wparam, lparam)
}

pub(super) unsafe fn update_vv_mode_hook(main_hwnd: HWND, enabled: bool) -> bool {
    if let Ok(mut hook_state) = vv_hook_state().lock() {
        hook_state.main_hwnd = main_hwnd as isize;
        hook_state.enabled = enabled;
        hook_state.trigger_vk = b'V' as u32;
        if !enabled {
            hook_state.session.cancel();
            hook_state.last_was_v = false;
            hook_state.last_v_target = 0;
            hook_state.last_v_at = None;
            hook_state.popup_active = false;
            hook_state.popup_target = 0;
            hook_state.popup_menu_active = false;
            hook_state.popup_menu_grace_until = None;
        }
    }
    let Ok(mut handle) = vv_hook_handle().lock() else {
        return false;
    };
    if enabled {
        if *handle == 0 {
            *handle = platform_hook::install_low_level_keyboard(Some(vv_keyboard_hook_proc));
        }
        *handle != 0
    } else if *handle != 0 {
        platform_hook::uninstall(*handle);
        *handle = 0;
        true
    } else {
        true
    }
}
