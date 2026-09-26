use super::prelude::*;
use crate::settings_model::SettingsUpdatePresentation;
use crate::win_system_params::{IDC_SET_UPDATE_NOTES, IDC_SET_UPDATE_SOURCE};
use crate::win_system_ui::settings_host_set_enabled;

fn about_update_presentation(state: &crate::shell::UpdateCheckState) -> SettingsUpdatePresentation {
    let mut presentation = settings_update_presentation(&SettingsUpdatePresentationInput {
        started: state.started,
        checking: state.checking,
        available: state.available,
        latest_tag: state.latest_tag.clone(),
        error: state.error.clone(),
    });
    if state.install_started {
        presentation.status_text = tr(
            "安装程序已启动，请按提示完成更新。",
            "The installer has started. Follow its instructions to finish updating.",
        )
        .into();
        presentation.button_text = tr("安装已启动", "Installer started").into();
    } else if state.downloading {
        let stage = if state.download_status.trim().is_empty() {
            tr("正在下载更新", "Downloading update")
        } else {
            &state.download_status
        };
        presentation.status_text = if state.total > 0 {
            format!(
                "{} · {}% ({:.1} / {:.1} MB)",
                stage,
                state
                    .downloaded
                    .saturating_mul(100)
                    .checked_div(state.total)
                    .unwrap_or(0)
                    .min(100),
                state.downloaded as f64 / 1_048_576.0,
                state.total as f64 / 1_048_576.0
            )
        } else {
            stage.to_string()
        };
        presentation.button_text = tr("正在更新…", "Updating...").into();
    } else if !state.checking && !state.error.trim().is_empty() {
        presentation.status_text = format!(
            "{} {}",
            tr("更新失败：", "Update failed:"),
            state
                .error
                .lines()
                .next()
                .unwrap_or_default()
                .chars()
                .take(120)
                .collect::<String>()
        );
        presentation.button_text = if state.available && state.installer.is_some() {
            tr("重试下载", "Retry download")
        } else if state.available {
            tr("打开发布页", "Open release page")
        } else {
            tr("再次检查", "Check again")
        }
        .into();
    } else if !state.checking && state.available {
        presentation.button_text = if state.installer.is_some() {
            tr("下载并安装", "Download and install")
        } else {
            tr("打开发布页", "Open release page")
        }
        .into();
    }
    presentation
}

fn update_action_enabled(state: &crate::shell::UpdateCheckState) -> bool {
    !state.checking && !state.downloading && !state.install_started
}

pub(super) unsafe fn settings_create_about_update_section(
    st: &mut SettingsWndState,
    b: &SettingsPageBuilder,
    flow: &mut SettingsFlowLayout,
) {
    let update_state = update_check_state_snapshot();
    let update = about_update_presentation(&update_state);
    let update_rect = flow.full_rect(settings_scale(24));
    st.lb_update_status = b.label(
        st,
        &update.status_text,
        update_rect.left,
        update_rect.top,
        update_rect.right - update_rect.left,
        update_rect.height(),
    );
    flow.consume_full(update_rect.height(), settings_scale(6));
    let update_button = flow.button_rect(settings_scale(160), settings_scale(32));
    st.btn_open_update = b.button(
        st,
        &update.button_text,
        IDC_SET_OPEN_UPDATE,
        update_button.left,
        update_button.top,
        update_button.right - update_button.left,
    );
    settings_host_set_enabled(st.btn_open_update, update_action_enabled(&update_state));
    if !st.btn_open_update.is_null() {
        st.ownerdraw_ctrls.push(st.btn_open_update);
    }
    let gap = settings_scale(8);
    let secondary_w = settings_scale(128);
    for (index, (text, id)) in [
        (tr("更新内容", "Release notes"), IDC_SET_UPDATE_NOTES),
        (tr("更新源设置", "Update source"), IDC_SET_UPDATE_SOURCE),
    ]
    .into_iter()
    .enumerate()
    {
        let button = b.button(
            st,
            text,
            id,
            update_button.right + gap + index as i32 * (secondary_w + gap),
            update_button.top,
            secondary_w,
        );
        if !button.is_null() {
            st.ownerdraw_ctrls.push(button);
            if id == IDC_SET_UPDATE_SOURCE {
                settings_host_set_enabled(button, update_action_enabled(&update_state));
            }
        }
    }
    flow.consume_full(settings_scale(32), 0);
}

pub(super) unsafe fn refresh_about_update_status(hwnd: HWND) {
    let ptr = platform_window::user_data(hwnd) as *mut SettingsWndState;
    if ptr.is_null() {
        return;
    }
    let st = &mut *ptr;
    if !st.ui.is_built(SettingsPage::About.index()) {
        return;
    }
    let state = update_check_state_snapshot();
    let presentation = about_update_presentation(&state);
    settings_set_text(st.lb_update_status, &presentation.status_text);
    settings_set_text(st.btn_open_update, &presentation.button_text);
    settings_host_set_enabled(st.btn_open_update, update_action_enabled(&state));
    for reg in st.ui.page_regs(SettingsPage::About.index()) {
        if windows_sys::Win32::UI::WindowsAndMessaging::GetDlgCtrlID(reg.hwnd)
            == IDC_SET_UPDATE_SOURCE as i32
        {
            settings_host_set_enabled(reg.hwnd, update_action_enabled(&state));
            repaint_settings_control(reg.hwnd);
        }
    }
    repaint_settings_control(st.lb_update_status);
    repaint_settings_control(st.btn_open_update);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn update_progress_and_install_state_disable_duplicate_actions() {
        let mut state = crate::shell::UpdateCheckState::default();
        assert!(update_action_enabled(&state));
        state.checking = true;
        assert!(!update_action_enabled(&state));
        state.checking = false;
        state.downloading = true;
        state.download_status = "Downloading".into();
        state.downloaded = 5 * 1_048_576;
        state.total = 10 * 1_048_576;
        let progress = about_update_presentation(&state);
        assert!(progress.status_text.contains("50%"));
        assert!(progress.status_text.contains("5.0 / 10.0 MB"));
        assert!(!update_action_enabled(&state));
        state.downloading = false;
        state.install_started = true;
        assert!(!update_action_enabled(&state));
        assert_eq!(
            about_update_presentation(&state).button_text,
            tr("安装已启动", "Installer started")
        );
    }

    #[test]
    fn available_without_verified_asset_opens_release_page_and_errors_remain_visible() {
        let mut state = crate::shell::UpdateCheckState::default();
        state.started = true;
        state.available = true;
        state.latest_tag = "1.0.99".into();
        assert_eq!(
            about_update_presentation(&state).button_text,
            tr("打开发布页", "Open release page")
        );
        state.error = "checksum mismatch\nprivate technical detail".into();
        let error = about_update_presentation(&state);
        assert!(error.status_text.contains("checksum mismatch"));
        assert!(!error.status_text.contains("private technical detail"));
        assert!(update_action_enabled(&state));
    }
}
