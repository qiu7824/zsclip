use super::{ClipKindFilter, UiRect};
use serde::{Deserialize, Serialize};

pub(crate) const MAX_CATEGORY_TABS: usize = 32;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CategoryTabMode {
    #[default]
    Legacy,
    Manual,
    Filter,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub(crate) struct CategoryTab {
    pub id: String,
    pub name: String,
    pub source: usize,
    pub group_id: i64,
    pub kind: String,
    pub visible: bool,
    #[serde(default)]
    pub mode: CategoryTabMode,
}
impl Default for CategoryTab {
    fn default() -> Self {
        Self {
            id: String::new(),
            name: "新分类".into(),
            source: 0,
            group_id: 0,
            kind: "all".into(),
            visible: true,
            mode: CategoryTabMode::Manual,
        }
    }
}
impl CategoryTab {
    pub fn is_manual(&self) -> bool {
        match self.mode {
            CategoryTabMode::Manual => true,
            CategoryTabMode::Filter => false,
            CategoryTabMode::Legacy => self.id != "records" && self.id != "phrases"
                && self.source == 0 && self.group_id == 0 && self.kind == "all",
        }
    }
    pub fn source_tab(&self) -> usize {
        if self.is_manual() { 0 } else { self.source.min(1) }
    }
    pub fn kind_filter(&self) -> ClipKindFilter {
        match self.kind.as_str() {
            "text" => ClipKindFilter::Text,
            "image" => ClipKindFilter::Image,
            "files" => ClipKindFilter::Files,
            "phrase" => ClipKindFilter::Phrase,
            _ => ClipKindFilter::All,
        }
    }
}
pub(crate) fn default_category_tabs() -> Vec<CategoryTab> {
    vec![
        CategoryTab {
            id: "records".into(),
            name: "复制记录".into(),
            mode: CategoryTabMode::Filter,
            ..Default::default()
        },
        CategoryTab {
            id: "phrases".into(),
            name: "常用短语".into(),
            source: 1,
            mode: CategoryTabMode::Filter,
            ..Default::default()
        },
    ]
}
pub(crate) fn validate_category_tabs(tabs: &[CategoryTab]) -> Result<(), &'static str> {
    if tabs.is_empty() || tabs.len() > MAX_CATEGORY_TABS {
        return Err("分类标签数量应为 1–32 个。");
    }
    if !tabs.iter().any(|tab| tab.visible) {
        return Err("至少保留一个显示中的分类标签。");
    }
    let mut ids = std::collections::BTreeSet::new();
    for tab in tabs {
        if tab.id.is_empty() || !ids.insert(&tab.id) {
            return Err("分类标签标识重复，请重新添加该标签。");
        }
        if tab.name.trim().is_empty()
            || tab.name.chars().count() > 32
            || tab.name.chars().any(char::is_control)
        {
            return Err("分类名称应为 1–32 个字符，不能包含换行或控制字符。");
        }
        if tab.source > 1
            || tab.group_id < 0
            || !["all", "text", "image", "files", "phrase"].contains(&tab.kind.as_str())
        {
            return Err("分类来源或文件类型无效。");
        }
    }
    Ok(())
}

pub(crate) fn normalize_category_tabs(tabs: &mut Vec<CategoryTab>) {
    tabs.truncate(MAX_CATEGORY_TABS);
    if tabs.is_empty() {
        *tabs = default_category_tabs();
        return;
    }
    let mut ids = std::collections::BTreeSet::new();
    for (index, tab) in tabs.iter_mut().enumerate() {
        tab.name = tab
            .name
            .trim()
            .chars()
            .filter(|c| !c.is_control())
            .take(32)
            .collect();
        if tab.name.is_empty() {
            tab.name = format!("分类 {}", index + 1);
        }
        if tab.id.is_empty() || ids.contains(&tab.id) {
            tab.id = format!("custom-{index}");
            while ids.contains(&tab.id) {
                tab.id.push('_');
            }
        }
        ids.insert(tab.id.clone());
        tab.source = tab.source.min(1);
        tab.group_id = tab.group_id.max(0);
        if !["all", "text", "image", "files", "phrase"].contains(&tab.kind.as_str()) {
            tab.kind = "all".into();
        }
        tab.mode = if tab.is_manual() { CategoryTabMode::Manual } else { CategoryTabMode::Filter };
    }
    if !tabs.iter().any(|tab| tab.visible) {
        tabs[0].visible = true;
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CategoryBarItem {
    pub index: usize,
    pub rect: UiRect,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CategoryBar {
    pub items: Vec<CategoryBarItem>,
    pub more: Option<UiRect>,
}
impl CategoryBar {
    pub fn new(tabs: &[CategoryTab], active: usize, rect: UiRect, dpi: u32) -> Self {
        let px = |n: i32| (n * dpi.max(96) as i32 + 48) / 96;
        let entries: Vec<_> = tabs
            .iter()
            .enumerate()
            .filter(|(_, t)| t.visible)
            .map(|(index, tab)| {
                let text = tab
                    .name
                    .chars()
                    .map(|ch| if ch.is_ascii() { 7 } else { 13 })
                    .sum::<i32>();
                (index, px((text + 32).max(144)))
            })
            .collect();
        if entries.is_empty() {
            return Self {
                items: Vec::new(),
                more: None,
            };
        }
        let available = (rect.right - rect.left).max(1);
        let overflow = entries.iter().map(|(_, w)| *w).sum::<i32>() > available;
        let more_width = if overflow { px(28).min(available) } else { 0 };
        let budget = (available - more_width).max(1);
        let active_pos = entries.iter().position(|(i, _)| *i == active).unwrap_or(0);
        let mut start = if overflow { active_pos } else { 0 };
        let mut used = entries[start].1.min(budget);
        if overflow {
            while start > 0 && used + entries[start - 1].1 <= budget {
                start -= 1;
                used += entries[start].1;
            }
        }
        let mut end = if overflow { active_pos + 1 } else { 1 };
        while end < entries.len() && used + entries[end].1 <= budget {
            used += entries[end].1;
            end += 1;
        }
        let left = rect.left + (available - used - more_width).max(0) / 2;
        let mut x = left;
        let items = entries[start..end]
            .iter()
            .map(|&(index, width)| {
                let right = (x + width).min(left + budget);
                let item = CategoryBarItem {
                    index,
                    rect: UiRect::new(x, rect.top, right, rect.bottom),
                };
                x = right;
                item
            })
            .collect();
        Self {
            items,
            more: overflow
                .then(|| UiRect::new(x, rect.top, (x + more_width).min(rect.right), rect.bottom)),
        }
    }
    pub fn hit(&self, x: i32, y: i32) -> Option<usize> {
        self.items
            .iter()
            .find(|item| contains(item.rect, x, y))
            .map(|item| item.index)
    }
}
pub(crate) fn contains(rect: UiRect, x: i32, y: i32) -> bool {
    x >= rect.left && x < rect.right && y >= rect.top && y < rect.bottom
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum QuickRowAction {
    Delete,
    Pin,
    Edit,
}
impl QuickRowAction {
    pub fn key(self) -> &'static str {
        match self {
            Self::Delete => "delete",
            Self::Pin => "pin",
            Self::Edit => "edit",
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::Delete => "删除",
            Self::Pin => "置顶",
            Self::Edit => "编辑",
        }
    }
}
pub(crate) fn row_action_order(value: &str) -> Vec<QuickRowAction> {
    let mut result = Vec::with_capacity(3);
    for key in value.split(',').chain(["delete", "pin", "edit"]) {
        let action = match key.trim() {
            "delete" => QuickRowAction::Delete,
            "pin" => QuickRowAction::Pin,
            "edit" => QuickRowAction::Edit,
            _ => continue,
        };
        if !result.contains(&action) {
            result.push(action);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manual_defaults_and_legacy_migration_do_not_import_all_history() {
        assert!(CategoryTab::default().is_manual());
        assert!(default_category_tabs().iter().all(|t| !t.is_manual()));
        let mut tabs: Vec<CategoryTab> = serde_json::from_str(r#"[
            {"id":"records","name":"我的记录"},
            {"id":"phrases","name":"常用内容","source":1},
            {"id":"custom","name":"项目"},
            {"id":"photos","name":"图片","kind":"image"},
            {"id":"work","name":"工程","group_id":7}]
        "#).unwrap();
        normalize_category_tabs(&mut tabs);
        assert!(!tabs[0].is_manual()); assert!(!tabs[1].is_manual());
        assert!(tabs[2].is_manual());
        assert!(!tabs[3].is_manual()); assert!(!tabs[4].is_manual());
        let restored: Vec<CategoryTab> = serde_json::from_str(&serde_json::to_string(&tabs).unwrap()).unwrap();
        assert_eq!(tabs, restored);
    }
    #[test]
    fn editable_categories_keep_sources_and_ids_when_renamed_or_reordered() {
        let mut tabs = default_category_tabs();
        tabs[0].name = "我的资料".into();
        tabs.push(CategoryTab {
            id: "pictures".into(),
            name: "工程照片".into(),
            kind: "image".into(),
            group_id: 7,
            ..Default::default()
        });
        tabs.swap(0, 2);
        validate_category_tabs(&tabs).unwrap();
        let roundtrip: Vec<CategoryTab> =
            serde_json::from_str(&serde_json::to_string(&tabs).unwrap()).unwrap();
        assert_eq!(roundtrip, tabs);
        assert_eq!(tabs[0].kind_filter(), ClipKindFilter::Image);
        tabs.remove(1);
        validate_category_tabs(&tabs).unwrap();
        assert!(tabs
            .iter()
            .any(|t| t.id == "records" && t.name == "我的资料"));
        tabs.iter_mut().for_each(|t| t.visible = false);
        assert!(validate_category_tabs(&tabs).is_err());
    }
    #[test]
    fn category_overflow_keeps_selected_tab_visible_without_overlapping_hit_targets() {
        let tabs: Vec<_> = (0..10)
            .map(|i| CategoryTab {
                id: i.to_string(),
                name: format!("分类{i}"),
                ..Default::default()
            })
            .collect();
        let bar = CategoryBar::new(&tabs, 8, UiRect::new(6, 36, 294, 66), 96);
        assert!(bar.items.iter().any(|x| x.index == 8));
        assert!(bar.more.is_some());
        for item in &bar.items {
            assert_eq!(bar.hit(item.rect.left + 2, 40), Some(item.index));
        }
        assert!(bar.items.last().unwrap().rect.right <= bar.more.unwrap().left);
        assert!(CategoryBar::new(&tabs, 8, UiRect::new(6, 36, 1800, 66), 96)
            .more
            .is_none());
    }
    #[test]
    fn row_buttons_deduplicate_order_and_retain_missing_actions() {
        assert_eq!(
            row_action_order("edit,edit,unknown,delete"),
            [
                QuickRowAction::Edit,
                QuickRowAction::Delete,
                QuickRowAction::Pin
            ]
        );
    }

    #[test]
    fn expanding_window_does_not_stretch_compact_tabs_or_change_their_font_metrics() {
        let tabs = default_category_tabs();
        let narrow = CategoryBar::new(&tabs, 0, UiRect::new(6, 36, 294, 66), 96);
        let wide = CategoryBar::new(&tabs, 0, UiRect::new(6, 36, 1200, 66), 96);
        assert_eq!(
            narrow
                .items
                .iter()
                .map(|t| t.rect.width())
                .collect::<Vec<_>>(),
            wide.items
                .iter()
                .map(|t| t.rect.width())
                .collect::<Vec<_>>()
        );
        assert_eq!(wide.items[0].rect.width(), 144);
        assert_eq!(narrow.items[0].rect.left + narrow.items[1].rect.right, 300);
        assert_eq!(wide.items[0].rect.left + wide.items[1].rect.right, 1206);
        assert!(wide.hit(1100, 50).is_none());
        let mut named = tabs;
        named[0].name = "更长的自定义分类名称".into();
        assert!(
            CategoryBar::new(&named, 0, UiRect::new(6, 36, 1200, 66), 96).items[0]
                .rect
                .width()
                > 144
        );
    }
}
