use super::*;

struct DbItem {
    id: i64,
    kind: String,
    preview: String,
    text: Option<String>,
    source_app: String,
    file_paths: Option<String>,
    image_bytes: Option<Vec<u8>>,
    image_path: Option<String>,
    image_width: i64,
    image_height: i64,
    pinned: i64,
    group_id: i64,
    created_at: String,
}

fn split_paths_blob(value: Option<String>) -> Option<Vec<String>> {
    let paths: Vec<String> = value
        .unwrap_or_default()
        .split('\n')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect();
    if paths.is_empty() { None } else { Some(paths) }
}

fn row_to_clip_item_impl(row: DbItem, summary_only: bool) -> ClipItem {
    let kind = match row.kind.as_str() {
        "image" => ClipKind::Image,
        "phrase" => ClipKind::Phrase,
        "files" => ClipKind::Files,
        _ => ClipKind::Text,
    };
    let file_paths = if matches!(kind, ClipKind::Files) {
        split_paths_blob(row.file_paths.or(row.text.clone()))
    } else {
        split_paths_blob(row.file_paths)
    };
    let preview = if matches!(kind, ClipKind::Files) {
        file_paths
            .as_ref()
            .map(|paths| build_files_preview(paths))
            .filter(|value| !value.is_empty())
            .unwrap_or(row.preview)
    } else {
        row.preview
    };
    ClipItem {
        id: row.id,
        kind,
        preview,
        text: if summary_only { None } else { row.text },
        source_app: row.source_app,
        file_paths,
        image_bytes: if summary_only { None } else { row.image_bytes },
        image_path: row.image_path,
        image_width: row.image_width.max(0) as usize,
        image_height: row.image_height.max(0) as usize,
        pinned: row.pinned == 1,
        group_id: row.group_id,
        created_at: row.created_at,
    }
}

fn row_to_clip_item(row: DbItem) -> ClipItem {
    row_to_clip_item_impl(row, false)
}

fn row_to_clip_item_summary(row: DbItem) -> ClipItem {
    row_to_clip_item_impl(row, true)
}

pub(super) fn clip_item_to_summary(item: &ClipItem) -> ClipItem {
    let file_paths = if matches!(item.kind, ClipKind::Files) {
        item.file_paths.clone()
    } else {
        None
    };
    let preview = if matches!(item.kind, ClipKind::Files) {
        file_paths
            .as_ref()
            .map(|paths| build_files_preview(paths))
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| item.preview.clone())
    } else {
        item.preview.clone()
    };
    ClipItem {
        id: item.id,
        kind: item.kind,
        preview,
        text: None,
        source_app: item.source_app.clone(),
        file_paths,
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
        .unwrap_or(0);
    let (year, month, day, _, _, _) = utc_secs_to_local_parts(now_secs);
    gregorian_to_days(year, month, day)
}

pub(super) fn db_load_items_page(
    query: &ItemsQuery,
    cursor: Option<ItemsCursor>,
    limit: usize,
) -> rusqlite::Result<(Vec<ClipItem>, Option<ItemsCursor>, bool)> {
    let (search_terms, time_filter, app_filter) = parse_search_query(query.search_text.trim());
    with_db(|conn| {
        let mut sql = String::from(
            "SELECT id, kind, preview, text_data, COALESCE(source_app, '') as source_app, file_paths, image_path, image_width, image_height, pinned, group_id, \
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
                 OR LOWER(COALESCE(source_app, '')) LIKE ? \
                 OR LOWER(COALESCE(file_paths, text_data, '')) LIKE ? \
                 OR LOWER(COALESCE(strftime('%m-%d %H:%M', datetime(created_at, 'localtime')), '')) LIKE ?)",
            );
            bind_values.push(SqlValue::from(like.clone()));
            bind_values.push(SqlValue::from(like.clone()));
            bind_values.push(SqlValue::from(like.clone()));
            bind_values.push(SqlValue::from(like));
        }

        if let Some(app_value) = app_filter {
            sql.push_str(" AND LOWER(COALESCE(source_app, '')) LIKE ?");
            bind_values.push(SqlValue::from(format!("%{}%", app_value)));
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
                text: row.get(3)?,
                source_app: row.get(4)?,
                file_paths: row.get(5)?,
                image_path: row.get(6)?,
                image_bytes: None,
                image_width: row.get(7)?,
                image_height: row.get(8)?,
                pinned: row.get(9)?,
                group_id: row.get(10)?,
                created_at: row.get(11)?,
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
            let still_alive = hwnd_value != 0 && IsWindow(hwnd_value as HWND) != 0;
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
            "SELECT id, kind, preview, text_data, COALESCE(source_app, '') as source_app, file_paths, image_data, image_width, image_height, pinned, group_id, image_path, \
             COALESCE(created_at, '') as created_at \
             FROM items WHERE id=?",
            params![id],
            |row| {
                Ok(row_to_clip_item(DbItem {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    preview: row.get(2)?,
                    text: row.get(3)?,
                    source_app: row.get(4)?,
                    file_paths: row.get(5)?,
                    image_bytes: row.get(6)?,
                    image_path: row.get(11)?,
                    image_width: row.get(7)?,
                    image_height: row.get(8)?,
                    pinned: row.get(9)?,
                    group_id: row.get(10)?,
                    created_at: row.get(12)?,
                }))
            },
        )
    })
    .ok()
}

pub(super) fn db_insert_item(
    category: i64,
    item: &ClipItem,
    signature: Option<&str>,
) -> rusqlite::Result<i64> {
    let kind = match item.kind {
        ClipKind::Image => "image",
        ClipKind::Phrase => "phrase",
        ClipKind::Files => "files",
        _ => "text",
    };
    let preview = item.preview.clone();
    let signature = signature.unwrap_or_default().trim().to_string();
    let text_data = item.text.clone();
    let source_app = item.source_app.clone();
    let file_paths = item.file_paths.as_ref().map(|paths| paths.join("\n"));
    let image_data = item.image_bytes.clone();
    let image_path = item.image_path.clone();
    with_db(|conn| {
        conn.execute(
            "INSERT INTO items(category, kind, preview, signature, text_data, source_app, file_paths, image_data, image_path, image_width, image_height, pinned, group_id)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                category,
                kind,
                preview,
                signature,
                text_data,
                source_app,
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

pub(super) fn db_find_duplicate_item_ids(
    category: i64,
    item: &ClipItem,
    signature: &str,
) -> Vec<i64> {
    if !signature.trim().is_empty() {
        let found = with_db(|conn| {
            let mut stmt = conn.prepare(
                "SELECT id FROM items WHERE category=? AND signature=? ORDER BY pinned DESC, id DESC",
            )?;
            let rows = stmt.query_map(params![category, signature], |row| row.get::<_, i64>(0))?;
            Ok(rows.filter_map(|row| row.ok()).collect::<Vec<i64>>())
        })
        .unwrap_or_default();
        if !found.is_empty() {
            return found;
        }
    }

    match item.kind {
        ClipKind::Text | ClipKind::Phrase => item
            .text
            .as_ref()
            .map(|text| {
                with_db(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT id FROM items WHERE category=? AND kind IN ('text','phrase') AND COALESCE(text_data, '')=? ORDER BY pinned DESC, id DESC",
                    )?;
                    let rows =
                        stmt.query_map(params![category, text], |row| row.get::<_, i64>(0))?;
                    Ok(rows.filter_map(|row| row.ok()).collect::<Vec<i64>>())
                })
                .unwrap_or_default()
            })
            .unwrap_or_default(),
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .map(|paths| {
                let joined = paths.join("\n");
                with_db(|conn| {
                    let mut stmt = conn.prepare(
                        "SELECT id FROM items WHERE category=? AND kind='files' AND COALESCE(file_paths, '')=? ORDER BY pinned DESC, id DESC",
                    )?;
                    let rows =
                        stmt.query_map(params![category, joined], |row| row.get::<_, i64>(0))?;
                    Ok(rows.filter_map(|row| row.ok()).collect::<Vec<i64>>())
                })
                .unwrap_or_default()
            })
            .unwrap_or_default(),
        ClipKind::Image => Vec::new(),
    }
}

pub(super) fn db_promote_item_to_top(item_id: i64) -> rusqlite::Result<i64> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        let (
            category,
            kind,
            preview,
            signature,
            text_data,
            source_app,
            file_paths,
            image_data,
            image_path,
            image_width,
            image_height,
            pinned,
            group_id,
            created_at,
        ) = tx.query_row(
            "SELECT category, kind, preview, COALESCE(signature, ''), text_data, COALESCE(source_app, ''), file_paths, image_data, image_path, image_width, image_height, pinned, group_id, COALESCE(created_at, CURRENT_TIMESTAMP)
             FROM items WHERE id=?",
            params![item_id],
            |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, Option<String>>(4)?,
                    row.get::<_, String>(5)?,
                    row.get::<_, Option<String>>(6)?,
                    row.get::<_, Option<Vec<u8>>>(7)?,
                    row.get::<_, Option<String>>(8)?,
                    row.get::<_, i64>(9)?,
                    row.get::<_, i64>(10)?,
                    row.get::<_, i64>(11)?,
                    row.get::<_, i64>(12)?,
                    row.get::<_, String>(13)?,
                ))
            },
        )?;

        tx.execute(
            "INSERT INTO items(category, kind, preview, signature, text_data, source_app, file_paths, image_data, image_path, image_width, image_height, pinned, group_id, created_at)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                category,
                kind,
                preview,
                signature,
                text_data,
                source_app,
                file_paths,
                image_data,
                image_path,
                image_width,
                image_height,
                pinned,
                group_id,
                created_at,
            ],
        )?;
        let new_id = tx.last_insert_rowid();
        tx.execute("DELETE FROM items WHERE id=?", params![item_id])?;
        tx.commit()?;
        Ok(new_id)
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

pub(super) fn db_prune_items(category: i64, max_items: usize) {
    if max_items == 0 {
        return;
    }
    let _ = with_db(|conn| {
        let count: i64 = conn
            .query_row(
                "SELECT COUNT(*) FROM items WHERE category=? AND pinned=0",
                params![category],
                |row| row.get::<_, i64>(0),
            )
            .unwrap_or(0);
        let excess = count - max_items as i64;
        if excess > 0 {
            conn.execute(
                "DELETE FROM items WHERE id IN (SELECT id FROM items WHERE category=? AND pinned=0 ORDER BY id ASC LIMIT ?)",
                params![category, excess],
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

pub(super) fn db_delete_unpinned_items(category: i64) -> rusqlite::Result<usize> {
    with_db(|conn| {
        let affected = conn.execute(
            "DELETE FROM items WHERE category=? AND pinned=0",
            params![category],
        )?;
        Ok(affected)
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
    db_insert_item(1, &clone, None)
}

pub(super) fn reload_state_from_db(state: &mut AppState) -> bool {
    ensure_db();
    let mut settings_changed = false;
    state.clear_payload_cache();
    state.record_groups = db_load_groups(0);
    state.phrase_groups = db_load_groups(1);
    if state.settings.vv_source_tab > 1 {
        state.settings.vv_source_tab = 0;
        settings_changed = true;
    }
    let vv_groups = state.groups_for_tab(state.settings.vv_source_tab);
    if state.settings.vv_group_id > 0
        && !vv_groups.iter().any(|group| group.id == state.settings.vv_group_id)
    {
        state.settings.vv_group_id = 0;
        settings_changed = true;
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
    settings_changed
}
