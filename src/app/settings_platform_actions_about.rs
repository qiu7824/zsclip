use super::prelude::*;
use super::update_dialog::{self, UpdateDialogRequest};
use crate::win_system_params::{IDC_SET_UPDATE_NOTES, IDC_SET_UPDATE_SOURCE};

fn begin_update_check() {
    start_update_check(|| unsafe {
        notify_update_state_changed();
    });
}

unsafe fn refresh_update_controls(hwnd: HWND) {
    if platform_window::is_window_alive(hwnd as isize) {
        refresh_about_update_status(hwnd);
        repaint_settings_window(hwnd, true);
    }
}

fn source_editor_content(
    result: Result<crate::update_feed::UpdateSource, String>,
) -> (crate::update_feed::UpdateSource, String) {
    let instructions = tr(
        "留空使用官方发布及蓝奏镜像；也可填写可信的 HTTPS 版本清单或含清单的蓝奏分享链接。",
        "Leave blank for official releases and Lanzou mirrors, or enter a trusted HTTPS manifest or Lanzou share containing a manifest.",
    );
    match result {
        Ok(source) => (source, instructions.into()),
        Err(error) => (
            crate::update_feed::UpdateSource::default(),
            format!("{}\n{}", instructions, error.chars().take(180).collect::<String>()),
        ),
    }
}

pub(super) unsafe fn handle_about_update_control(hwnd: HWND, control_id: isize) -> bool {
    match control_id {
        IDC_SET_OPEN_UPDATE => {
            execute_settings_platform_about_action(hwnd, SettingsAction::CheckForUpdates);
        }
        IDC_SET_UPDATE_NOTES => {
            let state = update_check_state_snapshot();
            let message = if state.latest_tag.is_empty() {
                tr(
                    "检查更新后可查看版本说明。",
                    "Check for updates to see the release notes.",
                )
                .to_string()
            } else {
                format!("{} {}", tr("版本：", "Version:"), state.latest_tag)
            };
            update_dialog::show(
                hwnd,
                UpdateDialogRequest {
                    title: tr("更新内容", "Release notes"),
                    message: &message,
                    text: &state.release_notes,
                    editable: false,
                    accept_label: None,
                },
                &mut |_| Ok(()),
            );
        }
        IDC_SET_UPDATE_SOURCE => {
            let (source, message) = source_editor_content(crate::shell::update_source());
            let saved = update_dialog::show(hwnd, UpdateDialogRequest {
                title: tr("更新源设置", "Update source"),
                message: &message,
                text: &source.manifest_url, editable: true,
                accept_label: Some(tr("保存并检查", "Save and check")),
            }, &mut |text| crate::shell::set_update_source(update_dialog::source_from_text(text)?));
            if saved {
                begin_update_check();
            }
            refresh_update_controls(hwnd);
        }
        _ => return false,
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn damaged_source_remains_editable_and_displays_the_read_error() {
        let (source, message) = source_editor_content(Err("损坏的版本配置".into()));
        assert!(source.manifest_url.is_empty());
        assert!(message.contains("损坏的版本配置"));
        let existing = crate::update_feed::UpdateSource {
            manifest_url: "https://updates.example.com/release.json".into(),
        };
        let (source, message) = source_editor_content(Ok(existing.clone()));
        assert!(source == existing);
        assert!(!message.contains("损坏的版本配置"));
    }
}

unsafe fn confirm_available_update(hwnd: HWND, state: crate::shell::UpdateCheckState) {
    let verified = state.installer.is_some();
    let message = if verified {
        format!(
            "{} {}\n{}",
            tr("新版本：", "New version:"),
            state.latest_tag,
            tr(
                "下载后将校验安装包并启动安装。",
                "The downloaded installer will be verified before installation starts."
            )
        )
    } else {
        format!(
            "{} {}\n{}",
            tr("新版本：", "New version:"),
            state.latest_tag,
            tr(
                "此发布没有可校验的安装包，可前往公开发布页下载。",
                "This release has no verifiable installer. You can visit its public release page."
            )
        )
    };
    let accepted = update_dialog::show(
        hwnd,
        UpdateDialogRequest {
            title: tr("软件更新", "Software update"),
            message: &message,
            text: &state.release_notes,
            editable: false,
            accept_label: Some(if verified {
                tr("下载并安装", "Download and install")
            } else {
                tr("打开发布页", "Open release page")
            }),
        },
        &mut |_| {
            if verified {
                let current = update_check_state_snapshot();
                if current.latest_tag != state.latest_tag || current.installer != state.installer {
                    return Err(tr(
                        "版本信息已变化，请关闭窗口后重新检查更新。",
                        "Release information changed. Close this window and check again.",
                    )
                    .into());
                }
                crate::shell::start_update_download(|| unsafe {
                    notify_update_state_changed();
                })
            } else {
                let url = if state.latest_url.is_empty() {
                    update_check_latest_url_or_default()
                } else {
                    state.latest_url.clone()
                };
                if !crate::update_feed::https_url(&url) {
                    return Err(tr(
                        "发布页面必须使用公开 HTTPS 地址。",
                        "The release page must use a public HTTPS address.",
                    )
                    .into());
                }
                Ok(())
            }
        },
    );
    if accepted && !verified {
        let url = if state.latest_url.is_empty() {
            update_check_latest_url_or_default()
        } else {
            state.latest_url
        };
        if crate::update_feed::https_url(&url) {
            open_path_with_shell(&url);
        }
    }
    refresh_update_controls(hwnd);
}

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
            if !update_state.checking && !update_state.downloading && !update_state.install_started
            {
                if update_state.available {
                    confirm_available_update(hwnd, update_state);
                } else {
                    begin_update_check();
                    refresh_update_controls(hwnd);
                }
            }
            true
        }
        _ => false,
    }
}
