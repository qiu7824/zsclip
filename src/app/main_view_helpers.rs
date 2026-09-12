use super::prelude::*;

pub(super) fn window_pin_rect(state: &AppState) -> RECT {
    main_layout_for_dpi(state.ui_dpi)
        .with_pin_button(window_pin_visible(state))
        .title_button_rect("window_pin")
        .into()
}

pub(super) fn window_pin_visible(state: &AppState) -> bool {
    state.role == WindowRole::Main && state.settings.show_pin_button
}

pub(super) unsafe fn main_window_z_order(hwnd: HWND) -> HWND {
    let ptr = get_state_ptr(hwnd);
    if !ptr.is_null() {
        if (*ptr).role == WindowRole::Quick {
            return HWND_TOPMOST;
        }
        if !(*ptr).settings.show_pin_button {
            (*ptr).window_pinned = false;
        }
        if (*ptr).window_pinned {
            return HWND_TOPMOST;
        }
    }
    HWND_NOTOPMOST
}

pub(super) unsafe fn toggle_window_pin(hwnd: HWND, state: &mut AppState) {
    let next = !state.window_pinned;
    if platform_window::set_pos(
        hwnd,
        if next { HWND_TOPMOST } else { HWND_NOTOPMOST },
        0,
        0,
        0,
        0,
        SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE,
    ) {
        state.window_pinned = next;
    }
    repaint_main_window(hwnd, false);
}

pub(super) fn main_theme_role_color(role: MainThemeRole, th: Theme) -> u32 {
    match role {
        MainThemeRole::Surface => th.surface,
        MainThemeRole::Surface2 => th.surface2,
        MainThemeRole::Stroke => th.stroke,
        MainThemeRole::SegmentSelected => th.nav_sel_fill,
        MainThemeRole::Background => th.bg,
        MainThemeRole::ControlBg => th.control_bg,
        MainThemeRole::ControlStroke => th.control_stroke,
        MainThemeRole::ButtonHover => th.button_hover,
        MainThemeRole::ButtonPressed => th.button_pressed,
        MainThemeRole::CloseHover => th.close_hover,
        MainThemeRole::ItemSelected => th.item_selected,
        MainThemeRole::ItemHovered => th.item_hover,
        MainThemeRole::Accent => th.accent,
        MainThemeRole::OnAccent => rgb(255, 255, 255),
        MainThemeRole::Text => th.text,
        MainThemeRole::TextMuted => th.text_muted,
    }
}

pub(super) fn pt_in_rect(x: i32, y: i32, rc: &RECT) -> bool {
    x >= rc.left && x < rc.right && y >= rc.top && y < rc.bottom
}

pub(super) fn row_supports_image_preview(item: &ClipItem, settings: &AppSettings) -> bool {
    settings.image_preview_enabled && item.kind == ClipKind::Image
}

pub(super) fn scroll_to_top_visible(state: &AppState) -> bool {
    state.scroll_y > state.layout().row_h
}

pub(super) fn note_scroll_time_hint(state: &mut AppState) {
    state.scroll_date_hint_until = Some(Instant::now() + std::time::Duration::from_millis(900));
}

pub(super) fn scroll_time_hint_visible(state: &AppState) -> bool {
    state.scroll_dragging
        || state
            .scroll_date_hint_until
            .map(|until| until > Instant::now())
            .unwrap_or(false)
}

pub(super) fn scroll_time_hint_label(state: &AppState) -> Option<String> {
    let index = state.layout().row_index_at_offset(state.scroll_y).max(0) as usize;
    let item = state.active_items().get(index)?;
    let label = crate::time_utils::scroll_created_at_label(&item.created_at)?;
    Some(if item.pinned {
        format!("{} · {label}", tr("置顶", "Pinned"))
    } else {
        label
    })
}

pub(super) fn scroll_time_hint_rect(state: &AppState) -> Option<RECT> {
    if !scroll_time_hint_visible(state) {
        return None;
    }
    scroll_time_hint_label(state)?;
    let layout = state.layout();
    let scale = |v: i32| v * layout.win_w / 300;
    let height = scale(26);
    let right = layout.list_x + layout.list_w - scale(18);
    let top_limit = layout.list_y + layout.list_pad + scale(4);
    let bottom_limit = layout.list_y + layout.list_h - layout.list_pad - height - scale(4);
    let top = if state.scroll_dragging {
        layout
            .scrollbar_thumb_rect(state.visible_count(), state.scroll_y)
            .map(|thumb| (thumb.top + thumb.bottom - height) / 2)
            .unwrap_or(top_limit)
    } else {
        top_limit
    }
    .clamp(top_limit, bottom_limit.max(top_limit));
    Some(RECT {
        left: (right - scale(180)).max(layout.list_x + scale(8)),
        top,
        right,
        bottom: top + height,
    })
}

pub(super) fn main_title_button_visibility(settings: &AppSettings) -> TitleButtonVisibility {
    TitleButtonVisibility {
        search: title_button_visible(settings, "search"),
        setting: title_button_visible(settings, "setting"),
        minimize: title_button_visible(settings, "min"),
        close: title_button_visible(settings, "close"),
    }
}

pub(super) fn main_empty_state_kind(state: &AppState) -> MainEmptyStateKind {
    if state.active_load_state().loading {
        MainEmptyStateKind::Loading
    } else if state.active_load_state().error.is_some() {
        MainEmptyStateKind::Error
    } else if state.settings.grouping_enabled && state.current_group_filter != 0 {
        MainEmptyStateKind::Group
    } else if state.tab_index == 0 {
        MainEmptyStateKind::Records
    } else {
        MainEmptyStateKind::Phrases
    }
}

pub(super) unsafe fn hovered_item_clone(state: &AppState) -> Option<ClipItem> {
    if state.hover_idx < 0 {
        return None;
    }
    state.active_items().get(state.hover_idx as usize).cloned()
}
