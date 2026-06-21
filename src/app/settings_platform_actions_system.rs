use super::prelude::*;

pub(super) unsafe fn execute_settings_platform_system_action(
    hwnd: HWND,
    action: SettingsAction,
) -> bool {
    match action {
        SettingsAction::DisableSystemClipboardHistory => {
            if let Err(e) = set_system_clipboard_history_enabled(false) {
                let message = format!("{}: {}", tr("屏蔽 Win+V 失败", "Disable Win+V failed"), e);
                show_native_dialog_message(
                    hwnd,
                    translate("系统剪贴板历史").as_ref(),
                    &message,
                    NativeDialogLevel::Error,
                );
            }
            true
        }
        SettingsAction::EnableSystemClipboardHistory => {
            if let Err(e) = set_system_clipboard_history_enabled(true) {
                let message = format!("{}: {}", tr("恢复 Win+V 失败", "Restore Win+V failed"), e);
                show_native_dialog_message(
                    hwnd,
                    translate("系统剪贴板历史").as_ref(),
                    &message,
                    NativeDialogLevel::Error,
                );
            }
            true
        }
        SettingsAction::RestartSystemShell => {
            if let Err(e) = restart_explorer_shell() {
                let message = format!(
                    "{}: {}",
                    tr("重启资源管理器失败", "Restart Explorer failed"),
                    e
                );
                show_native_dialog_message(
                    hwnd,
                    translate("系统剪贴板历史").as_ref(),
                    &message,
                    NativeDialogLevel::Error,
                );
            }
            true
        }
        _ => false,
    }
}
