use super::prelude::*;
use crate::app_core::customization::{contains, row_action_order, CategoryBar, QuickRowAction};

const TAG_ADD_BASE: usize = 44000;
const TAG_REMOVE_BASE: usize = 44100;

pub(super) fn tag_row_menu_entries(state: &AppState) -> Vec<NativePopupMenuEntry> {
    let ids = state.selected_db_ids();
    let counts = crate::clip_tags::counts(&ids);
    let mut add = Vec::new();
    let mut remove = Vec::new();
    for (index, tab) in state.settings.category_tabs.iter().enumerate().filter(|(_,t)| t.is_manual()) {
        let count = counts.as_ref().ok().and_then(|c| c.get(&tab.id)).copied().unwrap_or(0);
        let label = format!("{}{}", tab.name, if tab.visible { "" } else { "（隐藏）" });
        add.push(NativePopupMenuEntry::Command { id: TAG_ADD_BASE + index, label: label.clone(),
            enabled: !ids.is_empty() && counts.is_ok(), checked: count > 0 && count == ids.len() });
        remove.push(NativePopupMenuEntry::Command { id: TAG_REMOVE_BASE + index, label,
            enabled: count > 0 && counts.is_ok(), checked: false });
    }
    if add.is_empty() {
        add.push(NativePopupMenuEntry::Command { id: 0,
            label: tr("请先在设置中新增手动标签", "Create a manual tag in Settings first").into(),
            enabled: false, checked: false });
    }
    let mut entries = vec![NativePopupMenuEntry::Separator,
        NativePopupMenuEntry::Submenu { label: tr("添加到标签", "Add to Tag").into(), enabled: true, entries: add }];
    if !remove.is_empty() {
        entries.push(NativePopupMenuEntry::Submenu { label: tr("从标签移除", "Remove from Tag").into(), enabled: true, entries: remove });
    }
    entries
}

pub(super) fn change_tag_membership(state: &AppState, id: &str, ids: &[i64], remove: bool) -> Result<(), String> {
    if !state.settings.category_tabs.iter().any(|t| t.id == id && t.is_manual()) {
        return Err(tr("标签已更改，请重新打开菜单。", "The tag changed. Reopen the menu.").into());
    }
    crate::db_runtime::with_shared_app_data_generation(state.app_data_generation, || {
        if remove { crate::clip_tags::remove(id, ids) } else { crate::clip_tags::assign(id, ids) }
    }).ok_or_else(|| tr("数据已更新，请重新打开菜单。", "Data changed. Reopen the menu.").to_string())?
        .map_err(|_| tr("记录或标签无法更新，请重新打开菜单后重试。", "Could not update the tag. Reopen the menu and retry.").into())
}

pub(super) unsafe fn handle_tag_row_command(hwnd: HWND, state: &mut AppState, command: usize,
    tabs: &[crate::app_core::customization::CategoryTab], ids: &[i64], generation: u64) -> bool {
    let (base, remove) = if (TAG_ADD_BASE..TAG_ADD_BASE+32).contains(&command) { (TAG_ADD_BASE,false) }
        else if (TAG_REMOVE_BASE..TAG_REMOVE_BASE+32).contains(&command) { (TAG_REMOVE_BASE,true) }
        else { return false; };
    let result = if generation != state.app_data_generation {
        Err(tr("数据已更新，请重新打开菜单。", "Data changed. Reopen the menu.").into())
    } else if let Some(tab) = tabs.get(command-base) {
        change_tag_membership(state, &tab.id, ids, remove)
    } else { Err("标签不存在，请重新打开菜单。".into()) };
    match result {
        Ok(()) => { reload_state_from_db_persisting(state); sync_peer_windows_from_db(hwnd); }
        Err(message) => { platform_dialog::WindowsDialogHost::new().show_message(hwnd,
            tr("分类标签", "Tags"), &message, NativeDialogLevel::Error); }
    }
    repaint_main_window(hwnd, false);
    true
}

impl AppState {
    pub(super) fn active_category_index(&self) -> usize {
        self.settings
            .category_tabs
            .iter()
            .position(|t| {
                t.visible && t.id == self.active_category_id
            })
            .or_else(|| {
                self.settings
                    .category_tabs
                    .iter()
                    .position(|t| t.visible && t.source_tab() == self.tab_index)
            })
            .or_else(|| self.settings.category_tabs.iter().position(|t| t.visible))
            .unwrap_or(0)
    }
    pub(super) fn category_bar(&self) -> CategoryBar {
        let l = self.layout();
        CategoryBar::new(
            &self.settings.category_tabs,
            self.active_category_index(),
            UiRect::new(l.seg_x, l.seg_y, l.seg_x + l.seg_w, l.seg_y + l.seg_h),
            self.ui_dpi,
        )
    }
    pub(super) fn activate_category(&mut self, index: usize) {
        let Some(tab) = self
            .settings
            .category_tabs
            .get(index)
            .filter(|t| t.visible)
            .cloned()
        else {
            return;
        };
        self.active_category_id = tab.id.clone();
        let source = tab.source_tab();
        self.tab_index = source;
        self.tab_group_filters[source] = if tab.is_manual() { 0 } else { tab.group_id };
        self.current_group_filter = self.tab_group_filters[source];
        self.tab_kind_filters[source] = if tab.is_manual() { ClipKindFilter::All } else { tab.kind_filter() };
        self.scroll_y = 0;
        self.clear_selection();
        self.scroll_date_hint_until = None;
        self.refilter();
        remember_shared_tab_view_state(self);
    }
    pub(super) fn reconcile_category(&mut self) {
        let index = self.active_category_index();
        if let Some(tab) = self.settings.category_tabs.get(index).cloned() {
            self.active_category_id = tab.id.clone();
            let source = tab.source_tab();
            self.tab_index = source;
            self.current_group_filter = if tab.is_manual() { 0 } else { tab.group_id };
            self.tab_group_filters[source] = self.current_group_filter;
            self.tab_kind_filters[source] = if tab.is_manual() { ClipKindFilter::All } else { tab.kind_filter() };
        }
    }
    pub(super) fn active_manual_tab_id(&self) -> Option<&str> {
        self.settings.category_tabs.iter().find(|tab| tab.id == self.active_category_id
            && tab.visible && tab.is_manual()).map(|tab| tab.id.as_str())
    }
    pub(super) fn row_actions(&self, item: &ClipItem) -> Vec<QuickRowAction> {
        row_action_order(&self.settings.quick_action_order)
            .into_iter()
            .filter(|action| match action {
                QuickRowAction::Delete => self.settings.quick_delete_button,
                QuickRowAction::Pin => self.settings.quick_pin_button,
                QuickRowAction::Edit => {
                    self.settings.quick_edit_button
                        && matches!(item.kind, ClipKind::Text | ClipKind::Phrase)
                }
            })
            .collect()
    }
    pub(super) fn quick_action_hit(&self, x: i32, y: i32) -> Option<(i64, QuickRowAction)> {
        if self.search_results_pending() {
            return None;
        }
        let layout = self.layout();
        let row = layout.hit_test_row(x, y, self.visible_count(), self.scroll_y);
        if row != self.hover_idx {
            return None;
        }
        let item = self.active_items().get(usize::try_from(row).ok()?)?;
        let actions = self.row_actions(item);
        for (i, action) in actions.iter().enumerate() {
            if layout
                .quick_action_rect(
                    row,
                    self.visible_count(),
                    self.scroll_y,
                    (actions.len() - 1 - i) as i32,
                )
                .is_some_and(|rect| contains(rect, x, y))
            {
                return Some((item.id, *action));
            }
        }
        None
    }
}

pub(super) unsafe fn draw_category_bar(dc: HDC, state: &AppState, theme: Theme) {
    let layout = state.layout();
    let bar = state.category_bar();
    let selected = state.active_category_index();
    let px = |n: i32| (n * state.ui_dpi.max(96) as i32 + 48) / 96;
    let area = RECT {
        left: layout.seg_x,
        top: layout.seg_y,
        right: layout.seg_x + layout.seg_w,
        bottom: layout.seg_y + layout.seg_h,
    };
    let brush = platform_gdi::create_solid_brush(theme.bg);
    platform_gdi::fill_rect(dc, &area, brush);
    platform_gdi::delete_object(brush as _);
    let right = bar
        .more
        .map(|r| r.right)
        .or_else(|| bar.items.last().map(|b| b.rect.right))
        .unwrap_or(area.left);
    let left = bar
        .items
        .first()
        .map(|b| b.rect.left)
        .or_else(|| bar.more.map(|r| r.left))
        .unwrap_or(area.left);
    if right > left {
        draw_round_rect(
            dc,
            &RECT {
                left,
                right,
                ..area
            },
            theme.surface,
            theme.stroke,
            px(4),
        );
    }
    for button in bar.items {
        let rect: RECT = button.rect.into();
        let body = RECT {
            left: rect.left + px(2),
            top: rect.top + px(2),
            right: rect.right - px(2),
            bottom: rect.bottom - px(2),
        };
        if button.index == selected {
            draw_round_rect(
                dc,
                &body,
                main_theme_role_color(MainThemeRole::SegmentSelected, theme),
                theme.stroke,
                px(3),
            );
        } else if state.hover_category == button.index as i32 {
            draw_round_fill(dc, &body, theme.item_hover, px(3));
        }
        let text = state
            .settings
            .category_tabs
            .get(button.index)
            .map(|t| t.name.as_str())
            .unwrap_or("");
        let inner = RECT {
            left: rect.left + px(8),
            right: rect.right - px(8),
            ..rect
        };
        draw_text_ex(
            dc,
            text,
            &inner,
            if button.index == selected || state.hover_category == button.index as i32 {
                theme.text
            } else {
                theme.text_muted
            },
            px(13),
            false,
            true,
            ui_display_font_family(),
        );
    }
    if let Some(rect) = bar.more {
        let rect: RECT = rect.into();
        if state.hover_category == -2 {
            draw_round_fill(
                dc,
                &RECT {
                    left: rect.left + px(2),
                    top: rect.top + px(2),
                    right: rect.right - px(2),
                    bottom: rect.bottom - px(2),
                },
                theme.item_hover,
                px(3),
            );
        }
        draw_text_ex(
            dc,
            "⋯",
            &rect,
            theme.text_muted,
            px(16),
            false,
            true,
            ui_text_font_family(),
        );
    }
}

pub(super) unsafe fn handle_category_click(
    hwnd: HWND,
    state: &mut AppState,
    x: i32,
    y: i32,
) -> bool {
    let bar = state.category_bar();
    if let Some(index) = bar.hit(x, y) {
        state.activate_category(index);
        repaint_main_window(hwnd, false);
        return true;
    }
    if bar.more.is_some_and(|r| contains(r, x, y)) {
        let entries: Vec<_> = state
            .settings
            .category_tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| t.visible)
            .map(|(i, t)| NativePopupMenuEntry::Command {
                id: 43000 + i,
                label: t.name.clone(),
                enabled: true,
                checked: i == state.active_category_index(),
            })
            .collect();
        let point = platform_input::cursor_pos().unwrap_or(POINT { x: 0, y: 0 });
        vv_set_popup_menu_active(true);
        let selected = platform_menu::WindowsPopupMenuHost::new().present_popup_menu(
            hwnd,
            point.x,
            point.y,
            NativePopupMenuPlacement::TopLeft,
            &entries,
        );
        vv_set_popup_menu_active(false);
        if selected >= 43000 && selected < 43032 {
            state.activate_category(selected - 43000);
        }
        repaint_main_window(hwnd, false);
        schedule_hidden_memory_reclaim_after_activity();
        return true;
    }
    let layout = state.layout();
    contains(
        UiRect::new(
            layout.seg_x,
            layout.seg_y,
            layout.seg_x + layout.seg_w,
            layout.seg_y + layout.seg_h,
        ),
        x,
        y,
    )
}

pub(super) unsafe fn perform_quick_action(
    hwnd: HWND,
    state: &mut AppState,
    id: i64,
    action: QuickRowAction,
) {
    let Some(row) = state.active_items().iter().position(|item| item.id == id) else {
        return;
    };
    state.clear_selection();
    if let Some(plan) = state.list.context_menu_state_plan(row as i32, false, false) {
        state.list.apply_context_menu_state_plan(plan);
        let action = match action {
            QuickRowAction::Delete => MainRowMenuAction::Delete,
            QuickRowAction::Pin => MainRowMenuAction::Pin,
            QuickRowAction::Edit => MainRowMenuAction::Edit,
        };
        execute_row_command(hwnd, state, MainMenuCommandIntent::RowAction(action));
        repaint_main_window(hwnd, false);
    }
}
