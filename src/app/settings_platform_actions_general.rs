use super::prelude::*;

pub(super) unsafe fn execute_settings_platform_general_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    match action {
        SettingsAction::PickPasteSound => {
            match pick_paste_sound_file(&st.draft.paste_success_sound_path) {
                Ok(Some(path)) => {
                    st.draft.paste_success_sound_kind = "custom".to_string();
                    st.draft.paste_success_sound_path = path;
                    settings_set_text(
                        st.cb_paste_sound,
                        &paste_sound_display(&st.draft.paste_success_sound_kind),
                    );
                    settings_sync_page_state(st, SettingsPage::General.index());
                    repaint_settings_window(hwnd, true);
                }
                Ok(None) => {}
                Err(err) => {
                    let message = format!(
                        "{}: {}",
                        tr("选择提示音文件失败", "Failed to choose sound file"),
                        err
                    );
                    show_native_dialog_message(
                        hwnd,
                        tr("粘贴成功声音", "Paste success sound"),
                        &message,
                        NativeDialogLevel::Error,
                    );
                }
            }
            true
        }
        SettingsAction::CaptureSkippedWindowClass => {
            let skip_class_names = settings_host_text(st.ed_skip_class_names);
            let target = find_next_paste_target_after(hwnd, st.parent_hwnd, &skip_class_names);
            let identity_host = WindowsWindowIdentityHost::new();
            if !identity_host.exists(target) || identity_host.is_current_process_window(target) {
                show_native_dialog_message(
                    hwnd,
                    tr("捕获当前", "Capture current"),
                    tr(
                        "设置窗口后面没有可捕获的外部窗口。",
                        "No external window is available after the settings window.",
                    ),
                    NativeDialogLevel::Info,
                );
            } else {
                let class_name = vv_window_class_name(target);
                if class_name.trim().is_empty() {
                    show_native_dialog_message(
                        hwnd,
                        tr("捕获当前", "Capture current"),
                        tr(
                            "当前窗口没有可用类名。",
                            "The current window does not expose a usable class name.",
                        ),
                        NativeDialogLevel::Info,
                    );
                } else {
                    let merged = append_unique_skip_class_name(
                        &settings_host_text(st.ed_skip_class_names),
                        &class_name,
                    );
                    st.draft.paste_target_skip_class_names = merged.clone();
                    settings_set_text(st.ed_skip_class_names, &merged);
                    repaint_settings_control(st.ed_skip_class_names);
                }
            }
            true
        }
        _ => false,
    }
}
