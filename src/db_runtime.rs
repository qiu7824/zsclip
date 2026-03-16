use std::cell::RefCell;
use std::sync::OnceLock;

use rusqlite::Connection;

thread_local! {
    static DB_CONN: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

static DB_MIGRATED: OnceLock<()> = OnceLock::new();

fn migrate_db(conn: &Connection) -> rusqlite::Result<()> {
    let _ = conn.pragma_update(None, "journal_mode", "WAL");
    let _ = conn.pragma_update(None, "synchronous", "NORMAL");
    let _ = conn.pragma_update(None, "temp_store", "MEMORY");
    let _ = conn.pragma_update(None, "foreign_keys", "ON");
    let _ = conn.pragma_update(None, "cache_size", -8192i32);
    let _ = conn.pragma_update(None, "mmap_size", 134_217_728i64);
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS items(
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            category INTEGER NOT NULL,
            kind TEXT NOT NULL,
            preview TEXT NOT NULL,
            text_data TEXT,
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
            name TEXT NOT NULL UNIQUE,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_items_category_pinned_id ON items(category, pinned, id DESC);
        CREATE INDEX IF NOT EXISTS idx_items_group_id ON items(group_id, id DESC);
        CREATE INDEX IF NOT EXISTS idx_clip_groups_sort ON clip_groups(sort_order, id);
        ",
    )?;
    let _ = conn.execute("ALTER TABLE items ADD COLUMN file_paths TEXT", []);
    let _ = conn.execute("ALTER TABLE items ADD COLUMN image_path TEXT", []);
    let _ = conn.execute("ALTER TABLE items ADD COLUMN group_id INTEGER NOT NULL DEFAULT 0", []);
    Ok(())
}

fn ensure_connection(cell: &RefCell<Option<Connection>>) -> rusqlite::Result<()> {
    let mut slot = cell.borrow_mut();
    if slot.is_none() {
        *slot = Some(Connection::open(crate::app::db_file())?);
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
        let conn = slot.as_ref().expect("db connection initialized");
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
        let conn = slot.as_mut().expect("db connection initialized");
        f(conn)
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
