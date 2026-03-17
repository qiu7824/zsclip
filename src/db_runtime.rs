use std::cell::RefCell;
use std::sync::OnceLock;

use rusqlite::{Connection, OptionalExtension};

thread_local! {
    static DB_CONN: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

static DB_MIGRATED: OnceLock<()> = OnceLock::new();

fn clip_groups_has_category_column(conn: &Connection) -> rusqlite::Result<bool> {
    let mut stmt = conn.prepare("PRAGMA table_info(clip_groups)")?;
    let mut rows = stmt.query([])?;
    while let Some(row) = rows.next()? {
        let name: String = row.get(1)?;
        if name.eq_ignore_ascii_case("category") {
            return Ok(true);
        }
    }
    Ok(false)
}

fn migrate_clip_groups_schema(conn: &Connection) -> rusqlite::Result<()> {
    if !clip_groups_has_category_column(conn)? {
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
            category INTEGER NOT NULL DEFAULT 0,
            name TEXT NOT NULL,
            sort_order INTEGER NOT NULL DEFAULT 0,
            created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
        );
        CREATE INDEX IF NOT EXISTS idx_items_category_pinned_id ON items(category, pinned, id DESC);
        CREATE INDEX IF NOT EXISTS idx_items_group_id ON items(group_id, id DESC);
        CREATE INDEX IF NOT EXISTS idx_clip_groups_category_sort ON clip_groups(category, sort_order, id);
        CREATE UNIQUE INDEX IF NOT EXISTS idx_clip_groups_category_name ON clip_groups(category, name);
        ",
    )?;
    let _ = conn.execute("ALTER TABLE items ADD COLUMN file_paths TEXT", []);
    let _ = conn.execute("ALTER TABLE items ADD COLUMN image_path TEXT", []);
    let _ = conn.execute("ALTER TABLE items ADD COLUMN group_id INTEGER NOT NULL DEFAULT 0", []);
    migrate_clip_groups_schema(conn)?;
    migrate_phrase_group_assignments(conn)?;
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
