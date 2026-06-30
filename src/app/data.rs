use super::prelude::*;
use crate::platform::gdi as platform_gdi;

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

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct LanOriginMetadata {
    pub(super) message_id: String,
    pub(super) origin_device_id: String,
    pub(super) origin_seq: u64,
    pub(super) hash: String,
}

fn split_paths_blob(value: Option<String>) -> Option<Vec<String>> {
    let paths: Vec<String> = value
        .unwrap_or_default()
        .split('\n')
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.to_string())
        .collect();
    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
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

fn remove_stored_image_files(paths: Vec<String>) {
    if paths.is_empty() {
        return;
    }
    let root = data_dir().join("images");
    let root_canon = root.canonicalize().unwrap_or(root);
    for raw in paths {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let path = PathBuf::from(raw);
        let path_canon = path.canonicalize().unwrap_or(path);
        if path_canon.starts_with(&root_canon) {
            let _ = fs::remove_file(path_canon);
        }
    }
}

fn collect_image_paths_for_delete(
    conn: &rusqlite::Connection,
    sql: &str,
    params: &[&dyn rusqlite::ToSql],
) -> rusqlite::Result<Vec<String>> {
    let mut stmt = conn.prepare(sql)?;
    let rows = stmt.query_map(params, |row| row.get::<_, Option<String>>(0))?;
    Ok(rows
        .filter_map(|row| row.ok().flatten())
        .filter(|path| !path.trim().is_empty())
        .collect())
}

pub(super) fn db_cleanup_orphan_image_files() -> rusqlite::Result<usize> {
    let root = data_dir().join("images");
    if !root.is_dir() {
        return Ok(0);
    }
    let root_canon = root.canonicalize().unwrap_or(root);
    let referenced_paths = with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT image_path FROM items WHERE image_path IS NOT NULL AND TRIM(image_path)<>''",
        )?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        Ok(rows.filter_map(Result::ok).collect::<Vec<String>>())
    })?;

    let mut referenced = HashSet::<PathBuf>::new();
    for raw in referenced_paths {
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        let path = PathBuf::from(raw);
        let path_canon = path.canonicalize().unwrap_or(path);
        if path_canon.starts_with(&root_canon) {
            referenced.insert(path_canon);
        }
    }

    let mut removed = 0;
    if let Ok(entries) = fs::read_dir(&root_canon) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let path_canon = path.canonicalize().unwrap_or(path);
            if path_canon.starts_with(&root_canon)
                && !referenced.contains(&path_canon)
                && fs::remove_file(&path_canon).is_ok()
            {
                removed += 1;
            }
        }
    }
    Ok(removed)
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

pub(super) fn build_preview(text: &str) -> String {
    let one_line = text.replace(['\r', '\n'], " ").trim().to_string();
    if one_line.chars().count() > 72 {
        let mut s = String::new();
        for (idx, ch) in one_line.chars().enumerate() {
            if idx >= 72 {
                break;
            }
            s.push(ch);
        }
        s.push_str(" ...");
        s
    } else {
        one_line
    }
}

pub(super) fn build_files_preview(paths: &[String]) -> String {
    if paths.is_empty() {
        return String::new();
    }
    let names: Vec<String> = paths
        .iter()
        .map(|path| {
            let parsed = Path::new(path);
            parsed
                .file_name()
                .and_then(|value| value.to_str())
                .filter(|value| !value.trim().is_empty())
                .unwrap_or(path)
                .to_string()
        })
        .collect();
    match names.len() {
        0 => String::new(),
        1 => names[0].clone(),
        2 => format!("{} + {}", names[0], names[1]),
        _ => format!("{} + {} 等 {} 项", names[0], names[1], names.len()),
    }
}

pub(super) fn text_content_signature(text: &str) -> String {
    let normalized = normalize_captured_text(text);
    if normalized.is_empty() {
        String::new()
    } else {
        crc32_signature([b"text".as_slice(), normalized.as_bytes()])
    }
}

pub(super) fn image_content_signature(bytes: &[u8], width: usize, height: usize) -> String {
    if bytes.is_empty() || width == 0 || height == 0 {
        String::new()
    } else {
        let width = (width as u64).to_le_bytes();
        let height = (height as u64).to_le_bytes();
        crc32_signature([b"image".as_slice(), &width, &height, bytes])
    }
}

fn normalized_file_paths_for_signature(paths: &[String]) -> Vec<String> {
    let mut normalized: Vec<String> = paths
        .iter()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .map(|path| {
            let mut value = path.replace('/', "\\");
            while value.ends_with('\\') && value.len() > 3 {
                value.pop();
            }
            value.to_lowercase()
        })
        .collect();
    normalized.sort_unstable();
    normalized.dedup();
    normalized
}

pub(super) fn file_paths_signature(paths: &[String]) -> String {
    let normalized = normalized_file_paths_for_signature(paths);
    if normalized.is_empty() {
        String::new()
    } else {
        let joined = normalized.join("\n");
        crc32_signature([b"files".as_slice(), joined.as_bytes()])
    }
}

fn crc32_signature<const N: usize>(parts: [&[u8]; N]) -> String {
    let mut hasher = crc32fast::Hasher::new();
    for part in parts {
        hasher.update(&(part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    format!("crc:{:08x}", hasher.finalize())
}

pub(super) fn dedupe_signature_for_item(item: &ClipItem, fallback_signature: &str) -> String {
    let computed = match item.kind {
        ClipKind::Text | ClipKind::Phrase => item
            .text
            .as_deref()
            .map(text_content_signature)
            .unwrap_or_default(),
        ClipKind::Files => item
            .file_paths
            .as_deref()
            .map(|paths| {
                let fallback = fallback_signature.trim();
                if item.source_app.starts_with("LAN:")
                    && fallback.len() == 12
                    && fallback.starts_with("crc:")
                    && fallback[4..].chars().all(|ch| ch.is_ascii_hexdigit())
                {
                    fallback.to_string()
                } else {
                    file_paths_signature(paths)
                }
            })
            .or_else(|| item.text.as_deref().map(text_content_signature))
            .unwrap_or_default(),
        ClipKind::Image => {
            if let Some(bytes) = item.image_bytes.as_ref() {
                image_content_signature(bytes, item.image_width, item.image_height)
            } else if let Some(path) = item.image_path.as_deref() {
                load_image_bytes_from_path(path)
                    .map(|(bytes, width, height)| image_content_signature(&bytes, width, height))
                    .unwrap_or_default()
            } else {
                String::new()
            }
        }
    };
    if computed.is_empty() {
        fallback_signature.trim().to_string()
    } else {
        computed
    }
}

pub(super) fn build_qr_clip_item(text: &str) -> Option<(ClipItem, String)> {
    use qrcodegen::{QrCode, QrCodeEcc};

    let text = text.trim();
    if text.is_empty() {
        return None;
    }
    let qr = QrCode::encode_text(text, QrCodeEcc::Medium).ok()?;
    let module_count = qr.size().max(1) as usize;
    let border = 4usize;
    let total_modules = module_count + border * 2;
    let scale = (512usize / total_modules).clamp(4, 16);
    let side = total_modules * scale;
    let mut bytes = vec![255u8; side * side * 4];
    for y in 0..side {
        for x in 0..side {
            let mx = x / scale;
            let my = y / scale;
            let dark = mx >= border
                && my >= border
                && mx < border + module_count
                && my < border + module_count
                && qr.get_module((mx - border) as i32, (my - border) as i32);
            if dark {
                let idx = (y * side + x) * 4;
                bytes[idx] = 0;
                bytes[idx + 1] = 0;
                bytes[idx + 2] = 0;
                bytes[idx + 3] = 255;
            }
        }
    }
    let preview_text: String = text.chars().take(32).collect();
    let sig = image_content_signature(&bytes, side, side);
    Some((
        ClipItem {
            id: 0,
            kind: ClipKind::Image,
            preview: format!("{} {}", tr("二维码", "QR"), preview_text),
            text: None,
            source_app: String::new(),
            file_paths: None,
            image_bytes: Some(bytes),
            image_path: None,
            image_width: side,
            image_height: side,
            pinned: false,
            group_id: 0,
            created_at: now_utc_sqlite(),
        },
        sig,
    ))
}

pub(super) fn output_image_path() -> PathBuf {
    let base = data_dir().join("images");
    let _ = fs::create_dir_all(&base);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    base.join(format!("zsclip_{}.png", ts))
}

pub(super) fn write_image_bytes_to_path(
    out: &Path,
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Option<PathBuf> {
    use std::fs::File;
    use std::io::BufWriter;

    let file = File::create(out).ok()?;
    let writer = BufWriter::new(file);
    let mut encoder = png::Encoder::new(writer, width, height);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    let mut png_writer = encoder.write_header().ok()?;
    png_writer.write_image_data(bytes).ok()?;
    Some(out.to_path_buf())
}

pub(super) fn write_image_bytes_to_output_path(
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Option<PathBuf> {
    let out = output_image_path();
    write_image_bytes_to_path(&out, bytes, width, height)
}

pub(super) fn load_image_bytes_from_path(path: &str) -> Option<(Vec<u8>, usize, usize)> {
    let bytes = fs::read(path).ok()?;
    let image = image::load_from_memory(&bytes).ok()?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some((rgba.into_raw(), width as usize, height as usize))
}

pub(super) fn save_image_item(item: &ClipItem) -> Option<PathBuf> {
    if let Some(path) = item.image_path.as_ref() {
        let src = PathBuf::from(path);
        if src.exists() {
            return Some(src);
        }
    }
    let (bytes, width, height) = ensure_item_image_bytes(item)?;
    write_image_bytes_to_output_path(&bytes, width as u32, height as u32)
}

pub(crate) fn ensure_item_image_bytes(item: &ClipItem) -> Option<(Vec<u8>, usize, usize)> {
    if let Some(bytes) = &item.image_bytes {
        return Some((bytes.clone(), item.image_width, item.image_height));
    }
    if let Some(path) = item.image_path.as_ref() {
        if let Some(loaded) = load_image_bytes_from_path(path) {
            return Some(loaded);
        }
    }
    if item.kind != ClipKind::Image || item.id <= 0 {
        return None;
    }
    let full = db_load_item_full(item.id)?;
    if let Some(bytes) = full.image_bytes {
        return Some((bytes, full.image_width, full.image_height));
    }
    full.image_path
        .as_deref()
        .and_then(load_image_bytes_from_path)
}

fn build_image_thumbnail_rgba(
    bytes: &[u8],
    width: usize,
    height: usize,
    max_side: usize,
) -> Option<ImageThumbnail> {
    if bytes.len() < 4 || width == 0 || height == 0 || max_side == 0 {
        return None;
    }
    if width <= max_side && height <= max_side {
        return Some(ImageThumbnail {
            bytes: bytes.to_vec(),
            width,
            height,
        });
    }
    let scale = (max_side as f32 / width as f32).min(max_side as f32 / height as f32);
    let out_w = ((width as f32 * scale).round() as usize).max(1);
    let out_h = ((height as f32 * scale).round() as usize).max(1);
    let mut out = vec![0u8; out_w * out_h * 4];
    for y in 0..out_h {
        let src_y = y * height / out_h;
        for x in 0..out_w {
            let src_x = x * width / out_w;
            let src_idx = (src_y * width + src_x) * 4;
            let dst_idx = (y * out_w + x) * 4;
            out[dst_idx..dst_idx + 4].copy_from_slice(&bytes[src_idx..src_idx + 4]);
        }
    }
    Some(ImageThumbnail {
        bytes: out,
        width: out_w,
        height: out_h,
    })
}

fn spawn_image_thumbnail_load(hwnd: HWND, item_id: i64, path: String, max_side: usize) {
    let hwnd_raw = hwnd as isize;
    std::thread::spawn(move || {
        let image = load_image_bytes_from_path(&path).and_then(|(bytes, width, height)| {
            build_image_thumbnail_rgba(&bytes, width, height, max_side)
        });
        let payload = Box::new(ImageThumbReadyResult { item_id, image });
        unsafe {
            let _ = post_boxed_message(hwnd_raw, WM_IMAGE_THUMB_READY, 0, payload);
        }
    });
}

pub(super) fn ensure_item_thumbnail_bytes(
    state: &mut AppState,
    item: &ClipItem,
    max_side: usize,
) -> Option<(Vec<u8>, usize, usize)> {
    if item.id > 0 {
        if let Some(image) = state.image_thumb_cache.get(item.id) {
            return Some((image.bytes, image.width, image.height));
        }
    }
    let (bytes, width, height) = if let Some(bytes) = &item.image_bytes {
        (bytes.clone(), item.image_width, item.image_height)
    } else if let Some(path) = item.image_path.as_ref() {
        if item.id > 0 && state.image_thumb_loading.insert(item.id) {
            spawn_image_thumbnail_load(state.hwnd, item.id, path.clone(), max_side);
        }
        return None;
    } else {
        return None;
    };
    let thumb = build_image_thumbnail_rgba(&bytes, width, height, max_side)?;
    if item.id > 0 {
        state.image_thumb_cache.put(item.id, thumb.clone());
    }
    Some((thumb.bytes, thumb.width, thumb.height))
}

fn current_search_date_context() -> SearchDateContext {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let (year, month, day, _, _, _) = utc_secs_to_local_parts(now_secs);
    SearchDateContext::from_date(year, month, day)
}

pub(super) fn db_load_items_page(
    query: &ItemsQuery,
    cursor: Option<ItemsCursor>,
    limit: usize,
) -> rusqlite::Result<(Vec<ClipItem>, Option<ItemsCursor>, bool)> {
    let date_context = current_search_date_context();
    let (search_terms, time_filter, app_filter, near_query) =
        parse_search_query_with_context(query.search_text.trim(), date_context);
    with_db(|conn| {
        let select_columns = "id, kind, preview, text_data, COALESCE(source_app, '') as source_app, file_paths, image_path, image_width, image_height, pinned, group_id, COALESCE(created_at, '') as created_at";
        let mut sql = if near_query.is_some() {
            format!(
                "WITH base AS (SELECT {select_columns}, ROW_NUMBER() OVER (ORDER BY pinned DESC, id DESC) AS rn FROM items WHERE category=?"
            )
        } else {
            format!("SELECT {select_columns} FROM items WHERE category=?")
        };
        let mut bind_values = vec![SqlValue::from(query.category)];

        if query.group_id > 0 {
            sql.push_str(" AND group_id=?");
            bind_values.push(SqlValue::from(query.group_id));
        }

        let kind_values = query.kind_filter.db_kinds(query.category);
        if !kind_values.is_empty() {
            sql.push_str(" AND kind IN (");
            for index in 0..kind_values.len() {
                if index > 0 {
                    sql.push(',');
                }
                sql.push('?');
            }
            sql.push(')');
            for kind in kind_values {
                bind_values.push(SqlValue::from((*kind).to_string()));
            }
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
                let end_day = date_context.current_day;
                let start_day = end_day - (days.max(1) - 1);
                sql.push_str(
                    " AND date(created_at, 'localtime') >= ? AND date(created_at, 'localtime') <= ?",
                );
                bind_values.push(SqlValue::from(days_to_sqlite_date(start_day)));
                bind_values.push(SqlValue::from(days_to_sqlite_date(end_day)));
            }
            None => {}
        }

        if let Some(near_value) = near_query {
            let like = format!("%{}%", near_value);
            sql.push_str(
                "), hits AS (SELECT rn FROM base WHERE LOWER(preview) LIKE ? \
                 OR LOWER(COALESCE(source_app, '')) LIKE ? \
                 OR LOWER(COALESCE(file_paths, text_data, '')) LIKE ? \
                 OR LOWER(COALESCE(strftime('%m-%d %H:%M', datetime(created_at, 'localtime')), '')) LIKE ?), \
                 near_rows AS (SELECT DISTINCT base.rn FROM base JOIN hits ON base.rn BETWEEN hits.rn - 3 AND hits.rn + 3) \
                 SELECT id, kind, preview, text_data, source_app, file_paths, image_path, image_width, image_height, pinned, group_id, created_at \
                 FROM base WHERE rn IN (SELECT rn FROM near_rows)",
            );
            bind_values.push(SqlValue::from(like.clone()));
            bind_values.push(SqlValue::from(like.clone()));
            bind_values.push(SqlValue::from(like.clone()));
            bind_values.push(SqlValue::from(like));
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
        kind_filter: ClipKindFilter::All,
        near_query: None,
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

        if platform_window::is_window_alive(hwnd_value) {
            if let Ok(mut queue) = page_load_results().lock() {
                queue.push_back(result);
            }
            platform_window::post_message(hwnd_value, WM_ITEMS_PAGE_READY, 0, 0);
        }
    });
}

pub(super) fn spawn_startup_data_reconcile(hwnd: HWND, keep_duplicates: bool) {
    let hwnd_value = hwnd as isize;
    std::thread::spawn(move || {
        let deleted = db_reconcile_dedupe_signatures(0, keep_duplicates).unwrap_or(0);
        if platform_window::is_window_alive(hwnd_value) {
            platform_window::post_message(hwnd_value, WM_STARTUP_DATA_RECONCILED, deleted, 0);
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
        platform_gdi::invalidate_rect(hwnd, null(), 1);
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

pub(super) fn db_load_latest_item_with_signature(category: i64) -> Option<(ClipItem, String)> {
    with_db(|conn| {
        conn.query_row(
            "SELECT id, kind, preview, text_data, COALESCE(source_app, '') as source_app, file_paths, image_data, image_width, image_height, pinned, group_id, image_path, \
             COALESCE(created_at, '') as created_at, COALESCE(signature, '') as signature \
             FROM items WHERE category=? ORDER BY id DESC LIMIT 1",
            params![category],
            |row| {
                let item = row_to_clip_item(DbItem {
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
                });
                Ok((item, row.get::<_, String>(13)?))
            },
        )
    })
    .ok()
}

pub(super) fn db_latest_item_id(category: i64) -> Option<i64> {
    with_db(|conn| {
        conn.query_row(
            "SELECT id FROM items WHERE category=? ORDER BY id DESC LIMIT 1",
            params![category],
            |row| row.get::<_, i64>(0),
        )
    })
    .ok()
}

pub(super) fn db_load_lan_origin_metadata(item_id: i64) -> Option<LanOriginMetadata> {
    if item_id <= 0 {
        return None;
    }
    with_db(|conn| {
        conn.query_row(
            "SELECT COALESCE(lan_origin_message_id, ''), COALESCE(lan_origin_device_id, ''), \
             COALESCE(lan_origin_seq, 0), COALESCE(lan_origin_hash, '') \
             FROM items WHERE id=?",
            params![item_id],
            |row| {
                let origin_seq_i64: i64 = row.get(2)?;
                Ok(LanOriginMetadata {
                    message_id: row.get(0)?,
                    origin_device_id: row.get(1)?,
                    origin_seq: origin_seq_i64.max(0) as u64,
                    hash: row.get(3)?,
                })
            },
        )
    })
    .ok()
    .filter(|meta| {
        !meta.message_id.trim().is_empty()
            && !meta.origin_device_id.trim().is_empty()
            && meta.origin_seq > 0
            && !meta.hash.trim().is_empty()
    })
}

pub(super) fn db_save_lan_origin_metadata(
    item_id: i64,
    metadata: &LanOriginMetadata,
) -> rusqlite::Result<()> {
    if item_id <= 0 {
        return Ok(());
    }
    with_db_mut(|conn| {
        conn.execute(
            "UPDATE items SET lan_origin_message_id=?, lan_origin_device_id=?, lan_origin_seq=?, lan_origin_hash=? WHERE id=?",
            params![
                metadata.message_id.trim(),
                metadata.origin_device_id.trim(),
                metadata.origin_seq as i64,
                metadata.hash.trim(),
                item_id,
            ],
        )?;
        Ok(())
    })
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

pub(super) fn db_item_is_pinned(item_id: i64) -> bool {
    with_db(|conn| {
        conn.query_row(
            "SELECT pinned FROM items WHERE id=?",
            params![item_id],
            |row| row.get::<_, i64>(0),
        )
    })
    .map(|value| value == 1)
    .unwrap_or(false)
}

pub(super) fn db_latest_item_signature(category: i64) -> Option<String> {
    with_db(|conn| {
        conn.query_row(
            "SELECT COALESCE(signature, '') FROM items WHERE category=? ORDER BY id DESC LIMIT 1",
            params![category],
            |row| row.get::<_, String>(0),
        )
    })
    .ok()
    .map(|value| value.trim().to_string())
    .filter(|value| !value.is_empty())
}

pub(super) fn db_reconcile_dedupe_signatures(
    category: i64,
    keep_duplicates: bool,
) -> rusqlite::Result<usize> {
    let rows = with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, kind, preview, \
             CASE WHEN COALESCE(signature, '')='' THEN text_data ELSE NULL END as text_data, \
             COALESCE(source_app, '') as source_app, \
             CASE WHEN COALESCE(signature, '')='' THEN file_paths ELSE NULL END as file_paths, \
             CASE WHEN COALESCE(signature, '')='' THEN image_data ELSE NULL END as image_data, \
             CASE WHEN COALESCE(signature, '')='' THEN image_path ELSE NULL END as image_path, \
             image_width, image_height, pinned, group_id, \
             COALESCE(created_at, '') as created_at, COALESCE(signature, '') as signature \
             FROM items WHERE category=? ORDER BY id DESC",
        )?;
        let mapped = stmt.query_map(params![category], |row| {
            let item = row_to_clip_item(DbItem {
                id: row.get(0)?,
                kind: row.get(1)?,
                preview: row.get(2)?,
                text: row.get(3)?,
                source_app: row.get(4)?,
                file_paths: row.get(5)?,
                image_bytes: row.get(6)?,
                image_path: row.get(7)?,
                image_width: row.get(8)?,
                image_height: row.get(9)?,
                pinned: row.get(10)?,
                group_id: row.get(11)?,
                created_at: row.get(12)?,
            });
            Ok((item.id, item.pinned, row.get::<_, String>(13)?, item))
        })?;
        Ok(mapped.filter_map(Result::ok).collect::<Vec<_>>())
    })?;

    let mut updates = Vec::<(i64, String)>::new();
    let mut groups = HashMap::<String, (Vec<i64>, Vec<i64>)>::new();
    for (id, pinned, stored_signature, item) in rows {
        let signature = dedupe_signature_for_item(&item, &stored_signature);
        if signature.trim().is_empty() {
            continue;
        }
        let stored = stored_signature.trim();
        if stored != signature {
            updates.push((id, signature.clone()));
        }
        let entry = groups.entry(signature).or_default();
        if pinned {
            entry.0.push(id);
        } else {
            entry.1.push(id);
        }
    }

    let mut delete_ids = Vec::<i64>::new();
    if !keep_duplicates {
        for (_signature, (pinned_ids, nonpinned_ids)) in groups {
            if !pinned_ids.is_empty() {
                delete_ids.extend(nonpinned_ids);
            } else if nonpinned_ids.len() > 1 {
                delete_ids.extend(nonpinned_ids.into_iter().skip(1));
            }
        }
    }

    let mut deleted_image_paths = Vec::<String>::new();
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        for (id, signature) in &updates {
            tx.execute(
                "UPDATE items SET signature=? WHERE id=?",
                params![signature, id],
            )?;
        }
        for id in &delete_ids {
            let mut stmt = tx.prepare(
                "SELECT image_path FROM items WHERE id=? AND image_path IS NOT NULL AND TRIM(image_path)<>''",
            )?;
            let rows = stmt.query_map(params![id], |row| row.get::<_, Option<String>>(0))?;
            deleted_image_paths.extend(rows.filter_map(|row| row.ok().flatten()));
            tx.execute("DELETE FROM items WHERE id=?", params![id])?;
        }
        tx.commit()?;
        Ok(())
    })?;
    remove_stored_image_files(deleted_image_paths);
    let _ = db_cleanup_orphan_image_files();
    Ok(delete_ids.len())
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
        let _ = db_cleanup_orphan_image_files();
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
            let paths = collect_image_paths_for_delete(
                conn,
                "SELECT image_path FROM items WHERE category=? AND pinned=0 ORDER BY id ASC LIMIT ?",
                &[&category, &excess],
            )?;
            conn.execute(
                "DELETE FROM items WHERE id IN (SELECT id FROM items WHERE category=? AND pinned=0 ORDER BY id ASC LIMIT ?)",
                params![category, excess],
            )?;
            remove_stored_image_files(paths);
        }
        Ok(())
    });
    let _ = db_cleanup_orphan_image_files();
}

pub(super) fn db_delete_item(id: i64) -> rusqlite::Result<()> {
    with_db(|conn| {
        let paths = collect_image_paths_for_delete(
            conn,
            "SELECT image_path FROM items WHERE id=?",
            &[&id],
        )?;
        conn.execute("DELETE FROM items WHERE id=?", params![id])?;
        remove_stored_image_files(paths);
        Ok(())
    })
}

pub(super) fn db_delete_unpinned_items(category: i64) -> rusqlite::Result<usize> {
    with_db(|conn| {
        let paths = collect_image_paths_for_delete(
            conn,
            "SELECT image_path FROM items WHERE category=? AND pinned=0",
            &[&category],
        )?;
        let affected = conn.execute(
            "DELETE FROM items WHERE category=? AND pinned=0",
            params![category],
        )?;
        remove_stored_image_files(paths);
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
        tx.execute(
            "UPDATE items SET group_id=0 WHERE group_id=?",
            params![group_id],
        )?;
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

pub(super) fn db_item_text(item_id: i64) -> rusqlite::Result<String> {
    with_db(|conn| {
        conn.query_row(
            "SELECT COALESCE(text_data,'') FROM items WHERE id=?",
            [item_id],
            |row| row.get(0),
        )
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
        && !vv_groups
            .iter()
            .any(|group| group.id == state.settings.vv_group_id)
    {
        state.settings.vv_group_id = 0;
        settings_changed = true;
    }
    for idx in 0..state.tab_group_filters.len() {
        let gid = state.tab_group_filters[idx];
        if gid > 0
            && !state
                .groups_for_tab(idx)
                .iter()
                .any(|group| group.id == gid)
        {
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

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_item(
        category: i64,
        kind: &str,
        preview: &str,
        group_id: i64,
        created_at: &str,
    ) -> rusqlite::Result<i64> {
        with_db_mut(|conn| {
            conn.execute(
                "INSERT INTO items(category, kind, preview, signature, text_data, source_app, group_id, created_at) VALUES(?, ?, ?, ?, ?, '', ?, ?)",
                rusqlite::params![
                    category,
                    kind,
                    preview,
                    format!("{kind}:{preview}:{created_at}"),
                    preview,
                    group_id,
                    created_at
                ],
            )?;
            Ok(conn.last_insert_rowid())
        })
    }

    #[test]
    fn db_load_items_page_filters_by_virtual_clip_kind() {
        crate::db_runtime::with_test_db(|| {
            insert_item(0, "text", "alpha", 0, "2026-07-01 10:00:00")?;
            let image_id = insert_item(0, "image", "beta image", 0, "2026-07-01 10:01:00")?;
            insert_item(0, "files", "gamma file", 0, "2026-07-01 10:02:00")?;

            let query = ItemsQuery {
                category: 0,
                group_id: 0,
                search_text: String::new(),
                kind_filter: ClipKindFilter::Image,
                near_query: None,
            };
            let (items, _, _) = db_load_items_page(&query, None, 20)?;
            assert_eq!(
                items.iter().map(|item| item.id).collect::<Vec<_>>(),
                vec![image_id]
            );
            assert_eq!(items[0].kind, ClipKind::Image);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn db_load_items_page_near_query_returns_time_neighbors() {
        crate::db_runtime::with_test_db(|| {
            let mut ids = Vec::new();
            for index in 0..8 {
                let label = if index == 4 {
                    "target invoice"
                } else {
                    "plain row"
                };
                ids.push(insert_item(
                    0,
                    "text",
                    &format!("{label} {index}"),
                    0,
                    &format!("2026-07-01 10:0{index}:00"),
                )?);
            }

            let query = ItemsQuery {
                category: 0,
                group_id: 0,
                search_text: "附近:invoice".to_string(),
                kind_filter: ClipKindFilter::All,
                near_query: Some("invoice".to_string()),
            };
            let (items, _, _) = db_load_items_page(&query, None, 20)?;
            let returned = items.iter().map(|item| item.id).collect::<Vec<_>>();
            assert_eq!(
                returned,
                ids[1..=7].iter().rev().copied().collect::<Vec<_>>()
            );
            Ok(())
        })
        .unwrap();
    }
}
