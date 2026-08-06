use super::prelude::*;

unsafe fn settings_parent_generation(st: &SettingsWndState) -> Option<u64> {
    let parent = get_state_ptr(st.parent_hwnd);
    (!parent.is_null()).then(|| (*parent).app_data_generation)
}

unsafe fn refresh_settings_groups_after_stale_data(st: &mut SettingsWndState) {
    let parent = get_state_ptr(st.parent_hwnd);
    if !parent.is_null() {
        apply_loaded_settings(st.parent_hwnd, &mut *parent);
    }
    settings_groups_refresh_list(st, 0);
}

pub(super) unsafe fn execute_settings_group_action(
    hwnd: HWND,
    st: &mut SettingsWndState,
    action: SettingsAction,
) -> bool {
    match action {
        SettingsAction::AddGroup => {
            let Some(expected_generation) = settings_parent_generation(st) else {
                return true;
            };
            let request = settings_group_text_input_request(SettingsGroupTextInputKind::Add, "");
            if let Some(name) = WindowsTextInputDialogHost::new().prompt_text(hwnd, request) {
                let category = source_tab_category(settings_group_view_current(st));
                let result =
                    crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
                        db_create_named_group(category, &name)
                    });
                let Some(result) = result else {
                    refresh_settings_groups_after_stale_data(st);
                    return true;
                };
                match result {
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
            let Some(expected_generation) = settings_parent_generation(st) else {
                return true;
            };
            if let Some((_, group)) = settings_groups_selected(st) {
                let request = settings_group_text_input_request(
                    SettingsGroupTextInputKind::Rename,
                    &group.name,
                );
                if let Some(new_name) = WindowsTextInputDialogHost::new().prompt_text(hwnd, request)
                {
                    let result = crate::db_runtime::with_shared_app_data_generation(
                        expected_generation,
                        || db_rename_group(group.category, group.id, &new_name),
                    );
                    let Some(result) = result else {
                        refresh_settings_groups_after_stale_data(st);
                        return true;
                    };
                    if let Err(e) = result {
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
            let Some(expected_generation) = settings_parent_generation(st) else {
                return true;
            };
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
                    let result = crate::db_runtime::with_shared_app_data_generation(
                        expected_generation,
                        || db_delete_group(group.id),
                    );
                    let Some(result) = result else {
                        refresh_settings_groups_after_stale_data(st);
                        return true;
                    };
                    if let Err(e) = result {
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
