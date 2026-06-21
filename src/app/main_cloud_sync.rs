use super::prelude::*;

pub(super) unsafe fn queue_cloud_sync(
    hwnd: HWND,
    state: &mut AppState,
    action: CloudSyncAction,
    auto_triggered: bool,
) {
    if state.cloud_sync_in_progress {
        return;
    }
    if state.settings.cloud_webdav_url.trim().is_empty() {
        state.settings.cloud_last_sync_status = "未配置 WebDAV 地址".to_string();
        save_settings(&state.settings);
        refresh_settings_window_from_app(state);
        if !auto_triggered {
            show_native_dialog_message(
                hwnd,
                cloud_sync_action_label(action),
                "请先填写 WebDAV 地址。",
                NativeDialogLevel::Info,
            );
        }
        return;
    }

    if matches!(
        action,
        CloudSyncAction::SyncNow | CloudSyncAction::RestoreBackup
    ) {
        close_db();
    }

    state.cloud_sync_in_progress = true;
    state.settings.cloud_last_sync_status = cloud_sync_running_text(auto_triggered).to_string();
    save_settings(&state.settings);
    refresh_settings_window_from_app(state);
    spawn_cloud_sync_job(
        hwnd as isize,
        WM_CLOUD_SYNC_READY,
        action,
        auto_triggered,
        state.settings.clone(),
    );
}

pub(super) unsafe fn apply_ready_cloud_syncs(hwnd: HWND, state: &mut AppState) {
    let mut ready = VecDeque::new();
    if let Ok(mut queue) = cloud_sync_results().lock() {
        let mut pending = VecDeque::new();
        while let Some(result) = queue.pop_front() {
            if result.hwnd == hwnd as isize {
                ready.push_back(result);
            } else {
                pending.push_back(result);
            }
        }
        *queue = pending;
    }

    while let Some(ready_item) = ready.pop_front() {
        state.cloud_sync_in_progress = false;
        schedule_cloud_sync(state, false);
        match ready_item.result {
            Ok(outcome) => {
                state.settings.cloud_last_sync_status = outcome.status_text;
                save_settings(&state.settings);
                if outcome.reload_settings {
                    apply_loaded_settings(hwnd, state);
                } else if outcome.reload_data {
                    reload_state_from_db_persisting(state);
                    layout_children(hwnd);
                    repaint_main_window(hwnd, true);
                } else {
                    refresh_settings_window_from_app(state);
                    repaint_main_window(hwnd, true);
                }
                sync_peer_windows_from_settings(hwnd);
            }
            Err(err) => {
                state.settings.cloud_last_sync_status = format!("失败：{err}");
                save_settings(&state.settings);
                refresh_settings_window_from_app(state);
                sync_peer_windows_from_settings(hwnd);
                if !ready_item.auto_triggered {
                    show_native_dialog_message(
                        hwnd,
                        cloud_sync_action_label(ready_item.action),
                        &err,
                        NativeDialogLevel::Error,
                    );
                }
            }
        }
    }
}
