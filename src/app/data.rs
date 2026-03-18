use super::*;

struct DbItem {
    id: i64,
    kind: String,
    preview: String,
    text: Option<String>,
    file_paths: Option<String>,
    image_bytes: Option<Vec<u8>>,
    image_path: Option<String>,
    image_width: i64,
    image_height: i64,
    pinned: i64,
    group_id: i64,
    created_at: String,
}

fn row_to_clip_item(row: DbItem) -> ClipItem {
    ClipItem {
        id: row.id,
        kind: match row.kind.as_str() {
            "image" => ClipKind::Image,
            "phrase" => ClipKind::Phrase,
            "files" => ClipKind::Files,
            _ => ClipKind::Text,
        },
        preview: row.preview,
        text: row.text,
        file_paths: row
            .file_paths
            .map(|value| value.split('\n').map(|item| item.to_string()).collect()),
        image_bytes: row.image_bytes,
        image_path: row.image_path,
        image_width: row.image_width.max(0) as usize,
        image_height: row.image_height.max(0) as usize,
        pinned: row.pinned == 1,
        group_id: row.group_id,
        created_at: row.created_at,
    }
}

fn row_to_clip_item_summary(row: DbItem) -> ClipItem {
    ClipItem {
        id: row.id,
        kind: match row.kind.as_str() {
            "image" => ClipKind::Image,
            "phrase" => ClipKind::Phrase,
            "files" => ClipKind::Files,
            _ => ClipKind::Text,
        },
        preview: row.preview,
        text: None,
        file_paths: None,
        image_bytes: None,
        image_path: row.image_path,
        image_width: row.image_width.max(0) as usize,
        image_height: row.image_height.max(0) as usize,
        pinned: row.pinned == 1,
        group_id: row.group_id,
        created_at: row.created_at,
    }
}

pub(super) fn clip_item_to_summary(item: &ClipItem) -> ClipItem {
    ClipItem {
        id: item.id,
        kind: item.kind,
        preview: item.preview.clone(),
        text: None,
        file_paths: None,
        image_bytes: None,
        image_path: item.image_path.clone(),
        image_width: item.image_width,
        image_height: item.image_height,
        pinned: item.pinned,
        group_id: item.group_id,
        created_at: item.created_at.clone(),
    }
}

fn current_local_day_value() -> i64 {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
        + local_offset_secs();
    let (year, month, day, _, _, _) = unix_secs_to_parts(now_secs);
    gregorian_to_days(year, month, day)
}

pub(super) fn db_load_items_page(
    query: &ItemsQuery,
    cursor: Option<ItemsCursor>,
    limit: usize,
) -> rusqlite::Result<(Vec<ClipItem>, Option<ItemsCursor>, bool)> {
    let (search_terms, time_filter) = parse_search_query(query.search_text.trim());
    with_db(|conn| {
        let mut sql = String::from(
            "SELECT id, kind, preview, image_path, image_width, image_height, pinned, group_id, \
             COALESCE(created_at, '') as created_at \
             FROM items WHERE category=?",
        );
        let mut bind_values = vec![SqlValue::from(query.category)];

        if query.group_id > 0 {
            sql.push_str(" AND group_id=?");
            bind_values.push(SqlValue::from(query.group_id));
        }

        for term in search_terms {
            let like = format!("%{}%", term);
            sql.push_str(
                " AND (LOWER(preview) LIKE ? \
                 OR LOWER(COALESCE(strftime('%m-%d %H:%M', datetime(created_at, 'localtime')), '')) LIKE ?)",
            );
            bind_values.push(SqlValue::from(like.clone()));
            bind_values.push(SqlValue::from(like));
        }

        match time_filter {
            Some(SearchTimeFilter::ExactDay(day)) => {
                sql.push_str(" AND date(created_at, 'localtime') = ?");
                bind_values.push(SqlValue::from(days_to_sqlite_date(day)));
            }
            Some(SearchTimeFilter::RecentDays(days)) => {
                let end_day = current_local_day_value();
                let start_day = end_day - (days.max(1) - 1);
                sql.push_str(
                    " AND date(created_at, 'localtime') >= ? AND date(created_at, 'localtime') <= ?",
                );
                bind_values.push(SqlValue::from(days_to_sqlite_date(start_day)));
                bind_values.push(SqlValue::from(days_to_sqlite_date(end_day)));
            }
            None => {}
        }

        if let Some(cursor) = cursor {
            let pinned = if cursor.pinned { 1_i64 } else { 0_i64 };
            sql.push_str(" AND (pinned < ? OR (pinned = ? AND id < ?))");
            bind_values.push(SqlValue::from(pinned));
            bind_values.push(SqlValue::from(pinned));
            bind_values.push(SqlValue::from(cursor.id));
        }

        sql.push_str(" ORDER BY pinned DESC, id DESC LIMIT ?");
        bind_values.push(SqlValue::from(limit.max(1) as i64 + 1));

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(bind_values.iter()), |row| {
            Ok(DbItem {
                id: row.get(0)?,
                kind: row.get(1)?,
                preview: row.get(2)?,
                text: None,
                file_paths: None,
                image_path: row.get(3)?,
                image_bytes: None,
                image_width: row.get(4)?,
                image_height: row.get(5)?,
                pinned: row.get(6)?,
                group_id: row.get(7)?,
                created_at: row.get(8)?,
            })
        })?;

        let mut items: Vec<ClipItem> = rows
            .filter_map(|row| row.ok().map(row_to_clip_item_summary))
            .collect();
        let has_more = items.len() > limit.max(1);
        if has_more {
            items.truncate(limit.max(1));
        }
        let next_cursor = items.last().map(|item| ItemsCursor {
            pinned: item.pinned,
            id: item.id,
        });
        Ok((items, next_cursor, has_more))
    })
}

pub(super) fn db_load_vv_popup_items(category: i64, group_id: i64, limit: usize) -> Vec<ClipItem> {
    let query = ItemsQuery {
        category,
        group_id,
        search_text: String::new(),
    };
    db_load_items_page(&query, None, limit)
        .map(|(items, _, _)| items)
        .unwrap_or_default()
}

pub(super) fn spawn_items_page_load(
    hwnd: HWND,
    tab: usize,
    request_seq: u64,
    query: ItemsQuery,
    cursor: Option<ItemsCursor>,
    reset: bool,
) {
    let hwnd_value = hwnd as isize;
    std::thread::spawn(move || {
        let result = match db_load_items_page(&query, cursor, ITEMS_PAGE_SIZE) {
            Ok((items, next_cursor, has_more)) => PageLoadResult {
                hwnd: hwnd_value,
                tab,
                request_seq,
                query,
                reset,
                items,
                next_cursor,
                has_more,
                error: None,
            },
            Err(err) => PageLoadResult {
                hwnd: hwnd_value,
                tab,
                request_seq,
                query,
                reset,
                items: Vec::new(),
                next_cursor: cursor,
                has_more: false,
                error: Some(err.to_string()),
            },
        };

        unsafe {
            let still_alive = hwnd_value != 0
                && window_host_hwnds()
                    .into_iter()
                    .any(|host| host == hwnd_value as HWND && IsWindow(host) != 0);
            if still_alive {
                if let Ok(mut queue) = page_load_results().lock() {
                    queue.push_back(result);
                }
                let _ = PostMessageW(hwnd_value as HWND, WM_ITEMS_PAGE_READY, 0, 0);
            }
        }
    });
}

pub(super) unsafe fn apply_ready_page_loads(hwnd: HWND, state: &mut AppState) {
    let mut changed = false;
    if let Ok(mut queue) = page_load_results().lock() {
        let mut pending = VecDeque::new();
        while let Some(result) = queue.pop_front() {
            if result.hwnd != hwnd as isize {
                pending.push_back(result);
                continue;
            }
            changed |= state.apply_page_load_result(result);
        }
        *queue = pending;
    }
    if changed {
        InvalidateRect(hwnd, null(), 1);
    }
}

pub(super) fn db_load_item_full(id: i64) -> Option<ClipItem> {
    with_db(|conn| {
        conn.query_row(
            "SELECT id, kind, preview, text_data, file_paths, image_data, image_width, image_height, pinned, group_id, image_path, \
             COALESCE(created_at, '') as created_at \
             FROM items WHERE id=?",
            params![id],
            |row| {
                Ok(row_to_clip_item(DbItem {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    preview: row.get(2)?,
                    text: row.get(3)?,
                    file_paths: row.get(4)?,
                    image_bytes: row.get(5)?,
                    image_path: row.get(10)?,
                    image_width: row.get(6)?,
                    image_height: row.get(7)?,
                    pinned: row.get(8)?,
                    group_id: row.get(9)?,
                    created_at: row.get(11)?,
                }))
            },
        )
    })
    .ok()
}

pub(super) fn db_insert_item(category: i64, item: &ClipItem) -> rusqlite::Result<i64> {
    let kind = match item.kind {
        ClipKind::Image => "image",
        ClipKind::Phrase => "phrase",
        ClipKind::Files => "files",
        _ => "text",
    };
    let preview = item.preview.clone();
    let text_data = item.text.clone();
    let file_paths = item.file_paths.as_ref().map(|paths| paths.join("\n"));
    let image_data = item.image_bytes.clone();
    let image_path = item.image_path.clone();
    with_db(|conn| {
        conn.execute(
            "INSERT INTO items(category, kind, preview, text_data, file_paths, image_data, image_path, image_width, image_height, pinned, group_id)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                category,
                kind,
                preview,
                text_data,
                file_paths,
                image_data,
                image_path,
                item.image_width as i64,
                item.image_height as i64,
                if item.pinned { 1 } else { 0 },
                item.group_id,
            ],
        )?;
        Ok(conn.last_insert_rowid())
    })
}

pub(super) fn db_update_item_pinned(id: i64, pinned: bool) -> rusqlite::Result<()> {
    with_db(|conn| {
        conn.execute(
            "UPDATE items SET pinned=? WHERE id=?",
            params![if pinned { 1 } else { 0 }, id],
        )?;
        Ok(())
    })
}

pub(super) fn db_prune_items(max_items: usize) {
    if max_items == 0 {
        return;
    }
    let _ = with_db(|conn| {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM items WHERE pinned=0",
                [],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);
        let excess = count - max_items as i64;
        if excess > 0 {
            conn.execute(
                "DELETE FROM items WHERE id IN (SELECT id FROM items WHERE pinned=0 ORDER BY id ASC LIMIT ?)",
                params![excess],
            )?;
        }
        Ok(())
    });
}

pub(super) fn db_delete_item(id: i64) -> rusqlite::Result<()> {
    with_db(|conn| {
        conn.execute("DELETE FROM items WHERE id=?", params![id])?;
        Ok(())
    })
}

pub(super) fn db_load_groups(category: i64) -> Vec<ClipGroup> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, category, name FROM clip_groups WHERE category=? ORDER BY sort_order ASC, id ASC",
        )?;
        let rows = stmt.query_map([category], |row| {
            Ok(ClipGroup {
                id: row.get(0)?,
                category: row.get(1)?,
                name: row.get(2)?,
            })
        })?;
        Ok(rows.filter_map(|row| row.ok()).collect())
    })
    .unwrap_or_default()
}

pub(super) fn db_delete_group(group_id: i64) -> rusqlite::Result<()> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        tx.execute("UPDATE items SET group_id=0 WHERE group_id=?", params![group_id])?;
        tx.execute("DELETE FROM clip_groups WHERE id=?", params![group_id])?;
        tx.commit()?;
        Ok(())
    })
}

pub(super) fn db_set_groups_order(category: i64, group_ids: &[i64]) -> rusqlite::Result<()> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        for (idx, gid) in group_ids.iter().enumerate() {
            tx.execute(
                "UPDATE clip_groups SET sort_order=? WHERE id=? AND category=?",
                params![idx as i64 + 1, *gid, category],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
}

pub(super) fn db_assign_group(item_ids: &[i64], group_id: i64) -> rusqlite::Result<()> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        for item_id in item_ids {
            tx.execute(
                "UPDATE items SET group_id=? WHERE id=?",
                params![group_id, item_id],
            )?;
        }
        tx.commit()?;
        Ok(())
    })
}

pub(super) fn db_rename_group(
    category: i64,
    group_id: i64,
    new_name: &str,
) -> rusqlite::Result<()> {
    with_db(|conn| {
        conn.execute(
            "UPDATE clip_groups SET name=? WHERE id=? AND category=?",
            params![new_name, group_id, category],
        )?;
        Ok(())
    })
}

pub(super) fn db_create_named_group(category: i64, name: &str) -> rusqlite::Result<ClipGroup> {
    with_db(|conn| {
        let next_sort: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM clip_groups WHERE category=?",
                [category],
                |row| row.get(0),
            )
            .unwrap_or(1);
        conn.execute(
            "INSERT INTO clip_groups(category, name, sort_order) VALUES(?, ?, ?)",
            params![category, name, next_sort],
        )?;
        Ok(ClipGroup {
            id: conn.last_insert_rowid(),
            category,
            name: name.to_string(),
        })
    })
}

pub(super) fn db_update_item_text(item_id: i64, new_text: &str) -> rusqlite::Result<()> {
    let preview: String = new_text.chars().take(120).collect();
    with_db(|conn| {
        conn.execute(
            "UPDATE items SET text_data=?, preview=? WHERE id=?",
            params![new_text, preview, item_id],
        )?;
        Ok(())
    })
}

pub(super) fn db_touch_item_created_at(item_id: i64) -> rusqlite::Result<()> {
    with_db(|conn| {
        conn.execute(
            "UPDATE items SET created_at=CURRENT_TIMESTAMP WHERE id=?",
            params![item_id],
        )?;
        Ok(())
    })
}

pub(super) fn db_add_phrase_from_item(item: &ClipItem) -> rusqlite::Result<i64> {
    let mut clone = item.clone();
    clone.id = 0;
    clone.file_paths = None;
    clone.image_bytes = None;
    clone.image_width = 0;
    clone.image_height = 0;
    clone.kind = ClipKind::Phrase;
    if clone.text.is_none() {
        clone.text = Some(clone.preview.clone());
    }
    db_insert_item(1, &clone)
}

pub(super) fn reload_state_from_db(state: &mut AppState) {
    ensure_db();
    state.clear_payload_cache();
    state.record_groups = db_load_groups(0);
    state.phrase_groups = db_load_groups(1);
    if state.settings.vv_source_tab > 1 {
        state.settings.vv_source_tab = 0;
        save_settings(&state.settings);
    }
    let vv_groups = state.groups_for_tab(state.settings.vv_source_tab);
    if state.settings.vv_group_id > 0
        && !vv_groups.iter().any(|group| group.id == state.settings.vv_group_id)
    {
        state.settings.vv_group_id = 0;
        save_settings(&state.settings);
    }
    for idx in 0..state.tab_group_filters.len() {
        let gid = state.tab_group_filters[idx];
        if gid > 0 && !state.groups_for_tab(idx).iter().any(|group| group.id == gid) {
            state.tab_group_filters[idx] = 0;
        }
    }
    if state.tab_index < state.tab_group_filters.len() {
        state.current_group_filter = state.tab_group_filters[state.tab_index];
    }
    state.clear_selection();
    state.scroll_y = 0;
    state.invalidate_all_queries();
    state.refilter();
}
