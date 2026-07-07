use super::prelude::*;

pub(super) fn maybe_broadcast_lan_clip_item(state: &AppState, item: &ClipItem, signature: &str) {
    if state.role != WindowRole::Main || !state.settings.lan_sync_enabled {
        return;
    }
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
    let origin_seq = lan_sync::next_origin_seq();
    let preview = item.preview.chars().take(160).collect::<String>();
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let text = item.text.clone()?;
            if text.trim().is_empty() {
                return None;
            }
            Some(LanClipEnvelope {
                message_id: format!("{}-{origin_seq}", settings.lan_device_id),
                origin_device_id: settings.lan_device_id.clone(),
                origin_seq,
                kind: "text".to_string(),
                hash: if signature.is_empty() {
                    dedupe_signature_for_item(item, "")
                } else {
                    signature.to_string()
                },
                created_at_ms: now_epoch_ms(),
                preview,
                text: Some(text),
                image_png_base64: None,
                file_meta: Vec::new(),
            })
        }
        ClipKind::Image => {
            let png_bytes = lan_image_png_bytes(item)?;
            if png_bytes.len() > lan_sync::LAN_IMAGE_MAX_BYTES {
                return None;
            }
            Some(LanClipEnvelope {
                message_id: format!("{}-{origin_seq}", settings.lan_device_id),
                origin_device_id: settings.lan_device_id.clone(),
                origin_seq,
                kind: "image".to_string(),
                hash: if signature.is_empty() {
                    dedupe_signature_for_item(item, "")
                } else {
                    signature.to_string()
                },
                created_at_ms: now_epoch_ms(),
                preview,
                text: None,
                image_png_base64: Some(general_purpose::STANDARD.encode(png_bytes)),
                file_meta: Vec::new(),
            })
        }
        ClipKind::Files => None,
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
        .unwrap_or_else(|| format!("{}-db-{}", settings.lan_device_id, item.id.max(0)));
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
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let text = item.text.clone()?;
            if text.trim().is_empty() {
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
    }
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

fn now_epoch_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

pub(super) unsafe fn handle_lan_sync_ready(hwnd: HWND) {
    let _ = lan_sync::drain_pair_prompts();
    let incoming = lan_sync::drain_incoming_clips();
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let mirror_clipboard = state.settings.lan_receive_mode == "clipboard";
    for incoming_clip in incoming {
        let message_key = lan_message_key_from_envelope(&incoming_clip.envelope);
        if !state.remember_lan_message_key(&message_key) {
            continue;
        }
        if let Some(decoded) = lan_item_from_envelope(incoming_clip) {
            let clipboard_item = decoded.item.clone();
            let latest_envelope = decoded.latest_envelope.clone();
            let inserted = state.add_lan_clip_item(decoded.item, decoded.content_signature);
            if inserted {
                if let Some(item_id) = db_latest_item_id(0) {
                    let _ = db_save_lan_origin_metadata(
                        item_id,
                        &LanOriginMetadata {
                            message_id: latest_envelope.message_id.clone(),
                            origin_device_id: latest_envelope.origin_device_id.clone(),
                            origin_seq: latest_envelope.origin_seq,
                            hash: latest_envelope.hash.clone(),
                        },
                    );
                }
                lan_sync::set_latest_clip(Some(latest_envelope));
                if mirror_clipboard {
                    let _ = apply_lan_item_to_clipboard(state, &clipboard_item);
                }
            } else {
                remove_uninserted_image_file(&clipboard_item);
            }
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
    format!(
        "lan:{}:{}:{}",
        envelope.origin_device_id, envelope.origin_seq, envelope.hash
    )
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
                if path_canon.starts_with(&root_canon) {
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
            let content_signature = lan_text_content_signature(&text);
            let preview = if envelope.preview.trim().is_empty() {
                build_preview(&text)
            } else {
                envelope.preview
            };
            Some(LanDecodedClip {
                item: ClipItem {
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
            let output = output_image_path();
            fs::write(&output, png_bytes).ok()?;
            let preview = if envelope.preview.trim().is_empty() {
                format!("{} {}x{}", tr("局域网图片", "LAN image"), width, height)
            } else {
                envelope.preview
            };
            Some(LanDecodedClip {
                item: ClipItem {
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
