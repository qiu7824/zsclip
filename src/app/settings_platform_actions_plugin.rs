use super::prelude::*;

pub(super) unsafe fn execute_settings_platform_plugin_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    match action {
        SettingsAction::RestoreSearchEnginePreset => {
            let key = search_engine_key_from_display(&settings_host_text(st.cb_engine));
            settings_set_text(st.ed_tpl, search_engine_template(key));
            true
        }
        SettingsAction::DetectOcrRuntime => {
            if let Some(path) = detect_wechat_runtime_dir(&settings_host_text(st.ed_ocr_cloud_url))
            {
                st.draft.image_ocr_wechat_dir = path;
                settings_sync_page_state(st, SettingsPage::Plugin.index());
                repaint_settings_window(hwnd, true);
            } else {
                show_native_dialog_message(
                    hwnd,
                    tr("WinOCR（微信 OCR）", "WinOCR (WeChat OCR)"),
                    tr(
                        "未能自动检测到微信目录，请先启动微信或手动填写安装目录。",
                        "Could not auto-detect the WeChat directory. Please start WeChat or enter the install directory manually.",
                    ),
                    NativeDialogLevel::Info,
                );
            }
            true
        }
        #[cfg(feature = "mail-merge")]
        SettingsAction::OpenMailMerge => {
            WindowsMailMergeWindowHost::new().open_mail_merge(
                hwnd,
                NativeMailMergeWindowRequest {
                    initial_excel_path: None,
                },
            );
            true
        }
        #[cfg(not(feature = "mail-merge"))]
        SettingsAction::OpenMailMerge => {
            show_native_dialog_message(
                hwnd,
                tr("邮件合并", "Mail Merge"),
                tr(
                    "当前构建未启用邮件合并功能。",
                    "This build was compiled without the mail merge feature.",
                ),
                NativeDialogLevel::Info,
            );
            true
        }
        SettingsAction::OpenWpsTaskpaneDocs => {
            let path = std::env::current_dir()
                .unwrap_or_else(|_| PathBuf::from("."))
                .join("docs")
                .join("wps-taskpane.md");
            open_path_with_shell(path.to_string_lossy().as_ref());
            true
        }
        _ => false,
    }
}
