use super::prelude::*;

pub(super) const ABOUT_DATA_RETENTION_EN: &str = "History uses the record limit and a 512 MB payload budget, keeping the newest item. Unlimited disables pruning. Pinned items and phrases are retained. Database free pages are reused; unused images are reclaimed later.";

pub(super) unsafe fn settings_create_about_data_section(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    flow: &mut SettingsFlowLayout,
) {
    let directory_text = format!(
        "{}{}",
        tr("数据目录：", "Data directory: "),
        data_dir().to_string_lossy()
    );
    let directory_rect = flow.full_rect(settings_scale(32));
    b.label(
        st,
        &directory_text,
        directory_rect.left,
        directory_rect.top,
        directory_rect.width() - settings_scale(174),
        directory_rect.height(),
    );
    let button = b.button(st, tr("更改数据目录…", "Change data directory…"),
        crate::win_system_params::IDC_SET_DATA_DIRECTORY, directory_rect.right - settings_scale(160),
        directory_rect.top, settings_scale(160));
    if !button.is_null() { st.ownerdraw_ctrls.push(button); }
    flow.consume_full(directory_rect.height(), settings_scale(4));

    let info_text = tr(
        "普通历史按条数和 512 MB 内容预算清理，至少保留最新一条；条数不限时不自动清理。置顶和常用短语保留。数据库空闲页复用，未引用图片延迟回收。",
        ABOUT_DATA_RETENTION_EN,
    );
    let info_rect = flow.full_rect(settings_scale(24));
    b.label_auto(
        st,
        info_text,
        info_rect.left,
        info_rect.top,
        info_rect.right - info_rect.left,
        info_rect.height(),
    );
}

pub(super) unsafe fn change_data_directory(hwnd: HWND) {
    let dialog = platform_dialog::WindowsDialogHost::new();
    if std::env::var_os("ZSCLIP_DATA_DIR").is_some() {
        dialog.show_message(hwnd, tr("数据目录", "Data directory"),
            tr("当前目录由 ZSCLIP_DATA_DIR 指定，请修改该环境变量后重新启动。",
                "The directory is controlled by ZSCLIP_DATA_DIR. Change that variable and restart."), NativeDialogLevel::Info);
        return;
    }
    let current = data_dir();
    let initial = current.to_string_lossy();
    let Some(value) = WindowsTextInputDialogHost::new().prompt_text(hwnd,
        crate::app_core::NativeTextInputDialogRequest {
            title: tr("更改数据目录", "Change data directory"),
            label: tr("新目录绝对路径（空文件夹，重启生效）：",
                "New folder path (empty; applies on restart):"),
            initial: &initial,
        }) else { return; };
    let result = crate::data_location::config_file().ok_or_else(|| "无法获取用户配置目录。".to_string())
        .and_then(|file| crate::data_location::request_change(&file, &current, Path::new(value.trim())));
    match result {
        Ok(()) => dialog.show_message(hwnd, tr("数据目录", "Data directory"),
            tr("数据目录设置已保存。退出并重新启动 ZSClip 后，程序会复制校验历史、设置和图片，再使用新目录；原目录保留。",
                "Data directory saved. Quit and restart ZSClip to copy and verify history, settings and images, then use the new directory. The original is retained."), NativeDialogLevel::Info),
        Err(error) => dialog.show_message(hwnd, tr("数据目录", "Data directory"), &error, NativeDialogLevel::Error),
    };
}
