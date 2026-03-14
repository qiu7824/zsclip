use std::collections::BTreeSet;

use crate::clip_models::ClipItem;

#[derive(Clone, Debug)]
pub(crate) struct ClipListState {
    pub(crate) filtered_indices: Vec<usize>,
    pub(crate) tab_index: usize,
    pub(crate) search_on: bool,
    pub(crate) search_text: String,
    pub(crate) hover_idx: i32,
    pub(crate) sel_idx: i32,
    pub(crate) scroll_y: i32,
    pub(crate) current_group_filter: i64,
    pub(crate) tab_group_filters: [i64; 2],
    pub(crate) selected_rows: BTreeSet<i32>,
    pub(crate) selection_anchor: i32,
    pub(crate) context_row: i32,
}

impl Default for ClipListState {
    fn default() -> Self {
        Self {
            filtered_indices: Vec::new(),
            tab_index: 0,
            search_on: false,
            search_text: String::new(),
            hover_idx: -1,
            sel_idx: -1,
            scroll_y: 0,
            current_group_filter: 0,
            tab_group_filters: [0, 0],
            selected_rows: BTreeSet::new(),
            selection_anchor: -1,
            context_row: -1,
        }
    }
}

impl ClipListState {
    pub(crate) fn refilter_with(&mut self, items: &[ClipItem], grouping_enabled: bool) {
        let key = self.search_text.trim().to_lowercase();
        let group_filter = if grouping_enabled { self.current_group_filter } else { 0 };
        self.filtered_indices = items
            .iter()
            .enumerate()
            .filter_map(|(idx, item)| {
                let group_ok = group_filter == 0 || item.group_id == group_filter;
                let search_ok = key.is_empty() || item.preview.to_lowercase().contains(&key);
                if group_ok && search_ok { Some(idx) } else { None }
            })
            .collect();

        if self.sel_idx >= self.filtered_indices.len() as i32 {
            self.sel_idx = if self.filtered_indices.is_empty() { -1 } else { 0 };
        }
        if self.hover_idx >= self.filtered_indices.len() as i32 {
            self.hover_idx = -1;
        }
        let max_idx = self.filtered_indices.len() as i32;
        self.selected_rows = self.selected_rows.iter().copied().filter(|i| *i >= 0 && *i < max_idx).collect();
        if self.sel_idx >= max_idx {
            self.sel_idx = if max_idx > 0 { max_idx - 1 } else { -1 };
        }
    }

    pub(crate) fn clear_selection(&mut self) {
        self.sel_idx = -1;
        self.hover_idx = -1;
        self.selected_rows.clear();
        self.selection_anchor = -1;
        self.context_row = -1;
    }

    pub(crate) fn row_is_selected(&self, visible_idx: i32) -> bool {
        visible_idx >= 0 && (self.sel_idx == visible_idx || self.selected_rows.contains(&visible_idx))
    }

    pub(crate) fn selected_visible_rows(&self) -> Vec<i32> {
        let mut rows: Vec<i32> = self.selected_rows.iter().copied().collect();
        if self.sel_idx >= 0 && !rows.contains(&self.sel_idx) {
            rows.push(self.sel_idx);
        }
        rows.sort_unstable();
        rows
    }

    pub(crate) fn selected_source_indices(&self) -> Vec<usize> {
        let mut src: Vec<usize> = self
            .selected_visible_rows()
            .into_iter()
            .filter_map(|v| self.filtered_indices.get(v as usize).copied())
            .collect();
        src.sort_unstable();
        src.dedup();
        src
    }

    pub(crate) fn selected_count(&self) -> usize {
        self.selected_source_indices().len()
    }

    pub(crate) fn context_selection_count(&self) -> usize {
        let n = self.selected_count();
        if n == 0 && self.context_row >= 0 { 1 } else { n }
    }
}
