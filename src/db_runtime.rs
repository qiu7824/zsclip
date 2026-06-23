#![cfg_attr(windows, allow(dead_code))]

use std::cell::RefCell;
use std::sync::OnceLock;
use std::time::Duration;

use rusqlite::{Connection, OptionalExtension};

thread_local! {
    static DB_CONN: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

static DB_MIGRATED: OnceLock<()> = OnceLock::new();

fn table_has_column(conn: &Connection, table: &str, column: &str) -> rusqlite::Result<bool> {
    let table = validate_schema_table(table)?;
    let exists = conn
        .query_row(
            "SELECT name FROM pragma_table_info(?) WHERE lower(name)=lower(?) LIMIT 1",
            [table, column],
            |row| row.get::<_, String>(0),
        )
        .optional()?;
    Ok(exists.is_some())
}

fn ensure_table_column(
    conn: &Connection,
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<()> {
    if !table_has_column(conn, table, column)? {
        let table = validate_schema_table(table)?;
        let definition = validate_schema_column_definition(table, column, definition)?;
        conn.execute(&format!("ALTER TABLE {table} ADD COLUMN {definition}"), [])?;
    }
    Ok(())
}

fn validate_schema_table(table: &str) -> rusqlite::Result<&'static str> {
    match table {
        "items" => Ok("items"),
        "clip_groups" => Ok("clip_groups"),
        _ => Err(rusqlite::Error::InvalidParameterName(format!(
            "unsupported schema table: {table}"
        ))),
    }
}

fn validate_schema_column_definition(
    table: &str,
    column: &str,
    definition: &str,
) -> rusqlite::Result<&'static str> {
    match (table, column, definition) {
        ("items", "category", "category INTEGER NOT NULL DEFAULT 0") => {
            Ok("category INTEGER NOT NULL DEFAULT 0")
        }
        ("items", "kind", "kind TEXT NOT NULL DEFAULT 'text'") => {
            Ok("kind TEXT NOT NULL DEFAULT 'text'")
        }
        ("items", "preview", "preview TEXT NOT NULL DEFAULT ''") => {
            Ok("preview TEXT NOT NULL DEFAULT ''")
        }
        ("items", "signature", "signature TEXT NOT NULL DEFAULT ''") => {
            Ok("signature TEXT NOT NULL DEFAULT ''")
        }
        ("items", "text_data", "text_data TEXT") => Ok("text_data TEXT"),
        ("items", "source_app", "source_app TEXT NOT NULL DEFAULT ''") => {
            Ok("source_app TEXT NOT NULL DEFAULT ''")
        }
        ("items", "file_paths", "file_paths TEXT") => Ok("file_paths TEXT"),
        ("items", "image_data", "image_data BLOB") => Ok("image_data BLOB"),
        ("items", "image_path", "image_path TEXT") => Ok("image_path TEXT"),
        ("items", "image_width", "image_width INTEGER NOT NULL DEFAULT 0") => {
            Ok("image_width INTEGER NOT NULL DEFAULT 0")
        }
        ("items", "image_height", "image_height INTEGER NOT NULL DEFAULT 0") => {
            Ok("image_height INTEGER NOT NULL DEFAULT 0")
        }
        ("items", "pinned", "pinned INTEGER NOT NULL DEFAULT 0") => {
            Ok("pinned INTEGER NOT NULL DEFAULT 0")
        }
        ("items", "group_id", "group_id INTEGER NOT NULL DEFAULT 0") => {
            Ok("group_id INTEGER NOT NULL DEFAULT 0")
        }
        ("items", "created_at", "created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP") => {
            Ok("created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP")
        }
        ("items", "lan_origin_message_id", "lan_origin_message_id TEXT") => {
            Ok("lan_origin_message_id TEXT")
        }
        ("items", "lan_origin_device_id", "lan_origin_device_id TEXT") => {
            Ok("lan_origin_device_id TEXT")
        }
        ("items", "lan_origin_seq", "lan_origin_seq INTEGER") => Ok("lan_origin_seq INTEGER"),
        ("items", "lan_origin_hash", "lan_origin_hash TEXT") => Ok("lan_origin_hash TEXT"),
        ("clip_groups", "category", "category INTEGER NOT NULL DEFAULT 0") => {
            Ok("category INTEGER NOT NULL DEFAULT 0")
        }
        _ => Err(rusqlite::Error::InvalidParameterName(format!(
            "unsupported schema definition: {table}.{column}"
        ))),
    }
}

fn migrate_clip_groups_schema(conn: &Connection) -> rusqlite::Result<()> {
    if !table_has_column(conn, "clip_groups", "category")? {
        conn.execute_batch(
            "
            ALTER TABLE clip_groups RENAME TO clip_groups_legacy;
            CREATE TABLE clip_groups(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category INTEGER NOT NULL DEFAULT 0,
                name TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            INSERT INTO clip_groups(id, category, name, sort_order, created_at)
            SELECT id, 0, name, sort_order, created_at FROM clip_groups_legacy;
            DROP TABLE clip_groups_legacy;
            ",
        )?;
    }

    conn.execute_batch(
        "
        CREATE INDEX IF NOT EXISTS idx_clip_groups_category_sort ON clip_groups(category, sort_order, id);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_clip_groups_category_name ON clip_groups(category, name);
        ",
    )?;
    Ok(())
}

fn migrate_items_schema(conn: &Connection) -> rusqlite::Result<()> {
    ensure_table_column(
        conn,
        "items",
        "category",
        "category INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_table_column(conn, "items", "kind", "kind TEXT NOT NULL DEFAULT 'text'")?;
    ensure_table_column(conn, "items", "preview", "preview TEXT NOT NULL DEFAULT ''")?;
    ensure_table_column(
        conn,
        "items",
        "signature",
        "signature TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_table_column(conn, "items", "text_data", "text_data TEXT")?;
    ensure_table_column(
        conn,
        "items",
        "source_app",
        "source_app TEXT NOT NULL DEFAULT ''",
    )?;
    ensure_table_column(conn, "items", "file_paths", "file_paths TEXT")?;
    ensure_table_column(conn, "items", "image_data", "image_data BLOB")?;
    ensure_table_column(conn, "items", "image_path", "image_path TEXT")?;
    ensure_table_column(
        conn,
        "items",
        "image_width",
        "image_width INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_table_column(
        conn,
        "items",
        "image_height",
        "image_height INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_table_column(conn, "items", "pinned", "pinned INTEGER NOT NULL DEFAULT 0")?;
    ensure_table_column(
        conn,
        "items",
        "group_id",
        "group_id INTEGER NOT NULL DEFAULT 0",
    )?;
    ensure_table_column(
        conn,
        "items",
        "created_at",
        "created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP",
    )?;
    ensure_table_column(
        conn,
        "items",
        "lan_origin_message_id",
        "lan_origin_message_id TEXT",
    )?;
    ensure_table_column(
        conn,
        "items",
        "lan_origin_device_id",
        "lan_origin_device_id TEXT",
    )?;
    ensure_table_column(conn, "items", "lan_origin_seq", "lan_origin_seq INTEGER")?;
    ensure_table_column(conn, "items", "lan_origin_hash", "lan_origin_hash TEXT")?;
    Ok(())
}

fn migrate_phrase_group_assignments(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare(
        "
        SELECT DISTINCT g.id, g.name, g.sort_order, g.created_at
        FROM items i
        JOIN clip_groups g ON g.id = i.group_id
        WHERE i.category = 1 AND i.group_id <> 0 AND g.category = 0
        ORDER BY g.sort_order ASC, g.id ASC
        ",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, i64>(2)?,
            row.get::<_, String>(3)?,
        ))
    })?;

    for row in rows {
        let (old_id, name, sort_order, created_at) = row?;
        let existing_id = conn
            .query_row(
                "SELECT id FROM clip_groups WHERE category=1 AND name=?",
                [&name],
                |r| r.get::<_, i64>(0),
            )
            .optional()?;
        let new_id = if let Some(id) = existing_id {
            id
        } else {
            conn.execute(
                "INSERT INTO clip_groups(category, name, sort_order, created_at) VALUES(1, ?, ?, ?)",
                (&name, sort_order, &created_at),
            )?;
            conn.last_insert_rowid()
        };
        conn.execute(
            "UPDATE items SET group_id=? WHERE category=1 AND group_id=?",
            (new_id, old_id),
        )?;
    }
    Ok(())
}

fn configure_db_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.busy_timeout(Duration::from_millis(5_000))?;
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "temp_store", "MEMORY");
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    let _ = conn.pragma_update(None, "cache_size", -8192i32);
    let _ = conn.pragma_update(None, "mmap_size", 134_217_728i64);
    Ok(())
}

fn migrate_db(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS items(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category INTEGER NOT NULL,
            kind TEXT NOT NULL,
            preview TEXT NOT NULL,
            signature TEXT NOT NULL DEFAULT '',
            text_data TEXT,
            source_app TEXT NOT NULL DEFAULT '',
            file_paths TEXT,
            image_data BLOB,
            image_path TEXT,
            image_width INTEGER NOT NULL DEFAULT 0,
            image_height INTEGER NOT NULL DEFAULT 0,
            pinned INTEGER NOT NULL DEFAULT 0,
            group_id INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE TABLE IF NOT EXISTS clip_groups(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category INTEGER NOT NULL DEFAULT 0,
            name TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_items_category_pinned_id ON items(category, pinned, id DESC);
        CREATE INDEX IF NOT EXISTS idx_items_group_id ON items(group_id, id DESC);
        CREATE INDEX IF NOT EXISTS idx_items_category_signature ON items(category, signature, id DESC);
        CREATE INDEX IF NOT EXISTS idx_clip_groups_category_sort ON clip_groups(category, sort_order, id);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_clip_groups_category_name ON clip_groups(category, name);
        ",
    )?;
    migrate_items_schema(conn)?;
    migrate_clip_groups_schema(conn)?;
    migrate_phrase_group_assignments(conn)?;
    Ok(())
}

fn runtime_db_file() -> std::path::PathBuf {
    #[cfg(target_os = "windows")]
    {
        crate::app::runtime::db_file()
    }

    #[cfg(not(target_os = "windows"))]
    {
        let data_dir = std::env::current_exe()
            .ok()
            .and_then(|path| path.parent().map(|dir| dir.join("data")))
            .unwrap_or_else(|| std::path::PathBuf::from("data"));
        data_dir.join("clipboard.db")
    }
}

fn ensure_connection(cell: &RefCell<Option<Connection>>) -> rusqlite::Result<()> {
    let mut slot = cell.borrow_mut();
    if slot.is_none() {
        let db_file = runtime_db_file();
        if let Some(parent) = db_file.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(db_file)?;
        configure_db_connection(&conn)?;
        *slot = Some(conn);
    }
    if DB_MIGRATED.get().is_none() {
        if let Some(conn) = slot.as_ref() {
            migrate_db(conn)?;
            let _ = DB_MIGRATED.set(());
        }
    }
    Ok(())
}

pub(crate) fn ensure_db() {
    let _ = DB_CONN.with(ensure_connection);
}

pub(crate) fn with_db<T, F>(f: F) -> rusqlite::Result<T>
where
    F: FnOnce(&Connection) -> rusqlite::Result<T>,
{
    DB_CONN.with(|cell| {
        ensure_connection(cell)?;
        let slot = cell.borrow();
        let conn = slot.as_ref().ok_or(rusqlite::Error::InvalidQuery)?;
        f(conn)
    })
}

pub(crate) fn with_db_mut<T, F>(f: F) -> rusqlite::Result<T>
where
    F: FnOnce(&mut Connection) -> rusqlite::Result<T>,
{
    DB_CONN.with(|cell| {
        ensure_connection(cell)?;
        let mut slot = cell.borrow_mut();
        let conn = slot.as_mut().ok_or(rusqlite::Error::InvalidQuery)?;
        f(conn)
    })
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
pub(crate) fn item_text(item_id: i64) -> rusqlite::Result<Option<String>> {
    with_db(|conn| {
        conn.query_row(
            "SELECT COALESCE(text_data,'') FROM items WHERE id=?",
            [item_id],
            |row| row.get(0),
        )
        .optional()
    })
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct NativeClipboardInsertOutcome {
    pub(crate) item_id: Option<i64>,
    pub(crate) inserted: bool,
    pub(crate) reason: &'static str,
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
pub(crate) fn insert_native_clipboard_text(
    category: i64,
    text: &str,
    source_app: &str,
) -> rusqlite::Result<NativeClipboardInsertOutcome> {
    let normalized = normalize_native_captured_text(text);
    if normalized.is_empty() {
        return Ok(NativeClipboardInsertOutcome {
            item_id: None,
            inserted: false,
            reason: "empty_text",
        });
    }
    let preview = native_clip_preview(&normalized);
    let signature = native_clip_signature("text", &normalized, &[], &[], 0, 0);
    insert_native_clipboard_item(NativeClipboardInsert {
        category,
        kind: "text",
        preview: &preview,
        signature: &signature,
        text_data: Some(&normalized),
        source_app,
        file_paths: None,
        image_data: None,
        image_width: 0,
        image_height: 0,
    })
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
pub(crate) fn insert_native_clipboard_file_paths(
    category: i64,
    paths: &[String],
    source_app: &str,
) -> rusqlite::Result<NativeClipboardInsertOutcome> {
    let paths = paths
        .iter()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    if paths.is_empty() {
        return Ok(NativeClipboardInsertOutcome {
            item_id: None,
            inserted: false,
            reason: "empty_files",
        });
    }
    let joined = paths.join("\n");
    let preview = native_clip_preview(paths.first().map(String::as_str).unwrap_or(""));
    let signature = native_clip_signature("files", "", &paths, &[], 0, 0);
    insert_native_clipboard_item(NativeClipboardInsert {
        category,
        kind: "files",
        preview: &preview,
        signature: &signature,
        text_data: Some(&joined),
        source_app,
        file_paths: Some(&joined),
        image_data: None,
        image_width: 0,
        image_height: 0,
    })
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
pub(crate) fn insert_native_clipboard_image(
    category: i64,
    bytes: &[u8],
    width: usize,
    height: usize,
    source_app: &str,
) -> rusqlite::Result<NativeClipboardInsertOutcome> {
    if width == 0 || height == 0 || bytes.is_empty() {
        return Ok(NativeClipboardInsertOutcome {
            item_id: None,
            inserted: false,
            reason: "empty_image",
        });
    }
    let preview = format!("{width} x {height}");
    let signature = native_clip_signature("image", "", &[], bytes, width, height);
    insert_native_clipboard_item(NativeClipboardInsert {
        category,
        kind: "image",
        preview: &preview,
        signature: &signature,
        text_data: None,
        source_app,
        file_paths: None,
        image_data: Some(bytes),
        image_width: width as i64,
        image_height: height as i64,
    })
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
struct NativeClipboardInsert<'a> {
    category: i64,
    kind: &'a str,
    preview: &'a str,
    signature: &'a str,
    text_data: Option<&'a str>,
    source_app: &'a str,
    file_paths: Option<&'a str>,
    image_data: Option<&'a [u8]>,
    image_width: i64,
    image_height: i64,
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn insert_native_clipboard_item(
    item: NativeClipboardInsert<'_>,
) -> rusqlite::Result<NativeClipboardInsertOutcome> {
    with_db_mut(|conn| {
        let duplicate = conn
            .query_row(
                "SELECT id FROM items WHERE category=? AND signature=? ORDER BY id DESC LIMIT 1",
                rusqlite::params![item.category, item.signature],
                |row| row.get::<_, i64>(0),
            )
            .optional()?;
        if let Some(item_id) = duplicate {
            return Ok(NativeClipboardInsertOutcome {
                item_id: Some(item_id),
                inserted: false,
                reason: "duplicate",
            });
        }

        conn.execute(
            "INSERT INTO items(category, kind, preview, signature, text_data, source_app, file_paths, image_data, image_width, image_height, pinned, group_id)
             VALUES(?, ?, ?, ?, ?, ?, ?, ?, ?, ?, 0, 0)",
            rusqlite::params![
                item.category,
                item.kind,
                item.preview,
                item.signature,
                item.text_data,
                item.source_app,
                item.file_paths,
                item.image_data,
                item.image_width,
                item.image_height,
            ],
        )?;
        Ok(NativeClipboardInsertOutcome {
            item_id: Some(conn.last_insert_rowid()),
            inserted: true,
            reason: "inserted",
        })
    })
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn normalize_native_captured_text(text: &str) -> String {
    text.replace("\r\n", "\n")
        .replace('\r', "\n")
        .trim_matches(|ch| ch == '\0')
        .trim()
        .to_string()
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn native_clip_preview(text: &str) -> String {
    text.chars().take(120).collect()
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
fn native_clip_signature(
    kind: &str,
    text: &str,
    paths: &[String],
    image: &[u8],
    width: usize,
    height: usize,
) -> String {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(kind.as_bytes());
    hasher.update(b"\0");
    hasher.update(text.as_bytes());
    for path in paths {
        hasher.update(b"\0path:");
        hasher.update(path.as_bytes());
    }
    if !image.is_empty() {
        hasher.update(b"\0image:");
        hasher.update(width.to_string().as_bytes());
        hasher.update(b"x");
        hasher.update(height.to_string().as_bytes());
        hasher.update(b":");
        hasher.update(image);
    }
    format!("native:{kind}:{:08x}", hasher.finalize())
}

#[cfg(any(target_os = "linux", target_os = "macos", test))]
pub(crate) fn update_item_text(item_id: i64, new_text: &str) -> rusqlite::Result<bool> {
    let preview: String = new_text.chars().take(120).collect();
    with_db_mut(|conn| {
        let affected = conn.execute(
            "UPDATE items SET text_data=?, preview=? WHERE id=?",
            rusqlite::params![new_text, preview, item_id],
        )?;
        Ok(affected > 0)
    })
}

fn normalized_native_item_ids(ids: &[i64]) -> Vec<i64> {
    let mut ids = ids.iter().copied().filter(|id| *id > 0).collect::<Vec<_>>();
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn sql_placeholders(count: usize) -> String {
    std::iter::repeat("?")
        .take(count)
        .collect::<Vec<_>>()
        .join(",")
}

pub(crate) fn update_native_clip_items_pinned(
    ids: &[i64],
    pinned: bool,
) -> rusqlite::Result<usize> {
    let ids = normalized_native_item_ids(ids);
    if ids.is_empty() {
        return Ok(0);
    }
    with_db_mut(|conn| {
        let sql = format!(
            "UPDATE items SET pinned=? WHERE id IN ({})",
            sql_placeholders(ids.len())
        );
        let params = std::iter::once(if pinned { 1_i64 } else { 0_i64 }).chain(ids);
        conn.execute(&sql, rusqlite::params_from_iter(params))
    })
}

pub(crate) fn delete_native_clip_items(ids: &[i64]) -> rusqlite::Result<usize> {
    let ids = normalized_native_item_ids(ids);
    if ids.is_empty() {
        return Ok(0);
    }
    with_db_mut(|conn| {
        let sql = format!(
            "DELETE FROM items WHERE id IN ({})",
            sql_placeholders(ids.len())
        );
        conn.execute(&sql, rusqlite::params_from_iter(ids))
    })
}

fn split_native_paths_blob(value: Option<String>) -> Option<Vec<String>> {
    let paths = value
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|path| !path.is_empty())
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>();
    (!paths.is_empty()).then_some(paths)
}

fn native_clip_kind(kind: &str) -> crate::app_core::ClipKind {
    match kind {
        "image" => crate::app_core::ClipKind::Image,
        "files" => crate::app_core::ClipKind::Files,
        "phrase" => crate::app_core::ClipKind::Phrase,
        _ => crate::app_core::ClipKind::Text,
    }
}

pub(crate) fn native_clip_item(
    item_id: i64,
) -> rusqlite::Result<Option<crate::app_core::ClipItem>> {
    with_db(|conn| {
        conn.query_row(
            "SELECT id, kind, COALESCE(preview, ''), text_data, COALESCE(source_app, ''), \
             file_paths, image_data, COALESCE(image_path, ''), image_width, image_height, \
             pinned, group_id, COALESCE(created_at, '') FROM items WHERE id=?",
            [item_id],
            |row| {
                let kind_raw: String = row.get(1)?;
                let kind = native_clip_kind(&kind_raw);
                let text: Option<String> = row.get(3)?;
                let file_paths_raw: Option<String> = row.get(5)?;
                let file_paths = if kind == crate::app_core::ClipKind::Files {
                    split_native_paths_blob(file_paths_raw.or_else(|| text.clone()))
                } else {
                    split_native_paths_blob(file_paths_raw)
                };
                let image_path: String = row.get(7)?;
                Ok(crate::app_core::ClipItem {
                    id: row.get(0)?,
                    kind,
                    preview: row.get(2)?,
                    text,
                    source_app: row.get(4)?,
                    file_paths,
                    image_bytes: row.get(6)?,
                    image_path: (!image_path.trim().is_empty()).then_some(image_path),
                    image_width: row.get::<_, i64>(8)?.max(0) as usize,
                    image_height: row.get::<_, i64>(9)?.max(0) as usize,
                    pinned: row.get::<_, i64>(10)? == 1,
                    group_id: row.get(11)?,
                    created_at: row.get(12)?,
                })
            },
        )
        .optional()
    })
}

pub(crate) fn native_clip_list_items(
    category: i64,
    limit: usize,
) -> rusqlite::Result<Vec<crate::app_core::NativeHostClipListItemProjection>> {
    native_clip_list_items_for_group(category, 0, limit)
}

pub(crate) fn native_clip_list_items_for_group(
    category: i64,
    group_id: i64,
    limit: usize,
) -> rusqlite::Result<Vec<crate::app_core::NativeHostClipListItemProjection>> {
    with_db(|conn| {
        let mut sql = "SELECT id, kind, COALESCE(preview, ''), COALESCE(source_app, ''), pinned \
             FROM items WHERE category=?"
            .to_string();
        let mut values = vec![rusqlite::types::Value::from(category)];
        if group_id > 0 {
            sql.push_str(" AND group_id=?");
            values.push(rusqlite::types::Value::from(group_id));
        }
        sql.push_str(" ORDER BY pinned DESC, id DESC LIMIT ?");
        values.push(rusqlite::types::Value::from(limit.max(1) as i64));

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(rusqlite::params_from_iter(values), |row| {
            let id: i64 = row.get(0)?;
            let kind: String = row.get(1)?;
            let preview: String = row.get(2)?;
            let source_app: String = row.get(3)?;
            let pinned = row.get::<_, i64>(4)? == 1;
            let title = native_clip_list_title(&kind, &source_app);
            Ok(
                crate::app_core::NativeHostClipListItemProjection::with_metadata(
                    id,
                    title,
                    preview,
                    native_clip_kind(&kind),
                    pinned,
                ),
            )
        })?;
        rows.collect()
    })
}

fn native_clip_list_title(kind: &str, source_app: &str) -> String {
    let source_app = source_app.trim();
    if !source_app.is_empty() {
        return source_app.to_string();
    }
    match kind {
        "image" => "Image".to_string(),
        "files" => "Files".to_string(),
        "phrase" => "Phrase".to_string(),
        _ => "Text".to_string(),
    }
}

pub(crate) fn native_clip_groups(
    category: i64,
) -> rusqlite::Result<Vec<crate::app_core::ClipGroup>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, category, name FROM clip_groups WHERE category=? ORDER BY sort_order ASC, id ASC",
        )?;
        let rows = stmt.query_map([category], |row| {
            Ok(crate::app_core::ClipGroup {
                id: row.get(0)?,
                category: row.get(1)?,
                name: row.get(2)?,
            })
        })?;
        rows.collect()
    })
}

pub(crate) fn create_native_clip_group(
    category: i64,
    name: &str,
) -> rusqlite::Result<crate::app_core::ClipGroup> {
    with_db_mut(|conn| {
        let next_sort: i64 = conn
            .query_row(
                "SELECT COALESCE(MAX(sort_order), 0) + 1 FROM clip_groups WHERE category=?",
                [category],
                |row| row.get(0),
            )
            .unwrap_or(1);
        conn.execute(
            "INSERT INTO clip_groups(category, name, sort_order) VALUES(?, ?, ?)",
            rusqlite::params![category, name, next_sort],
        )?;
        Ok(crate::app_core::ClipGroup {
            id: conn.last_insert_rowid(),
            category,
            name: name.to_string(),
        })
    })
}

pub(crate) fn rename_native_clip_group(
    category: i64,
    group_id: i64,
    new_name: &str,
) -> rusqlite::Result<bool> {
    with_db_mut(|conn| {
        let affected = conn.execute(
            "UPDATE clip_groups SET name=? WHERE id=? AND category=?",
            rusqlite::params![new_name, group_id, category],
        )?;
        Ok(affected > 0)
    })
}

pub(crate) fn delete_native_clip_group(group_id: i64) -> rusqlite::Result<bool> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        tx.execute(
            "UPDATE items SET group_id=0 WHERE group_id=?",
            rusqlite::params![group_id],
        )?;
        let affected = tx.execute(
            "DELETE FROM clip_groups WHERE id=?",
            rusqlite::params![group_id],
        )?;
        tx.commit()?;
        Ok(affected > 0)
    })
}

pub(crate) fn set_native_clip_groups_order(
    category: i64,
    group_ids: &[i64],
) -> rusqlite::Result<usize> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        let mut affected = 0;
        for (idx, gid) in group_ids.iter().enumerate() {
            affected += tx.execute(
                "UPDATE clip_groups SET sort_order=? WHERE id=? AND category=?",
                rusqlite::params![idx as i64 + 1, *gid, category],
            )?;
        }
        tx.commit()?;
        Ok(affected)
    })
}

pub(crate) fn move_native_clip_group(
    category: i64,
    group_id: i64,
    step: i32,
) -> rusqlite::Result<bool> {
    let groups = native_clip_groups(category)?;
    let Some(index) = groups.iter().position(|group| group.id == group_id) else {
        return Ok(false);
    };
    let next_index = index as i32 + step;
    if next_index < 0 || next_index >= groups.len() as i32 {
        return Ok(false);
    }
    let mut ids = groups.iter().map(|group| group.id).collect::<Vec<_>>();
    ids.swap(index, next_index as usize);
    set_native_clip_groups_order(category, &ids)?;
    Ok(true)
}

pub(crate) fn assign_native_clip_group(item_ids: &[i64], group_id: i64) -> rusqlite::Result<usize> {
    with_db_mut(|conn| {
        let tx = conn.unchecked_transaction()?;
        let mut affected = 0;
        for item_id in item_ids.iter().copied().filter(|item_id| *item_id > 0) {
            affected += tx.execute(
                "UPDATE items SET group_id=? WHERE id=?",
                rusqlite::params![group_id, item_id],
            )?;
        }
        tx.commit()?;
        Ok(affected)
    })
}

pub(crate) fn checkpoint_db() -> rusqlite::Result<()> {
    with_db(|conn| {
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")?;
        Ok(())
    })
}

pub(crate) fn close_db() {
    DB_CONN.with(|cell| {
        let mut slot = cell.borrow_mut();
        if let Some(conn) = slot.as_mut() {
            let _ = conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE);");
        }
        *slot = None;
    });
}

#[cfg(test)]
pub(crate) fn with_test_db<T, F>(f: F) -> rusqlite::Result<T>
where
    F: FnOnce() -> rusqlite::Result<T>,
{
    DB_CONN.with(|cell| {
        let previous = cell.borrow_mut().take();
        let conn = Connection::open_in_memory()?;
        configure_db_connection(&conn)?;
        migrate_db(&conn)?;
        *cell.borrow_mut() = Some(conn);
        let result = f();
        *cell.borrow_mut() = previous;
        result
    })
}

#[cfg(test)]
pub(crate) fn with_test_db_path<T, F>(path: &std::path::Path, f: F) -> rusqlite::Result<T>
where
    F: FnOnce() -> rusqlite::Result<T>,
{
    DB_CONN.with(|cell| {
        let previous = cell.borrow_mut().take();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let conn = Connection::open(path)?;
        configure_db_connection(&conn)?;
        migrate_db(&conn)?;
        *cell.borrow_mut() = Some(conn);
        let result = f();
        *cell.borrow_mut() = previous;
        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn item_text_update_reports_affected_row_and_updates_preview() {
        with_test_db(|| {
            let item_id = with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'old', 'sig', 'old', 'test')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;

            assert_eq!(item_text(item_id)?, Some("old".to_string()));
            assert!(update_item_text(item_id, "new clipboard text")?);
            assert_eq!(item_text(item_id)?, Some("new clipboard text".to_string()));
            let preview: String = with_db(|conn| {
                conn.query_row("SELECT preview FROM items WHERE id=?", [item_id], |row| {
                    row.get(0)
                })
            })?;
            assert_eq!(preview, "new clipboard text");
            assert!(!update_item_text(item_id + 10_000, "missing")?);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn native_clip_list_items_projects_recent_database_rows() {
        with_test_db(|| {
            with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app, pinned) VALUES(0, 'text', 'older text', 'old', 'older text', 'Notes', 0)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app, pinned) VALUES(0, 'files', 'report.xlsx', 'file', NULL, '', 1)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app, pinned) VALUES(1, 'phrase', 'phrase row', 'phrase', 'phrase row', '', 0)",
                    [],
                )?;
                Ok(())
            })?;

            let items = native_clip_list_items(0, 10)?;
            assert_eq!(items.len(), 2);
            assert_eq!(items[0].title, "Files");
            assert_eq!(items[0].preview, "report.xlsx");
            assert_eq!(items[0].kind, crate::app_core::ClipKind::Files);
            assert!(items[0].pinned);
            assert_eq!(items[1].title, "Notes");
            assert_eq!(items[1].preview, "older text");
            assert_eq!(items[1].kind, crate::app_core::ClipKind::Text);
            assert!(!items[1].pinned);

            let phrases = native_clip_list_items(1, 10)?;
            assert_eq!(phrases.len(), 1);
            assert_eq!(phrases[0].title, "Phrase");
            assert_eq!(phrases[0].kind, crate::app_core::ClipKind::Phrase);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn native_clip_item_loads_full_payload_for_native_hosts() {
        with_test_db(|| {
            let (text_id, file_id, image_id) = with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'hello', 'text', 'hello native', 'Notes')",
                    [],
                )?;
                let text_id = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, file_paths, source_app) VALUES(0, 'files', 'files', 'files', '/tmp/a.txt\n/tmp/b.txt', '')",
                    [],
                )?;
                let file_id = conn.last_insert_rowid();
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, image_data, image_width, image_height, source_app) VALUES(0, 'image', 'image', 'image', x'FF0000FF', 1, 1, '')",
                    [],
                )?;
                let image_id = conn.last_insert_rowid();
                Ok((text_id, file_id, image_id))
            })?;

            let text = native_clip_item(text_id)?.unwrap();
            assert_eq!(text.kind, crate::app_core::ClipKind::Text);
            assert_eq!(text.text.as_deref(), Some("hello native"));
            assert_eq!(text.source_app, "Notes");

            let files = native_clip_item(file_id)?.unwrap();
            assert_eq!(files.kind, crate::app_core::ClipKind::Files);
            assert_eq!(
                files.file_paths,
                Some(vec!["/tmp/a.txt".to_string(), "/tmp/b.txt".to_string()])
            );

            let image = native_clip_item(image_id)?.unwrap();
            assert_eq!(image.kind, crate::app_core::ClipKind::Image);
            assert_eq!(image.image_bytes, Some(vec![255, 0, 0, 255]));
            assert_eq!((image.image_width, image.image_height), (1, 1));
            assert!(native_clip_item(image_id + 10_000)?.is_none());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn native_clip_groups_create_rename_order_delete_and_assign_items() {
        with_test_db(|| {
            let item_id = with_db_mut(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app) VALUES(0, 'text', 'clip', 'group-clip', 'clip', 'test')",
                    [],
                )?;
                Ok(conn.last_insert_rowid())
            })?;

            let first = create_native_clip_group(0, "First")?;
            let second = create_native_clip_group(0, "Second")?;
            assert_eq!(
                native_clip_groups(0)?
                    .iter()
                    .map(|group| group.name.as_str())
                    .collect::<Vec<_>>(),
                vec!["First", "Second"]
            );

            assert!(rename_native_clip_group(0, first.id, "Renamed")?);
            assert_eq!(native_clip_groups(0)?[0].name, "Renamed");
            assert_eq!(set_native_clip_groups_order(0, &[second.id, first.id])?, 2);
            assert_eq!(native_clip_groups(0)?[0].id, second.id);
            assert!(move_native_clip_group(0, first.id, -1)?);
            assert_eq!(native_clip_groups(0)?[0].id, first.id);

            assert_eq!(assign_native_clip_group(&[item_id], second.id)?, 1);
            let grouped = native_clip_list_items_for_group(0, second.id, 10)?;
            assert_eq!(grouped.len(), 1);
            assert_eq!(grouped[0].id, item_id);

            assert!(delete_native_clip_group(second.id)?);
            let group_id: i64 = with_db(|conn| {
                conn.query_row("SELECT group_id FROM items WHERE id=?", [item_id], |row| {
                    row.get(0)
                })
            })?;
            assert_eq!(group_id, 0);
            Ok(())
        })
        .unwrap();
    }
}
