use super::prelude::*;

fn localize_group_filter_entry(entry: NativePopupMenuEntry) -> NativePopupMenuEntry {
    match entry {
        NativePopupMenuEntry::Command {
            id,
            label,
            enabled,
            checked,
        } => NativePopupMenuEntry::Command {
            id,
            label: if label == "All" {
                translate("全部").into_owned()
            } else {
                label
            },
            enabled,
            checked,
        },
        NativePopupMenuEntry::Submenu {
            label,
            enabled,
            entries,
        } => NativePopupMenuEntry::Submenu {
            label,
            enabled,
            entries: entries
                .into_iter()
                .map(localize_group_filter_entry)
                .collect(),
        },
        NativePopupMenuEntry::Separator => NativePopupMenuEntry::Separator,
    }
}

fn native_row_action_menu_ids_from_specs() -> Vec<usize> {
    native_host_row_action_button_specs()
        .into_iter()
        .map(|spec| spec.action.menu_id())
        .collect()
}

pub(super) unsafe fn show_row_menu(
    hwnd: HWND,
    x: i32,
    y: i32,
    tab_index: usize,
    state: &AppState,
    selected_count: usize,
    has_unpinned: bool,
    current_kind: ClipKind,
    current_is_dir: bool,
    current_is_excel: bool,
    current_can_ocr: bool,
    current_can_translate: bool,
) -> usize {
    let groups = state.groups_for_tab(tab_index);
    let grouping_enabled = state.settings.grouping_enabled;
    let native_row_action_ids = native_row_action_menu_ids_from_specs();
    debug_assert!(native_row_action_ids.contains(&menu_ids::ROW_PASTE));
    debug_assert!(native_row_action_ids.contains(&menu_ids::ROW_EDIT));
    let entries = native_host_full_row_popup_menu_entries_for_groups(
        groups,
        NativeHostRowPopupMenuInput {
            menu: MainRowMenuInput {
                selected_count,
                has_unpinned,
                current_kind,
                grouping_enabled,
                current_can_ocr,
                current_can_translate,
                current_is_excel,
                quick_search_enabled: state.settings.quick_search_enabled,
                qr_quick_enabled: state.settings.qr_quick_enabled,
                super_mail_merge_enabled: state.settings.super_mail_merge_enabled,
                lan_push_available: {
                    #[cfg(feature = "lan-sync")]
                    {
                        state.settings.lan_sync_enabled && !lan_sync::trusted_devices().is_empty()
                    }
                    #[cfg(not(feature = "lan-sync"))]
                    {
                        false
                    }
                },
            },
            labels: MainRowMenuLabelInput {
                selected_count,
                has_unpinned,
                current_is_dir,
            },
            empty_group_label: translate("（暂无分组）").into_owned(),
        },
        |label| translate(label).into_owned(),
    );

    let rc = window_rect_or_empty(hwnd);
    let pt = POINT {
        x: rc.left + x,
        y: rc.top + y,
    };
    let cmd = platform_menu::WindowsPopupMenuHost::new().present_popup_menu(
        hwnd,
        pt.x,
        pt.y,
        NativePopupMenuPlacement::TopLeft,
        &entries,
    );
    vv_set_popup_menu_active(false);
    cmd
}

pub(super) unsafe fn show_group_filter_menu(
    hwnd: HWND,
    x: i32,
    y: i32,
    tab_index: usize,
    state: &AppState,
) -> usize {
    if !state.settings.grouping_enabled {
        return 0;
    }
    let groups = state.groups_for_tab(tab_index);
    let current_group_id = if tab_index < state.tab_group_filters.len() {
        state.tab_group_filters[tab_index]
    } else {
        state.current_group_filter
    };
    let entries = native_host_group_filter_popup_menu_entries_for_groups(groups, current_group_id)
        .into_iter()
        .map(localize_group_filter_entry)
        .collect::<Vec<_>>();
    vv_set_popup_menu_active(true);
    let cmd = platform_menu::WindowsPopupMenuHost::new().present_popup_menu(
        hwnd,
        x,
        y,
        NativePopupMenuPlacement::BottomLeft,
        &entries,
    );
    vv_set_popup_menu_active(false);
    cmd
}
