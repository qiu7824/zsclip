//! Many-to-many membership; the original clipboard item remains authoritative.
use rusqlite::{params, Connection};
use std::collections::{BTreeSet, HashMap};

pub(crate) fn assign(tab_id: &str, ids: &[i64]) -> rusqlite::Result<()> {
    if tab_id.is_empty() || tab_id.len() > 256 { return Err(rusqlite::Error::InvalidQuery); }
    crate::db_runtime::with_db_mut(|db| {
        let tx = db.unchecked_transaction()?;
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            let allowed: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM items WHERE id=? AND
                (kind NOT IN ('text','phrase') OR (zsclip_is_protected(COALESCE(NULLIF(text_data,''),preview,''))=0
                AND zsclip_is_protected_html(rich_text_html)=0 AND zsclip_is_protected(phrase_title)=0)))", [id], |r| r.get(0))?;
            if !allowed { return Err(rusqlite::Error::QueryReturnedNoRows); }
            tx.execute("INSERT OR IGNORE INTO clip_tab_items(tab_id,item_id) VALUES(?1,?2)", params![tab_id,id])?;
        }
        tx.commit()
    })
}

pub(crate) fn remove(tab_id: &str, ids: &[i64]) -> rusqlite::Result<()> {
    crate::db_runtime::with_db_mut(|db| {
        let tx = db.unchecked_transaction()?;
        for id in ids {
            tx.execute("DELETE FROM clip_tab_items WHERE tab_id=?1 AND item_id=?2", params![tab_id,id])?;
        }
        tx.commit()
    })
}

pub(crate) fn counts(ids: &[i64]) -> rusqlite::Result<HashMap<String,usize>> {
    crate::db_runtime::with_db(|db| {
        let mut counts = HashMap::new();
        let mut query = db.prepare("SELECT tab_id FROM clip_tab_items WHERE item_id=?")?;
        for id in ids.iter().copied().collect::<BTreeSet<_>>() {
            for row in query.query_map([id], |r| r.get::<_,String>(0))? {
                *counts.entry(row?).or_default() += 1;
            }
        }
        Ok(counts)
    })
}

pub(crate) fn transfer(db: &Connection, old_ids: &[i64], new_id: i64) -> rusqlite::Result<()> {
    for id in old_ids {
        db.execute("INSERT OR IGNORE INTO clip_tab_items(tab_id,item_id)
            SELECT tab_id,?1 FROM clip_tab_items WHERE item_id=?2", params![new_id,id])?;
    }
    Ok(())
}

pub(crate) fn merge(old_ids: &[i64], keep_id: i64) -> rusqlite::Result<()> {
    crate::db_runtime::with_db_mut(|db| {
        let tx = db.unchecked_transaction()?;
        transfer(&tx, old_ids, keep_id)?;
        tx.commit()
    })
}

pub(crate) fn delete_memberships(tab_ids: &[String]) -> rusqlite::Result<()> {
    crate::db_runtime::with_db_mut(|db| {
        let tx = db.unchecked_transaction()?;
        for id in tab_ids { tx.execute("DELETE FROM clip_tab_items WHERE tab_id=?", [id])?; }
        tx.commit()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn tag_memberships_survive_database_reopen_and_snapshot() {
        let root = std::env::var_os("ZSCLIP_TEST_TEMP_ROOT").map(std::path::PathBuf::from)
            .unwrap_or_else(std::env::temp_dir).join(format!("zsclip-tag-persistence-{}",std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path=root.join("source.sqlite3");
        let backup=root.join("snapshot.sqlite3");
        crate::db_runtime::with_test_db_path(&path,|| {
            crate::db_runtime::with_db(|db|db.execute_batch("INSERT INTO items(id,category,kind,preview,text_data)
                VALUES(1,0,'text','saved','saved');"))?;
            assign("saved-tag",&[1])?;
            crate::db_runtime::with_db(|db| { db.execute("VACUUM INTO ?",[backup.to_str().unwrap()])?; Ok(()) })
        }).unwrap();
        for file in [&path,&backup] {
            crate::db_runtime::with_test_db_path(file,|| {
                assert_eq!(counts(&[1])?["saved-tag"],1);
                Ok(())
            }).unwrap();
        }
    }
    #[test]
    fn memberships_are_independent_idempotent_and_deleted_without_deleting_items() {
        crate::db_runtime::with_test_db(|| {
            crate::db_runtime::with_db(|db| db.execute_batch("INSERT INTO items(id,category,kind,preview,text_data)
                VALUES(1,0,'text','one','one'),(2,1,'phrase','two','two');"))?;
            assign("work", &[1,2])?; assign("work", &[1,1])?; assign("home", &[1])?;
            assert_eq!(counts(&[1,2])?["work"],2);
            remove("work", &[1])?;
            assert_eq!(counts(&[1])?["home"],1);
            assert!(!counts(&[1])?.contains_key("work"));
            assert!(assign("invalid-batch", &[1,99]).is_err());
            assert!(!counts(&[1])?.contains_key("invalid-batch"));
            delete_memberships(&["work".into()])?;
            crate::db_runtime::with_db(|db| {
                assert_eq!(db.query_row("SELECT count(*) FROM items",[],|r|r.get::<_,i64>(0))?,2);
                db.execute("DELETE FROM items WHERE id=1",[])?;
                assert_eq!(db.query_row("SELECT count(*) FROM clip_tab_items",[],|r|r.get::<_,i64>(0))?,0);
                Ok(())
            })
        }).unwrap();
    }
}
