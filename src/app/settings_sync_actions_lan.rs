use super::prelude::*;

pub(super) unsafe fn execute_settings_lan_sync_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    match action {
        SettingsAction::RefreshLanDevices => {
            settings_collect_to_app(st);
            let pst = get_state_ptr(st.parent_hwnd);
            if !pst.is_null() {
                crate::lan_sync::trigger_discovery(&(*pst).settings);
            }
            settings_sync_page_state(st, SettingsPage::Cloud.index());
            true
        }
        SettingsAction::PairLanDevice => {
            settings_collect_to_app(st);
            if settings_lan_selected_pair(st).is_some() {
                show_native_dialog_message(
                    hwnd,
                    tr("局域网同步", "LAN Sync"),
                    tr(
                        "这是待允许请求，请点击允许配对",
                        "This is a pending request. Click Allow Pairing.",
                    ),
                    NativeDialogLevel::Info,
                );
            } else {
                let host = settings_lan_selected_device(st)
                    .map(|device| format!("{}:{}", device.addr, device.tcp_port))
                    .filter(|host| !host.trim().is_empty())
                    .unwrap_or_else(|| st.draft.lan_manual_host.clone());
                crate::lan_sync::start_pair_with_host(st.parent_hwnd, st.draft.clone(), host);
            }
            settings_sync_page_state(st, SettingsPage::Cloud.index());
            true
        }
        SettingsAction::AcceptLanPairing => {
            if let Some(pair) = settings_lan_selected_pair(st) {
                crate::lan_sync::accept_pair_request(&pair.pair_id);
            } else {
                show_native_dialog_message(
                    hwnd,
                    tr("局域网同步", "LAN Sync"),
                    tr(
                        "请先在附近设备列表中选择一个 [待允许] 请求",
                        "Please select a pending request first",
                    ),
                    NativeDialogLevel::Info,
                );
            }
            settings_sync_page_state(st, SettingsPage::Cloud.index());
            true
        }
        SettingsAction::RejectLanPairing => {
            if let Some(pair) = settings_lan_selected_pair(st) {
                crate::lan_sync::reject_pair_request(&pair.pair_id);
            }
            settings_sync_page_state(st, SettingsPage::Cloud.index());
            true
        }
        SettingsAction::CopyLanPairUrl | SettingsAction::CopyLanSetupUrl => {
            copy_settings_lan_pairing_url(hwnd, st, action);
            true
        }
        SettingsAction::OpenLanSetupPage => {
            open_settings_lan_setup_page(hwnd, st);
            true
        }
        _ => false,
    }
}

unsafe fn copy_settings_lan_pairing_url(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) {
    if multi_sync_mode_from_settings(&st.draft) != "lan" {
        show_native_dialog_message(
            hwnd,
            tr("多端同步", "Multi-device Sync"),
            tr(
                "请先在同步方案中选择局域网，再复制扫码绑定链接。",
                "Choose LAN as the sync method before copying a pairing link.",
            ),
            NativeDialogLevel::Info,
        );
        return;
    }

    let url = if action == SettingsAction::CopyLanPairUrl {
        crate::lan_sync::mobile_pair_url(&st.draft)
    } else {
        crate::lan_sync::mobile_setup_url(&st.draft)
    };
    if let Some(url) = url {
        skip_next_clipboard_update_for_all_hosts();
        set_ignore_clipboard_for_all_hosts(1200);
        copy_text_to_clipboard_in_background(url);
        let sender = if action == SettingsAction::CopyLanPairUrl {
            st.btn_lan_copy_pair
        } else {
            st.btn_lan_copy_setup
        };
        if !sender.is_null() {
            repaint_settings_control(sender);
        }
    } else {
        show_native_dialog_message(
            hwnd,
            tr("扫码绑定", "Pairing QR"),
            tr(
                "请先保存并启动局域网服务，再复制链接。",
                "Save and start the LAN service before copying the link.",
            ),
            NativeDialogLevel::Info,
        );
    }
}

unsafe fn open_settings_lan_setup_page(hwnd: HWND, st: &SettingsWndState) {
    if multi_sync_mode_from_settings(&st.draft) != "lan" {
        show_native_dialog_message(
            hwnd,
            tr("多端同步", "Multi-device Sync"),
            tr(
                "请先在同步方案中选择局域网，再打开扫码绑定页。",
                "Choose LAN as the sync method before opening the pairing QR page.",
            ),
            NativeDialogLevel::Info,
        );
    } else if let Some(url) = crate::lan_sync::mobile_setup_url(&st.draft) {
        open_path_with_shell(&url);
    } else {
        show_native_dialog_message(
            hwnd,
            tr("局域网同步", "LAN Sync"),
            tr(
                "扫码绑定页暂时不可用，请确认局域网服务已启动。",
                "The mobile setup page is unavailable. Check that LAN sync is running.",
            ),
            NativeDialogLevel::Warning,
        );
    }
}
