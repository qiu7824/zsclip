#![cfg_attr(windows, allow(dead_code))]

use std::cell::Cell;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, RwLock, Weak};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use rusqlite::config::DbConfig;
use rusqlite::{params_from_iter, Connection, OptionalExtension};

use crate::app_core::{parse_search_query_with_context, SearchDateContext, SearchTimeFilter};
use crate::time_utils::{days_to_sqlite_date, utc_secs_to_local_parts};

#[derive(Default)]
enum DbConnectionTarget {
    #[default]
    Runtime,
    #[cfg(test)]
    InMemory,
    #[cfg(test)]
    Path(std::path::PathBuf),
}

#[derive(Default)]
struct DbConnectionSlot {
    connection: Option<Connection>,
    target: DbConnectionTarget,
}

type SharedDbConnectionSlot = Arc<Mutex<DbConnectionSlot>>;

thread_local! {
    static DB_CONN: SharedDbConnectionSlot = register_db_connection_slot();
    static APP_DATA_READ_DEPTH: Cell<usize> = const { Cell::new(0) };
}

// Files in the application data directory and the database form one logical
// state. Normal work takes the shared gate before touching either resource;
// restore takes the exclusive gate before the exclusive database gate.
static APP_DATA_ACCESS_GATE: OnceLock<RwLock<()>> = OnceLock::new();
static APP_DATA_REPLACEMENT_EPOCH: AtomicU64 = AtomicU64::new(0);
// Normal database work holds a shared gate. Restore takes the exclusive gate and
// clears every registered TLS connection for the file before replacing it.
static DB_ACCESS_GATE: OnceLock<RwLock<()>> = OnceLock::new();
static DB_CONNECTION_SLOTS: OnceLock<Mutex<Vec<Weak<Mutex<DbConnectionSlot>>>>> = OnceLock::new();
static DB_MIGRATION_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static DB_MIGRATED: AtomicBool = AtomicBool::new(false);
// Even values are stable database generations. An odd value means a file
// replacement is in progress. A DB operation that began before a replacement
// must never resume against the newly restored file.
static DB_REPLACEMENT_EPOCH: AtomicU64 = AtomicU64::new(0);

fn app_data_access_gate() -> &'static RwLock<()> {
    APP_DATA_ACCESS_GATE.get_or_init(|| RwLock::new(()))
}

struct AppDataReadDepthGuard;

impl Drop for AppDataReadDepthGuard {
    fn drop(&mut self) {
        APP_DATA_READ_DEPTH.with(|depth| {
            let previous = depth.get();
            debug_assert!(previous > 0);
            depth.set(previous.saturating_sub(1));
        });
    }
}

/// Runs normal settings/image/database work under the shared application-data
/// gate. Shared entry is re-entrant on one thread so a combined image+DB action
/// may call ordinary DB helpers without reversing the data -> DB lock order.
pub(crate) fn with_shared_app_data<T, F>(action: F) -> T
where
    F: FnOnce() -> T,
{
    let nested = APP_DATA_READ_DEPTH.with(|depth| depth.get() > 0);
    let _access_guard = if nested {
        None
    } else {
        Some(
            app_data_access_gate()
                .read()
                .unwrap_or_else(|poisoned| poisoned.into_inner()),
        )
    };
    APP_DATA_READ_DEPTH.with(|depth| depth.set(depth.get().saturating_add(1)));
    let _depth_guard = AppDataReadDepthGuard;
    action()
}

/// Executes only if the caller's in-memory state belongs to the current
/// application-data generation, while preventing a restore until it returns.
pub(crate) fn with_shared_app_data_generation<T, F>(expected: u64, action: F) -> Option<T>
where
    F: FnOnce() -> T,
{
    // Fast-fail UI and async callbacks while a restore owns the exclusive
    // application-data gate. Waiting here would freeze the window for the
    // entire backup/swap operation only to reject the stale callback later.
    let observed = APP_DATA_REPLACEMENT_EPOCH.load(Ordering::Acquire);
    if observed != expected || observed & 1 != 0 {
        return None;
    }
    with_shared_app_data(|| {
        let current = APP_DATA_REPLACEMENT_EPOCH.load(Ordering::Acquire);
        if current != expected || current & 1 != 0 {
            None
        } else {
            Some(action())
        }
    })
}

pub(crate) fn current_app_data_generation() -> u64 {
    // This is an advisory, non-blocking observation. An odd value tells UI
    // callbacks to abandon work immediately while replacement is active;
    // callers that touch data still enter one of the guarded helpers above.
    APP_DATA_REPLACEMENT_EPOCH.load(Ordering::Acquire)
}

/// Takes a consistent application-data snapshot without changing its
/// generation. Normal settings/image/database work is paused for the callback;
/// callers that also need the DB gate must acquire it from inside this scope.
pub(crate) fn with_exclusive_app_data_snapshot<T, F>(snapshot: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let read_depth = APP_DATA_READ_DEPTH.with(Cell::get);
    if read_depth != 0 {
        return Err("cannot snapshot application data from a shared data operation".to_string());
    }
    let _access_guard = app_data_access_gate()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    snapshot()
}

struct AppDataReplacementEpochGuard;

impl AppDataReplacementEpochGuard {
    fn begin() -> Self {
        let previous = APP_DATA_REPLACEMENT_EPOCH.fetch_add(1, Ordering::AcqRel);
        debug_assert_eq!(previous & 1, 0);
        Self
    }
}

impl Drop for AppDataReplacementEpochGuard {
    fn drop(&mut self) {
        let previous = APP_DATA_REPLACEMENT_EPOCH.fetch_add(1, Ordering::AcqRel);
        debug_assert_eq!(previous & 1, 1);
    }
}

/// Replaces settings/images (and, for a full restore, the database) while all
/// normal application-data work is stopped. Nested callers must acquire the DB
/// exclusive gate only after entering this function.
pub(crate) fn with_exclusive_app_data_replacement<T, F>(replace: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let read_depth = APP_DATA_READ_DEPTH.with(Cell::get);
    if read_depth != 0 {
        return Err("cannot replace application data from a shared data operation".to_string());
    }
    let _access_guard = app_data_access_gate()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _replacement_epoch = AppDataReplacementEpochGuard::begin();
    replace()
}

fn db_access_gate() -> &'static RwLock<()> {
    DB_ACCESS_GATE.get_or_init(|| RwLock::new(()))
}

fn db_connection_slots() -> &'static Mutex<Vec<Weak<Mutex<DbConnectionSlot>>>> {
    DB_CONNECTION_SLOTS.get_or_init(|| Mutex::new(Vec::new()))
}

fn db_migration_lock() -> &'static Mutex<()> {
    DB_MIGRATION_LOCK.get_or_init(|| Mutex::new(()))
}

fn register_db_connection_slot() -> SharedDbConnectionSlot {
    let slot = Arc::new(Mutex::new(DbConnectionSlot::default()));
    let mut slots = db_connection_slots()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    slots.retain(|entry| entry.strong_count() > 0);
    slots.push(Arc::downgrade(&slot));
    slot
}

fn db_replacement_interrupted_error() -> rusqlite::Error {
    rusqlite::Error::SqliteFailure(
        rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_ABORT),
        Some("database operation crossed a restore boundary".to_string()),
    )
}

fn db_operation_read_guard() -> rusqlite::Result<std::sync::RwLockReadGuard<'static, ()>> {
    let epoch_before = DB_REPLACEMENT_EPOCH.load(Ordering::Acquire);
    if epoch_before & 1 != 0 {
        return Err(db_replacement_interrupted_error());
    }
    let guard = db_access_gate()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let epoch_after = DB_REPLACEMENT_EPOCH.load(Ordering::Acquire);
    if epoch_before != epoch_after || epoch_after & 1 != 0 {
        drop(guard);
        return Err(db_replacement_interrupted_error());
    }
    Ok(guard)
}

struct DbReplacementEpochGuard;

impl DbReplacementEpochGuard {
    fn begin() -> Self {
        let previous = DB_REPLACEMENT_EPOCH.fetch_add(1, Ordering::AcqRel);
        debug_assert_eq!(previous & 1, 0);
        Self
    }
}

impl Drop for DbReplacementEpochGuard {
    fn drop(&mut self) {
        let previous = DB_REPLACEMENT_EPOCH.fetch_add(1, Ordering::AcqRel);
        debug_assert_eq!(previous & 1, 1);
    }
}

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
        ("items", "rich_text_html", "rich_text_html TEXT") => Ok("rich_text_html TEXT"),
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
    ensure_table_column(conn, "items", "rich_text_html", "rich_text_html TEXT")?;
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

fn configure_runtime_wal_connection(conn: &Connection) -> rusqlite::Result<()> {
    conn.pragma_update(None, "wal_autocheckpoint", 0i32)?;
    conn.set_db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE, true)?;
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
            rich_text_html TEXT,
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

fn quick_check_database(conn: &Connection) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare("PRAGMA quick_check")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let result = row.get::<_, String>(0)?;
        if !result.eq_ignore_ascii_case("ok") {
            return Err(rusqlite::Error::InvalidParameterName(format!(
                "database quick_check failed: {result}"
            )));
        }
    }
    Ok(())
}

fn collect_restored_image_files(
    root: &std::path::Path,
    dir: &std::path::Path,
    output: &mut Vec<std::path::PathBuf>,
) -> std::io::Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if entry.file_type()?.is_dir() {
            collect_restored_image_files(root, &path, output)?;
        } else if entry.file_type()?.is_file() {
            if let Ok(relative) = path.strip_prefix(root) {
                output.push(relative.to_path_buf());
            }
        }
    }
    Ok(())
}

fn safe_image_relative_path(raw: &str) -> Option<std::path::PathBuf> {
    let normalized = raw.trim().replace('\\', "/");
    if normalized.is_empty() {
        return None;
    }
    let parts = normalized.split('/').collect::<Vec<_>>();
    let suffix = parts
        .iter()
        .rposition(|part| part.eq_ignore_ascii_case("images"))
        .map(|index| &parts[index.saturating_add(1)..])
        .filter(|parts| !parts.is_empty())
        .or_else(|| {
            let looks_absolute = normalized.starts_with('/')
                || parts
                    .first()
                    .map(|part| part.ends_with(':'))
                    .unwrap_or(false);
            (!looks_absolute).then_some(parts.as_slice())
        })?;
    let mut relative = std::path::PathBuf::new();
    for part in suffix {
        if part.is_empty() || *part == "." || *part == ".." || part.contains(':') {
            return None;
        }
        relative.push(part);
    }
    (!relative.as_os_str().is_empty()).then_some(relative)
}

fn restored_image_path_matches(left: &std::path::Path, right: &std::path::Path) -> bool {
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn resolve_restored_image_relative_path(
    raw: &str,
    available: &[std::path::PathBuf],
) -> Option<std::path::PathBuf> {
    if let Some(candidate) = safe_image_relative_path(raw) {
        if let Some(found) = available
            .iter()
            .find(|path| restored_image_path_matches(path, &candidate))
        {
            return Some(found.clone());
        }
    }

    let normalized = raw.trim().replace('\\', "/");
    let file_name = normalized
        .rsplit('/')
        .find(|part| !part.is_empty() && *part != "." && *part != ".." && !part.contains(':'))?;
    let matches = available
        .iter()
        .filter(|path| {
            path.file_name()
                .map(|name| {
                    #[cfg(windows)]
                    {
                        name.to_string_lossy().eq_ignore_ascii_case(file_name)
                    }
                    #[cfg(not(windows))]
                    {
                        name == std::ffi::OsStr::new(file_name)
                    }
                })
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();
    (matches.len() == 1).then(|| matches[0].clone())
}

pub(crate) fn prepare_restored_database(
    db_file: &std::path::Path,
    staged_images_dir: &std::path::Path,
    active_images_dir: &std::path::Path,
) -> Result<(), String> {
    let mut conn = Connection::open(db_file)
        .map_err(|err| format!("恢复文件不是有效的 SQLite 数据库：{err}"))?;
    conn.busy_timeout(Duration::from_millis(5_000))
        .map_err(|err| format!("设置恢复数据库超时失败：{err}"))?;
    quick_check_database(&conn).map_err(|err| format!("恢复数据库完整性检查失败：{err}"))?;
    let has_items: i64 = conn
        .query_row(
            "SELECT COUNT(*) FROM sqlite_master WHERE type='table' AND name='items'",
            [],
            |row| row.get(0),
        )
        .map_err(|err| format!("恢复数据库结构检查失败：{err}"))?;
    if has_items == 0 {
        return Err("恢复数据库缺少 items 表。".to_string());
    }

    migrate_db(&conn).map_err(|err| format!("恢复数据库迁移失败：{err}"))?;

    let mut available_images = Vec::new();
    collect_restored_image_files(staged_images_dir, staged_images_dir, &mut available_images)
        .map_err(|err| format!("读取恢复图片暂存目录失败：{err}"))?;
    let image_rows = {
        let mut stmt = conn
            .prepare(
                "SELECT id, COALESCE(image_path, ''), image_data IS NOT NULL \
                 FROM items WHERE image_path IS NOT NULL AND TRIM(image_path)<>''",
            )
            .map_err(|err| format!("读取恢复图片路径失败：{err}"))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, bool>(2)?,
                ))
            })
            .map_err(|err| format!("读取恢复图片路径失败：{err}"))?;
        rows.collect::<rusqlite::Result<Vec<_>>>()
            .map_err(|err| format!("读取恢复图片路径失败：{err}"))?
    };
    let tx = conn
        .transaction()
        .map_err(|err| format!("更新恢复图片路径失败：{err}"))?;
    for (id, old_path, has_image_data) in image_rows {
        if let Some(relative) = resolve_restored_image_relative_path(&old_path, &available_images) {
            let restored_path = active_images_dir.join(relative);
            tx.execute(
                "UPDATE items SET image_path=? WHERE id=?",
                rusqlite::params![restored_path.to_string_lossy().to_string(), id],
            )
            .map_err(|err| format!("更新恢复图片路径失败：{err}"))?;
        } else if has_image_data {
            tx.execute("UPDATE items SET image_path=NULL WHERE id=?", [id])
                .map_err(|err| format!("清理失效恢复图片路径失败：{err}"))?;
        } else {
            return Err(format!("恢复数据库引用了归档中不存在的图片：{old_path}"));
        }
    }
    tx.commit()
        .map_err(|err| format!("提交恢复图片路径失败：{err}"))?;

    conn.prepare(
        "SELECT id, category, kind, preview, signature, source_app, pinned, group_id \
         FROM items ORDER BY id DESC LIMIT 1",
    )
    .map_err(|err| format!("恢复数据库关键查询检查失败：{err}"))?;
    quick_check_database(&conn).map_err(|err| format!("迁移后恢复数据库完整性检查失败：{err}"))?;
    checkpoint_connection(&conn).map_err(|err| format!("写入恢复数据库 WAL 失败：{err}"))?;
    drop(conn);
    for sidecar in [wal_file_path(db_file), shm_file_path(db_file)] {
        match std::fs::remove_file(&sidecar) {
            Ok(()) => {}
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
            Err(err) => {
                return Err(format!(
                    "无法清理恢复数据库暂存文件 {}：{err}",
                    sidecar.to_string_lossy()
                ));
            }
        }
    }
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

#[cfg(target_os = "windows")]
const DB_CHECKPOINT_HELPER_ARG: &str = "--zsclip-checkpoint-after-parent";

#[cfg(target_os = "windows")]
pub(crate) fn maybe_run_db_checkpoint_helper_from_args() -> Option<i32> {
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new(DB_CHECKPOINT_HELPER_ARG)) {
        return None;
    }
    let parent_pid = args.next()?.to_string_lossy().parse::<u32>().ok()?;
    if !crate::platform::process::wait_for_process_exit(parent_pid) {
        return Some(1);
    }
    let db_file = runtime_db_file();
    if !db_file.exists() || !wal_file_path(&db_file).exists() {
        return Some(0);
    }
    let result = Connection::open(&db_file).and_then(|conn| {
        conn.busy_timeout(Duration::from_millis(1_000))?;
        checkpoint_connection(&conn)
    });
    Some(if result.is_ok() { 0 } else { 1 })
}

#[cfg(target_os = "windows")]
pub(crate) fn spawn_db_checkpoint_after_current_process_exit() {
    use std::os::windows::process::CommandExt;

    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let mut command = std::process::Command::new(executable);
    command
        .arg(DB_CHECKPOINT_HELPER_ARG)
        .arg(crate::platform::process::current_process_id().to_string())
        .creation_flags(0x0800_0000);
    let _ = command.spawn();
}

fn open_connection(target: &DbConnectionTarget) -> rusqlite::Result<Connection> {
    match target {
        DbConnectionTarget::Runtime => {
            let db_file = runtime_db_file();
            if let Some(parent) = db_file.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| rusqlite::Error::InvalidPath(db_file.clone()))?;
            }
            Connection::open(db_file)
        }
        #[cfg(test)]
        DbConnectionTarget::InMemory => Connection::open_in_memory(),
        #[cfg(test)]
        DbConnectionTarget::Path(path) => {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| rusqlite::Error::InvalidPath(path.clone()))?;
            }
            Connection::open(path)
        }
    }
}

fn ensure_connection(slot: &mut DbConnectionSlot) -> rusqlite::Result<()> {
    if slot.connection.is_none() {
        let conn = open_connection(&slot.target)?;
        configure_db_connection(&conn)?;
        if matches!(&slot.target, DbConnectionTarget::Runtime) {
            configure_runtime_wal_connection(&conn)?;
        }
        slot.connection = Some(conn);
    }

    if !DB_MIGRATED.load(Ordering::Acquire) {
        let _migration_guard = db_migration_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        if !DB_MIGRATED.load(Ordering::Relaxed) {
            if let Some(conn) = slot.connection.as_ref() {
                migrate_db(conn)?;
                DB_MIGRATED.store(true, Ordering::Release);
            }
        }
    }
    Ok(())
}

pub(crate) fn ensure_db() {
    with_shared_app_data(|| {
        let _access_guard = db_access_gate()
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        DB_CONN.with(|slot| {
            let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let _ = ensure_connection(&mut slot);
        });
    });
}

pub(crate) fn with_db<T, F>(f: F) -> rusqlite::Result<T>
where
    F: FnOnce(&Connection) -> rusqlite::Result<T>,
{
    with_shared_app_data(|| {
        let _access_guard = db_operation_read_guard()?;
        DB_CONN.with(|slot| {
            let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            ensure_connection(&mut slot)?;
            let conn = slot
                .connection
                .as_ref()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            f(conn)
        })
    })
}

pub(crate) fn with_db_mut<T, F>(f: F) -> rusqlite::Result<T>
where
    F: FnOnce(&mut Connection) -> rusqlite::Result<T>,
{
    with_shared_app_data(|| {
        let _access_guard = db_operation_read_guard()?;
        DB_CONN.with(|slot| {
            let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            ensure_connection(&mut slot)?;
            let conn = slot
                .connection
                .as_mut()
                .ok_or(rusqlite::Error::InvalidQuery)?;
            f(conn)
        })
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
pub(crate) fn insert_native_phrase_from_item(
    item: &crate::app_core::ClipItem,
    source_app: &str,
) -> rusqlite::Result<NativeClipboardInsertOutcome> {
    let text = item
        .text
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| item.preview.trim());
    let normalized = normalize_native_captured_text(text);
    if normalized.is_empty() {
        return Ok(NativeClipboardInsertOutcome {
            item_id: None,
            inserted: false,
            reason: "empty_phrase",
        });
    }
    let preview = native_clip_preview(&normalized);
    let signature = native_clip_signature("phrase", &normalized, &[], &[], 0, 0);
    insert_native_clipboard_item(NativeClipboardInsert {
        category: 1,
        kind: "phrase",
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
            "SELECT id, kind, COALESCE(preview, ''), text_data, rich_text_html, COALESCE(source_app, ''), \
             file_paths, image_data, COALESCE(image_path, ''), image_width, image_height, \
             pinned, group_id, COALESCE(created_at, '') FROM items WHERE id=?",
            [item_id],
            |row| {
                let kind_raw: String = row.get(1)?;
                let kind = native_clip_kind(&kind_raw);
                let text: Option<String> = row.get(3)?;
                let file_paths_raw: Option<String> = row.get(6)?;
                let file_paths = if kind == crate::app_core::ClipKind::Files {
                    split_native_paths_blob(file_paths_raw.or_else(|| text.clone()))
                } else {
                    split_native_paths_blob(file_paths_raw)
                };
                let image_path: String = row.get(8)?;
                Ok(crate::app_core::ClipItem {
                    id: row.get(0)?,
                    kind,
                    preview: row.get(2)?,
                    text,
                    rich_text_html: row.get(4)?,
                    source_app: row.get(5)?,
                    file_paths,
                    image_bytes: row.get(7)?,
                    image_path: (!image_path.trim().is_empty()).then_some(image_path),
                    image_width: row.get::<_, i64>(9)?.max(0) as usize,
                    image_height: row.get::<_, i64>(10)?.max(0) as usize,
                    pinned: row.get::<_, i64>(11)? == 1,
                    group_id: row.get(12)?,
                    created_at: row.get(13)?,
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
    native_clip_list_items_for_group_kind_filter(
        category,
        group_id,
        crate::app_core::ClipKindFilter::All,
        limit,
    )
}

pub(crate) fn native_clip_list_items_for_group_kind_filter(
    category: i64,
    group_id: i64,
    kind_filter: crate::app_core::ClipKindFilter,
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
        let kind_values = kind_filter.db_kinds(category);
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
                values.push(rusqlite::types::Value::from((*kind).to_string()));
            }
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

pub(crate) fn native_clip_list_items_for_query(
    category: i64,
    group_id: i64,
    kind_filter: crate::app_core::ClipKindFilter,
    search_text: &str,
    limit: usize,
) -> rusqlite::Result<Vec<crate::app_core::NativeHostClipListItemProjection>> {
    let date_context = current_native_search_date_context();
    let (search_terms, time_filter, app_filter, near_query) =
        parse_search_query_with_context(search_text.trim(), date_context);
    with_db(|conn| {
        let select_columns = "id, kind, COALESCE(preview, '') AS preview, COALESCE(source_app, '') AS source_app, pinned, COALESCE(file_paths, text_data, '') AS searchable_data, COALESCE(created_at, '') AS created_at";
        let mut sql = if near_query.is_some() {
            format!(
                "WITH base AS (SELECT {select_columns}, ROW_NUMBER() OVER (ORDER BY pinned DESC, id DESC) AS rn FROM items WHERE category=?"
            )
        } else {
            format!("SELECT {select_columns} FROM items WHERE category=?")
        };
        let mut values = vec![rusqlite::types::Value::from(category)];

        append_native_clip_query_filters(
            &mut sql,
            &mut values,
            category,
            group_id,
            kind_filter,
            search_terms,
            time_filter,
            app_filter,
            date_context,
        );

        if let Some(near_value) = near_query {
            let like = format!("%{}%", near_value.to_lowercase());
            sql.push_str(
                "), hits AS (SELECT rn FROM base WHERE LOWER(preview) LIKE ? \
                 OR LOWER(source_app) LIKE ? \
                 OR LOWER(searchable_data) LIKE ? \
                 OR LOWER(COALESCE(strftime('%m-%d %H:%M', datetime(created_at, 'localtime')), '')) LIKE ?), \
                 near_rows AS (SELECT DISTINCT base.rn FROM base JOIN hits ON base.rn BETWEEN hits.rn - 3 AND hits.rn + 3) \
                 SELECT id, kind, preview, source_app, pinned, searchable_data, created_at \
                 FROM base WHERE rn IN (SELECT rn FROM near_rows)",
            );
            values.push(rusqlite::types::Value::from(like.clone()));
            values.push(rusqlite::types::Value::from(like.clone()));
            values.push(rusqlite::types::Value::from(like.clone()));
            values.push(rusqlite::types::Value::from(like));
        }

        sql.push_str(" ORDER BY pinned DESC, id DESC LIMIT ?");
        values.push(rusqlite::types::Value::from(limit.max(1) as i64));

        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params_from_iter(values.iter()), |row| {
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

fn current_native_search_date_context() -> SearchDateContext {
    let now_secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0);
    let (year, month, day, _, _, _) = utc_secs_to_local_parts(now_secs);
    SearchDateContext::from_date(year, month, day)
}

#[allow(clippy::too_many_arguments)]
fn append_native_clip_query_filters(
    sql: &mut String,
    values: &mut Vec<rusqlite::types::Value>,
    category: i64,
    group_id: i64,
    kind_filter: crate::app_core::ClipKindFilter,
    search_terms: Vec<String>,
    time_filter: Option<SearchTimeFilter>,
    app_filter: Option<String>,
    date_context: SearchDateContext,
) {
    if group_id > 0 {
        sql.push_str(" AND group_id=?");
        values.push(rusqlite::types::Value::from(group_id));
    }
    let kind_values = kind_filter.db_kinds(category);
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
            values.push(rusqlite::types::Value::from((*kind).to_string()));
        }
    }
    for term in search_terms {
        let like = format!("%{}%", term.to_lowercase());
        sql.push_str(
            " AND (LOWER(COALESCE(preview, '')) LIKE ? \
             OR LOWER(COALESCE(source_app, '')) LIKE ? \
             OR LOWER(COALESCE(file_paths, text_data, '')) LIKE ? \
             OR LOWER(COALESCE(strftime('%m-%d %H:%M', datetime(created_at, 'localtime')), '')) LIKE ?)",
        );
        values.push(rusqlite::types::Value::from(like.clone()));
        values.push(rusqlite::types::Value::from(like.clone()));
        values.push(rusqlite::types::Value::from(like.clone()));
        values.push(rusqlite::types::Value::from(like));
    }
    if let Some(app_value) = app_filter {
        sql.push_str(" AND LOWER(COALESCE(source_app, '')) LIKE ?");
        values.push(rusqlite::types::Value::from(format!(
            "%{}%",
            app_value.to_lowercase()
        )));
    }
    match time_filter {
        Some(SearchTimeFilter::ExactDay(day)) => {
            sql.push_str(" AND date(created_at, 'localtime') = ?");
            values.push(rusqlite::types::Value::from(days_to_sqlite_date(day)));
        }
        Some(SearchTimeFilter::RecentDays(days)) => {
            let end_day = date_context.current_day;
            let start_day = end_day - (days.max(1) - 1);
            sql.push_str(
                " AND date(created_at, 'localtime') >= ? AND date(created_at, 'localtime') <= ?",
            );
            values.push(rusqlite::types::Value::from(days_to_sqlite_date(start_day)));
            values.push(rusqlite::types::Value::from(days_to_sqlite_date(end_day)));
        }
        None => {}
    }
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

fn validate_wal_checkpoint_result(
    busy: i64,
    log_frames: i64,
    checkpointed_frames: i64,
) -> rusqlite::Result<()> {
    if busy != 0 || (log_frames >= 0 && checkpointed_frames < log_frames) {
        return Err(rusqlite::Error::SqliteFailure(
            rusqlite::ffi::Error::new(rusqlite::ffi::SQLITE_BUSY),
            Some(format!(
                "WAL checkpoint incomplete: busy={busy}, log={log_frames}, checkpointed={checkpointed_frames}"
            )),
        ));
    }
    Ok(())
}

fn checkpoint_connection(conn: &Connection) -> rusqlite::Result<()> {
    let (busy, log_frames, checkpointed_frames) =
        conn.query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })?;
    validate_wal_checkpoint_result(busy, log_frames, checkpointed_frames)
}

fn wal_file_path(path: &std::path::Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}-wal", path.to_string_lossy()))
}

fn shm_file_path(path: &std::path::Path) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("{}-shm", path.to_string_lossy()))
}

pub(crate) fn checkpoint_db() -> rusqlite::Result<()> {
    with_db(checkpoint_connection)
}

pub(crate) fn close_db() {
    let connection = with_shared_app_data(|| {
        let _access_guard = db_access_gate()
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        DB_CONN.with(|slot| {
            let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            slot.connection.take()
        })
    });
    drop(connection);
}

fn normalized_db_path(path: &std::path::Path) -> std::path::PathBuf {
    let absolute = if path.is_absolute() {
        path.to_path_buf()
    } else {
        std::env::current_dir()
            .map(|current| current.join(path))
            .unwrap_or_else(|_| path.to_path_buf())
    };
    std::fs::canonicalize(&absolute).unwrap_or(absolute)
}

pub(crate) fn is_runtime_db_file(path: &std::path::Path) -> bool {
    normalized_db_path(path) == normalized_db_path(&runtime_db_file())
}

fn connection_target_path(target: &DbConnectionTarget) -> Option<std::path::PathBuf> {
    match target {
        DbConnectionTarget::Runtime => Some(runtime_db_file()),
        #[cfg(test)]
        DbConnectionTarget::InMemory => None,
        #[cfg(test)]
        DbConnectionTarget::Path(path) => Some(path.clone()),
    }
}

fn db_paths_match(left: &std::path::Path, right: &std::path::Path) -> bool {
    let left = normalized_db_path(left);
    let right = normalized_db_path(right);
    #[cfg(windows)]
    {
        left.to_string_lossy()
            .eq_ignore_ascii_case(&right.to_string_lossy())
    }
    #[cfg(not(windows))]
    {
        left == right
    }
}

fn close_db_connections_for_path(db_file: &std::path::Path) -> Result<(), String> {
    let live_slots = {
        let mut slots = db_connection_slots()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let live = slots.iter().filter_map(Weak::upgrade).collect::<Vec<_>>();
        slots.retain(|entry| entry.strong_count() > 0);
        live
    };

    let mut closed_connections = Vec::new();
    for slot in live_slots {
        let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
        let targets_db_file = connection_target_path(&slot.target)
            .map(|target| db_paths_match(&target, db_file))
            .unwrap_or(false);
        if !targets_db_file {
            continue;
        }
        if let Some(conn) = slot.connection.take() {
            closed_connections.push(conn);
        }
    }
    drop(closed_connections);

    if !db_file.exists() {
        return Ok(());
    }
    let conn =
        Connection::open(db_file).map_err(|err| format!("打开数据库以写入 WAL 失败：{err}"))?;
    conn.busy_timeout(Duration::from_millis(5_000))
        .map_err(|err| format!("设置 WAL 写入超时失败：{err}"))?;
    checkpoint_connection(&conn).map_err(|err| format!("关闭数据库连接前写入 WAL 失败：{err}"))?;
    drop(conn);
    Ok(())
}

fn with_exclusive_db_access<T, F>(db_file: &std::path::Path, action: F) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let _access_guard = db_access_gate()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    close_db_connections_for_path(db_file)?;
    action()
}

/// Runs while every `with_db` call is blocked and all registered connections for
/// `db_file` are closed. The callback must not call `with_db`/`with_db_mut`.
pub(crate) fn with_exclusive_db_snapshot<T, F>(
    db_file: &std::path::Path,
    snapshot: F,
) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    with_exclusive_db_access(db_file, snapshot)
}

/// Replaces database-owned files while all registered DB access is blocked.
/// The callback must not call `with_db`/`with_db_mut`.
pub(crate) fn with_exclusive_db_file_replacement<T, F>(
    db_file: &std::path::Path,
    replace: F,
) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    let _access_guard = db_access_gate()
        .write()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    let _replacement_epoch = DbReplacementEpochGuard::begin();
    close_db_connections_for_path(db_file)?;
    let result = replace();
    DB_MIGRATED.store(false, Ordering::Release);
    result
}

#[cfg(test)]
pub(crate) fn with_test_db<T, F>(f: F) -> rusqlite::Result<T>
where
    F: FnOnce() -> rusqlite::Result<T>,
{
    let previous = {
        let _access_guard = db_access_gate()
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        DB_CONN.with(|slot| {
            let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let previous = std::mem::replace(
                &mut *slot,
                DbConnectionSlot {
                    connection: None,
                    target: DbConnectionTarget::InMemory,
                },
            );
            let setup = Connection::open_in_memory().and_then(|conn| {
                configure_db_connection(&conn)?;
                migrate_db(&conn)?;
                slot.connection = Some(conn);
                Ok(())
            });
            if let Err(err) = setup {
                *slot = previous;
                return Err(err);
            }
            Ok(previous)
        })?
    };
    let result = f();
    let _access_guard = db_access_gate()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    DB_CONN.with(|slot| {
        *slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = previous;
    });
    result
}

#[cfg(test)]
pub(crate) fn with_test_db_path<T, F>(path: &std::path::Path, f: F) -> rusqlite::Result<T>
where
    F: FnOnce() -> rusqlite::Result<T>,
{
    let previous = {
        let _access_guard = db_access_gate()
            .read()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        DB_CONN.with(|slot| {
            let mut slot = slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner());
            let previous = std::mem::replace(
                &mut *slot,
                DbConnectionSlot {
                    connection: None,
                    target: DbConnectionTarget::Path(path.to_path_buf()),
                },
            );
            let setup: rusqlite::Result<()> = (|| {
                if let Some(parent) = path.parent() {
                    std::fs::create_dir_all(parent)
                        .map_err(|_| rusqlite::Error::InvalidPath(path.to_path_buf()))?;
                }
                let conn = Connection::open(path)?;
                configure_db_connection(&conn)?;
                migrate_db(&conn)?;
                slot.connection = Some(conn);
                Ok(())
            })();
            if let Err(err) = setup {
                *slot = previous;
                return Err(err);
            }
            Ok(previous)
        })?
    };
    let result = f();
    let _access_guard = db_access_gate()
        .read()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    DB_CONN.with(|slot| {
        *slot.lock().unwrap_or_else(|poisoned| poisoned.into_inner()) = previous;
    });
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;

    fn db_runtime_test_path(label: &str) -> std::path::PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_nanos())
            .unwrap_or(0);
        std::env::var_os("ZSCLIP_TEST_TEMP_ROOT")
            .map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join(format!("zsclip-{label}-{}-{nonce}.db", std::process::id()))
    }

    #[test]
    fn wal_checkpoint_result_rejects_busy_and_partial_completion() {
        assert!(validate_wal_checkpoint_result(0, -1, -1).is_ok());
        assert!(validate_wal_checkpoint_result(0, 4, 4).is_ok());
        assert!(validate_wal_checkpoint_result(1, 4, 4).is_err());
        assert!(validate_wal_checkpoint_result(0, 4, 3).is_err());
    }

    #[test]
    fn runtime_connection_close_skips_implicit_checkpoint() {
        let db_file = db_runtime_test_path("runtime-checkpoint-policy");
        let conn = Connection::open(&db_file).unwrap();
        configure_db_connection(&conn).unwrap();
        configure_runtime_wal_connection(&conn).unwrap();

        let auto_checkpoint: i64 = conn
            .query_row("PRAGMA wal_autocheckpoint", [], |row| row.get(0))
            .unwrap();
        assert_eq!(auto_checkpoint, 0);
        assert!(conn
            .db_config(DbConfig::SQLITE_DBCONFIG_NO_CKPT_ON_CLOSE)
            .unwrap());

        conn.execute_batch("CREATE TABLE sample(value TEXT); INSERT INTO sample VALUES('ok');")
            .unwrap();
        let wal_file = wal_file_path(&db_file);
        let wal_bytes_before_close = std::fs::metadata(&wal_file).unwrap().len();
        assert!(wal_bytes_before_close > 32);
        drop(conn);
        assert_eq!(
            std::fs::metadata(&wal_file).unwrap().len(),
            wal_bytes_before_close
        );

        let conn = Connection::open(&db_file).unwrap();
        let value: String = conn
            .query_row("SELECT value FROM sample", [], |row| row.get(0))
            .unwrap();
        assert_eq!(value, "ok");
        checkpoint_connection(&conn).unwrap();
        drop(conn);
        for path in [
            db_file.clone(),
            wal_file_path(&db_file),
            shm_file_path(&db_file),
        ] {
            let _ = std::fs::remove_file(path);
        }
    }

    #[test]
    fn generation_checks_do_not_wait_for_an_active_app_data_replacement() {
        let stable_generation = current_app_data_generation();
        assert_eq!(stable_generation & 1, 0);
        let (entered_tx, entered_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let replacement = thread::spawn(move || {
            with_exclusive_app_data_replacement(|| {
                entered_tx.send(()).unwrap();
                release_rx.recv().unwrap();
                Ok(())
            })
            .unwrap();
        });
        entered_rx.recv_timeout(Duration::from_secs(2)).unwrap();

        let (observed_tx, observed_rx) = mpsc::channel();
        let observer = thread::spawn(move || {
            let observed_generation = current_app_data_generation();
            let guarded =
                with_shared_app_data_generation(stable_generation, || "unexpected".to_string());
            observed_tx.send((observed_generation, guarded)).unwrap();
        });
        let (observed_generation, guarded) = observed_rx
            .recv_timeout(Duration::from_millis(500))
            .expect("generation observation must not block on restore");
        assert_eq!(observed_generation & 1, 1);
        assert!(guarded.is_none());

        release_tx.send(()).unwrap();
        observer.join().unwrap();
        replacement.join().unwrap();
        assert_eq!(current_app_data_generation() & 1, 0);
    }

    #[test]
    fn exclusive_replacement_waits_for_work_and_reopens_other_thread_tls_connection() {
        let db_file = db_runtime_test_path("exclusive-replacement");
        let replacement_db_file = db_runtime_test_path("exclusive-replacement-source");
        let replacement_conn = Connection::open(&replacement_db_file).unwrap();
        configure_db_connection(&replacement_conn).unwrap();
        migrate_db(&replacement_conn).unwrap();
        replacement_conn
            .execute(
                "INSERT INTO items(category, kind, preview, signature, source_app) VALUES(0, 'text', 'after restore', 'after', 'test')",
                [],
            )
            .unwrap();
        replacement_conn
            .execute_batch("PRAGMA wal_checkpoint(TRUNCATE);")
            .unwrap();
        drop(replacement_conn);

        let worker_db_file = db_file.clone();
        let (operation_entered_tx, operation_entered_rx) = mpsc::channel();
        let (release_operation_tx, release_operation_rx) = mpsc::channel();
        let (replacement_done_tx, replacement_done_rx) = mpsc::channel();
        let (observed_preview_tx, observed_preview_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            with_test_db_path(&worker_db_file, || {
                with_db(|conn| {
                    conn.execute(
                        "INSERT INTO items(category, kind, preview, signature, source_app) VALUES(0, 'text', 'before restore', 'before', 'test')",
                        [],
                    )?;
                    operation_entered_tx.send(()).unwrap();
                    release_operation_rx.recv().unwrap();
                    Ok(())
                })?;
                replacement_done_rx.recv().unwrap();
                let preview: String = with_db(|conn| {
                    conn.query_row("SELECT preview FROM items LIMIT 1", [], |row| row.get(0))
                })?;
                observed_preview_tx.send(preview).unwrap();
                Ok(())
            })
            .unwrap();
        });
        operation_entered_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap();

        let (replacement_entered_tx, replacement_entered_rx) = mpsc::channel();
        let releaser = thread::spawn(move || {
            assert!(replacement_entered_rx
                .recv_timeout(Duration::from_millis(100))
                .is_err());
            release_operation_tx.send(()).unwrap();
            replacement_entered_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap();
        });
        with_exclusive_db_file_replacement(&db_file, || {
            replacement_entered_tx.send(()).unwrap();
            for sidecar in [
                format!("{}-wal", db_file.to_string_lossy()),
                format!("{}-shm", db_file.to_string_lossy()),
            ] {
                match std::fs::remove_file(&sidecar) {
                    Ok(()) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => return Err(err.to_string()),
                }
            }
            std::fs::remove_file(&db_file).map_err(|err| err.to_string())?;
            std::fs::copy(&replacement_db_file, &db_file).map_err(|err| err.to_string())?;
            Ok(())
        })
        .unwrap();
        replacement_done_tx.send(()).unwrap();

        releaser.join().unwrap();
        worker.join().unwrap();
        assert_eq!(
            observed_preview_rx
                .recv_timeout(Duration::from_secs(2))
                .unwrap(),
            "after restore"
        );
        let _ = std::fs::remove_file(&db_file);
        let _ = std::fs::remove_file(format!("{}-wal", db_file.to_string_lossy()));
        let _ = std::fs::remove_file(format!("{}-shm", db_file.to_string_lossy()));
        let _ = std::fs::remove_file(&replacement_db_file);
        let _ = std::fs::remove_file(format!("{}-wal", replacement_db_file.to_string_lossy()));
        let _ = std::fs::remove_file(format!("{}-shm", replacement_db_file.to_string_lossy()));
    }

    #[test]
    fn write_started_during_replacement_is_aborted_without_touching_new_database() {
        let db_file = db_runtime_test_path("replacement-epoch");
        let replacement_db_file = db_runtime_test_path("replacement-epoch-source");
        let replacement_conn = Connection::open(&replacement_db_file).unwrap();
        configure_db_connection(&replacement_conn).unwrap();
        migrate_db(&replacement_conn).unwrap();
        replacement_conn
            .execute(
                "INSERT INTO items(category, kind, preview, signature, source_app) VALUES(0, 'text', 'restored row', 'restored', 'test')",
                [],
            )
            .unwrap();
        checkpoint_connection(&replacement_conn).unwrap();
        drop(replacement_conn);

        let worker_db_file = db_file.clone();
        let (worker_ready_tx, worker_ready_rx) = mpsc::channel();
        let (start_stale_write_tx, start_stale_write_rx) = mpsc::channel();
        let (stale_write_result_tx, stale_write_result_rx) = mpsc::channel();
        let worker = thread::spawn(move || {
            with_test_db_path(&worker_db_file, || {
                worker_ready_tx.send(()).unwrap();
                start_stale_write_rx.recv().unwrap();
                let result = with_db_mut(|conn| {
                    conn.execute(
                        "INSERT INTO items(category, kind, preview, signature, source_app) VALUES(0, 'text', 'stale row', 'stale', 'test')",
                        [],
                    )?;
                    Ok(())
                });
                stale_write_result_tx.send(result).unwrap();
                Ok(())
            })
            .unwrap();
        });
        worker_ready_rx
            .recv_timeout(Duration::from_secs(2))
            .unwrap();

        with_exclusive_db_file_replacement(&db_file, || {
            start_stale_write_tx.send(()).unwrap();
            let err = stale_write_result_rx
                .recv_timeout(Duration::from_secs(2))
                .expect("stale write must not wait for replacement to finish")
                .expect_err("stale write must be rejected");
            match err {
                rusqlite::Error::SqliteFailure(error, _) => {
                    assert_eq!(error.extended_code, rusqlite::ffi::SQLITE_ABORT);
                }
                other => panic!("unexpected stale write error: {other}"),
            }

            for sidecar in [
                format!("{}-wal", db_file.to_string_lossy()),
                format!("{}-shm", db_file.to_string_lossy()),
            ] {
                match std::fs::remove_file(&sidecar) {
                    Ok(()) => {}
                    Err(err) if err.kind() == std::io::ErrorKind::NotFound => {}
                    Err(err) => return Err(err.to_string()),
                }
            }
            std::fs::remove_file(&db_file).map_err(|err| err.to_string())?;
            std::fs::copy(&replacement_db_file, &db_file).map_err(|err| err.to_string())?;
            Ok(())
        })
        .unwrap();

        worker.join().unwrap();
        let active_conn = Connection::open(&db_file).unwrap();
        let restored_count: i64 = active_conn
            .query_row(
                "SELECT COUNT(*) FROM items WHERE signature='restored'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let stale_count: i64 = active_conn
            .query_row(
                "SELECT COUNT(*) FROM items WHERE signature='stale'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(restored_count, 1);
        assert_eq!(stale_count, 0);
        drop(active_conn);

        let _ = std::fs::remove_file(&db_file);
        let _ = std::fs::remove_file(format!("{}-wal", db_file.to_string_lossy()));
        let _ = std::fs::remove_file(format!("{}-shm", db_file.to_string_lossy()));
        let _ = std::fs::remove_file(&replacement_db_file);
        let _ = std::fs::remove_file(format!("{}-wal", replacement_db_file.to_string_lossy()));
        let _ = std::fs::remove_file(format!("{}-shm", replacement_db_file.to_string_lossy()));
    }

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
    fn native_clip_list_items_for_query_supports_near_search_and_kind_filter() {
        with_test_db(|| {
            with_db_mut(|conn| {
                for (preview, kind) in [
                    ("alpha context", "text"),
                    ("invoice needle", "text"),
                    ("omega context", "text"),
                    ("invoice image", "image"),
                    ("outside row", "text"),
                ] {
                    conn.execute(
                        "INSERT INTO items(category, kind, preview, signature, text_data, source_app, created_at) VALUES(0, ?, ?, ?, ?, 'Notes', datetime('now'))",
                        rusqlite::params![kind, preview, preview, preview],
                    )?;
                }
                Ok(())
            })?;

            let near = native_clip_list_items_for_query(
                0,
                0,
                crate::app_core::ClipKindFilter::All,
                "附近:needle",
                10,
            )?;
            let previews = near
                .iter()
                .map(|item| item.preview.as_str())
                .collect::<Vec<_>>();
            assert!(previews.contains(&"invoice needle"));
            assert!(previews.contains(&"alpha context"));
            assert!(previews.contains(&"omega context"));

            let images = native_clip_list_items_for_query(
                0,
                0,
                crate::app_core::ClipKindFilter::Image,
                "near::invoice",
                10,
            )?;
            assert_eq!(images.len(), 1);
            assert_eq!(images[0].preview, "invoice image");
            assert_eq!(images[0].kind, crate::app_core::ClipKind::Image);
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
