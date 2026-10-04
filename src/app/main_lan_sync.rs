use super::prelude::*;

pub(super) fn maybe_broadcast_lan_clip_item(state: &AppState, item: &ClipItem, signature: &str) {
    if state.role != WindowRole::Main || !state.settings.lan_sync_enabled {
        return;
    }
    if !crate::lan_sync_core::LanSyncMode::from_key(&state.settings.lan_sync_mode).send_automatic() { return; }
    if matches!(item.kind, ClipKind::Files) {
        if let Some(paths) = &item.file_paths {
            lan_sync::push_small_files_to_trusted(&state.settings, paths.clone());
        }
        return;
    }
    if let Some(envelope) = lan_envelope_from_item(&state.settings, item, signature) {
        lan_sync::broadcast_clip(&state.settings, envelope);
    }
}

struct LanDecodedClip {
    item: ClipItem,
    content_signature: String,
    latest_envelope: LanClipEnvelope,
}

fn lan_envelope_from_item(
    settings: &AppSettings,
    item: &ClipItem,
    signature: &str,
) -> Option<LanClipEnvelope> {
    // Persisted row identity survives process restarts and is shared by push and pull.
    if item.id <= 0 { return None; }
    lan_latest_envelope_from_item(settings,item,signature)
}

#[cfg(test)]
mod lan_origin_identity_tests {
    use super::*;

    #[test]
    fn lan_discarded_file_cleanup_preserves_existing_database_references() {
        crate::db_runtime::with_test_db(|| {
            let folder = data_dir().join("lan_received");
            fs::create_dir_all(&folder).unwrap();
            let nonce = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let retained = folder.join(format!("retained-{nonce}.txt"));
            let discarded = folder.join(format!("discarded-{nonce}.txt"));
            fs::write(&retained,b"synthetic retained file").unwrap();
            fs::write(&discarded,b"synthetic discarded file").unwrap();
            let id = crate::db_runtime::with_db(|conn| {
                conn.execute("INSERT INTO items(category,kind,preview,text_data) VALUES(0,'text','synthetic file','synthetic file')",[])?;
                Ok(conn.last_insert_rowid())
            })?;
            let mut item = db_load_item_full(id).unwrap();
            item.kind = ClipKind::Files; item.text = None; item.source_app = "LAN: synthetic".into();
            item.file_paths = Some(vec![retained.to_string_lossy().to_string()]);
            db_insert_item(0,&item,None)?;
            item.file_paths.as_mut().unwrap().push(discarded.to_string_lossy().to_string());
            remove_uninserted_image_file(&item);
            assert!(retained.is_file());
            assert!(!discarded.exists());
            fs::remove_file(&retained).unwrap();
            Ok(())
        }).unwrap();
    }

    #[test]
    fn lan_local_push_and_pull_share_stable_content_bound_row_identity() {
        crate::db_runtime::with_test_db(|| {
            let id=crate::db_runtime::with_db(|conn| {
                conn.execute("INSERT INTO items(category,kind,preview,text_data) VALUES(0,'text','synthetic identity','synthetic identity')",[])?;
                Ok(conn.last_insert_rowid())
            })?;
            let mut settings=AppSettings::default();settings.lan_device_id="identity-pc".into();
            let item=db_load_item_full(id).unwrap();
            let first=lan_envelope_from_item(&settings,&item,"").unwrap();
            let pulled=lan_latest_envelope_from_item(&settings,&item,"").unwrap();
            let reopened=lan_envelope_from_item(&settings.clone(),&db_load_item_full(id).unwrap(),"").unwrap();
            assert_eq!(first.message_id,pulled.message_id);
            assert_eq!(first.message_id,reopened.message_id);
            assert_eq!(first.origin_seq,id as u64);
            let original_text = item.text.as_deref().unwrap();
            for edited_text in [format!(" {original_text} "), format!("{original_text}\n"), format!("{original_text}\u{200d}")] {
                db_update_item_text(id, &edited_text)?;
                let edited_item = db_load_item_full(id).unwrap();
                let edited = lan_envelope_from_item(&settings, &edited_item, "").unwrap();
                assert_eq!(edited.message_id, lan_latest_envelope_from_item(&settings, &edited_item, "").unwrap().message_id);
                assert_ne!(first.message_id, edited.message_id, "wire payload edits need a new identity");
                assert_eq!(edited.text.as_deref(), Some(edited_text.as_str()));
            }
            let received=LanOriginMetadata {message_id:"remote-id".into(),origin_device_id:"remote-phone".into(),origin_seq:99,hash:"text:sha256:synthetic".into()};
            db_save_lan_origin_metadata(id,&received)?;
            db_update_item_text(id,"edited synthetic identity")?;
            assert!(db_load_lan_origin_metadata(id).is_none());
            let edited=lan_latest_envelope_from_item(&settings,&db_load_item_full(id).unwrap(),"").unwrap();
            assert_ne!(first.message_id,edited.message_id);
            assert_eq!(edited.origin_device_id,settings.lan_device_id);
            assert!(!edited.message_id.contains("edited synthetic"));
            Ok(())
        }).unwrap();
    }
}

pub(super) fn lan_latest_envelope_from_item(
    settings: &AppSettings,
    item: &ClipItem,
    signature: &str,
) -> Option<LanClipEnvelope> {
    let signature = dedupe_signature_for_item(item, signature);
    if signature.trim().is_empty() || settings.lan_device_id.trim().is_empty() {
        return None;
    }
    let preview = item.preview.chars().take(160).collect::<String>();
    let metadata = db_load_lan_origin_metadata(item.id);
    let message_id = metadata
        .as_ref()
        .map(|meta| meta.message_id.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_default();
    let origin_device_id = metadata
        .as_ref()
        .map(|meta| meta.origin_device_id.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| settings.lan_device_id.clone());
    let origin_seq = metadata
        .as_ref()
        .map(|meta| meta.origin_seq)
        .filter(|seq| *seq > 0)
        .unwrap_or_else(|| item.id.max(0) as u64);
    let envelope_hash = metadata
        .as_ref()
        .map(|meta| meta.hash.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| signature.clone());
    let base = || LanClipEnvelope {
        message_id: message_id.clone(),
        origin_device_id: origin_device_id.clone(),
        origin_seq,
        kind: String::new(),
        hash: envelope_hash.clone(),
        created_at_ms: now_epoch_ms(),
        preview: preview.clone(),
        text: None,
        image_png_base64: None,
        file_meta: Vec::new(),
    };
    let mut envelope = match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let text = item.text.clone()?;
            if text.trim().is_empty() || crate::db_runtime::text_is_protected(&text) {
                return None;
            }
            let mut envelope = base();
            envelope.kind = "text".to_string();
            envelope.text = Some(text);
            Some(envelope)
        }
        ClipKind::Image => {
            let png_bytes = lan_image_png_bytes(item)?;
            if png_bytes.len() > lan_sync::LAN_IMAGE_MAX_BYTES {
                return None;
            }
            let mut envelope = base();
            envelope.kind = "image".to_string();
            envelope.image_png_base64 = Some(general_purpose::STANDARD.encode(png_bytes));
            Some(envelope)
        }
        ClipKind::Files => {
            let mut envelope = base();
            envelope.kind = "files".to_string();
            if let Some(paths) = item.file_paths.as_ref() {
                envelope.file_meta = paths
                    .iter()
                    .map(|path| {
                        let path_buf = PathBuf::from(path);
                        LanFileMeta {
                            name: path_buf
                                .file_name()
                                .and_then(|name| name.to_str())
                                .unwrap_or(path)
                                .to_string(),
                            size: fs::metadata(&path_buf).map(|meta| meta.len()).unwrap_or(0),
                            relative_path: String::new(),
                        }
                    })
                    .collect();
            }
            Some(envelope)
        }
    }?;
    if envelope.message_id.is_empty() {
        use sha2::Digest;
        let payload = serde_json::to_vec(&(&envelope.kind, &envelope.text, &envelope.image_png_base64, &envelope.file_meta)).ok()?;
        envelope.message_id = format!("{}-db-{}-{:x}", settings.lan_device_id, item.id.max(0), sha2::Sha256::digest(payload));
    }
    Some(envelope)
}

pub(super) fn refresh_lan_latest_from_db(settings: &AppSettings) {
    if !settings.lan_sync_enabled {
        lan_sync::set_latest_clip(None);
        return;
    }
    let latest = db_load_latest_item_with_signature(0)
        .and_then(|(item, signature)| lan_latest_envelope_from_item(settings, &item, &signature));
    lan_sync::set_latest_clip(latest);
}

fn lan_image_png_bytes(item: &ClipItem) -> Option<Vec<u8>> {
    if let Some(path) = item.image_path.as_deref() {
        let bytes = fs::read(path).ok()?;
        if bytes.len() <= lan_sync::LAN_IMAGE_MAX_BYTES
            && png_dimensions_from_bytes(&bytes).is_some()
        {
            return Some(bytes);
        }
    }
    let bytes = item.image_bytes.as_ref()?;
    encode_rgba_png_bytes(bytes, item.image_width as u32, item.image_height as u32)
}

fn encode_rgba_png_bytes(bytes: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
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

fn png_dimensions_from_bytes(bytes: &[u8]) -> Option<(usize, usize)> {
    let cursor = std::io::Cursor::new(bytes);
    let decoder = png::Decoder::new(cursor);
    let reader = decoder.read_info().ok()?;
    let info = reader.info();
    Some((info.width as usize, info.height as usize))
}

fn write_lan_image_png(bytes: &[u8]) -> Option<PathBuf> {
    use std::io::Write;

    for _ in 0..8 {
        let output = output_image_path();
        let mut file = match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&output)
        {
            Ok(file) => file,
            Err(err) if err.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(_) => return None,
        };
        if file.write_all(bytes).is_ok() && file.sync_all().is_ok() {
            return Some(output);
        }
        let _ = fs::remove_file(output);
        return None;
    }
    None
}

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(super) unsafe fn handle_lan_sync_ready(hwnd: HWND) {
    let requests = lan_sync::drain_pair_prompts();
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    if let Some(request) = requests.first() {
        let response = platform_dialog::WindowsDialogHost::new().confirm(
            hwnd, "允许设备连接",
            &format!("设备：{}\n地址：{}\n安全码：{}\n\n允许此设备通过局域网同步剪贴板？", request.device_name, request.addr, request.code),
            NativeDialogLevel::Question, NativeDialogButtons::YesNo,
        );
        if response == NativeDialogResponse::Yes { lan_sync::accept_pair_request(&request.pair_id); }
        else { lan_sync::reject_pair_request(&request.pair_id); }
    }
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() { return; }
    let state = &mut *ptr;
    let incoming = lan_sync::drain_incoming_clips();
    let _reservations: Vec<_> = incoming.iter().map(|clip| lan_sync::incoming_clip_reservation(&clip.envelope)).collect();
    let mirror_clipboard = state.settings.lan_receive_mode == "clipboard";
    for incoming_clip in incoming {
        if !incoming_clip.manual && !crate::lan_sync_core::LanSyncMode::from_key(&state.settings.lan_sync_mode).receive_automatic() { continue; }
        let expected_generation = state.app_data_generation;
        let processed =
            crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
                if !matches!(crate::db_runtime::has_lan_receipt(&incoming_clip.envelope.origin_device_id, &incoming_clip.envelope.message_id), Ok(false)) {
                    if incoming_clip.envelope.kind == "files" {
                        if let Some(decoded) = lan_item_from_envelope(incoming_clip) { remove_uninserted_image_file(&decoded.item); }
                    }
                    return;
                }
                if let Some(decoded) = lan_item_from_envelope(incoming_clip) {
                    let clipboard_item = decoded.item.clone();
                    let latest_envelope = decoded.latest_envelope.clone();
                    let origin=LanOriginMetadata {message_id:latest_envelope.message_id.clone(),origin_device_id:latest_envelope.origin_device_id.clone(),origin_seq:latest_envelope.origin_seq,hash:latest_envelope.hash.clone()};
                    let signature=decoded.content_signature;
                    let inserted=state.add_lan_clip_item(decoded.item,signature.clone(),&origin);
                    if inserted.is_ok() {
                        lan_sync::set_latest_clip(Some(latest_envelope));
                        if mirror_clipboard {
                            let item = if inserted==Ok(false) {
                                db_find_duplicate_item_ids(0,&clipboard_item,&signature).first().copied()
                                    .and_then(db_load_item_full)
                            } else { Some(clipboard_item.clone()) };
                            if let Some(item)=item { let _ = apply_lan_item_to_clipboard(state, &item); }
                        }
                    } else {
                        remove_uninserted_image_file(&clipboard_item);
                    }
                }
            });
        if processed.is_none() {
            break;
        }
    }
    repaint_main_window(hwnd, true);
    refresh_settings_cloud_page_after_lan_sync(state.settings_hwnd);
}

unsafe fn apply_lan_item_to_clipboard(state: &mut AppState, item: &ClipItem) -> bool {
    let ok = match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let Some(text) = &item.text else {
                return false;
            };
            let ok = platform_clipboard::WindowsClipboardHost::write_text(text);
            if ok {
                state.note_programmatic_clipboard_signature(
                    text_content_signature(text),
                    CLIPBOARD_IGNORE_MS_PASTE,
                );
            }
            ok
        }
        ClipKind::Image => {
            let Some((bytes, width, height)) = ensure_item_image_bytes(item) else {
                return false;
            };
            let ok =
                platform_clipboard::WindowsClipboardHost::write_image_rgba(&bytes, width, height);
            if ok {
                state.note_programmatic_clipboard_signature(
                    image_content_signature(&bytes, width, height),
                    CLIPBOARD_IGNORE_MS_PASTE,
                );
            }
            ok
        }
        ClipKind::Files => {
            let Some(paths) = &item.file_paths else {
                return false;
            };
            let ok = platform_clipboard::WindowsClipboardHost::write_file_paths(paths);
            if ok {
                state.note_programmatic_clipboard_signature(
                    file_paths_signature(paths),
                    CLIPBOARD_IGNORE_MS_PASTE,
                );
            }
            ok
        }
    };
    if ok {
        set_ignore_clipboard_for_all_hosts(CLIPBOARD_IGNORE_MS_PASTE);
    }
    ok
}

fn lan_message_key_from_envelope(envelope: &LanClipEnvelope) -> String {
    crate::lan_sync_core::lan_message_identity(envelope)
}

fn lan_text_content_signature(text: &str) -> String {
    text_content_signature(text)
}

fn lan_image_content_signature(envelope_hash: &str, png_bytes: &[u8]) -> String {
    let envelope_hash = envelope_hash.trim();
    if envelope_hash.len() == 12
        && envelope_hash.starts_with("crc:")
        && envelope_hash[4..].chars().all(|ch| ch.is_ascii_hexdigit())
    {
        envelope_hash.to_string()
    } else {
        let cursor = std::io::Cursor::new(png_bytes);
        let decoder = png::Decoder::new(cursor);
        let mut reader = match decoder.read_info() {
            Ok(reader) => reader,
            Err(_) => return String::new(),
        };
        let output_size = reader.output_buffer_size();
        let mut buffer = vec![0; output_size];
        let info = match reader.next_frame(&mut buffer) {
            Ok(info) => info,
            Err(_) => return String::new(),
        };
        let bytes = &buffer[..info.buffer_size()];
        let rgba = match info.color_type {
            png::ColorType::Rgba => bytes.to_vec(),
            png::ColorType::Rgb => {
                let mut out =
                    Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
                for chunk in bytes.chunks_exact(3) {
                    out.extend_from_slice(&[chunk[0], chunk[1], chunk[2], 255]);
                }
                out
            }
            png::ColorType::GrayscaleAlpha => {
                let mut out =
                    Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
                for chunk in bytes.chunks_exact(2) {
                    out.extend_from_slice(&[chunk[0], chunk[0], chunk[0], chunk[1]]);
                }
                out
            }
            png::ColorType::Grayscale => {
                let mut out =
                    Vec::with_capacity((info.width as usize) * (info.height as usize) * 4);
                for value in bytes {
                    out.extend_from_slice(&[*value, *value, *value, 255]);
                }
                out
            }
            _ => return String::new(),
        };
        image_content_signature(&rgba, info.width as usize, info.height as usize)
    }
}

pub(super) fn remove_uninserted_image_file(item: &ClipItem) {
    match item.kind {
        ClipKind::Image => {
            if let Some(path) = item.image_path.as_deref() {
                let _ = fs::remove_file(path);
            }
        }
        ClipKind::Files if item.source_app.starts_with("LAN:") => {
            let Some(paths) = item.file_paths.as_ref() else {
                return;
            };
            let root = data_dir().join("lan_received");
            let root_canon = root.canonicalize().unwrap_or(root);
            for raw in paths {
                let path = PathBuf::from(raw);
                let path_canon = path.canonicalize().unwrap_or(path);
                let referenced = with_db(|conn| {
                    let mut statement = conn.prepare("SELECT COALESCE(file_paths,'') FROM items WHERE kind='files'")?;
                    let rows = statement.query_map([], |row| row.get::<_,String>(0))?;
                    for value in rows {
                        for stored in value?.lines() {
                            let stored = PathBuf::from(stored);
                            let stored = stored.canonicalize().unwrap_or(stored);
                            if stored == path_canon || (cfg!(windows) && stored.to_string_lossy().eq_ignore_ascii_case(&path_canon.to_string_lossy())) { return Ok(true); }
                        }
                    }
                    Ok(false)
                });
                if path_canon.starts_with(&root_canon) && matches!(referenced, Ok(false)) {
                    let _ = fs::remove_file(path_canon);
                }
            }
        }
        _ => {}
    }
}

fn lan_item_from_envelope(incoming: lan_sync::LanIncomingClip) -> Option<LanDecodedClip> {
    let source = format!("LAN: {}", incoming.source_device_name);
    let envelope = incoming.envelope;
    let latest_envelope = envelope.clone();
    let message_key = lan_message_key_from_envelope(&envelope);
    match envelope.kind.as_str() {
        "text" => {
            let text = envelope.text?;
            if crate::db_runtime::text_is_protected(&text) { return None; }
            let content_signature = lan_text_content_signature(&text);
            let preview = if envelope.preview.trim().is_empty() {
                build_preview(&text)
            } else {
                envelope.preview
            };
            Some(LanDecodedClip {
                item: ClipItem {
                    phrase_title: String::new(),
                    id: 0,
                    kind: ClipKind::Text,
                    preview,
                    text: Some(text),
                    rich_text_html: None,
                    source_app: source,
                    file_paths: None,
                    image_bytes: None,
                    image_path: None,
                    image_width: 0,
                    image_height: 0,
                    pinned: false,
                    group_id: 0,
                    created_at: now_utc_sqlite(),
                },
                content_signature,
                latest_envelope,
            })
        }
        "image" => {
            let encoded = envelope.image_png_base64?;
            if encoded.len() > lan_sync::LAN_IMAGE_MAX_BYTES * 2 {
                return None;
            }
            let png_bytes = general_purpose::STANDARD.decode(encoded).ok()?;
            if png_bytes.len() > lan_sync::LAN_IMAGE_MAX_BYTES {
                return None;
            }
            let (width, height) = png_dimensions_from_bytes(&png_bytes)?;
            let content_signature = lan_image_content_signature(&envelope.hash, &png_bytes);
            let output = write_lan_image_png(&png_bytes)?;
            let preview = if envelope.preview.trim().is_empty() {
                format!("{} {}x{}", tr("局域网图片", "LAN image"), width, height)
            } else {
                envelope.preview
            };
            Some(LanDecodedClip {
                item: ClipItem {
                    phrase_title: String::new(),
                    id: 0,
                    kind: ClipKind::Image,
                    preview,
                    text: None,
                    rich_text_html: None,
                    source_app: source,
                    file_paths: None,
                    image_bytes: None,
                    image_path: Some(output.to_string_lossy().to_string()),
                    image_width: width,
                    image_height: height,
                    pinned: false,
                    group_id: 0,
                    created_at: now_utc_sqlite(),
                },
                content_signature,
                latest_envelope,
            })
        }
        "files" => {
            let content_signature = if envelope.hash.trim().starts_with("crc:")
                && envelope.hash.trim().len() == 12
                && envelope.hash.trim()[4..]
                    .chars()
                    .all(|character| character.is_ascii_hexdigit())
            {
                envelope.hash.trim().to_string()
            } else {
                message_key
            };
            let paths: Vec<String> = envelope
                .file_meta
                .iter()
                .map(|meta| {
                    data_dir()
                        .join(&meta.relative_path)
                        .to_string_lossy()
                        .to_string()
                })
                .collect();
            if paths.is_empty() {
                return None;
            }
            let preview = if envelope.preview.trim().is_empty() {
                build_files_preview(&paths)
            } else {
                envelope.preview
            };
            Some(LanDecodedClip {
                item: ClipItem {
                    phrase_title: String::new(),
                    id: 0,
                    kind: ClipKind::Files,
                    preview,
                    text: None,
                    rich_text_html: None,
                    source_app: source,
                    file_paths: Some(paths),
                    image_bytes: None,
                    image_path: None,
                    image_width: 0,
                    image_height: 0,
                    pinned: false,
                    group_id: 0,
                    created_at: now_utc_sqlite(),
                },
                content_signature,
                latest_envelope,
            })
        }
        _ => None,
    }
}
