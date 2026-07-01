use super::prelude::*;

const VV_POPUP_CLASS: &str = "ZsClipVvPopup";
const VV_IMM_POINT_MAX_X_DRIFT: i32 = 120;
const VV_IMM_POINT_MAX_Y_DRIFT: i32 = 180;

#[derive(Copy, Clone)]
struct VvOverlayAnchor {
    left: i32,
    edge_y: i32,
    align_bottom: bool,
    exact_rect: bool,
}

fn vv_choose_overlay_edge(
    top: i32,
    bottom: i32,
    popup_height: i32,
    work_area: &RECT,
) -> (i32, bool) {
    let below_space = work_area.bottom - bottom;
    let above_space = top - work_area.top;
    let align_bottom = below_space < popup_height && above_space > below_space;
    let edge_y = if align_bottom { top } else { bottom };
    (edge_y, align_bottom)
}

fn vv_anchor_within(
    anchor: &VvOverlayAnchor,
    reference: &VvOverlayAnchor,
    max_dx: i32,
    max_dy: i32,
) -> bool {
    (anchor.left - reference.left).abs() <= max_dx
        && (anchor.edge_y - reference.edge_y).abs() <= max_dy
}

fn vv_imm_point_anchor_is_plausible(
    anchor: &VvOverlayAnchor,
    caret_anchor: Option<&VvOverlayAnchor>,
    focus_anchor: Option<&VvOverlayAnchor>,
) -> bool {
    if anchor.exact_rect {
        return true;
    }
    if let Some(caret) = caret_anchor {
        if vv_anchor_within(
            anchor,
            caret,
            VV_IMM_POINT_MAX_X_DRIFT,
            VV_IMM_POINT_MAX_Y_DRIFT,
        ) {
            return true;
        }
        if let Some(focus) = focus_anchor {
            return vv_anchor_within(
                anchor,
                focus,
                VV_IMM_POINT_MAX_X_DRIFT + 60,
                VV_IMM_POINT_MAX_Y_DRIFT + 40,
            );
        }
        return false;
    }
    if let Some(focus) = focus_anchor {
        return vv_anchor_within(
            anchor,
            focus,
            VV_IMM_POINT_MAX_X_DRIFT + 60,
            VV_IMM_POINT_MAX_Y_DRIFT + 40,
        );
    }
    true
}

pub(super) fn vv_popup_resolved_group_id(state: &AppState, group_id: i64) -> i64 {
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    if group_id > 0
        && state
            .groups_for_tab(source_tab)
            .iter()
            .any(|g| g.id == group_id)
    {
        group_id
    } else {
        0
    }
}

fn vv_popup_layout() -> MainVvPopupLayout {
    MainVvPopupLayout::default()
}

fn vv_popup_layout_for_window(hwnd: HWND) -> MainVvPopupLayout {
    vv_popup_layout().scaled(unsafe { platform_dpi::layout_dpi_for_window(hwnd) })
}

unsafe fn draw_vv_popup_text_command(hdc: HDC, command: &MainVvPopupTextCommand, th: Theme) {
    let rect: RECT = command.rect.into();
    draw_text_ex(
        hdc as _,
        &command.text,
        &rect,
        main_theme_role_color(command.color, th),
        command.size,
        command.bold,
        main_text_command_centered(command.horizontal_align),
        main_text_command_font(command.font),
    );
}

fn vv_popup_group_name(state: &AppState) -> String {
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    let all_label = if source_tab == 0 {
        "全部记录"
    } else {
        "全部短语"
    };
    group_name_for_display(
        state.groups_for_tab(source_tab),
        state.vv_popup_group_id,
        all_label,
    )
}

fn vv_popup_rebuild_items(state: &mut AppState) {
    let group_id = vv_popup_resolved_group_id(state, state.vv_popup_group_id);
    state.vv_popup_group_id = group_id;
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    state.vv_popup_items =
        db_load_vv_popup_items(source_tab as i64, group_id, MAIN_VV_POPUP_MAX_ITEMS)
            .into_iter()
            .enumerate()
            .map(|(i, item)| VvPopupEntry { index: i + 1, item })
            .collect();
}

unsafe fn vv_popup_show_group_menu(hwnd: HWND, state: &AppState) -> Option<i64> {
    let source_tab = normalize_source_tab(state.settings.vv_source_tab);
    let groups = state.groups_for_tab(source_tab);
    let current_group_id = vv_popup_resolved_group_id(state, state.vv_popup_group_id);
    let plan =
        main_group_filter_menu_plan(current_group_id, groups, false, ClipKindFilter::All, &[]);
    let entries = main_group_filter_popup_entries(
        &plan,
        translate(if source_tab == 0 {
            "全部记录"
        } else {
            "全部短语"
        })
        .into_owned(),
    );
    let layout = platform_window::client_rect(hwnd)
        .map(|rc| vv_popup_layout_for_window(hwnd).with_width(rc.right - rc.left))
        .unwrap_or_else(|| vv_popup_layout_for_window(hwnd));
    let rect: RECT = layout.group_rect().into();
    let mut pt = POINT {
        x: rect.left,
        y: rect.bottom + 4,
    };
    platform_window::client_to_screen(hwnd, &mut pt);
    vv_set_popup_menu_active(true);
    let cmd = platform_menu::WindowsPopupMenuHost::new().present_popup_menu(
        hwnd,
        pt.x,
        pt.y,
        NativePopupMenuPlacement::TopLeft,
        &entries,
    );
    vv_set_popup_menu_active(false);
    match main_group_filter_selection_for_id(cmd) {
        Some(MainGroupFilterSelection::All) => Some(0),
        Some(MainGroupFilterSelection::Group { index }) => groups.get(index).map(|g| g.id),
        Some(MainGroupFilterSelection::Kind { .. }) => None,
        None => None,
    }
}

unsafe fn vv_popup_hwnd(main_hwnd: HWND) -> HWND {
    let raw = *VV_POPUP_HWND.get_or_init(|| {
        let layout = vv_popup_layout_for_window(main_hwnd);
        let mut host = WindowsTransientWindowHost::new(VV_POPUP_CLASS, Some(vv_popup_wnd_proc));
        match host.create_transient_window(NativeTransientWindowRequest {
            owner: main_hwnd,
            bounds: UiRect::new(0, 0, layout.width, layout.header_h + 24),
        }) {
            NativeTransientWindowPresentation::Created(handle) => handle as isize,
            NativeTransientWindowPresentation::Failed => 0,
        }
    });
    raw as HWND
}

pub(super) fn current_vv_popup_hwnd() -> HWND {
    VV_POPUP_HWND.get().copied().unwrap_or(0) as HWND
}

unsafe fn vv_thread_caret_anchor(
    target: HWND,
    popup_height: i32,
    work_area: &RECT,
) -> Option<VvOverlayAnchor> {
    let caret = WindowsTextCaretHost::new().thread_caret_anchor(target)?;
    if !caret.has_vertical_span() {
        return None;
    }
    let (edge_y, align_bottom) =
        vv_choose_overlay_edge(caret.top, caret.bottom, popup_height, work_area);
    Some(VvOverlayAnchor {
        left: caret.left,
        edge_y,
        align_bottom,
        exact_rect: true,
    })
}

unsafe fn vv_accessible_caret_anchor(
    focus_hwnd: HWND,
    popup_height: i32,
    work_area: &RECT,
) -> Option<VvOverlayAnchor> {
    let caret = WindowsTextCaretHost::new().accessible_caret_anchor(focus_hwnd)?;
    if !caret.has_vertical_span() {
        return None;
    }
    let (edge_y, align_bottom) =
        vv_choose_overlay_edge(caret.top, caret.bottom, popup_height, work_area);
    Some(VvOverlayAnchor {
        left: caret.left,
        edge_y,
        align_bottom,
        exact_rect: true,
    })
}

unsafe fn vv_imm_overlay_anchor(
    focus_hwnd: HWND,
    popup_height: i32,
    work_area: &RECT,
) -> Option<VvOverlayAnchor> {
    let mut ime_host = WindowsImeHost::new();
    for index in 0..=3 {
        match ime_host.candidate_anchor(focus_hwnd, index) {
            Some(NativeImeCandidateAnchor::CandidatePoint { position }) => {
                let (edge_y, align_bottom) =
                    vv_choose_overlay_edge(position.y, position.y, popup_height, work_area);
                return Some(VvOverlayAnchor {
                    left: position.x,
                    edge_y,
                    align_bottom,
                    exact_rect: false,
                });
            }
            Some(NativeImeCandidateAnchor::ExcludeRect { rect }) => {
                let (edge_y, align_bottom) =
                    vv_choose_overlay_edge(rect.top, rect.bottom, popup_height, work_area);
                return Some(VvOverlayAnchor {
                    left: rect.left,
                    edge_y,
                    align_bottom,
                    exact_rect: true,
                });
            }
            None => {}
        }
    }

    if let Some(anchor) = ime_host.composition_anchor(focus_hwnd) {
        match anchor {
            NativeImeCompositionAnchor::Point { position } => {
                let (edge_y, align_bottom) =
                    vv_choose_overlay_edge(position.y, position.y, popup_height, work_area);
                return Some(VvOverlayAnchor {
                    left: position.x,
                    edge_y,
                    align_bottom,
                    exact_rect: false,
                });
            }
            NativeImeCompositionAnchor::Rect { rect } => {
                let (edge_y, align_bottom) =
                    vv_choose_overlay_edge(rect.top, rect.bottom, popup_height, work_area);
                return Some(VvOverlayAnchor {
                    left: rect.left,
                    edge_y,
                    align_bottom,
                    exact_rect: true,
                });
            }
        }
    }

    None
}

unsafe fn vv_focus_rect_anchor(
    focus_hwnd: HWND,
    popup_height: i32,
    work_area: &RECT,
) -> Option<VvOverlayAnchor> {
    let max_width = (work_area.right - work_area.left) - 40;
    let anchor = WindowsTextCaretHost::new().focus_rect_anchor(focus_hwnd, max_width, 180)?;
    let (edge_y, align_bottom) =
        vv_choose_overlay_edge(anchor.top, anchor.bottom, popup_height, work_area);
    Some(VvOverlayAnchor {
        left: anchor.left,
        edge_y,
        align_bottom,
        exact_rect: true,
    })
}

unsafe fn vv_cursor_anchor(popup_height: i32, work_area: &RECT) -> Option<VvOverlayAnchor> {
    let anchor = WindowsTextCaretHost::new().cursor_anchor()?;
    let (edge_y, align_bottom) =
        vv_choose_overlay_edge(anchor.top, anchor.bottom, popup_height, work_area);
    Some(VvOverlayAnchor {
        left: anchor.left,
        edge_y,
        align_bottom,
        exact_rect: false,
    })
}

unsafe fn vv_focus_hwnd_for_target(target: HWND) -> HWND {
    WindowsTextCaretHost::new().focus_handle_for_target(target)
}

fn present_vv_popup_window(popup: HWND, bounds: UiRect) {
    let mut host = WindowsTransientWindowHost::new(VV_POPUP_CLASS, Some(vv_popup_wnd_proc));
    host.present_transient_window(popup, bounds);
}

fn hide_vv_popup_window(popup: HWND) {
    let mut host = WindowsTransientWindowHost::new(VV_POPUP_CLASS, Some(vv_popup_wnd_proc));
    host.hide_transient_window(popup);
}

pub(super) fn destroy_vv_popup_window(popup: HWND) {
    let mut host = WindowsTransientWindowHost::new(VV_POPUP_CLASS, Some(vv_popup_wnd_proc));
    host.destroy_transient_window(popup);
}

unsafe fn vv_popup_move_near_target(state: &AppState, popup: HWND) -> bool {
    if !platform_window::exists(popup) {
        return false;
    }
    let focus_hwnd = vv_focus_hwnd_for_target(state.vv_popup_target);
    if focus_hwnd.is_null() {
        return false;
    }
    let layout = vv_popup_layout_for_window(focus_hwnd);
    let mut wa = platform_monitor::nearest_work_rect_for_window(focus_hwnd);
    let height = layout.height(state.vv_popup_items.len());
    let caret_anchor = vv_accessible_caret_anchor(focus_hwnd, height, &wa)
        .or_else(|| vv_thread_caret_anchor(focus_hwnd, height, &wa));
    let focus_anchor = vv_focus_rect_anchor(focus_hwnd, height, &wa);
    let imm_anchor = vv_imm_overlay_anchor(focus_hwnd, height, &wa).filter(|anchor| {
        vv_imm_point_anchor_is_plausible(anchor, caret_anchor.as_ref(), focus_anchor.as_ref())
    });
    let anchor = imm_anchor
        .or(caret_anchor)
        .or(focus_anchor)
        .or_else(|| vv_cursor_anchor(height, &wa));
    let Some(anchor) = anchor else {
        return false;
    };
    wa = platform_monitor::nearest_work_rect_for_point(POINT {
        x: anchor.left,
        y: anchor.edge_y,
    });
    let mut x = anchor.left;
    let mut y = if anchor.align_bottom {
        anchor.edge_y - height
    } else {
        anchor.edge_y
    };
    if x + layout.width > wa.right {
        x = wa.right - layout.width;
    }
    if x < wa.left {
        x = wa.left;
    }
    if y < wa.top {
        y = wa.top;
    }
    if y + height > wa.bottom {
        y = wa.bottom - height;
    }
    present_vv_popup_window(popup, UiRect::new(x, y, x + layout.width, y + height));
    true
}

pub(super) unsafe fn vv_popup_sync_hook_state(visible: bool, target: HWND) {
    if let Ok(mut guard) = vv_hook_state().lock() {
        guard.popup_active = visible;
        guard.popup_target = if visible { target as isize } else { 0 };
        if !visible {
            guard.popup_menu_active = false;
            guard.popup_menu_grace_until = None;
            guard.last_was_v = false;
            guard.last_v_target = 0;
            guard.last_v_at = None;
        }
    }
}

pub(super) unsafe fn vv_popup_hide(_hwnd: HWND, state: &mut AppState) {
    state.vv_popup_visible = false;
    state.vv_popup_pending_target = null_mut();
    state.vv_popup_pending_retries = 0;
    state.vv_popup_target = null_mut();
    state.vv_popup_replaces_ime = false;
    state.vv_popup_group_id = 0;
    state.vv_popup_items.clear();
    vv_popup_sync_hook_state(false, null_mut());
    let popup = current_vv_popup_hwnd();
    if platform_window::exists(popup) {
        hide_vv_popup_window(popup);
    }
}

pub(super) unsafe fn vv_popup_show(hwnd: HWND, state: &mut AppState, target: HWND) -> bool {
    state.vv_popup_group_id = vv_popup_resolved_group_id(state, state.settings.vv_group_id);
    vv_popup_rebuild_items(state);
    state.vv_popup_target = target;
    state.vv_popup_pending_retries = 0;
    state.vv_popup_visible = true;
    state.vv_popup_replaces_ime = false;
    vv_popup_sync_hook_state(true, target);
    let popup = vv_popup_hwnd(hwnd);
    if !vv_popup_move_near_target(state, popup) {
        vv_popup_hide(hwnd, state);
        return false;
    }
    let focus_hwnd = vv_focus_hwnd_for_target(target);
    let ime_replaced_trigger = if focus_hwnd.is_null() {
        false
    } else {
        let work_area = platform_monitor::nearest_work_rect_for_window(focus_hwnd);
        let layout = vv_popup_layout_for_window(focus_hwnd);
        vv_imm_overlay_anchor(
            focus_hwnd,
            layout.height(state.vv_popup_items.len()),
            &work_area,
        )
        .is_some()
    };
    send_escape_key();
    state.vv_popup_replaces_ime = ime_replaced_trigger;
    platform_gdi::invalidate_rect(popup, null(), 1);
    let _ = vv_popup_move_near_target(state, popup);
    true
}

unsafe extern "system" fn vv_popup_wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_NCCREATE => {
            let cs = &*(lparam as *const CREATESTRUCTW);
            platform_window::set_user_data(hwnd, cs.lpCreateParams as isize);
            platform_appearance::set_rounded_corners(hwnd);
            1
        }
        WM_PAINT => {
            let main_hwnd = platform_window::user_data(hwnd) as HWND;
            let ptr = get_state_ptr(main_hwnd);
            let mut ps: PAINTSTRUCT = zeroed();
            let hdc = platform_gdi::begin_paint(hwnd, &mut ps);
            if !hdc.is_null() && !ptr.is_null() {
                let state = &*ptr;
                let th = Theme::default();
                let rc = platform_window::client_rect(hwnd).unwrap_or_else(|| zeroed());
                let layout =
                    vv_popup_layout_for_window(hwnd).with_width((rc.right - rc.left).max(1));
                let bg = platform_gdi::create_solid_brush(th.surface);
                platform_gdi::fill_rect(hdc, &rc, bg);
                platform_gdi::delete_object(bg as _);
                let strings = MainVvPopupRenderStrings {
                    title: tr("VV 模式", "VV Mode").to_string(),
                    hint: tr(
                        "输入 1-9 直接粘贴，Esc 取消",
                        "Press 1-9 to paste, Esc to cancel",
                    )
                    .to_string(),
                    empty: tr("当前分组暂无记录", "No records in this group").to_string(),
                };
                let render_items = state
                    .vv_popup_items
                    .iter()
                    .map(|entry| {
                        let label = if entry.item.kind == ClipKind::Image {
                            format_created_at_local(&entry.item.created_at, &entry.item.preview)
                        } else {
                            entry.item.preview.clone()
                        };
                        MainVvPopupRenderItem {
                            index: entry.index,
                            label,
                        }
                    })
                    .collect::<Vec<_>>();
                let plan = layout.render_plan(
                    rc.into(),
                    &strings,
                    &vv_popup_group_name(state),
                    &render_items,
                );
                #[cfg(feature = "vv-paste")]
                {
                    let vv_select_specs = crate::app_core::native_host_vv_select_specs(
                        &plan,
                        rc.right - rc.left,
                        rc.bottom - rc.top,
                    );
                    debug_assert_eq!(vv_select_specs.len(), render_items.len());
                    for (index, spec) in vv_select_specs.iter().enumerate() {
                        debug_assert_eq!(spec.action.index, index);
                    }
                }
                for command in plan.paint_commands {
                    draw_main_paint_command(hdc, command, th);
                }
                for command in &plan.text_commands {
                    draw_vv_popup_text_command(hdc, command, th);
                }
            }
            platform_gdi::end_paint(hwnd, &ps);
            0
        }
        WM_LBUTTONUP => {
            let main_hwnd = platform_window::user_data(hwnd) as HWND;
            let ptr = get_state_ptr(main_hwnd);
            if ptr.is_null() {
                return 0;
            }
            let state = &mut *ptr;
            let x = get_x_lparam(lparam);
            let y = get_y_lparam(lparam);
            let layout = platform_window::client_rect(hwnd)
                .map(|rc| vv_popup_layout_for_window(hwnd).with_width(rc.right - rc.left))
                .unwrap_or_else(|| vv_popup_layout_for_window(hwnd));
            match layout.hit_test(x, y, state.vv_popup_items.len()) {
                MainVvPopupHit::Group => {
                    if let Some(group_id) = vv_popup_show_group_menu(hwnd, state) {
                        state.vv_popup_group_id = vv_popup_resolved_group_id(state, group_id);
                        vv_popup_rebuild_items(state);
                        let _ = WindowsPasteTargetHost::new()
                            .force_paste_target_foreground(state.vv_popup_target);
                        vv_popup_sync_hook_state(true, state.vv_popup_target);
                        vv_popup_move_near_target(state, hwnd);
                        platform_gdi::invalidate_rect(hwnd, null(), 1);
                        let _ = vv_popup_move_near_target(state, hwnd);
                    }
                }
                MainVvPopupHit::Row(row) => {
                    platform_window::post_hwnd_message(main_hwnd, WM_VV_SELECT, row, 0);
                }
                MainVvPopupHit::None => {}
            }
            0
        }
        WM_SIZE => {
            platform_gdi::invalidate_rect(hwnd, null(), 1);
            0
        }
        _ => platform_window::default_window_proc(hwnd, msg, wparam, lparam),
    }
}
