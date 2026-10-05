//! Record identity and protection checks shared by native VV presentation and delivery.
use crate::app_core::{ClipItem, NativeHostClipListItemProjection};
use rusqlite::OptionalExtension;

#[derive(Clone)]
pub(crate) struct NativeVvSnapshot {
    pub items: Vec<NativeHostClipListItemProjection>,
    pub category: i64,
    pub group_id: i64,
    pub data_generation: u64,
    pub protection_revision: String,
}

impl NativeVvSnapshot {
    pub(crate) fn capture(category: i64, group_id: i64) -> Result<Self, String> {
        let data_generation = crate::db_runtime::current_app_data_generation();
        crate::db_runtime::with_shared_app_data_generation(data_generation, || {
            let protection_revision = crate::db_runtime::search_protection_revision().map_err(|e| e.to_string())?;
            let items = crate::db_runtime::with_search_protection(|| {
                crate::db_runtime::native_clip_list_items_for_group(category, group_id, 9)
            }).map_err(|e| e.to_string())?;
            let snapshot = Self { items, category, group_id, data_generation, protection_revision };
            snapshot.validate()?;
            Ok(snapshot)
        }).unwrap_or_else(|| Err("VV history changed while opening".into()))
    }

    pub(crate) fn validate(&self) -> Result<(), String> {
        if crate::db_runtime::current_app_data_generation() != self.data_generation || self.data_generation & 1 != 0 {
            return Err("VV history was replaced".into());
        }
        if crate::db_runtime::search_protection_revision().map_err(|e| e.to_string())? != self.protection_revision {
            return Err("VV protection state changed".into());
        }
        Ok(())
    }

    pub(crate) fn load_item(&self, index: usize) -> Result<ClipItem, String> {
        let id = self.items.get(index).ok_or("VV candidate is no longer available")?.id;
        crate::db_runtime::with_shared_app_data_generation(self.data_generation, || {
            self.validate()?;
            let item = crate::db_runtime::with_search_protection(|| {
                let identity = crate::db_runtime::with_db(|conn| {
                    conn.query_row("SELECT category, group_id FROM items WHERE id=?1", [id], |row| {
                        Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?))
                    }).optional()
                })?;
                if !identity.is_some_and(|(category, group)| category == self.category && (self.group_id <= 0 || group == self.group_id)) {
                    return Ok(None);
                }
                crate::db_runtime::native_clip_item(id)
            }).map_err(|e| e.to_string())?.ok_or("VV candidate was deleted, moved or protected")?;
            self.validate()?;
            Ok(item)
        }).unwrap_or_else(|| Err("VV history was replaced".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selection_keeps_displayed_id_after_new_capture_and_reordering() {
        crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
            let id = crate::db_runtime::insert_native_clipboard_text(0, "displayed body", "VV test")?.item_id.unwrap();
            let snapshot = NativeVvSnapshot::capture(0, 0).unwrap();
            crate::db_runtime::insert_native_clipboard_text(0, "new clipboard body", "VV test")?;
            let selected = snapshot.load_item(0).unwrap();
            assert_eq!(selected.id, id);
            assert_eq!(selected.text.as_deref(), Some("displayed body"));
            crate::db_runtime::with_db(|conn| conn.execute("UPDATE items SET category=1 WHERE id=?1", [id]))?;
            assert!(snapshot.load_item(0).is_err());
            Ok(())
        })).unwrap();
    }

    #[test]
    fn full_body_is_not_preview_and_stale_or_deleted_candidates_are_rejected() {
        crate::db_runtime::with_test_protected_texts(&[], || crate::db_runtime::with_test_db(|| {
            let body = format!("{}\n\n末尾🙂", "long body ".repeat(1600));
            let id = crate::db_runtime::insert_native_clipboard_text(1, &body, "VV test")?.item_id.unwrap();
            let snapshot = NativeVvSnapshot::capture(1, 0).unwrap();
            assert!(snapshot.items[0].preview.chars().count() < body.chars().count());
            assert_eq!(snapshot.load_item(0).unwrap().text.as_deref(), Some(body.as_str()));
            let mut stale = snapshot.clone();
            stale.data_generation = stale.data_generation.wrapping_add(2);
            assert!(stale.load_item(0).is_err());
            crate::db_runtime::with_test_protected_texts(&[body.as_str()], || {
                assert!(snapshot.validate().is_err());
                assert!(snapshot.load_item(0).is_err());
            });
            crate::db_runtime::with_db(|conn| conn.execute("DELETE FROM items WHERE id=?1", [id]))?;
            assert!(snapshot.load_item(0).is_err());
            Ok(())
        })).unwrap();
    }
}
