use super::prelude::*;

pub(super) fn reload_state_from_db_persisting(state: &mut AppState) {
    if reload_state_from_db(state) {
        save_state_settings(state);
    }
}

fn item_payload_missing(item: &ClipItem) -> bool {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            item.text.is_none()
                || (item.rich_text_html.is_none() && item.preview.trim_start().starts_with("HTML"))
        }
        ClipKind::Files => item.file_paths.is_none() && item.text.is_none(),
        ClipKind::Image => item.image_bytes.is_none() && item.image_path.is_none(),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ClipItemAddOutcome {
    Applied,
    Duplicate,
    RetryableFailure,
}

fn sorted_front_insert_index(items: &[ClipItem], pinned: bool) -> usize {
    if pinned {
        0
    } else {
        items
            .iter()
            .position(|item| !item.pinned)
            .unwrap_or(items.len())
    }
}

fn insert_loaded_item_at_sorted_front(items: &mut Vec<ClipItem>, item: ClipItem) -> usize {
    let index = sorted_front_insert_index(items, item.pinned);
    items.insert(index, clip_item_to_summary(&item));
    index
}

impl AppState {
    pub(super) fn should_skip_transient_duplicate_capture(
        &mut self,
        signature: &str,
        source_app: &str,
        sequence: u32,
    ) -> bool {
        if signature.is_empty() {
            return false;
        }
        let now = Instant::now();
        while self
            .recent_capture_signatures
            .front()
            .map(|(_, _, at)| {
                now.saturating_duration_since(*at).as_millis()
                    > TRANSIENT_DUPLICATE_QUEUE_MS as u128
            })
            .unwrap_or(false)
        {
            self.recent_capture_signatures.pop_front();
        }
        let same_seq = sequence != 0 && self.last_clipboard_seq == sequence;
        let same_sig = self.last_capture_signature == signature
            && self
                .last_capture_at
                .map(|at| {
                    now.saturating_duration_since(at).as_millis()
                        <= TRANSIENT_DUPLICATE_CAPTURE_MS as u128
                })
                .unwrap_or(false);
        let recent_sig = self
            .recent_capture_signatures
            .iter()
            .rev()
            .any(|(sig, _app, at)| {
                sig == signature
                    && now.saturating_duration_since(*at).as_millis()
                        <= TRANSIENT_DUPLICATE_CAPTURE_MS as u128
            });
        if sequence != 0 {
            self.last_clipboard_seq = sequence;
        }
        self.last_capture_signature.clear();
        self.last_capture_signature.push_str(signature);
        self.last_capture_source_app.clear();
        self.last_capture_source_app.push_str(source_app);
        self.last_capture_at = Some(now);
        self.recent_capture_signatures.push_back((
            signature.to_string(),
            source_app.to_string(),
            now,
        ));
        while self.recent_capture_signatures.len() > 8 {
            self.recent_capture_signatures.pop_front();
        }
        same_seq || same_sig || recent_sig
    }

    pub(super) fn remember_lan_message_key(&mut self, key: &str) -> bool {
        let key = key.trim();
        if key.is_empty() {
            return true;
        }
        if self.recent_lan_message_keys.iter().any(|item| item == key) {
            return false;
        }
        self.recent_lan_message_keys.push_back(key.to_string());
        while self.recent_lan_message_keys.len() > LAN_RECENT_MESSAGE_LIMIT {
            self.recent_lan_message_keys.pop_front();
        }
        true
    }

    pub(super) fn note_programmatic_clipboard_signature(&mut self, signature: String, ms: u64) {
        self.recent_programmatic_clipboard_signature = signature;
        self.recent_programmatic_clipboard_until =
            Some(Instant::now() + std::time::Duration::from_millis(ms.max(1)));
    }

    pub(super) fn consume_skip_next_clipboard_update_once(&mut self, sequence: u32) -> bool {
        if !self.skip_next_clipboard_update_once {
            return false;
        }
        self.skip_next_clipboard_update_once = false;
        if sequence != 0 {
            self.last_clipboard_seq = sequence;
        }
        true
    }

    pub(super) fn consume_recent_programmatic_clipboard_signature(
        &mut self,
        signature: &str,
    ) -> bool {
        if signature.is_empty() || self.recent_programmatic_clipboard_signature.is_empty() {
            return false;
        }
        let now = Instant::now();
        let still_active = self
            .recent_programmatic_clipboard_until
            .map(|until| now < until)
            .unwrap_or(false);
        if !still_active {
            self.recent_programmatic_clipboard_signature.clear();
            self.recent_programmatic_clipboard_until = None;
            return false;
        }
        if self.recent_programmatic_clipboard_signature == signature {
            self.recent_programmatic_clipboard_signature.clear();
            self.recent_programmatic_clipboard_until = None;
            return true;
        }
        false
    }

    pub(super) fn selected_items_owned(&self) -> Vec<ClipItem> {
        self.selected_source_indices()
            .into_iter()
            .filter_map(|i| self.active_items().get(i).cloned())
            .collect()
    }

    pub(super) fn clear_payload_cache(&mut self) {
        self.payload_cache.clear();
        self.image_thumb_cache.clear();
        self.image_thumb_loading.clear();
        self.image_thumb_failed.clear();
    }

    pub(super) fn cache_full_item(&mut self, item: ClipItem) {
        self.payload_cache.put(item);
    }

    pub(super) fn remove_cached_item(&mut self, id: i64) {
        self.payload_cache.remove(id);
        self.image_thumb_cache.remove(id);
        self.image_thumb_loading.remove(&id);
        self.image_thumb_failed.remove(&id);
    }

    pub(super) fn current_scroll_anchor(&self) -> Option<(i64, i32)> {
        let top_visible = self.layout().row_index_at_offset(self.scroll_y).max(0) as usize;
        let offset = self.scroll_y - self.layout().row_offset(top_visible);
        let item = self.active_items().get(top_visible)?;
        if item.id > 0 {
            Some((item.id, offset))
        } else {
            None
        }
    }

    pub(super) fn restore_scroll_anchor(&mut self, anchor: Option<(i64, i32)>) {
        if let Some((id, offset)) = anchor {
            if let Some(visible_idx) = self.active_items().iter().position(|item| item.id == id) {
                self.scroll_y = self.layout().row_offset(visible_idx) + offset;
            }
        }
        self.clamp_scroll();
        self.maybe_request_more_for_active_tab();
    }

    pub(super) fn reload_state_from_db_preserve_scroll(&mut self, anchor: Option<(i64, i32)>) {
        reload_state_from_db_persisting(self);
        self.restore_scroll_anchor(anchor);
    }

    pub(super) fn remove_items_from_active_tab(&mut self, ids: &[i64]) {
        if ids.is_empty() {
            return;
        }
        let id_set: HashSet<i64> = ids.iter().copied().collect();
        self.items_for_tab_mut(self.tab_index)
            .retain(|item| !id_set.contains(&item.id));
    }

    pub(super) fn remove_duplicate_history_items(&mut self, ids: &[i64]) {
        if ids.is_empty() {
            return;
        }
        let id_set: HashSet<i64> = ids.iter().copied().collect();
        self.records.retain(|item| !id_set.contains(&item.id));
        self.phrases.retain(|item| !id_set.contains(&item.id));
    }

    pub(super) fn promote_loaded_item_to_top(&mut self, old_id: i64, new_id: i64) -> Option<usize> {
        if old_id <= 0 || new_id <= 0 {
            return None;
        }
        let items = self.items_for_tab_mut(self.tab_index);
        let Some(pos) = items.iter().position(|item| item.id == old_id) else {
            return None;
        };
        let mut item = items.remove(pos);
        item.id = new_id;
        Some(insert_loaded_item_at_sorted_front(items, item))
    }

    pub(super) fn insert_loaded_record_at_sorted_front(&mut self, item: ClipItem) -> usize {
        insert_loaded_item_at_sorted_front(&mut self.records, item)
    }

    pub(super) fn load_item_full_cached(&mut self, id: i64) -> Option<ClipItem> {
        if id <= 0 {
            return None;
        }
        if let Some(item) = self.payload_cache.get(id) {
            return Some(item);
        }
        let item = db_load_item_full(id)?;
        self.payload_cache.put(item.clone());
        Some(item)
    }

    pub(super) fn resolve_item_for_use(&mut self, item: &ClipItem) -> Option<ClipItem> {
        if item.id <= 0 {
            return Some(item.clone());
        }
        if item_payload_missing(item) {
            self.load_item_full_cached(item.id)
        } else {
            Some(item.clone())
        }
    }

    pub(super) fn current_item_for_use(&mut self) -> Option<ClipItem> {
        let item = self.current_item_owned()?;
        self.resolve_item_for_use(&item)
    }

    pub(super) fn selected_items_for_use(&mut self) -> Vec<ClipItem> {
        let items = self.selected_items_owned();
        items
            .into_iter()
            .filter_map(|item| self.resolve_item_for_use(&item))
            .collect()
    }

    pub(super) fn context_selection_has_unpinned(&self) -> bool {
        let items = self.selected_items_owned();
        if items.is_empty() {
            self.current_item().map(|it| !it.pinned).unwrap_or(false)
        } else {
            items.iter().any(|it| !it.pinned)
        }
    }

    pub(super) fn add_clip_item(&mut self, item: ClipItem, signature: String) -> bool {
        self.add_clip_item_inner(item, signature, true, false) == ClipItemAddOutcome::Applied
    }

    pub(super) fn add_clip_item_for_capture(
        &mut self,
        item: ClipItem,
        signature: String,
    ) -> Result<bool, ()> {
        match self.add_clip_item_inner(item, signature, true, false) {
            ClipItemAddOutcome::Applied => Ok(true),
            ClipItemAddOutcome::Duplicate => Ok(false),
            ClipItemAddOutcome::RetryableFailure => Err(()),
        }
    }

    pub(super) fn add_lan_clip_item(&mut self, item: ClipItem, signature: String) -> bool {
        self.add_clip_item_inner(item, signature, false, true) == ClipItemAddOutcome::Applied
    }

    fn add_clip_item_inner(
        &mut self,
        item: ClipItem,
        signature: String,
        broadcast_lan: bool,
        force_dedupe: bool,
    ) -> ClipItemAddOutcome {
        let cleanup_item = item.clone();
        let expected_generation = self.app_data_generation;
        crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
            self.add_clip_item_inner_locked(item, signature, broadcast_lan, force_dedupe)
        })
        .unwrap_or_else(|| {
            remove_uninserted_image_file(&cleanup_item);
            ClipItemAddOutcome::RetryableFailure
        })
    }

    fn add_clip_item_inner_locked(
        &mut self,
        mut item: ClipItem,
        signature: String,
        broadcast_lan: bool,
        force_dedupe: bool,
    ) -> ClipItemAddOutcome {
        let signature = dedupe_signature_for_item(&item, &signature);
        let full_dedupe = force_dedupe || self.settings.dedupe_filter_enabled;

        if !signature.is_empty()
            && db_latest_item_signature(0)
                .as_deref()
                .is_some_and(|latest| latest == signature)
        {
            remove_uninserted_image_file(&item);
            return ClipItemAddOutcome::Duplicate;
        }

        if full_dedupe && !signature.is_empty() {
            let duplicate_ids = db_find_duplicate_item_ids(0, &item, &signature);
            if let Some(existing_id) = duplicate_ids.first().copied() {
                if force_dedupe {
                    remove_uninserted_image_file(&item);
                    return ClipItemAddOutcome::Duplicate;
                }
                let anchor = self.current_scroll_anchor();
                let existing_pinned = db_item_is_pinned(existing_id);
                let removed_ids: Vec<i64> = if existing_pinned {
                    duplicate_ids
                        .iter()
                        .copied()
                        .filter(|id| *id != existing_id && !db_item_is_pinned(*id))
                        .collect()
                } else {
                    duplicate_ids.into_iter().skip(1).collect()
                };
                if !removed_ids.is_empty() {
                    for id in &removed_ids {
                        let _ = db_delete_item(*id);
                        self.remove_cached_item(*id);
                    }
                    self.remove_duplicate_history_items(&removed_ids);
                }
                if existing_pinned {
                    if !removed_ids.is_empty() {
                        self.reload_state_from_db_preserve_scroll(anchor);
                        unsafe {
                            sync_peer_windows_from_db(self.hwnd);
                        }
                    }
                    remove_uninserted_image_file(&item);
                    return ClipItemAddOutcome::Duplicate;
                }
                if let Ok(new_id) = db_promote_item_to_top(existing_id) {
                    remove_uninserted_image_file(&item);
                    self.remove_cached_item(existing_id);
                    self.remove_cached_item(new_id);
                    if self
                        .promote_loaded_item_to_top(existing_id, new_id)
                        .is_none()
                    {
                        reload_state_from_db_persisting(self);
                    } else {
                        self.refilter();
                    }
                    if self.tab_index == 0 {
                        self.sel_idx = self
                            .records
                            .iter()
                            .position(|item| item.id == new_id)
                            .unwrap_or(0) as i32;
                        self.scroll_y = 0;
                    }
                    unsafe {
                        sync_peer_windows_from_db(self.hwnd);
                    }
                    refresh_lan_latest_from_db(&self.settings);
                    return ClipItemAddOutcome::Applied;
                }
                remove_uninserted_image_file(&item);
                return ClipItemAddOutcome::RetryableFailure;
            }
        }
        let insert_result = db_insert_item(0, &item, Some(signature.as_str()));
        item.id = insert_result.unwrap_or(0);
        if item.id <= 0 {
            remove_uninserted_image_file(&item);
            return ClipItemAddOutcome::RetryableFailure;
        }
        // DB assigns created_at with CURRENT_TIMESTAMP; fill memory so date headers render correctly.
        if item.created_at.is_empty() {
            item.created_at = now_utc_sqlite();
        }
        if item.id > 0 {
            self.cache_full_item(item.clone());
        }
        let summary = clip_item_to_summary(&item);
        let visible_query = self.load_state_for_tab(0).query.clone();
        let inserted_index = if matches!(visible_query, Some(ref query) if query.group_id == 0 && query.search_text.trim().is_empty())
        {
            let index = self.insert_loaded_record_at_sorted_front(summary);
            if self.tab_index == 0 {
                self.list.apply_visible_len(self.records.len());
            }
            Some(index)
        } else {
            self.invalidate_tab_query(0, self.tab_index == 0);
            None
        };
        let max_items = self.settings.max_items;
        if max_items > 0 {
            self.invalidate_tab_query(0, self.tab_index == 0);
        }
        if self.tab_index == 0 {
            self.sel_idx = inserted_index.unwrap_or(0) as i32;
        }
        self.refilter();
        unsafe {
            sync_peer_windows_from_db(self.hwnd);
        }
        if broadcast_lan {
            maybe_broadcast_lan_clip_item(self, &item, &signature);
        }
        refresh_lan_latest_from_db(&self.settings);
        ClipItemAddOutcome::Applied
    }

    pub(super) fn list_view_height(&self) -> i32 {
        self.layout().list_view_height()
    }

    pub(super) fn clamp_scroll(&mut self) {
        self.scroll_y = self
            .layout()
            .clamp_scroll(self.scroll_y, self.visible_count());
    }

    pub(super) fn ensure_visible(&mut self, idx: i32) {
        self.scroll_y = self
            .layout()
            .ensure_visible(self.scroll_y, idx, self.visible_count());
    }

    pub(super) fn layout(&self) -> MainUiLayout {
        let mut layout =
            main_layout_for_dpi(self.ui_dpi).with_app_icon_visible(self.settings.app_icon_visible);
        layout = layout.with_pin_button(window_pin_visible(self));
        let search_right = layout.title_button_rect("search").left - 4;
        layout.search_w = (search_right - layout.search_left).max(30);
        let scale = layout.row_h;
        layout = layout.with_row_heights(self.active_items().iter().map(|item| {
            let height = match item.kind {
                ClipKind::Image if self.settings.image_preview_enabled => {
                    self.settings.image_row_height.clamp(80, 320)
                }
                ClipKind::Files => self.settings.file_row_height.clamp(32, 160),
                _ => self.settings.text_row_height.clamp(32, 160),
            };
            (height * scale + 22) / 44
        }));
        layout
    }

    pub(super) fn quick_action_rect_slot(&self, visible_idx: i32, slot: i32) -> Option<RECT> {
        self.layout()
            .quick_action_rect(visible_idx, self.visible_count(), self.scroll_y, slot)
            .map(Into::into)
    }

    pub(super) fn segment_rects(&self) -> (RECT, RECT) {
        let (left, right) = self.layout().segment_rects();
        (left.into(), right.into())
    }

    pub(super) fn scroll_to_top_rect(&self) -> RECT {
        self.layout().scroll_to_top_button_rect().into()
    }

    pub(super) fn selected_db_ids(&self) -> Vec<i64> {
        self.selected_source_indices()
            .into_iter()
            .filter_map(|i| self.active_items().get(i).map(|it| it.id))
            .filter(|id| *id > 0)
            .collect()
    }
}

#[cfg(test)]
mod sorted_insert_tests {
    use super::*;

    fn item(id: i64, pinned: bool) -> ClipItem {
        ClipItem {
            id,
            kind: ClipKind::Text,
            preview: id.to_string(),
            text: None,
            rich_text_html: None,
            source_app: String::new(),
            file_paths: None,
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned,
            group_id: 0,
            created_at: String::new(),
        }
    }

    #[test]
    fn sorted_front_insert_keeps_unpinned_items_below_pinned_items() {
        let mut items = vec![item(3, true), item(2, true), item(1, false)];
        let index = insert_loaded_item_at_sorted_front(&mut items, item(4, false));

        assert_eq!(index, 2);
        assert_eq!(
            items.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![3, 2, 4, 1]
        );
    }

    #[test]
    fn sorted_front_insert_puts_newest_pinned_item_first() {
        let mut items = vec![item(3, true), item(2, false), item(1, false)];
        let index = insert_loaded_item_at_sorted_front(&mut items, item(4, true));

        assert_eq!(index, 0);
        assert_eq!(
            items.iter().map(|item| item.id).collect::<Vec<_>>(),
            vec![4, 3, 2, 1]
        );
    }
}
