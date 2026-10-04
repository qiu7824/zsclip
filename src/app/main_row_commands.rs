use super::prelude::*;

unsafe fn quick_search_open(settings: &AppSettings, text: &str) {
    if !settings.quick_search_enabled {
        return;
    }
    let mut raw = text.trim().to_string();
    if raw.is_empty() {
        return;
    }
    if raw.chars().count() > 200 {
        raw = raw.chars().take(200).collect();
    }
    let query = raw.replace(['\r', '\n'], " ");
    let encoded_query = url_encode_component(&query);
    let encoded_raw = url_encode_component(&raw);
    let template = if settings.search_template.trim().is_empty() {
        search_engine_template(&settings.search_engine).to_string()
    } else {
        settings.search_template.clone()
    };
    let url = template
        .replace("{key}", &encoded_query)
        .replace("{q}", &encoded_query)
        .replace("{raw}", &encoded_raw);
    open_path_with_shell(&url);
}

fn url_encode_component(value: &str) -> String {
    let mut encoded = String::with_capacity(value.len() * 3 / 2);
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(*byte as char)
            }
            b' ' => encoded.push('+'),
            _ => encoded.push_str(&format!("%{:02X}", *byte)),
        }
    }
    encoded
}

fn select_context_row(state: &mut AppState) -> bool {
    let Some(plan) = state.list.context_row_selection_plan() else {
        return false;
    };
    state.list.apply_context_row_selection_plan(plan);
    true
}

unsafe fn execute_row_external_action(hwnd: HWND, state: &mut AppState, action: MainRowMenuAction) {
    let current = state.current_item_for_use();
    let selected = if matches!(
        action,
        MainRowMenuAction::CopyPath | MainRowMenuAction::LanPush
    ) {
        state.selected_items_for_use()
    } else {
        Vec::new()
    };
    let Some(plan) = main_row_external_action_plan(action, current.as_ref(), &selected) else {
        return;
    };
    match plan {
        MainRowExternalActionPlan::OpenPaths(paths) => {
            for path in paths {
                open_path_with_shell(&path);
            }
        }
        MainRowExternalActionPlan::OpenParentFolders(paths) => {
            for path in paths {
                open_parent_folder(&path);
            }
        }
        MainRowExternalActionPlan::CopyText(text) => {
            let copied = platform_clipboard::WindowsClipboardHost::write_text(&text);
            play_copy_success_sound_if_enabled(state, copied);
        }
        MainRowExternalActionPlan::LanPushFiles(paths) => {
            #[cfg(feature = "lan-sync")]
            if paths.is_empty() {
                platform_dialog::WindowsDialogHost::new().show_message(
                    hwnd,
                    tr("局域网同步", "LAN Sync"),
                    tr("没有可推送的文件", "No files to push"),
                    NativeDialogLevel::Info,
                );
            } else {
                lan_sync::push_files_to_trusted(&state.settings, paths);
            }
            #[cfg(not(feature = "lan-sync"))]
            {
                let _ = paths;
            }
        }
        MainRowExternalActionPlan::QuickSearch(text) => {
            quick_search_open(&state.settings, &text);
        }
        MainRowExternalActionPlan::TextTranslate(text) => {
            spawn_text_translate_text_job(
                hwnd,
                state.app_data_generation,
                state.settings.clone(),
                text,
            );
        }
        MainRowExternalActionPlan::QrText(text) => {
            if let Some((qr_item, sig)) = build_qr_clip_item(&text) {
                let _ = apply_item_to_clipboard(state, &qr_item);
                state.add_clip_item(qr_item, sig);
                repaint_main_window(hwnd, true);
            } else {
                platform_dialog::WindowsDialogHost::new().show_message(
                    hwnd,
                    tr("快捷转换二维码", "Quick QR Convert"),
                    tr("二维码生成失败", "QR generation failed"),
                    NativeDialogLevel::Error,
                );
            }
        }
    }
}

unsafe fn execute_row_dialog_action(hwnd: HWND, state: &mut AppState, action: MainRowMenuAction) {
    let current = state.current_item_for_use();
    let Some(plan) = main_row_dialog_action_plan(action, current.as_ref()) else {
        return;
    };
    match plan {
        #[cfg(feature = "mail-merge")]
        MainRowDialogActionPlan::MailMerge { excel_path } => {
            WindowsMailMergeWindowHost::new().open_mail_merge(
                hwnd,
                NativeMailMergeWindowRequest {
                    initial_excel_path: excel_path.as_deref(),
                },
            );
        }
        #[cfg(not(feature = "mail-merge"))]
        MainRowDialogActionPlan::MailMerge { .. } => {
            platform_dialog::WindowsDialogHost::new().show_message(
                hwnd,
                tr("邮件合并", "Mail Merge"),
                tr(
                    "当前构建未启用邮件合并功能。",
                    "This build was compiled without the mail merge feature.",
                ),
                NativeDialogLevel::Info,
            );
        }
        MainRowDialogActionPlan::EditItem { item_id, title } => {
            let phrase_title = current.as_ref().filter(|item| state.settings.phrase_titles_enabled && item.kind == ClipKind::Phrase)
                .map(|item| item.phrase_title.clone());
            let expected_generation = state.app_data_generation;
            let Some(initial_text) =
                crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
                    db_item_text(item_id)
                })
            else {
                apply_loaded_settings(hwnd, state);
                return;
            };
            let initial_text = match initial_text {
                Ok(text) => text,
                Err(_) => {
                    platform_dialog::WindowsDialogHost::new().show_message(hwnd,
                        tr("编辑记录", "Edit Record"), tr("记录无法读取，请重新打开后重试。", "The record could not be read. Reopen it and try again."), NativeDialogLevel::Error);
                    return;
                }
            };
            let initial_size =
                if state.settings.edit_dialog_w > 0 && state.settings.edit_dialog_h > 0 {
                    Some(UiSize {
                        width: state.settings.edit_dialog_w,
                        height: state.settings.edit_dialog_h,
                    })
                } else {
                    None
                };
            let mut save_handler = |text: &str| {
                crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
                    db_update_item_text(item_id, text).map_err(|_| {
                        tr("保存失败，请稍后重试。", "Save failed. Please try again.").to_string()
                    })
                })
                .unwrap_or_else(|| {
                    Err(tr(
                        "数据已在恢复期间更新，请关闭编辑框后重试。",
                        "Data changed during restore. Close the editor and try again.",
                    )
                    .to_string())
                })
            };
            let request = NativeEditTextDialogRequest {
                title: &title, initial_text: &initial_text, initial_size,
            };
            let result = if let Some(phrase_title) = phrase_title {
                let mut save_phrase = |title: &str, body: &str| {
                    crate::db_runtime::with_shared_app_data_generation(expected_generation, || db_save_phrase(item_id, title, body))
                        .unwrap_or_else(|| Err("数据已更新，请重新打开编辑框".into()))
                };
                WindowsEditTextDialogHost::new().open_phrase_editor(hwnd, request, &phrase_title, &mut save_phrase)
            } else { WindowsEditTextDialogHost::new().open_edit_text(
                hwnd,
                request,
                &mut save_handler,
            ) };
            if let Some(size) = result.final_size {
                state.settings.edit_dialog_w = size.width;
                state.settings.edit_dialog_h = size.height;
                save_state_settings(state);
            }
            if result.saved {
                if state.app_data_generation != crate::db_runtime::current_app_data_generation() {
                    apply_loaded_settings(hwnd, state);
                    return;
                }
                reload_state_from_db_persisting(state);
                state.refilter();
                sync_peer_windows_from_db(hwnd);
                refresh_lan_latest_from_db(&state.settings);
                repaint_main_window(hwnd, true);
            }
        }
    }
}

unsafe fn execute_row_current_item_action(
    hwnd: HWND,
    state: &mut AppState,
    action: MainRowMenuAction,
) {
    let current = state.current_item_for_use();
    let Some(plan) = main_row_current_item_action_plan(action, current.as_ref()) else {
        return;
    };
    match plan {
        #[cfg(feature = "sticker")]
        MainRowCurrentItemActionPlan::Sticker { item } => {
            show_image_sticker(hwnd, &item, &state.settings);
        }
        #[cfg(not(feature = "sticker"))]
        MainRowCurrentItemActionPlan::Sticker { .. } => {
            platform_dialog::WindowsDialogHost::new().show_message(
                hwnd,
                tr("贴图", "Sticker"),
                tr(
                    "当前构建未启用贴图功能。",
                    "This build was compiled without the sticker feature.",
                ),
                NativeDialogLevel::Info,
            );
        }
        MainRowCurrentItemActionPlan::SaveImage { item } => {
            super::image_save::save_image_as(hwnd, &item);
        }
        MainRowCurrentItemActionPlan::ImageOcr { item } => {
            spawn_image_ocr_job(
                hwnd,
                state.app_data_generation,
                state.settings.clone(),
                item,
            );
        }
    }
}

unsafe fn execute_row_data_action(hwnd: HWND, state: &mut AppState, action: MainRowMenuAction) {
    let current = state.current_item_for_use();
    let selected = if matches!(action, MainRowMenuAction::ToPhrase) {
        state.selected_items_for_use()
    } else {
        Vec::new()
    };
    if action == MainRowMenuAction::ToPhrase && selected.len() <= 1 {
        if let Some(item) = selected.first().or(current.as_ref()) {
            let expected_generation = state.app_data_generation;
            let initial = item.text.as_deref().unwrap_or(&item.preview);
            let mut saved_id = None;
            let mut save = |title: &str, body: &str| {
                let mut edited = item.clone();
                edited.phrase_title = crate::app_core::normalize_phrase_title(title)?.to_string();
                if body != initial { edited.rich_text_html = None; }
                edited.text = Some(body.to_string());
                edited.preview = body.chars().take(120).collect();
                let result = crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
                    if let Some(id) = saved_id { db_save_phrase(id, title, body) }
                    else {
                        let id = db_add_phrase_from_item(&edited).map_err(|_| "保存短语失败".to_string())?;
                        if id == 0 { return Err("受保护内容不能保存到常用短语".into()); }
                        saved_id = Some(id);
                        Ok(())
                    }
                });
                result.unwrap_or_else(|| Err("数据已更新，请重新打开编辑框".into()))
            };
            let request = NativeEditTextDialogRequest {
                title: tr("保存常用短语", "Save Phrase"), initial_text: initial, initial_size: None,
            };
            let result = if state.settings.phrase_titles_enabled {
                WindowsEditTextDialogHost::new().open_phrase_editor(hwnd, request, &item.phrase_title, &mut save)
            } else {
                let mut save_body = |body: &str| save(&item.phrase_title, body);
                WindowsEditTextDialogHost::new().open_edit_text(hwnd, request, &mut save_body)
            };
            if result.saved {
                reload_state_from_db_persisting(state);
                state.refilter();
                sync_peer_windows_from_db(hwnd);
                repaint_main_window(hwnd, true);
            }
        }
        return;
    }
    let Some(plan) =
        main_row_data_action_plan(action, current.as_ref(), &selected, state.tab_index)
    else {
        return;
    };
    execute_row_data_plan(hwnd, state, plan);
}

pub(super) const RENAME_PHRASE_COMMAND: usize = 41021;

pub(super) unsafe fn rename_phrase(hwnd: HWND, state: &mut AppState, item: &ClipItem) {
    if !state.settings.phrase_titles_enabled || item.kind != ClipKind::Phrase { return; }
    let generation = state.app_data_generation;
    let Some(title) = WindowsTextInputDialogHost::new().prompt_text(hwnd,
        crate::app_core::NativeTextInputDialogRequest { title: tr("重命名短语", "Rename Phrase"),
            label: tr("标题（可空，最多 60 个字符）：", "Title (optional, up to 60 characters):"), initial: &item.phrase_title }) else { return; };
    let result = crate::db_runtime::with_shared_app_data_generation(generation, || {
        db_save_phrase(item.id, &title, item.text.as_deref().unwrap_or(&item.preview))
    }).unwrap_or_else(|| Err("数据已更新，请重新打开编辑框".into()));
    match result {
        Ok(()) => { reload_state_from_db_persisting(state); state.refilter(); sync_peer_windows_from_db(hwnd); }
        Err(message) => { platform_dialog::WindowsDialogHost::new().show_message(hwnd, tr("重命名短语", "Rename Phrase"), &message, NativeDialogLevel::Error); }
    }
}

unsafe fn execute_row_data_plan(hwnd: HWND, state: &mut AppState, plan: MainRowDataActionPlan) {
    let expected_generation = state.app_data_generation;
    if crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
        execute_row_data_plan_locked(hwnd, state, plan);
    })
    .is_none()
    {
        apply_loaded_settings(hwnd, state);
    }
}

unsafe fn execute_row_data_plan_locked(
    hwnd: HWND,
    state: &mut AppState,
    plan: MainRowDataActionPlan,
) {
    match plan {
        MainRowDataActionPlan::AddToPhrase {
            items,
            refilter_current_tab,
        } => {
            for item in &items {
                let _ = db_add_phrase_from_item(item);
            }
            state.invalidate_tab_query(1, refilter_current_tab);
            if refilter_current_tab {
                state.refilter();
            }
            sync_peer_windows_from_db(hwnd);
            repaint_main_window(hwnd, true);
        }
        MainRowDataActionPlan::UpdatePinned {
            ids,
            pinned,
            invalidate_tab,
            clear_selection,
        } => {
            for &id in &ids {
                let _ = db_update_item_pinned(id, pinned);
                state.remove_cached_item(id);
            }
            if clear_selection {
                state.clear_selection();
            }
            state.invalidate_tab_query(invalidate_tab, true);
            state.refilter();
            sync_peer_windows_from_db(hwnd);
            repaint_main_window(hwnd, true);
        }
        MainRowDataActionPlan::DeleteItems {
            ids,
            clear_selection,
            preserve_scroll_anchor,
        } => {
            let anchor = preserve_scroll_anchor
                .then(|| state.current_scroll_anchor())
                .flatten();
            for &id in &ids {
                let _ = db_delete_item(id);
                state.remove_cached_item(id);
            }
            state.remove_items_from_active_tab(&ids);
            if clear_selection {
                state.clear_selection();
            }
            state.refilter();
            if preserve_scroll_anchor {
                state.restore_scroll_anchor(anchor);
            }
            sync_peer_windows_from_db(hwnd);
            refresh_lan_latest_from_db(&state.settings);
            repaint_main_window(hwnd, true);
        }
        MainRowDataActionPlan::DeleteUnpinned {
            category,
            active_tab,
            clear_selection,
            preserve_scroll_anchor,
        } => {
            let anchor = preserve_scroll_anchor
                .then(|| state.current_scroll_anchor())
                .flatten();
            if db_delete_unpinned_items(category).is_ok() {
                state
                    .items_for_tab_mut(active_tab)
                    .retain(|item| item.pinned);
                if clear_selection {
                    state.clear_selection();
                }
                state.refilter();
                if preserve_scroll_anchor {
                    state.restore_scroll_anchor(anchor);
                }
                sync_peer_windows_from_db(hwnd);
                refresh_lan_latest_from_db(&state.settings);
                repaint_main_window(hwnd, true);
            }
        }
        MainRowDataActionPlan::AssignGroup {
            ids,
            group_id,
            refilter_after_reload,
        } => {
            if db_assign_group(&ids, group_id).is_ok() {
                reload_state_from_db_persisting(state);
                if refilter_after_reload {
                    state.refilter();
                }
                sync_peer_windows_from_db(hwnd);
                repaint_main_window(hwnd, true);
            }
        }
    }
}

unsafe fn execute_pin_selection_data_plan(hwnd: HWND, state: &mut AppState) {
    let current = state.current_item_owned();
    let selected = state.selected_items_owned();
    if let Some(plan) = main_row_pin_data_plan(current.as_ref(), &selected, state.tab_index) {
        execute_row_data_plan(hwnd, state, plan);
    }
}

pub(super) unsafe fn execute_delete_selection_data_plan(hwnd: HWND, state: &mut AppState) {
    let current = state.current_item_owned();
    let selected = state.selected_items_owned();
    if let Some(plan) = main_row_delete_items_data_plan(current.as_ref(), &selected) {
        execute_row_data_plan(hwnd, state, plan);
    }
}

pub(super) unsafe fn execute_row_command(
    hwnd: HWND,
    state: &mut AppState,
    intent: MainMenuCommandIntent,
) {
    if state.app_data_generation != crate::db_runtime::current_app_data_generation() {
        apply_loaded_settings(hwnd, state);
        return;
    }
    match intent {
        MainMenuCommandIntent::RowPaste => {
            if select_context_row(state) {
                paste_selected(hwnd, state);
                repaint_main_window(hwnd, false);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::Copy) => {
            if select_context_row(state) {
                let copied = copy_selection_to_clipboard(state);
                play_copy_success_sound_if_enabled(state, copied);
                state.clear_selection();
                repaint_main_window(hwnd, false);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::Pin) => {
            if select_context_row(state) {
                execute_pin_selection_data_plan(hwnd, state);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::Delete) => {
            if select_context_row(state) {
                execute_delete_selection_data_plan(hwnd, state);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::DeleteUnpinned) => {
            let plan = main_row_delete_unpinned_data_plan(
                source_tab_category(state.tab_index),
                state.tab_index,
            );
            execute_row_data_plan(hwnd, state, plan);
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::ToPhrase) => {
            if select_context_row(state) {
                execute_row_data_action(hwnd, state, MainRowMenuAction::ToPhrase);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::Sticker) => {
            if select_context_row(state) {
                execute_row_current_item_action(hwnd, state, MainRowMenuAction::Sticker);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::SaveImage) => {
            if select_context_row(state) {
                execute_row_current_item_action(hwnd, state, MainRowMenuAction::SaveImage);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::OpenPath) => {
            if select_context_row(state) {
                execute_row_external_action(hwnd, state, MainRowMenuAction::OpenPath);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::OpenFolder) => {
            if select_context_row(state) {
                execute_row_external_action(hwnd, state, MainRowMenuAction::OpenFolder);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::CopyPath) => {
            if select_context_row(state) {
                execute_row_external_action(hwnd, state, MainRowMenuAction::CopyPath);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::LanPush) => {
            if select_context_row(state) {
                execute_row_external_action(hwnd, state, MainRowMenuAction::LanPush);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::QuickSearch) => {
            if select_context_row(state) {
                execute_row_external_action(hwnd, state, MainRowMenuAction::QuickSearch);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::ExportFile) => {
            if select_context_row(state) {
                if let Some(item) = state.current_item_for_use() {
                    if let Some(path) = materialize_item_as_file(&item) {
                        if let Some(parent) = path.parent().and_then(|p| p.to_str()) {
                            open_path_with_shell(parent);
                        }
                    }
                }
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::ImageOcr) => {
            if select_context_row(state) {
                execute_row_current_item_action(hwnd, state, MainRowMenuAction::ImageOcr);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::TextTranslate) => {
            if select_context_row(state) {
                execute_row_external_action(hwnd, state, MainRowMenuAction::TextTranslate);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::QrImage) => {
            if select_context_row(state) {
                execute_row_external_action(hwnd, state, MainRowMenuAction::QrImage);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::MailMerge) => {
            if !state.settings.super_mail_merge_enabled {
                return;
            }
            if select_context_row(state) {
                execute_row_dialog_action(hwnd, state, MainRowMenuAction::MailMerge);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::Edit) => {
            if select_context_row(state) {
                execute_row_dialog_action(hwnd, state, MainRowMenuAction::Edit);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::RemoveFromGroup) => {
            if select_context_row(state) {
                let ids = state.selected_db_ids();
                if let Some(plan) = main_row_group_assignment_plan(&ids, 0, false) {
                    execute_row_data_plan(hwnd, state, plan);
                }
            }
        }
        MainMenuCommandIntent::GroupFilterAll => {
            let plan = state.list.group_filter_plan(state.tab_index, 0);
            state.list.apply_group_filter_plan(plan);
            let tab_index = state.tab_index;
            if let Some(slot) = state.tab_kind_filters.get_mut(tab_index) {
                *slot = ClipKindFilter::All;
            }
            state.refilter();
            repaint_main_window(hwnd, true);
        }
        MainMenuCommandIntent::AssignRowGroup { index } => {
            if select_context_row(state) {
                if let Some(group_id) = state
                    .groups_for_tab(state.tab_index)
                    .get(index)
                    .map(|g| g.id)
                {
                    let ids = state.selected_db_ids();
                    if let Some(plan) = main_row_group_assignment_plan(&ids, group_id, true) {
                        execute_row_data_plan(hwnd, state, plan);
                    }
                }
            }
        }
        MainMenuCommandIntent::GroupFilter { index } => {
            if let Some(group_id) = state
                .groups_for_tab(state.tab_index)
                .get(index)
                .map(|g| g.id)
            {
                let plan = state.list.group_filter_plan(state.tab_index, group_id);
                state.list.apply_group_filter_plan(plan);
                state.refilter();
                repaint_main_window(hwnd, true);
            }
        }
        MainMenuCommandIntent::GroupKindFilter { index } => {
            let tab_index = state.tab_index;
            if let Some(filter) = clip_kind_filter_options_for_tab(tab_index).get(index) {
                if let Some(slot) = state.tab_kind_filters.get_mut(tab_index) {
                    *slot = *filter;
                }
                state.refilter();
                repaint_main_window(hwnd, true);
            }
        }
        MainMenuCommandIntent::RowAction(MainRowMenuAction::AddToGroup) => {}
        MainMenuCommandIntent::Tray(_) => {}
    }
}
