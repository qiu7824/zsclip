use super::prelude::*;

pub(super) unsafe fn execute_settings_group_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    match action {
        SettingsAction::AddGroup => {
            let request = settings_group_text_input_request(SettingsGroupTextInputKind::Add, "");
            if let Some(name) = WindowsTextInputDialogHost::new().prompt_text(hwnd, request) {
                let category = source_tab_category(settings_group_view_current(st));
                match db_create_named_group(category, &name) {
                    Ok(group) => {
                        settings_groups_refresh_list(st, group.id);
                        let pst = get_state_ptr(st.parent_hwnd);
                        if !pst.is_null() {
                            reload_state_from_db_persisting(&mut *pst);
                            platform_gdi::invalidate_rect(st.parent_hwnd, null(), 1);
                        }
                    }
                    Err(e) => {
                        let message =
                            format!("{}: {}", tr("新建分组失败", "Failed to create group"), e);
                        show_native_dialog_message(
                            hwnd,
                            translate("分组").as_ref(),
                            &message,
                            NativeDialogLevel::Error,
                        );
                    }
                }
            }
            true
        }
        SettingsAction::RenameGroup => {
            if let Some((_, group)) = settings_groups_selected(st) {
                let request = settings_group_text_input_request(
                    SettingsGroupTextInputKind::Rename,
                    &group.name,
                );
                if let Some(new_name) = WindowsTextInputDialogHost::new().prompt_text(hwnd, request)
                {
                    if let Err(e) = db_rename_group(group.category, group.id, &new_name) {
                        let message = format!("{}: {}", tr("重命名失败", "Rename failed"), e);
                        show_native_dialog_message(
                            hwnd,
                            translate("分组").as_ref(),
                            &message,
                            NativeDialogLevel::Error,
                        );
                    } else {
                        settings_groups_refresh_list(st, group.id);
                        let pst = get_state_ptr(st.parent_hwnd);
                        if !pst.is_null() {
                            reload_state_from_db_persisting(&mut *pst);
                            platform_gdi::invalidate_rect(st.parent_hwnd, null(), 1);
                        }
                    }
                }
            } else {
                show_native_dialog_message(
                    hwnd,
                    translate("分组").as_ref(),
                    translate("请先选择一个分组。").as_ref(),
                    NativeDialogLevel::Info,
                );
            }
            true
        }
        SettingsAction::DeleteGroup => {
            if let Some((_, group)) = settings_groups_selected(st) {
                let ask = format!(
                    "{} \"{}\"?\n{}",
                    tr("确认删除分组", "Delete group"),
                    group.name,
                    tr(
                        "不会删除记录，只会清空这些记录的分组。",
                        "Records will be kept. Only their group assignment will be cleared."
                    )
                );
                if confirm_native_dialog(
                    hwnd,
                    translate("分组").as_ref(),
                    &ask,
                    NativeDialogLevel::Question,
                    NativeDialogButtons::YesNo,
                ) == NativeDialogResponse::Yes
                {
                    if let Err(e) = db_delete_group(group.id) {
                        let message =
                            format!("{}: {}", tr("删除分组失败", "Delete group failed"), e);
                        show_native_dialog_message(
                            hwnd,
                            translate("分组").as_ref(),
                            &message,
                            NativeDialogLevel::Error,
                        );
                    } else {
                        settings_groups_refresh_list(st, 0);
                        let pst = get_state_ptr(st.parent_hwnd);
                        if !pst.is_null() {
                            reload_state_from_db_persisting(&mut *pst);
                            platform_gdi::invalidate_rect(st.parent_hwnd, null(), 1);
                        }
                    }
                }
            }
            true
        }
        SettingsAction::MoveGroupUp => {
            settings_groups_move(st, -1);
            true
        }
        SettingsAction::MoveGroupDown => {
            settings_groups_move(st, 1);
            true
        }
        SettingsAction::GroupSelectionChanged => {
            settings_groups_sync_name(st);
            true
        }
        SettingsAction::ShowRecordGroups => {
            st.group_view_tab = 0;
            settings_sync_group_overview(st);
            true
        }
        SettingsAction::ShowPhraseGroups => {
            st.group_view_tab = 1;
            settings_sync_group_overview(st);
            true
        }
        _ => false,
    }
}
