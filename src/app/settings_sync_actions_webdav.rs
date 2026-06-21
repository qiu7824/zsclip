use super::prelude::*;

pub(super) unsafe fn execute_settings_webdav_sync_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    let cloud_action = match action {
        SettingsAction::SyncWebDavNow => CloudSyncAction::SyncNow,
        SettingsAction::UploadWebDavConfig => CloudSyncAction::UploadConfig,
        SettingsAction::ApplyWebDavConfig => CloudSyncAction::ApplyRemoteConfig,
        SettingsAction::RestoreWebDavBackup => CloudSyncAction::RestoreBackup,
        _ => return false,
    };

    settings_collect_to_app(st);
    if !st.draft.cloud_sync_enabled {
        show_native_dialog_message(
            hwnd,
            tr("多端同步", "Multi-device Sync"),
            tr(
                "请先在同步方案中选择 WebDAV。",
                "Choose WebDAV as the sync method first.",
            ),
            NativeDialogLevel::Info,
        );
        return true;
    }

    let pst = get_state_ptr(st.parent_hwnd);
    if !pst.is_null() {
        queue_cloud_sync(st.parent_hwnd, &mut *pst, cloud_action, false);
        settings_apply_from_app(st);
    }
    true
}
