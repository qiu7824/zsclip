use super::prelude::*;

pub(super) fn main_theme_role_color(role: MainThemeRole, th: Theme) -> u32 {
    match role {
        MainThemeRole::Surface => th.surface,
        MainThemeRole::Surface2 => th.surface2,
        MainThemeRole::Stroke => th.stroke,
        MainThemeRole::SegmentSelected => {
            if th.bg == rgb(255, 255, 255) {
                th.surface2
            } else {
                th.nav_sel_fill
            }
        }
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
