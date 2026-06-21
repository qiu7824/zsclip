use crate::db_runtime::with_db;
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::fs;

pub(crate) const MULTI_SYNC_PROTOCOL: &str = "ZSCLIP_MULTI_SYNC_V1";
pub(crate) const MULTI_SYNC_VERSION: u32 = 1;
pub(crate) const MULTI_SYNC_IMAGE_MAX_BYTES: usize = 10 * 1024 * 1024;
pub(crate) const MULTI_SYNC_MANIFEST_FILE_NAME: &str = "zsSyncClipboard.json";
pub(crate) const MULTI_SYNC_LEGACY_MANIFEST_FILE_NAME: &str = "SyncClipboard.json";

pub(crate) fn transport_status_label(webdav_enabled: bool, lan_enabled: bool) -> &'static str {
    match (webdav_enabled, lan_enabled) {
        (_, true) => "多端同步：当前方案为局域网；扫码绑定后在同一 Wi-Fi 内传输。",
        (true, false) => "多端同步：当前方案为 WebDAV；适合跨网络同步。",
        (false, false) => "多端同步：未选择同步方案。请选择 WebDAV 或局域网。",
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct RemoteTextImportOutcome {
    pub(crate) imported: bool,
    pub(crate) signature: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MultiSyncItem {
    pub(crate) id: String,
    #[serde(rename = "type")]
    pub(crate) kind: String,
    pub(crate) hash: String,
    pub(crate) preview: String,
    pub(crate) content: Option<String>,
    #[serde(rename = "hasData")]
    pub(crate) has_data: bool,
    #[serde(rename = "dataName", skip_serializing_if = "Option::is_none")]
    pub(crate) data_name: Option<String>,
    pub(crate) size: u64,
    pub(crate) source_app: String,
    pub(crate) created_at: String,
    pub(crate) width: Option<i64>,
    pub(crate) height: Option<i64>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct MultiSyncManifest {
    pub(crate) protocol: String,
    pub(crate) version: u32,
    pub(crate) transport: String,
    pub(crate) clip: Option<MultiSyncItem>,
}

pub(crate) struct MultiSyncDbRow {
    pub(crate) id: i64,
    pub(crate) kind: String,
    pub(crate) preview: String,
    pub(crate) text: String,
    pub(crate) source_app: String,
    pub(crate) image_path: String,
    pub(crate) image_data_len: i64,
    pub(crate) width: i64,
    pub(crate) height: i64,
    pub(crate) created_at: String,
}

pub(crate) fn latest_manifest(transport: &str) -> rusqlite::Result<MultiSyncManifest> {
    Ok(MultiSyncManifest {
        protocol: MULTI_SYNC_PROTOCOL.to_string(),
        version: MULTI_SYNC_VERSION,
        transport: transport.to_string(),
        clip: load_latest_item()?,
    })
}

pub(crate) fn import_remote_text_clip(
    manifest: &MultiSyncManifest,
) -> rusqlite::Result<Option<RemoteTextImportOutcome>> {
    if manifest.protocol != MULTI_SYNC_PROTOCOL {
        return Ok(None);
    }
    let Some(clip) = manifest.clip.as_ref() else {
        return Ok(None);
    };
    if clip.kind != "text" {
        return Ok(None);
    }
    let Some(content) = clip
        .content
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        return Ok(None);
    };
    let signature = remote_clip_signature(&manifest.transport, clip);
    let preview = if clip.preview.trim().is_empty() {
        content.chars().take(80).collect::<String>()
    } else {
        clip.preview.trim().chars().take(160).collect::<String>()
    };
    let source_app = if clip.source_app.trim().is_empty() {
        format!("WebDAV: {}", manifest.transport)
    } else {
        format!("WebDAV: {}", clip.source_app.trim())
    };
    let inserted = with_db(|conn| {
        let exists: Option<i64> = conn
            .query_row(
                "SELECT id FROM items WHERE category=0 AND signature=? LIMIT 1",
                params![signature],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_some() {
            return Ok(false);
        }
        conn.execute(
            "INSERT INTO items(category, kind, preview, signature, text_data, source_app, pinned, group_id)
             VALUES(0, 'text', ?, ?, ?, ?, 0, 0)",
            params![preview, signature, content, source_app],
        )?;
        Ok(true)
    })?;
    Ok(Some(RemoteTextImportOutcome {
        imported: inserted,
        signature,
    }))
}

pub(crate) fn import_remote_image_clip(
    manifest: &MultiSyncManifest,
    png_bytes: &[u8],
) -> rusqlite::Result<Option<RemoteTextImportOutcome>> {
    if manifest.protocol != MULTI_SYNC_PROTOCOL {
        return Ok(None);
    }
    let Some(clip) = manifest.clip.as_ref() else {
        return Ok(None);
    };
    if clip.kind != "image" || clip.data_name.is_none() {
        return Ok(None);
    }
    if png_bytes.is_empty() || png_bytes.len() > MULTI_SYNC_IMAGE_MAX_BYTES {
        return Ok(None);
    }
    let Some((rgba, width, height)) = decode_png_to_rgba(png_bytes) else {
        return Ok(None);
    };
    let signature = remote_clip_signature(&manifest.transport, clip);
    let preview = if clip.preview.trim().is_empty() {
        clip.data_name
            .as_deref()
            .unwrap_or("WebDAV 图片")
            .trim()
            .chars()
            .take(160)
            .collect::<String>()
    } else {
        clip.preview.trim().chars().take(160).collect::<String>()
    };
    let source_app = if clip.source_app.trim().is_empty() {
        format!("WebDAV: {}", manifest.transport)
    } else {
        format!("WebDAV: {}", clip.source_app.trim())
    };
    let inserted = with_db(|conn| {
        let exists: Option<i64> = conn
            .query_row(
                "SELECT id FROM items WHERE category=0 AND signature=? LIMIT 1",
                params![signature],
                |row| row.get(0),
            )
            .optional()?;
        if exists.is_some() {
            return Ok(false);
        }
        conn.execute(
            "INSERT INTO items(category, kind, preview, signature, image_data, image_width, image_height, source_app, pinned, group_id)
             VALUES(0, 'image', ?, ?, ?, ?, ?, ?, 0, 0)",
            params![preview, signature, rgba, width as i64, height as i64, source_app],
        )?;
        Ok(true)
    })?;
    Ok(Some(RemoteTextImportOutcome {
        imported: inserted,
        signature,
    }))
}

pub(crate) fn remote_clip_signature(transport: &str, clip: &MultiSyncItem) -> String {
    format!(
        "multi:{}:{}:{}:{}",
        transport.trim().if_empty("unknown"),
        clip.id.trim(),
        clip.hash.trim(),
        clip.size
    )
}

trait IfEmpty {
    fn if_empty<'a>(&'a self, fallback: &'a str) -> &'a str;
}

impl IfEmpty for str {
    fn if_empty<'a>(&'a self, fallback: &'a str) -> &'a str {
        if self.is_empty() {
            fallback
        } else {
            self
        }
    }
}

pub(crate) fn load_latest_item() -> rusqlite::Result<Option<MultiSyncItem>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, kind, COALESCE(preview, ''), COALESCE(text_data, ''), \
             COALESCE(source_app, ''), COALESCE(image_path, ''), \
             COALESCE(length(image_data), 0), image_width, image_height, \
             COALESCE(created_at, '') \
             FROM items WHERE category=0 AND kind IN ('text', 'phrase', 'image') \
             ORDER BY id DESC LIMIT 50",
        )?;
        let rows = stmt.query_map([], |row| {
            Ok(MultiSyncDbRow {
                id: row.get(0)?,
                kind: row.get(1)?,
                preview: row.get(2)?,
                text: row.get(3)?,
                source_app: row.get(4)?,
                image_path: row.get(5)?,
                image_data_len: row.get(6)?,
                width: row.get(7)?,
                height: row.get(8)?,
                created_at: row.get(9)?,
            })
        })?;
        for row in rows {
            if let Some(item) = item_from_db_row(row?) {
                return Ok(Some(item));
            }
        }
        Ok(None)
    })
}

pub(crate) fn item_from_db_row(row: MultiSyncDbRow) -> Option<MultiSyncItem> {
    match row.kind.as_str() {
        "text" | "phrase" => {
            let text = row.text.trim().to_string();
            if text.is_empty() {
                return None;
            }
            let preview = if row.preview.trim().is_empty() {
                text.chars().take(80).collect()
            } else {
                row.preview
            };
            Some(MultiSyncItem {
                id: format!("db:text:{}", row.id),
                kind: "text".to_string(),
                hash: format!("md5:{:x}", md5::compute(text.as_bytes())),
                preview,
                size: text.len() as u64,
                content: Some(text),
                has_data: false,
                data_name: None,
                source_app: row.source_app,
                created_at: row.created_at,
                width: None,
                height: None,
            })
        }
        "image" => {
            let size = readable_image_size(&row.image_path, row.image_data_len)?;
            let preview = if row.preview.trim().is_empty() {
                format!("image {}", row.id)
            } else {
                row.preview
            };
            Some(MultiSyncItem {
                id: format!("db:image:{}", row.id),
                kind: "image".to_string(),
                hash: format!("image:{}:{}", row.id, size),
                preview,
                size,
                content: None,
                has_data: true,
                data_name: Some(image_data_name(row.id)),
                source_app: row.source_app,
                created_at: row.created_at,
                width: (row.width > 0).then_some(row.width),
                height: (row.height > 0).then_some(row.height),
            })
        }
        _ => None,
    }
}

pub(crate) fn image_data_name(id: i64) -> String {
    format!("zsclip_image_{id}.png")
}

pub(crate) fn image_id_from_data_name(name: &str) -> Option<i64> {
    let stem = name
        .trim()
        .strip_prefix("zsclip_image_")?
        .strip_suffix(".png")?;
    let id = stem.parse::<i64>().ok()?;
    (id > 0).then_some(id)
}

pub(crate) fn load_image_png(id: i64) -> rusqlite::Result<Option<Vec<u8>>> {
    let row = with_db(|conn| {
        conn.query_row(
            "SELECT image_data, COALESCE(image_path, ''), image_width, image_height \
             FROM items WHERE category=0 AND kind='image' AND id=?",
            params![id],
            |row| {
                Ok((
                    row.get::<_, Option<Vec<u8>>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, i64>(2)?,
                    row.get::<_, i64>(3)?,
                ))
            },
        )
        .optional()
    })?;
    let Some((image_data, image_path, width, height)) = row else {
        return Ok(None);
    };
    if !image_path.trim().is_empty() {
        if let Ok(bytes) = fs::read(image_path.trim()) {
            if bytes.len() <= MULTI_SYNC_IMAGE_MAX_BYTES
                && png_dimensions_from_bytes(&bytes).is_some()
            {
                return Ok(Some(bytes));
            }
        }
    }
    let Some(bytes) = image_data else {
        return Ok(None);
    };
    let Some(encoded) = encode_rgba_png_bytes(&bytes, width as u32, height as u32) else {
        return Ok(None);
    };
    if encoded.len() > MULTI_SYNC_IMAGE_MAX_BYTES {
        return Ok(None);
    }
    Ok(Some(encoded))
}

fn readable_image_size(image_path: &str, image_data_len: i64) -> Option<u64> {
    let image_path = image_path.trim();
    if !image_path.is_empty() {
        if let Ok(meta) = fs::metadata(image_path) {
            if meta.is_file() && meta.len() > 0 && meta.len() <= MULTI_SYNC_IMAGE_MAX_BYTES as u64 {
                return Some(meta.len());
            }
        }
    }
    if image_data_len > 0 {
        return Some(image_data_len as u64);
    }
    None
}

fn encode_rgba_png_bytes(bytes: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || bytes.len() != width as usize * height as usize * 4 {
        return None;
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(bytes).ok()?;
    }
    Some(out)
}

fn png_dimensions_from_bytes(bytes: &[u8]) -> Option<(u32, u32)> {
    let cursor = std::io::Cursor::new(bytes);
    let decoder = png::Decoder::new(cursor);
    let reader = decoder.read_info().ok()?;
    let info = reader.info();
    Some((info.width, info.height))
}

fn decode_png_to_rgba(bytes: &[u8]) -> Option<(Vec<u8>, u32, u32)> {
    let cursor = std::io::Cursor::new(bytes);
    let decoder = png::Decoder::new(cursor);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()];
    let info = reader.next_frame(&mut buf).ok()?;
    let data = &buf[..info.buffer_size()];
    let rgba = match info.color_type {
        png::ColorType::Rgba => data.to_vec(),
        png::ColorType::Rgb => {
            let mut out = Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
            for chunk in data.chunks_exact(3) {
                out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
            }
            out
        }
        png::ColorType::GrayscaleAlpha => {
            let mut out = Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
            for chunk in data.chunks_exact(2) {
                out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
            }
            out
        }
        png::ColorType::Grayscale => {
            let mut out = Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
            for v in data {
                out.extend_from_slice(&[*v, *v, *v, 255]);
            }
            out
        }
        _ => return None,
    };
    Some((rgba, info.width, info.height))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bdd_text_manifest_keeps_inline_content() {
        let item = item_from_db_row(MultiSyncDbRow {
            id: 12,
            kind: "text".to_string(),
            preview: "".to_string(),
            text: " hello ".to_string(),
            source_app: "notepad.exe".to_string(),
            image_path: "".to_string(),
            image_data_len: 0,
            width: 0,
            height: 0,
            created_at: "2026-06-02 10:00:00".to_string(),
        })
        .unwrap();

        assert_eq!(item.id, "db:text:12");
        assert_eq!(item.kind, "text");
        assert_eq!(item.content.as_deref(), Some("hello"));
        assert!(!item.has_data);
        assert_eq!(item.data_name, None);
        assert!(item.hash.starts_with("md5:"));
    }

    #[test]
    fn bdd_image_manifest_points_to_lazy_file_data() {
        let item = item_from_db_row(MultiSyncDbRow {
            id: 9,
            kind: "image".to_string(),
            preview: "shot".to_string(),
            text: "".to_string(),
            source_app: "snip.exe".to_string(),
            image_path: "".to_string(),
            image_data_len: 2048,
            width: 640,
            height: 360,
            created_at: "2026-06-02 10:00:00".to_string(),
        })
        .unwrap();

        assert_eq!(item.id, "db:image:9");
        assert_eq!(item.kind, "image");
        assert_eq!(item.content, None);
        assert!(item.has_data);
        assert_eq!(item.data_name.as_deref(), Some("zsclip_image_9.png"));
        assert_eq!(image_id_from_data_name("zsclip_image_9.png"), Some(9));
        assert_eq!(image_id_from_data_name("../zsclip_image_9.png"), None);
    }

    #[test]
    fn bdd_skips_unreadable_image_rows() {
        assert!(item_from_db_row(MultiSyncDbRow {
            id: 3,
            kind: "image".to_string(),
            preview: "missing".to_string(),
            text: "".to_string(),
            source_app: "".to_string(),
            image_path: "".to_string(),
            image_data_len: 0,
            width: 0,
            height: 0,
            created_at: "".to_string(),
        })
        .is_none());
    }

    #[test]
    fn bdd_latest_manifest_names_transport() {
        let manifest = MultiSyncManifest {
            protocol: MULTI_SYNC_PROTOCOL.to_string(),
            version: MULTI_SYNC_VERSION,
            transport: "webdav".to_string(),
            clip: None,
        };

        assert_eq!(manifest.protocol, "ZSCLIP_MULTI_SYNC_V1");
        assert_eq!(manifest.transport, "webdav");
    }

    #[test]
    fn bdd_empty_latest_manifest_serializes_null_clip() {
        crate::db_runtime::with_test_db(|| {
            let manifest = latest_manifest("webdav")?;
            let json = serde_json::to_value(&manifest).unwrap();

            assert_eq!(json["protocol"], MULTI_SYNC_PROTOCOL);
            assert_eq!(json["transport"], "webdav");
            assert!(json["clip"].is_null());
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn bdd_transport_status_describes_selected_multi_sync_mode() {
        assert!(transport_status_label(false, false).contains("未选择同步方案"));
        assert!(transport_status_label(true, false).contains("当前方案为 WebDAV"));
        assert!(transport_status_label(false, true).contains("当前方案为局域网"));
        assert!(transport_status_label(true, true).contains("当前方案为局域网"));
    }

    #[test]
    fn bdd_remote_clip_signature_is_stable_for_webdav_text() {
        let clip = MultiSyncItem {
            id: "db:text:7".to_string(),
            kind: "text".to_string(),
            hash: "md5:abc".to_string(),
            preview: "hello".to_string(),
            content: Some("hello world".to_string()),
            has_data: false,
            data_name: None,
            size: 11,
            source_app: "Android".to_string(),
            created_at: "2026-06-02 10:00:00".to_string(),
            width: None,
            height: None,
        };

        assert_eq!(
            remote_clip_signature("webdav", &clip),
            "multi:webdav:db:text:7:md5:abc:11"
        );
    }

    #[test]
    fn bdd_android_webdav_manifest_deserializes_for_windows_import() {
        let manifest: MultiSyncManifest = serde_json::from_str(
            r#"{
              "protocol": "ZSCLIP_MULTI_SYNC_V1",
              "version": 1,
              "transport": "webdav",
              "clip": {
                "id": "android:text:android-1:42",
                "type": "text",
                "hash": "md5:9a0364b9e99bb480dd25e1f0284c8555",
                "preview": "hello\ncloud",
                "content": "hello\ncloud",
                "hasData": false,
                "size": 11,
                "source_app": "Android",
                "created_at": "42"
              }
            }"#,
        )
        .unwrap();
        let clip = manifest.clip.as_ref().unwrap();

        assert_eq!(manifest.protocol, MULTI_SYNC_PROTOCOL);
        assert_eq!(clip.kind, "text");
        assert_eq!(clip.content.as_deref(), Some("hello\ncloud"));
        assert_eq!(clip.source_app, "Android");
        assert_eq!(
            remote_clip_signature(&manifest.transport, clip),
            "multi:webdav:android:text:android-1:42:md5:9a0364b9e99bb480dd25e1f0284c8555:11"
        );
    }

    #[test]
    fn bdd_android_webdav_manifest_imports_once_into_windows_history() {
        crate::db_runtime::with_test_db(|| {
            let manifest: MultiSyncManifest = serde_json::from_str(
                r#"{
                  "protocol": "ZSCLIP_MULTI_SYNC_V1",
                  "version": 1,
                  "transport": "webdav",
                  "clip": {
                    "id": "android:text:android-1:42",
                    "type": "text",
                    "hash": "md5:9a0364b9e99bb480dd25e1f0284c8555",
                    "preview": "hello\ncloud",
                    "content": "hello\ncloud",
                    "hasData": false,
                    "size": 11,
                    "source_app": "Android",
                    "created_at": "42"
                  }
                }"#,
            )
            .unwrap();

            let first = import_remote_text_clip(&manifest)?.unwrap();
            assert!(first.imported);
            assert_eq!(
                first.signature,
                "multi:webdav:android:text:android-1:42:md5:9a0364b9e99bb480dd25e1f0284c8555:11"
            );

            let row: (String, String, String, String) = crate::db_runtime::with_db(|conn| {
                conn.query_row(
                    "SELECT kind, preview, text_data, source_app FROM items WHERE signature=?",
                    [&first.signature],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
                )
            })?;
            assert_eq!(row.0, "text");
            assert_eq!(row.1, "hello\ncloud");
            assert_eq!(row.2, "hello\ncloud");
            assert_eq!(row.3, "WebDAV: Android");

            let second = import_remote_text_clip(&manifest)?.unwrap();
            assert!(!second.imported);
            let count: i64 = crate::db_runtime::with_db(|conn| {
                conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            })?;
            assert_eq!(count, 1);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn bdd_android_webdav_image_manifest_imports_once_into_windows_history() {
        crate::db_runtime::with_test_db(|| {
            let manifest: MultiSyncManifest = serde_json::from_str(
                r#"{
                  "protocol": "ZSCLIP_MULTI_SYNC_V1",
                  "version": 1,
                  "transport": "webdav",
                  "clip": {
                    "id": "android:image:android-1:99",
                    "type": "image",
                    "hash": "md5:png",
                    "preview": "shot.png",
                    "content": null,
                    "hasData": true,
                    "dataName": "zsclip_image_99.png",
                    "size": 70,
                    "source_app": "Android",
                    "created_at": "99"
                  }
                }"#,
            )
            .unwrap();
            let png = encode_rgba_png_bytes(&[255, 0, 0, 255], 1, 1).unwrap();

            let first = import_remote_image_clip(&manifest, &png)?.unwrap();
            assert!(first.imported);
            let second = import_remote_image_clip(&manifest, &png)?.unwrap();
            assert!(!second.imported);

            let row: (String, String, i64, i64, String) = crate::db_runtime::with_db(|conn| {
                conn.query_row(
                    "SELECT kind, preview, image_width, image_height, source_app FROM items WHERE signature=?",
                    [&first.signature],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                )
            })?;
            assert_eq!(row.0, "image");
            assert_eq!(row.1, "shot.png");
            assert_eq!((row.2, row.3), (1, 1));
            assert_eq!(row.4, "WebDAV: Android");
            Ok(())
        })
        .unwrap();
    }
}
