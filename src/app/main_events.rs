use super::prelude::*;

fn windows_main_action_from_native_specs(command: &Command) -> Option<NativeHostUiAction> {
    native_host_main_action_button_specs()
        .into_iter()
        .map(|spec| spec.action)
        .find(|action| action.command() == *command)
}

pub(super) unsafe fn execute_main_menu_command(hwnd: HWND, intent: MainMenuCommandIntent) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    match intent {
        MainMenuCommandIntent::Tray(action) => {
            let plan = main_tray_action_plan(MainTrayActionInput {
                action,
                clipboard_capture_enabled: state.settings.clipboard_capture_enabled,
                lan_sync_enabled: state.settings.lan_sync_enabled,
            });
            match plan {
                MainTrayActionPlan::ToggleWindow => {
                    toggle_window_visibility(hwnd);
                }
                MainTrayActionPlan::SetClipboardCapture { enabled } => {
                    state.settings.clipboard_capture_enabled = enabled;
                    save_settings(&state.settings);
                    if !enabled {
                        let sequence = platform_clipboard::WindowsClipboardHost::sequence_number();
                        if sequence != 0 {
                            state.last_clipboard_seq = sequence;
                        }
                        reset_clipboard_retry(hwnd, state);
                    }
                }
                #[cfg(feature = "lan-sync")]
                MainTrayActionPlan::SetLanSync { enabled } => {
                    state.settings.lan_sync_enabled = enabled;
                    if lan_sync::ensure_device_identity(&mut state.settings) {
                        save_settings(&state.settings);
                    }
                    save_settings(&state.settings);
                    refresh_lan_latest_from_db(&state.settings);
                    lan_sync::refresh_service(hwnd, &state.settings);
                    request_settings_window_repaint(state.settings_hwnd);
                }
                #[cfg(not(feature = "lan-sync"))]
                MainTrayActionPlan::SetLanSync { .. } => {}
                MainTrayActionPlan::Exit => {
                    destroy_main_window(hwnd);
                }
            }
        }
        row_intent => {
            execute_row_command(hwnd, state, row_intent);
        }
    }
    let plan = state.list.context_menu_finish_plan();
    state.list.apply_context_menu_finish_plan(plan);
}

pub(super) unsafe fn execute_main_ui_command(hwnd: HWND, command: Command) {
    let routed_command = windows_main_action_from_native_specs(&command)
        .map(|native_action| native_action.command())
        .unwrap_or(command);
    let Some(action) = main_host_action_for_command(&routed_command) else {
        return;
    };
    match main_host_execution_plan(action) {
        MainHostExecutionPlan::Search(request) => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                let plan = search_visibility_plan_for_request(state, request);
                apply_search_visibility_plan(hwnd, state, plan);
            }
        }
        MainHostExecutionPlan::OpenSettings => open_settings_window(hwnd),
        MainHostExecutionPlan::HideWindow => {
            WindowsMainWindowHost::new(Some(wnd_proc)).hide_main_window(hwnd);
        }
        MainHostExecutionPlan::CloseWindow => {
            WindowsMainWindowHost::new(Some(wnd_proc)).request_main_window_close(hwnd);
        }
        MainHostExecutionPlan::InvokeMenuCommand(intent) => execute_main_menu_command(hwnd, intent),
    }
}

pub(super) unsafe fn drain_main_ui_commands(hwnd: HWND) {
    loop {
        let command = {
            let ptr = get_state_ptr(hwnd);
            if ptr.is_null() {
                return;
            }
            (*ptr).ui_commands.pop()
        };
        let Some(command) = command else {
            break;
        };
        execute_main_ui_command(hwnd, command);
    }
}

pub(super) unsafe fn handle_main_timer_task(hwnd: HWND, task: MainTimerTask) {
    match task {
        MainTimerTask::StartupRecovery => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                retry_startup_integrations(hwnd, &mut *ptr);
            }
        }
        MainTimerTask::VvWatch => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                let identity_host = WindowsWindowIdentityHost::new();
                if state.vv_popup_visible
                    && !vv_popup_menu_active()
                    && (!identity_host.is_foreground(state.vv_popup_target)
                        || !identity_host.exists(state.vv_popup_target))
                {
                    vv_popup_hide(hwnd, state);
                }
            }
        }
        MainTimerTask::VvShow => {
            timer::stop(hwnd, ID_TIMER_VV_SHOW);
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                let target = state.vv_popup_pending_target;
                let identity_host = WindowsWindowIdentityHost::new();
                if identity_host.exists(target) && identity_host.is_foreground(target) {
                    state.vv_popup_pending_target = null_mut();
                    if !vv_popup_show(hwnd, state, target) && state.vv_popup_pending_retries > 0 {
                        state.vv_popup_pending_target = target;
                        state.vv_popup_pending_retries -= 1;
                        timer::start(hwnd, ID_TIMER_VV_SHOW, VV_SHOW_RETRY_DELAY_MS);
                    }
                } else {
                    state.vv_popup_pending_target = null_mut();
                    state.vv_popup_pending_retries = 0;
                }
            }
        }
        MainTimerTask::Paste => {
            timer::stop(hwnd, ID_TIMER_PASTE);
            let mut should_send_paste = true;
            let mut should_play_sound = false;
            let mut paste_target = null_mut();
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                let target = state.paste_target_override;
                paste_target = target;
                if !target.is_null() {
                    should_send_paste =
                        WindowsPasteTargetHost::new().force_paste_target_foreground(target);
                    if should_send_paste {
                        restore_hotkey_focus_target(state, target);
                        should_send_paste = can_send_ctrl_v_to_target(state, target);
                    }
                }
                if should_send_paste {
                    platform_input::send_backspace_times(state.paste_backspace_count);
                }
                state.paste_backspace_count = 0;
                state.paste_target_override = null_mut();
                clear_hotkey_passthrough_state(state);
                should_play_sound = state.settings.paste_success_sound_enabled;
            }
            if should_send_paste {
                platform_input::send_ctrl_v();
                if should_play_sound {
                    let ptr = get_state_ptr(hwnd);
                    if !ptr.is_null() {
                        play_paste_success_sound(
                            &(*ptr).settings.paste_success_sound_kind,
                            &(*ptr).settings.paste_success_sound_path,
                        );
                    }
                }
            } else if !ptr.is_null() {
                show_paste_failure_message(hwnd, &*ptr, paste_target);
            }
        }
        MainTimerTask::SearchDebounce => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                stop_search_debounce_timer(hwnd, state);
                apply_search_filter(hwnd, state);
            }
        }
        MainTimerTask::HiddenReclaim => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                cancel_hidden_memory_reclaim(hwnd, state);
                if !platform_window::is_visible(hwnd) || platform_window::is_minimized(hwnd) {
                    reclaim_hidden_window_memory(hwnd, state);
                    trim_hidden_process_working_set();
                }
            }
        }
        MainTimerTask::ClipboardRetry => capture_clipboard_guarded(hwnd),
        MainTimerTask::DpiFit => {
            timer::stop(hwnd, ID_TIMER_DPI_FIT);
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                ensure_main_window_size_for_monitor(hwnd, &mut *ptr);
            }
        }
        MainTimerTask::ScrollFade => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                if state.hover_scroll {
                    state.scroll_fade_alpha = 255;
                } else {
                    state.scroll_fade_alpha = state.scroll_fade_alpha.saturating_sub(30);
                    if state.scroll_fade_alpha == 0 {
                        stop_flagged_timer(
                            hwnd,
                            ID_TIMER_SCROLL_FADE,
                            &mut state.scroll_fade_timer,
                        );
                    }
                }
                repaint_main_window(hwnd, false);
            }
        }
        MainTimerTask::EdgeAutoHide => handle_edge_auto_hide_tick(hwnd),
        MainTimerTask::OutsideHide => handle_outside_hide_tick(hwnd),
        MainTimerTask::CloudSync => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                if !state.cloud_sync_in_progress
                    && state.role == WindowRole::Main
                    && state
                        .cloud_sync_next_due
                        .map(|due| due <= Instant::now())
                        .unwrap_or(false)
                {
                    queue_cloud_sync(hwnd, state, CloudSyncAction::SyncNow, true);
                }
            }
        }
    }
}

pub(super) unsafe fn dispatch_main_ui_event(hwnd: HWND, event: UiEvent) -> bool {
    match event {
        UiEvent::Application(event) => handle_main_application_event(hwnd, event),
        UiEvent::CloseRequested => {
            handle_main_close_requested(hwnd);
            return true;
        }
        UiEvent::Lifecycle(event) => {
            handle_main_lifecycle_event(hwnd, event);
            return matches!(event, LifecycleEvent::Unmount);
        }
        UiEvent::PointerMove { position } => handle_mouse_move(hwnd, position),
        UiEvent::PointerHover { position } => handle_mouse_hover_main(hwnd, position),
        UiEvent::PointerLeave => handle_mouse_leave_main(hwnd),
        UiEvent::PointerCancel => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                cancel_main_scroll_drag(hwnd, &mut *ptr);
            }
        }
        UiEvent::PointerButton {
            position,
            button: UiMouseButton::Left,
            pressed: true,
            click_count: 2,
        } => handle_lbutton_dblclk(hwnd, position),
        UiEvent::PointerButton {
            position,
            button: UiMouseButton::Left,
            pressed: true,
            ..
        } => handle_lbutton_down(hwnd, position),
        UiEvent::PointerButton {
            position,
            button: UiMouseButton::Left,
            pressed: false,
            ..
        } => handle_lbutton_up(hwnd, position),
        UiEvent::PointerButton {
            position,
            button: UiMouseButton::Right,
            pressed: false,
            ..
        } => handle_rbutton_up(hwnd, position),
        UiEvent::MouseWheel { delta, .. } => handle_mouse_wheel(hwnd, delta),
        UiEvent::Key {
            code,
            state: UiKeyState::Down,
            system: false,
        } => handle_keydown(hwnd, code),
        UiEvent::ControlCommand {
            control_id,
            notification,
        } => handle_control_command(hwnd, control_id as usize, notification),
        UiEvent::GlobalHotkey { id } => handle_global_hotkey(hwnd, id),
        UiEvent::Timer { id } => {
            if let Some(task) = main_timer_task_for_id(id as usize, MAIN_TIMER_IDS) {
                handle_main_timer_task(hwnd, task);
            }
        }
        UiEvent::WindowSize { size, minimized } => handle_main_window_size(hwnd, size, minimized),
        UiEvent::AppActivationChanged { active } => {
            handle_main_app_activation_changed(hwnd, active)
        }
        UiEvent::SystemMetricsChanged => handle_main_system_metrics_changed(hwnd),
        UiEvent::WindowMoved => handle_main_window_moved(hwnd),
        UiEvent::WindowMoveCompleted => handle_main_window_move_completed(hwnd),
        UiEvent::DpiChanged { dpi } => handle_main_dpi_changed(hwnd, dpi),
        UiEvent::ClipboardChanged => capture_clipboard_guarded(hwnd),
        _ => return false,
    }
    drain_main_ui_commands(hwnd);
    true
}

pub(super) unsafe fn handle_main_application_event(hwnd: HWND, event: ApplicationEvent) {
    match event {
        #[cfg(feature = "lan-sync")]
        ApplicationEvent::LanSyncReady => handle_lan_sync_ready(hwnd),
        #[cfg(not(feature = "lan-sync"))]
        ApplicationEvent::LanSyncReady => {}
        ApplicationEvent::VvShowRequested { target } => {
            let ptr = get_state_ptr(hwnd);
            if ptr.is_null() {
                return;
            }
            let state = &mut *ptr;
            let mut target = target.0 as HWND;
            let identity_host = WindowsWindowIdentityHost::new();
            if !identity_host.exists(target) {
                let foreground = identity_host.foreground_handle();
                if identity_host.exists(foreground) {
                    target = foreground;
                }
            }
            if identity_host.exists(target) {
                state.vv_popup_pending_target = target;
                state.vv_popup_pending_retries = VV_SHOW_RETRY_MAX;
                timer::stop(hwnd, ID_TIMER_VV_SHOW);
                timer::start(hwnd, ID_TIMER_VV_SHOW, VV_SHOW_RETRY_DELAY_MS);
            } else {
                state.vv_popup_pending_target = null_mut();
                state.vv_popup_pending_retries = 0;
            }
        }
        ApplicationEvent::VvHideRequested => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                vv_popup_hide(hwnd, &mut *ptr);
            }
        }
        ApplicationEvent::VvSelectRequested { index } => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                handle_vv_select(hwnd, &mut *ptr, index);
            }
        }
        ApplicationEvent::ClipboardChanged { .. } => capture_clipboard_guarded(hwnd),
        ApplicationEvent::ItemsPageReady => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                apply_ready_page_loads(hwnd, &mut *ptr);
            }
        }
        ApplicationEvent::StartupDataReconciled { deleted } => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() && deleted > 0 {
                let state = &mut *ptr;
                reload_state_from_db_persisting(state);
                refresh_lan_latest_from_db(&state.settings);
                sync_peer_windows_from_db(hwnd);
                repaint_main_window(hwnd, true);
            }
        }
        ApplicationEvent::CloudSyncReady => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                apply_ready_cloud_syncs(hwnd, &mut *ptr);
            }
        }
        ApplicationEvent::UpdateCheckReady => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                request_settings_window_repaint((*ptr).settings_hwnd);
            }
        }
        ApplicationEvent::ShellIntegrationRestored => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() && (*ptr).role == WindowRole::Main {
                let state = &mut *ptr;
                state.startup_recovery_ticks = STARTUP_RECOVERY_TICKS;
                retry_startup_integrations(hwnd, state);
            }
        }
        ApplicationEvent::TrayCallback { code } => handle_tray(hwnd, code),
    }
}

pub(super) unsafe fn handle_main_async_event(hwnd: HWND, event: MainAsyncEvent) {
    match event {
        MainAsyncEvent::ImagePaste(payload) => {
            let ptr = get_state_ptr(hwnd);
            if ptr.is_null() {
                return;
            }
            let state = &mut *ptr;
            if let Some((bytes, width, height)) = payload.image {
                if platform_clipboard::WindowsClipboardHost::write_image_rgba(&bytes, width, height)
                {
                    state.note_programmatic_clipboard_signature(
                        image_content_signature(&bytes, width, height),
                        1200,
                    );
                    set_ignore_clipboard_for_all_hosts(1200);
                    paste_after_clipboard_ready_to_target(
                        hwnd,
                        state,
                        payload.target.0 as HWND,
                        payload.hide_main,
                        payload.backspaces,
                    );
                }
            }
        }
        MainAsyncEvent::ImageOcr(payload) => {
            handle_text_processing_result(
                hwnd,
                payload.text,
                payload.error,
                tr("图片转文字失败", "Image OCR failed"),
                tr("图片转文字", "Image OCR"),
            );
        }
        MainAsyncEvent::TextTranslate(payload) => {
            handle_text_processing_result(
                hwnd,
                payload.text,
                payload.error,
                tr("文本翻译失败", "Text translation failed"),
                tr("文本翻译", "Text Translate"),
            );
        }
        MainAsyncEvent::ImageThumbnail(payload) => {
            let ptr = get_state_ptr(hwnd);
            if !ptr.is_null() {
                let state = &mut *ptr;
                state.image_thumb_loading.remove(&payload.item_id);
                if let Some(image) = payload.image {
                    state.image_thumb_cache.put(payload.item_id, image);
                    repaint_main_window(hwnd, false);
                }
            }
        }
    }
}

pub(super) unsafe fn handle_text_processing_result(
    hwnd: HWND,
    text: Option<String>,
    error: Option<String>,
    error_prefix: &str,
    title: &str,
) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    if let Some(text) = text {
        let normalized = text.replace("\r\n", "\n");
        let preview = build_preview(&normalized);
        let signature = text_content_signature(&normalized);
        skip_next_clipboard_update_for_all_hosts();
        let _ = platform_clipboard::WindowsClipboardHost::write_text(&normalized);
        state.add_clip_item(
            ClipItem {
                id: 0,
                kind: ClipKind::Text,
                preview,
                text: Some(normalized),
                rich_text_html: None,
                source_app: String::new(),
                file_paths: None,
                image_bytes: None,
                image_path: None,
                image_width: 0,
                image_height: 0,
                pinned: false,
                group_id: 0,
                created_at: String::new(),
            },
            signature,
        );
        repaint_main_window(hwnd, true);
    } else if let Some(error) = error {
        let message = format!("{error_prefix}: {error}");
        show_native_dialog_message(hwnd, title, &message, NativeDialogLevel::Error);
    }
}
