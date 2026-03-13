use std::cell::RefCell;
use std::sync::OnceLock;

use rusqlite::Connection;

thread_local! {
    static DB_CONN: RefCell<Option<Connection>> = const { RefCell::new(None) };
}

static DB_MIGRATED: OnceLock<()> = OnceLock::new();

fn migrate_db(conn: &Connection) -> rusqlite::Result<()> {
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
        ",
    )?;
    let _ = conn.execute("ALTER TABLE items ADD COLUMN file_paths TEXT", []);
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
