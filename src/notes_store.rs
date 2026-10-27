//! Local, independently stored notes. Bodies are loaded one at a time.
use rusqlite::{params, Connection, OptionalExtension};
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

pub(crate) const MAX_NOTE_BYTES: usize = 1024 * 1024;
const MAX_NOTES: i64 = 1000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NoteSummary {
    pub id: i64,
    pub title: String,
}

pub(crate) struct NotesStore(Connection);

fn title(body: &str) -> String {
    body.lines()
        .find(|line| !line.trim().is_empty())
        .unwrap_or("")
        .trim()
        .chars()
        .filter(|ch| !ch.is_control())
        .take(64)
        .collect()
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis()
        .min(i64::MAX as u128) as i64
}

impl NotesStore {
    pub fn open(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let connection = Connection::open(path).map_err(|error| error.to_string())?;
        connection
            .busy_timeout(Duration::from_millis(350))
            .map_err(|error| error.to_string())?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL;
            PRAGMA cache_size=-512; PRAGMA mmap_size=0;
            CREATE TABLE IF NOT EXISTS notes(id INTEGER PRIMARY KEY, title TEXT NOT NULL,
                body TEXT NOT NULL, updated_at INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS note_preferences(key TEXT PRIMARY KEY, value INTEGER NOT NULL);")
            .map_err(|error| error.to_string())?;
        Ok(Self(connection))
    }

    pub fn list(&self) -> Result<Vec<NoteSummary>, String> {
        let mut query = self
            .0
            .prepare("SELECT id,substr(title,1,64) FROM notes ORDER BY id DESC LIMIT 1000")
            .map_err(|error| error.to_string())?;
        let rows = query
            .query_map([], |row| {
                Ok(NoteSummary {
                    id: row.get(0)?,
                    title: row.get(1)?,
                })
            })
            .map_err(|error| error.to_string())?;
        rows.collect::<Result<_, _>>()
            .map_err(|error| error.to_string())
    }

    pub fn read(&self, id: i64) -> Result<String, String> {
        let bytes: i64 = self
            .0
            .query_row(
                "SELECT length(CAST(body AS BLOB)) FROM notes WHERE id=?",
                [id],
                |row| row.get(0),
            )
            .map_err(|error| error.to_string())?;
        if bytes > MAX_NOTE_BYTES as i64 {
            return Err("便签内容超过 1 MB，未载入编辑器。".into());
        }
        let body: String = self
            .0
            .query_row("SELECT body FROM notes WHERE id=?", [id], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if body.contains('\0') {
            return Err("便签含有不支持的空字符，原内容未修改。".into());
        }
        Ok(body)
    }

    pub fn create(&mut self) -> Result<i64, String> {
        let transaction = self.0.transaction().map_err(|error| error.to_string())?;
        let count: i64 = transaction
            .query_row("SELECT count(*) FROM notes", [], |row| row.get(0))
            .map_err(|error| error.to_string())?;
        if count >= MAX_NOTES {
            return Err("便签数量已达到 1000 条，请先整理已有便签。".into());
        }
        transaction
            .execute(
                "INSERT INTO notes(title,body,updated_at) VALUES('','',?)",
                [now()],
            )
            .map_err(|error| error.to_string())?;
        let id = transaction.last_insert_rowid();
        transaction.commit().map_err(|error| error.to_string())?;
        Ok(id)
    }

    pub fn save(&mut self, id: i64, body: &str) -> Result<(), String> {
        if body.contains('\0') {
            return Err("便签不支持空字符，原内容保持不变。".into());
        }
        if body.len() > MAX_NOTE_BYTES {
            return Err("每条便签最多保存 1 MB 文本，原内容保持不变。".into());
        }
        let changed = self
            .0
            .execute(
                "UPDATE notes SET title=?,body=?,updated_at=? WHERE id=?",
                params![title(body), body, now(), id],
            )
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("便签已不存在，编辑内容仍保留在窗口中。".into());
        }
        Ok(())
    }

    pub fn delete(&mut self, id: i64) -> Result<(), String> {
        let changed = self
            .0
            .execute("DELETE FROM notes WHERE id=?", [id])
            .map_err(|error| error.to_string())?;
        if changed != 1 {
            return Err("便签已不存在。".into());
        }
        Ok(())
    }

    pub fn selected(&self) -> Option<i64> {
        self.0
            .query_row(
                "SELECT value FROM note_preferences WHERE key='selected'",
                [],
                |row| row.get(0),
            )
            .optional()
            .ok()
            .flatten()
    }

    pub fn select(&self, id: i64) -> Result<(), String> {
        self.0
            .execute(
                "INSERT INTO note_preferences(key,value) VALUES('selected',?)
            ON CONFLICT(key) DO UPDATE SET value=excluded.value",
                [id],
            )
            .map(|_| ())
            .map_err(|error| error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "zsclip-notes-{}-{}-{name}.db",
            std::process::id(),
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    #[test]
    fn notes_are_independent_persistent_and_titles_do_not_change_bodies() {
        let path = path("roundtrip");
        let (first, second);
        {
            let mut store = NotesStore::open(&path).unwrap();
            first = store.create().unwrap();
            second = store.create().unwrap();
            store
                .save(first, "  工作记录\r\n第一项😀\r\n第二项")
                .unwrap();
            store.save(second, "独立便签").unwrap();
            store.select(first).unwrap();
            assert_eq!(
                store
                    .list()
                    .unwrap()
                    .iter()
                    .map(|note| note.id)
                    .collect::<Vec<_>>(),
                [second, first]
            );
            assert_eq!(store.list().unwrap()[1].title, "工作记录");
        }
        {
            let mut store = NotesStore::open(&path).unwrap();
            assert_eq!(store.selected(), Some(first));
            assert_eq!(
                store.read(first).unwrap(),
                "  工作记录\r\n第一项😀\r\n第二项"
            );
            store.delete(second).unwrap();
            assert_eq!(store.list().unwrap().len(), 1);
            assert!(store.read(second).is_err());
            assert_eq!(
                store.read(first).unwrap(),
                "  工作记录\r\n第一项😀\r\n第二项"
            );
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn rejected_updates_preserve_existing_text_and_corrupt_files_are_not_replaced() {
        let path = path("limits");
        {
            let mut store = NotesStore::open(&path).unwrap();
            let id = store.create().unwrap();
            store.save(id, "保留这条内容").unwrap();
            assert!(store.save(id, &"x".repeat(MAX_NOTE_BYTES + 1)).is_err());
            assert_eq!(store.read(id).unwrap(), "保留这条内容");
            assert!(store.save(id + 100, "missing").is_err());
        }
        std::fs::remove_file(&path).unwrap();
        std::fs::write(&path, b"not-a-notes-database").unwrap();
        assert!(NotesStore::open(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"not-a-notes-database");
        std::fs::remove_file(path).unwrap();
    }
}
