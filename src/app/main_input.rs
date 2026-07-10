use super::prelude::*;

fn windows_main_tool_action_available(action: NativeHostMainToolAction) -> bool {
    native_host_main_tool_button_specs()
        .into_iter()
        .any(|spec| spec.action == action)
}

pub(super) unsafe fn handle_mouse_wheel(hwnd: HWND, delta: i32) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let update =
        state
            .layout()
            .scroll_update_for_wheel(state.scroll_y, state.visible_count(), delta);
    state.list.apply_scroll_update(update);
    state.maybe_request_more_for_active_tab();
    show_main_scrollbar_feedback(hwnd, state, true);
}

pub(super) unsafe fn handle_mouse_move(hwnd: HWND, position: UiPoint) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let _ = track_main_pointer_leave(hwnd);
    let state = &mut *ptr;
    let x = position.x;
    let y = position.y;
    let layout = state.layout();
    let title_buttons = main_title_button_visibility(&state.settings);
    let current_hover = main_hover_target_from_state(state);

    if state.scroll_dragging {
        let transition = layout.pointer_move_transition(
            x,
            y,
            state.visible_count(),
            state.scroll_y,
            title_buttons,
            scroll_to_top_visible(state),
            current_hover,
            true,
            state.scroll_drag_start_y,
            state.scroll_drag_start_scroll,
        );
        if let Some(target_scroll) = transition.drag_scroll_y {
            let update = state.list.scroll_position_update_plan(target_scroll);
            state.list.apply_scroll_update(update);
            state.maybe_request_more_for_active_tab();
            show_main_scrollbar_feedback(hwnd, state, false);
            return;
        }
    }

    if state.down_row >= 0 && platform_input::primary_mouse_button_down() {
        let dx = (x - state.down_x).abs();
        let dy = (y - state.down_y).abs();
        let drag_cx = platform_window::system_metric(SM_CXDRAG).max(4);
        let drag_cy = platform_window::system_metric(SM_CYDRAG).max(4);
        if dx >= drag_cx || dy >= drag_cy {
            let drag_row = state.down_row;
            state.down_row = -1;
            if begin_row_drag_export(hwnd, state, drag_row) {
                repaint_main_window(hwnd, false);
                return;
            }
        }
    }

    let transition = layout.pointer_move_transition(
        x,
        y,
        state.visible_count(),
        state.scroll_y,
        title_buttons,
        scroll_to_top_visible(state),
        current_hover,
        false,
        state.scroll_drag_start_y,
        state.scroll_drag_start_scroll,
    );
    if let Some(hover) = transition.hover {
        apply_main_hover_target(state, hover.next);
        // 悬停时立即显示滚动条（满透明）
        if hover.show_scrollbar_feedback {
            show_main_scrollbar_feedback(hwnd, state, false);
        }

        if hover.row_changed {
            hide_hover_preview();
        }

        if hover.target_changed {
            repaint_main_window(hwnd, false);
        }
    }
}

pub(super) unsafe fn handle_lbutton_down(hwnd: HWND, position: UiPoint) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let x = position.x;
    let y = position.y;
    let layout = state.layout();
    let target = layout.pointer_down_target(
        x,
        y,
        state.visible_count(),
        state.scroll_y,
        TitleButtonVisibility {
            search: title_button_visible(&state.settings, "search"),
            setting: title_button_visible(&state.settings, "setting"),
            minimize: title_button_visible(&state.settings, "min"),
            close: title_button_visible(&state.settings, "close"),
        },
        state.search_on,
        scroll_to_top_visible(state),
    );
    let down_state_plan = layout.pointer_down_state_plan(target, x, y, state.scroll_y);

    match target {
        MainPointerDownTarget::TitleDrag => {
            if state.role == WindowRole::Quick || state.main_window_noactivate {
                WindowsMainWindowHost::new(Some(wnd_proc))
                    .set_main_window_activation_policy(hwnd, true);
            }
            begin_main_window_drag(hwnd);
            return;
        }
        MainPointerDownTarget::TitleButton(_) => {
            apply_main_pointer_down_state_plan(state, down_state_plan);
            repaint_main_window(hwnd, false);
            return;
        }
        MainPointerDownTarget::ScrollToTop => {
            apply_main_pointer_down_state_plan(state, down_state_plan);
            repaint_main_window(hwnd, false);
            return;
        }
        MainPointerDownTarget::ScrollbarThumb => {
            apply_main_pointer_down_state_plan(state, down_state_plan);
            show_main_scrollbar_feedback(hwnd, state, false);
            capture_main_pointer(hwnd);
            return;
        }
        MainPointerDownTarget::ScrollbarTrack => {
            apply_main_pointer_down_state_plan(state, down_state_plan);
            if let Some(update) = state.layout().scroll_update_for_track_click(
                state.scroll_y,
                state.visible_count(),
                y,
            ) {
                state.list.apply_scroll_update(update);
                state.maybe_request_more_for_active_tab();
                show_main_scrollbar_feedback(hwnd, state, false);
            }
            return;
        }
        MainPointerDownTarget::Tab(tab) => {
            apply_main_pointer_down_state_plan(state, down_state_plan);
            let plan = state.list.tab_switch_plan(tab);
            state.list.apply_tab_switch_plan(plan);
            state.refilter();
            remember_shared_tab_view_state(state);
            repaint_main_window(hwnd, true);
            return;
        }
        MainPointerDownTarget::Row(idx) => {
            let clear_plan =
                layout.pointer_down_state_plan(MainPointerDownTarget::None, x, y, state.scroll_y);
            apply_main_pointer_down_state_plan(state, clear_plan);
            let Some(focus_plan) = state.list.row_pointer_down_focus_plan(idx) else {
                return;
            };
            state.list.apply_row_pointer_down_focus_plan(focus_plan);
            state.ensure_visible(idx);
            let (ctrl, shift) = hotkey::ctrl_shift_from_pressed_state();
            if !ctrl
                && !shift
                && state.hotkey_passthrough_active
                && !state.hotkey_passthrough_edit.is_null()
            {
                let mut handled = false;
                if let Some(src_idx) = state.visible_src_idx(idx as usize) {
                    if let Some(item) = state.active_items().get(src_idx).cloned() {
                        if let Some(del_rc) = row_quick_delete_rect(state, idx, &item) {
                            if !pt_in_rect(x, y, &del_rc) {
                                handled = try_apply_to_explorer_rename(state, &item);
                            }
                        } else {
                            handled = try_apply_to_explorer_rename(state, &item);
                        }
                    }
                }
                if handled {
                    let plan = main_paste_completion_plan(
                        MainPasteCompletionKind::DirectEdit,
                        MainPasteCompletionInput {
                            item_id: 0,
                            move_pasted_item_to_top: false,
                            click_hide: state.settings.click_hide,
                            paste_success_sound_enabled: false,
                        },
                    );
                    execute_paste_completion_plan(hwnd, state, plan);
                    return;
                }
            }
            apply_main_pointer_down_state_plan(state, down_state_plan);
            repaint_main_window(hwnd, false);
        }
        MainPointerDownTarget::None => {
            apply_main_pointer_down_state_plan(state, down_state_plan);
        }
    }
}

pub(super) unsafe fn handle_lbutton_up(hwnd: HWND, position: UiPoint) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let x = position.x;
    let y = position.y;
    let transition = state.layout().pointer_up_transition(
        x,
        y,
        state.visible_count(),
        state.scroll_y,
        state.down_btn,
        state.down_to_top,
        state.down_row,
    );
    state.down_btn = "";

    if state.scroll_dragging {
        cancel_main_scroll_drag(hwnd, state);
        return;
    }

    match transition.target {
        MainPointerUpTarget::TitleButton { key, activated } => {
            if !activated {
                repaint_main_window(hwnd, false);
                return;
            }

            let window_command = main_title_button_window_command_for_key(key);
            queue_main_window_command_intent(hwnd, state, window_command);
            state.hover_btn = "";
            repaint_main_window(hwnd, false);
            if matches!(window_command, MainWindowCommandIntent::OpenSettings) {
                hide_main_window(hwnd);
            }
            return;
        }
        MainPointerUpTarget::ScrollToTop { activated } => {
            let plan = state.list.scroll_to_top_release_plan(activated);
            state.list.apply_scroll_to_top_release_plan(plan);
            state.down_to_top = plan.down_scroll_to_top;
            if plan.show_scrollbar_feedback {
                show_main_scrollbar_feedback(hwnd, state, false);
            }
            repaint_main_window(hwnd, false);
            return;
        }
        MainPointerUpTarget::Row(release) => {
            let release_state = state.list.row_release_state_plan(release);
            state.list.apply_row_release_state_plan(release_state);
            state.down_row = release_state.down_row;
            state.down_x = release_state.down_x;
            state.down_y = release_state.down_y;
            if !release.accepted {
                repaint_main_window(hwnd, false);
                return;
            }
            let idx = release.release_row;
            let row_has_item = state
                .visible_src_idx(idx as usize)
                .and_then(|src_idx| state.active_items().get(src_idx))
                .is_some();
            let modifiers = hotkey::pointer_modifiers_from_pressed_state();
            let action = state.layout().row_release_action(
                release,
                x,
                y,
                state.visible_count(),
                state.scroll_y,
                state.hover_idx,
                row_has_item && state.settings.quick_delete_button,
                modifiers,
            );
            match action {
                MainRowReleaseAction::QuickDelete { .. } => {
                    execute_delete_selection_data_plan(hwnd, state);
                }
                MainRowReleaseAction::Select { row, modifiers } => {
                    state.list.apply_primary_pointer_selection(
                        row,
                        modifiers.ctrl,
                        modifiers.shift,
                    );
                    repaint_main_window(hwnd, false);
                }
                MainRowReleaseAction::Paste { .. } => {
                    // 单击逻辑统一走粘贴入口，资源管理器重命名会在这里走直接写 Edit 的专用路径。
                    paste_selected(hwnd, state);
                    repaint_main_window(hwnd, false);
                }
                MainRowReleaseAction::None => {
                    repaint_main_window(hwnd, false);
                }
            }
        }
        MainPointerUpTarget::None => {
            let plan = state.list.pointer_up_press_clear_plan();
            state.down_row = plan.down_row;
            state.down_x = plan.down_x;
            state.down_y = plan.down_y;
        }
    }
}

pub(super) unsafe fn handle_lbutton_dblclk(hwnd: HWND, position: UiPoint) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let x = position.x;
    let y = position.y;
    let idx = hit_test_row(state, x, y);
    if let Some(plan) = state.list.row_double_click_state_plan(idx) {
        state.list.apply_row_double_click_focus_plan(plan);
        paste_selected(hwnd, state);
        state.list.apply_row_double_click_finish_plan(plan);
        repaint_main_window(hwnd, false);
    }
}

pub(super) unsafe fn handle_rbutton_up(hwnd: HWND, position: UiPoint) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    hide_hover_preview();
    let x = position.x;
    let y = position.y;
    let (tab0, tab1) = state.segment_rects();
    if state.settings.grouping_enabled
        && windows_main_tool_action_available(NativeHostMainToolAction::GroupFilter)
        && (pt_in_rect(x, y, &tab0) || pt_in_rect(x, y, &tab1))
    {
        let target_tab = if pt_in_rect(x, y, &tab1) {
            1usize
        } else {
            0usize
        };
        let pt = platform_input::cursor_pos().unwrap_or(POINT { x: 0, y: 0 });
        let cmd = show_group_filter_menu(hwnd, pt.x, pt.y, target_tab, state);
        match main_group_filter_selection_for_id(cmd) {
            Some(MainGroupFilterSelection::All) => {
                let plan = state.list.group_filter_plan(target_tab, 0);
                state.list.apply_group_filter_plan(plan);
                if let Some(slot) = state.tab_kind_filters.get_mut(target_tab) {
                    *slot = ClipKindFilter::All;
                }
                state.refilter();
                remember_shared_tab_view_state(state);
            }
            Some(MainGroupFilterSelection::Group { index }) => {
                if let Some(group_id) = state.groups_for_tab(target_tab).get(index).map(|g| g.id) {
                    let plan = state.list.group_filter_plan(target_tab, group_id);
                    state.list.apply_group_filter_plan(plan);
                    state.refilter();
                    remember_shared_tab_view_state(state);
                }
            }
            Some(MainGroupFilterSelection::Kind { index }) => {
                if let Some(filter) = clip_kind_filter_options_for_tab(target_tab).get(index) {
                    if let Some(slot) = state.tab_kind_filters.get_mut(target_tab) {
                        *slot = *filter;
                    }
                    state.refilter();
                    remember_shared_tab_view_state(state);
                }
            }
            None => {}
        }
        repaint_main_window(hwnd, true);
        return;
    }

    let idx = hit_test_row(state, x, y);
    if idx < 0 {
        return;
    }
    if !windows_main_tool_action_available(NativeHostMainToolAction::RowMenu) {
        return;
    }

    let (ctrl, shift) = hotkey::ctrl_shift_from_pressed_state();

    let Some(plan) = state.list.context_menu_state_plan(idx, ctrl, shift) else {
        return;
    };
    let menu_row = plan.row;
    let menu_selection_count = plan.context_selection_count;
    state.list.apply_context_menu_state_plan(plan);

    state.ensure_visible(menu_row);
    let current_item = state.current_item_for_use();
    let current_kind = current_item
        .as_ref()
        .map(|it| it.kind)
        .unwrap_or(ClipKind::Text);
    let current_is_dir = current_item
        .as_ref()
        .map(is_directory_item)
        .unwrap_or(false);
    let current_is_excel = current_item
        .as_ref()
        .and_then(|it| it.file_paths.as_ref())
        .and_then(|paths| paths.first())
        .map(|path| {
            let lower = path.to_ascii_lowercase();
            lower.ends_with(".xls")
                || lower.ends_with(".xlsx")
                || lower.ends_with(".xlsm")
                || lower.ends_with(".csv")
        })
        .unwrap_or(false);
    let current_can_ocr = state.settings.image_ocr_provider != "off"
        && current_item
            .as_ref()
            .and_then(image_input_for_ocr)
            .is_some();
    let current_can_translate = state.settings.text_translate_provider != "off"
        && current_item
            .as_ref()
            .and_then(|item| {
                main_row_external_action_plan(MainRowMenuAction::TextTranslate, Some(item), &[])
            })
            .is_some();
    let cmd = show_row_menu(
        hwnd,
        x,
        y,
        state.tab_index,
        state,
        menu_selection_count,
        state.context_selection_has_unpinned(),
        current_kind,
        current_is_dir,
        current_is_excel,
        current_can_ocr,
        current_can_translate,
    );
    if cmd != 0 {
        if let Some(command) = main_menu_command_for_id(cmd) {
            state.ui_commands.push(command);
            drain_main_ui_commands(hwnd);
        }
    }
    repaint_main_window(hwnd, false);
}

fn move_main_selection(state: &mut AppState, delta: i32, extend: bool) {
    if let Some(plan) = state.list.keyboard_move_selection_plan(delta, extend) {
        state.list.apply_selection_plan(plan);
        state.ensure_visible(state.sel_idx);
    }
}

unsafe fn queue_main_shortcut_row_command(
    hwnd: HWND,
    state: &mut AppState,
    row_command: MainShortcutRowCommand,
) {
    let plan = state.list.shortcut_row_command_plan(row_command);
    state.list.apply_shortcut_row_command_plan(plan);
    state
        .ui_commands
        .push(main_menu_command_for_shortcut_row_command(plan.command));
    drain_main_ui_commands(hwnd);
}

unsafe fn queue_main_window_command_intent(
    hwnd: HWND,
    state: &mut AppState,
    window_command: MainWindowCommandIntent,
) {
    state
        .ui_commands
        .push(main_window_command_for_intent(window_command));
    drain_main_ui_commands(hwnd);
}

unsafe fn execute_main_shortcut_action(
    hwnd: HWND,
    state: &mut AppState,
    action: MainShortcutAction,
) {
    let escape_plan =
        matches!(action, MainShortcutAction::Escape).then(|| state.list.escape_shortcut_plan());
    match main_shortcut_execution_plan(action, escape_plan) {
        MainShortcutExecutionPlan::MoveSelection { delta, extend } => {
            move_main_selection(state, delta, extend);
            repaint_main_window(hwnd, false);
        }
        MainShortcutExecutionPlan::ActivateSelection => {
            match state.list.activate_selection_plan() {
                MainActivateSelectionPlan::CopySelectionThenPaste => {
                    copy_selection_to_clipboard(state);
                    state.clear_selection();
                    paste_after_clipboard_ready(hwnd, state, state.settings.click_hide);
                }
                MainActivateSelectionPlan::PasteSelection => {
                    paste_selected(hwnd, state);
                }
            }
            repaint_main_window(hwnd, false);
        }
        MainShortcutExecutionPlan::SelectAll => {
            let plan = state.list.select_all_selection_plan();
            state.list.apply_selection_plan(plan);
            repaint_main_window(hwnd, false);
        }
        MainShortcutExecutionPlan::RowCommand(row_command) => {
            queue_main_shortcut_row_command(hwnd, state, row_command);
        }
        MainShortcutExecutionPlan::ClearSelection => {
            state.clear_selection();
            repaint_main_window(hwnd, false);
        }
        MainShortcutExecutionPlan::CloseSearch => {
            close_search_ui(hwnd, state);
        }
        MainShortcutExecutionPlan::WindowCommand(window_command) => {
            queue_main_window_command_intent(hwnd, state, window_command);
        }
        MainShortcutExecutionPlan::Noop => {}
    }
}

pub(super) unsafe fn handle_keydown(hwnd: HWND, vk: u32) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let modifiers = hotkey::shortcut_modifiers_from_pressed_state();
    if let Some(action) = main_shortcut_action(hotkey::shortcut_key_from_vk(vk), modifiers) {
        execute_main_shortcut_action(hwnd, state, action);
    }
}

pub(super) unsafe fn handle_nchittest(hwnd: HWND, lparam: LPARAM) -> LRESULT {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return platform_window::default_window_proc(hwnd, WM_NCHITTEST, 0, lparam);
    }
    let state = &mut *ptr;

    let mut pt = POINT {
        x: get_x_lparam(lparam),
        y: get_y_lparam(lparam),
    };
    platform_window::screen_to_client(hwnd, &mut pt);

    let hit = state.layout().frame_hit_target(
        pt.x,
        pt.y,
        TitleButtonVisibility {
            search: title_button_visible(&state.settings, "search"),
            setting: title_button_visible(&state.settings, "setting"),
            minimize: title_button_visible(&state.settings, "min"),
            close: title_button_visible(&state.settings, "close"),
        },
        state.search_on,
        state.role != WindowRole::Quick && !state.main_window_noactivate,
    );
    if hit == MainFrameHitTarget::Caption {
        return HTCAPTION as LRESULT;
    }

    HTCLIENT as LRESULT
}

pub(super) fn hit_test_row(state: &AppState, x: i32, y: i32) -> i32 {
    state
        .layout()
        .hit_test_row(x, y, state.visible_count(), state.scroll_y)
}

pub(super) fn row_shows_delete_button(state: &AppState, visible_idx: i32) -> bool {
    state.settings.quick_delete_button && state.hover_idx == visible_idx
}

pub(super) fn row_quick_delete_rect(
    state: &AppState,
    visible_idx: i32,
    _item: &ClipItem,
) -> Option<RECT> {
    if !row_shows_delete_button(state, visible_idx) {
        return None;
    }
    state.quick_action_rect_slot(visible_idx, 0)
}

pub(super) fn apply_main_pointer_down_state_plan(
    state: &mut AppState,
    plan: MainPointerDownStatePlan,
) {
    state.down_btn = plan.down_title_button;
    state.down_to_top = plan.down_scroll_to_top;
    state.down_row = plan.down_row;
    state.down_x = plan.down_x;
    state.down_y = plan.down_y;
    if let Some(drag) = plan.scroll_drag_start {
        state.scroll_dragging = true;
        state.scroll_drag_start_y = drag.pointer_y;
        state.scroll_drag_start_scroll = drag.scroll_y;
    }
}

pub(super) fn main_hover_target_from_state(state: &AppState) -> MainHoverTarget {
    MainHoverTarget {
        title_button: state.hover_btn,
        tab: state.hover_tab,
        scrollbar: state.hover_scroll,
        scroll_to_top: state.hover_to_top,
        row: state.hover_idx,
    }
}

pub(super) fn apply_main_hover_target(state: &mut AppState, target: MainHoverTarget) {
    state.hover_btn = target.title_button;
    state.hover_tab = target.tab;
    state.hover_scroll = target.scrollbar;
    state.hover_to_top = target.scroll_to_top;
    state.hover_idx = target.row;
}
