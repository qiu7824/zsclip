use super::prelude::*;

pub(super) unsafe fn copy_selection_to_clipboard(state: &mut AppState) -> bool {
    let current = state.current_item_for_use();
    let selected = state.selected_items_for_use();
    let Some(plan) = main_copy_selection_plan(current.as_ref(), &selected) else {
        return false;
    };
    match plan {
        MainCopySelectionPlan::CopyCurrentItem => {
            let Some(item) = current else {
                return false;
            };
            apply_item_to_clipboard(state, &item)
        }
        MainCopySelectionPlan::CopyMergedText(merged) => {
            let ok = platform_clipboard::WindowsClipboardHost::write_text(&merged);
            if ok {
                state.note_programmatic_clipboard_signature(
                    text_content_signature(&merged),
                    CLIPBOARD_IGNORE_MS_PASTE,
                );
                set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_PASTE);
            }
            ok
        }
    }
}

pub(super) unsafe fn apply_item_to_clipboard(state: &mut AppState, item_ref: &ClipItem) -> bool {
    let full_item;
    let item: &ClipItem = if let Some(resolved) = state.resolve_item_for_use(item_ref) {
        full_item = resolved;
        &full_item
    } else {
        return false;
    };

    let ok = match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            if let Some(text) = &item.text {
                let prepared = maybe_ai_clean_text(state, text);
                let ok = platform_clipboard::WindowsClipboardHost::write_text(&prepared);
                if ok {
                    state.note_programmatic_clipboard_signature(
                        text_content_signature(&prepared),
                        CLIPBOARD_IGNORE_MS_PASTE,
                    );
                }
                ok
            } else {
                false
            }
        }
        ClipKind::Image => {
            if let Some((bytes, width, height)) = ensure_item_image_bytes(item) {
                let ok = platform_clipboard::WindowsClipboardHost::write_image_rgba(
                    &bytes, width, height,
                );
                if ok {
                    state.note_programmatic_clipboard_signature(
                        image_content_signature(&bytes, width, height),
                        CLIPBOARD_IGNORE_MS_PASTE,
                    );
                }
                ok
            } else {
                false
            }
        }
        ClipKind::Files => {
            if let Some(paths) = &item.file_paths {
                let ok = platform_clipboard::WindowsClipboardHost::write_file_paths(paths);
                if ok {
                    state.note_programmatic_clipboard_signature(
                        file_paths_signature(paths),
                        CLIPBOARD_IGNORE_MS_PASTE,
                    );
                }
                ok
            } else if let Some(text) = &item.text {
                let prepared = maybe_ai_clean_text(state, text);
                let ok = platform_clipboard::WindowsClipboardHost::write_text(&prepared);
                if ok {
                    state.note_programmatic_clipboard_signature(
                        text_content_signature(&prepared),
                        CLIPBOARD_IGNORE_MS_PASTE,
                    );
                }
                ok
            } else {
                false
            }
        }
    };
    if ok {
        set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_PASTE);
    }
    ok
}

unsafe fn apply_item_to_clipboard_plain_text(state: &mut AppState, item_ref: &ClipItem) -> bool {
    let full_item;
    let item: &ClipItem = if let Some(resolved) = state.resolve_item_for_use(item_ref) {
        full_item = resolved;
        &full_item
    } else {
        return false;
    };
    let text = match item.kind {
        ClipKind::Text | ClipKind::Phrase => item
            .text
            .as_ref()
            .map(|t| normalize_plain_text_for_paste(t)),
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .map(|paths| paths.join("\n"))
            .or_else(|| {
                item.text
                    .as_ref()
                    .map(|t| normalize_plain_text_for_paste(t))
            }),
        ClipKind::Image => None,
    };
    let Some(text) = text else {
        return false;
    };
    let ok = platform_clipboard::WindowsClipboardHost::write_text(&text);
    if ok {
        state.note_programmatic_clipboard_signature(
            text_content_signature(&text),
            CLIPBOARD_IGNORE_MS_PASTE,
        );
        set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_PASTE);
    }
    ok
}

fn spawn_async_image_paste_load(
    hwnd: HWND,
    item_id: i64,
    target: HWND,
    hide_main: bool,
    backspaces: u8,
) {
    let hwnd_raw = hwnd as isize;
    let target_token = NativeWindowToken(target as usize);
    std::thread::spawn(move || {
        let image = db_load_item_full(item_id).and_then(|full| {
            if let Some(bytes) = full.image_bytes {
                Some((bytes, full.image_width, full.image_height))
            } else {
                full.image_path
                    .as_deref()
                    .and_then(load_image_bytes_from_path)
            }
        });
        let payload = Box::new(ImagePasteReadyResult {
            image,
            target: target_token,
            hide_main,
            backspaces,
        });
        unsafe {
            let _ = post_boxed_message(hwnd_raw, WM_IMAGE_PASTE_READY, 0, payload);
        }
    });
}

pub(super) unsafe fn queue_async_image_paste_if_needed(
    hwnd: HWND,
    _state: &mut AppState,
    item_ref: &ClipItem,
    target: HWND,
    hide_main: bool,
    backspaces: u8,
) -> bool {
    if item_ref.kind != ClipKind::Image
        || item_ref.id <= 0
        || item_ref.image_bytes.is_some()
        || item_ref.image_path.is_some()
    {
        return false;
    }
    if !WindowsWindowIdentityHost::new().exists(target) {
        return false;
    }
    spawn_async_image_paste_load(hwnd, item_ref.id, target, hide_main, backspaces);
    true
}

pub(super) unsafe fn try_apply_to_explorer_rename(
    state: &mut AppState,
    item_ref: &ClipItem,
) -> bool {
    if !state.hotkey_passthrough_active || state.hotkey_passthrough_edit.is_null() {
        return false;
    }
    if !WindowsWindowIdentityHost::new().exists(state.hotkey_passthrough_edit) {
        clear_hotkey_passthrough_state(state);
        return false;
    }

    let full_item = match state.resolve_item_for_use(item_ref) {
        Some(item) => item,
        None => return false,
    };

    let text = match full_item.kind {
        ClipKind::Text | ClipKind::Phrase => full_item
            .text
            .as_ref()
            .map(|text| maybe_ai_clean_text(state, text)),
        ClipKind::Files => full_item
            .text
            .as_ref()
            .map(|text| maybe_ai_clean_text(state, text)),
        ClipKind::Image => None,
    };

    let Some(text) = text else {
        return false;
    };

    let ok =
        WindowsPasteTargetHost::new().set_paste_target_text(state.hotkey_passthrough_edit, &text);
    if ok {
        set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_DIRECT_EDIT);
        clear_hotkey_passthrough_state(state);
    }
    ok
}

unsafe fn maybe_promote_pasted_item(hwnd: HWND, state: &mut AppState, item_id: i64) {
    if !state.settings.move_pasted_item_to_top || item_id <= 0 {
        return;
    }
    if let Ok(new_id) = db_promote_item_to_top(item_id) {
        let anchor = state.current_scroll_anchor();
        state.remove_cached_item(item_id);
        state.remove_cached_item(new_id);
        if !state.promote_loaded_item_to_top(item_id, new_id) {
            state.reload_state_from_db_preserve_scroll(anchor);
        } else {
            state.refilter();
            state.restore_scroll_anchor(anchor);
        }
        sync_peer_windows_from_db(hwnd);
    }
}

pub(super) fn paste_completion_input(state: &AppState, item_id: i64) -> MainPasteCompletionInput {
    MainPasteCompletionInput {
        item_id,
        move_pasted_item_to_top: state.settings.move_pasted_item_to_top,
        click_hide: state.settings.click_hide,
        paste_success_sound_enabled: state.settings.paste_success_sound_enabled,
    }
}

fn paste_preparation_input(state: &AppState, item: &ClipItem) -> MainPastePreparationInput {
    MainPastePreparationInput {
        item_kind: item.kind,
        item_id: item.id,
        image_payload_loaded: item.image_bytes.is_some() || item.image_path.is_some(),
        direct_edit_candidate: state.hotkey_passthrough_active
            && !state.hotkey_passthrough_edit.is_null(),
        plain_text_paste_mode: state.plain_text_paste_mode,
    }
}

pub(super) unsafe fn execute_paste_completion_plan(
    hwnd: HWND,
    state: &mut AppState,
    plan: MainPasteCompletionPlan,
) {
    execute_paste_completion_plan_to_target(hwnd, state, plan, None);
}

pub(super) unsafe fn execute_paste_completion_plan_to_target(
    hwnd: HWND,
    state: &mut AppState,
    plan: MainPasteCompletionPlan,
    target: Option<HWND>,
) {
    if let Some(item_id) = plan.promote_item_id {
        maybe_promote_pasted_item(hwnd, state, item_id);
    }
    if plan.play_success_sound {
        play_paste_success_sound(
            &state.settings.paste_success_sound_kind,
            &state.settings.paste_success_sound_path,
        );
    }
    if plan.hide_main_now {
        WindowsMainWindowHost::new(Some(wnd_proc)).hide_main_window(hwnd);
    }
    if plan.reset_plain_text_paste_mode {
        state.plain_text_paste_mode = false;
    }
    if plan.clear_selection {
        state.clear_selection();
    }
    if plan.clear_hover {
        clear_main_hover_state(hwnd);
    }
    if plan.send_paste_after_clipboard {
        if let Some(target) = target {
            paste_after_clipboard_ready_to_target(
                hwnd,
                state,
                target,
                plan.paste_hide_main,
                plan.paste_backspaces,
            );
        } else {
            paste_after_clipboard_ready(hwnd, state, plan.paste_hide_main);
        }
    }
}

pub(super) unsafe fn paste_selected(hwnd: HWND, state: &mut AppState) {
    let Some(item_ref) = state.current_item().cloned() else {
        return;
    };
    let preparation = main_paste_preparation_plan(paste_preparation_input(state, &item_ref));
    for step in preparation.steps {
        match step {
            MainPastePreparationStep::DirectEdit => {
                if try_apply_to_explorer_rename(state, &item_ref) {
                    let plan = main_paste_completion_plan(
                        MainPasteCompletionKind::DirectEdit,
                        paste_completion_input(state, item_ref.id),
                    );
                    execute_paste_completion_plan(hwnd, state, plan);
                    return;
                }
            }
            MainPastePreparationStep::AsyncImage => {
                let async_target = effective_paste_target(state, hwnd);
                if queue_async_image_paste_if_needed(
                    hwnd,
                    state,
                    &item_ref,
                    async_target,
                    state.settings.click_hide,
                    0,
                ) {
                    let plan = main_paste_completion_plan(
                        MainPasteCompletionKind::AsyncImage,
                        paste_completion_input(state, item_ref.id),
                    );
                    execute_paste_completion_plan(hwnd, state, plan);
                    return;
                }
            }
            MainPastePreparationStep::Clipboard { plain_text } => {
                let applied = if plain_text {
                    apply_item_to_clipboard_plain_text(state, &item_ref)
                } else {
                    apply_item_to_clipboard(state, &item_ref)
                };
                if !applied {
                    show_clipboard_write_failure_message(hwnd);
                    return;
                }
                let plan = main_paste_completion_plan(
                    MainPasteCompletionKind::Clipboard,
                    paste_completion_input(state, item_ref.id),
                );
                execute_paste_completion_plan(hwnd, state, plan);
                return;
            }
        }
    }
}

pub(super) unsafe fn restore_hotkey_focus_target(state: &AppState, target: HWND) {
    let focus = state.hotkey_passthrough_focus;
    WindowsPasteTargetHost::new().restore_paste_target_focus(target, focus);
}

pub(super) unsafe fn can_send_ctrl_v_to_target(state: &AppState, target: HWND) -> bool {
    let identity_host = WindowsWindowIdentityHost::new();
    if !identity_host.exists(target) {
        return false;
    }
    if !identity_host.is_foreground(target) {
        return false;
    }
    if vv_is_qq_wps_process(&window_process_name(target)) {
        return true;
    }
    WindowsPasteTargetHost::new()
        .paste_target_focus_status(target, state.hotkey_passthrough_focus)
        .allows_paste_attempt()
}

unsafe fn paste_failure_message_for_target(state: &AppState, target: HWND) -> String {
    let identity_host = WindowsWindowIdentityHost::new();
    let detail = if !identity_host.exists(target) {
        tr(
            "目标窗口已经关闭。",
            "The target window is no longer available.",
        )
    } else if !identity_host.is_foreground(target) {
        tr(
            "未能把目标窗口切到前台。",
            "The target window could not be brought to the foreground.",
        )
    } else {
        match WindowsPasteTargetHost::new()
            .paste_target_focus_status(target, state.hotkey_passthrough_focus)
        {
            PasteTargetFocusStatus::Unknown => tr(
                "目标窗口当前没有可用输入焦点。",
                "The target window does not currently expose a usable input focus.",
            ),
            PasteTargetFocusStatus::NoActiveFocus => tr(
                "目标窗口没有活动输入框。",
                "The target window does not have an active input control.",
            ),
            PasteTargetFocusStatus::OutsideTarget => tr(
                "当前焦点不在目标输入区域里。",
                "The current focus is no longer inside the target input area.",
            ),
            PasteTargetFocusStatus::InsideTarget => tr(
                "目标窗口拒绝了粘贴。",
                "The target window rejected the paste action.",
            ),
        }
    };
    format!(
        "{}\r\n\r\n{}",
        detail,
        tr(
            "内容已经写入剪贴板，你可以回到目标窗口后手动粘贴。",
            "The content is already in the clipboard. You can switch back and paste it manually.",
        )
    )
}

pub(super) unsafe fn show_paste_failure_message(hwnd: HWND, state: &AppState, target: HWND) {
    let text = paste_failure_message_for_target(state, target);
    platform_dialog::WindowsDialogHost::new().show_message(
        hwnd,
        translate("粘贴失败").as_ref(),
        &text,
        NativeDialogLevel::Warning,
    );
}

pub(super) unsafe fn show_clipboard_write_failure_message(hwnd: HWND) {
    platform_dialog::WindowsDialogHost::new().show_message(
        hwnd,
        translate("复制失败").as_ref(),
        translate("内容未能写入系统剪贴板，请重试一次。").as_ref(),
        NativeDialogLevel::Warning,
    );
}

pub(super) unsafe fn effective_paste_target(state: &AppState, hwnd: HWND) -> HWND {
    let skip_class_names = paste_target_skip_classes(&state.settings);
    if !state.paste_target_override.is_null() {
        return state.paste_target_override;
    }
    if state.hotkey_passthrough_active
        && !state.hotkey_passthrough_target.is_null()
        && !paste_window_class_is_skipped(state.hotkey_passthrough_target, skip_class_names)
    {
        return state.hotkey_passthrough_target;
    }
    if state.role == WindowRole::Quick {
        let fg = WindowsWindowIdentityHost::new().foreground_handle();
        if is_viable_paste_window(fg, hwnd, skip_class_names) {
            return fg;
        }
    }
    find_next_paste_target(hwnd, skip_class_names)
}

pub(super) unsafe fn paste_after_clipboard_ready(
    hwnd: HWND,
    state: &mut AppState,
    hide_main: bool,
) {
    let target = effective_paste_target(state, hwnd);
    paste_after_clipboard_ready_to_target(hwnd, state, target, hide_main, 0);
}

pub(super) unsafe fn paste_after_clipboard_ready_to_target(
    hwnd: HWND,
    state: &mut AppState,
    target: HWND,
    hide_main: bool,
    backspaces: u8,
) {
    state.paste_target_override = target;
    state.paste_backspace_count = backspaces;
    if !target.is_null() {
        if hide_main {
            WindowsMainWindowHost::new(Some(wnd_proc)).hide_main_window(hwnd);
        }
        let _ = WindowsPasteTargetHost::new().force_paste_target_foreground(target);
        restore_hotkey_focus_target(state, target);
        timer::stop(hwnd, ID_TIMER_PASTE);
        timer::start(hwnd, ID_TIMER_PASTE, 150);
    } else {
        clear_hotkey_passthrough_state(state);
        WindowsMainWindowHost::new(Some(wnd_proc)).foreground_main_window(hwnd);
        if state.search_on {
            WindowsMainSearchControlHost::new().focus_search(state.search_hwnd);
        }
        platform_dialog::WindowsDialogHost::new().show_message(
            hwnd,
            translate("粘贴失败").as_ref(),
            translate("没有找到可粘贴的目标窗口，内容已经保留在剪贴板中。").as_ref(),
            NativeDialogLevel::Warning,
        );
    }
}
