use super::prelude::*;

pub(super) unsafe fn execute_settings_platform_about_action(
    hwnd: HWND,
    action: SettingsAction,
) -> bool {
    match action {
        SettingsAction::OpenSourceRepository => {
            if open_source_url().trim().is_empty() {
                show_native_dialog_message(
                    hwnd,
                    translate("开源地址").as_ref(),
                    translate(
                        "当前还没有配置开源地址，请先在 Cargo.toml 的 package.repository 中填写。",
                    )
                    .as_ref(),
                    NativeDialogLevel::Info,
                );
            } else {
                open_path_with_shell(open_source_url());
            }
            true
        }
        SettingsAction::CheckForUpdates => {
            let update_state = update_check_state_snapshot();
            if !update_state.checking {
                if update_state.available {
                    let url = update_check_latest_url_or_default();
                    open_path_with_shell(&url);
                } else {
                    start_update_check(|| unsafe {
                        notify_update_state_changed();
                    });
                    repaint_settings_window(hwnd, true);
                }
            }
            true
        }
        _ => false,
    }
}
