use super::prelude::*;

fn strip_invisible_text_chars(input: &str) -> String {
    input
        .chars()
        .filter(|ch| {
            !matches!(
                ch,
                '\u{feff}' | '\u{200b}' | '\u{200c}' | '\u{200d}' | '\u{2060}'
            )
        })
        .collect()
}

fn strip_markdown_links_and_images(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    let bytes = input.as_bytes();
    let mut i = 0usize;
    while i < bytes.len() {
        let is_image = bytes[i] == b'!' && i + 1 < bytes.len() && bytes[i + 1] == b'[';
        let is_link = bytes[i] == b'[';
        if is_image || is_link {
            let label_start = if is_image { i + 2 } else { i + 1 };
            if let Some(label_end_rel) = input[label_start..].find(']') {
                let label_end = label_start + label_end_rel;
                let after_label = label_end + 1;
                if after_label < bytes.len() && bytes[after_label] == b'(' {
                    if let Some(url_end_rel) = input[after_label + 1..].find(')') {
                        out.push_str(input[label_start..label_end].trim());
                        i = after_label + 2 + url_end_rel;
                        continue;
                    }
                }
            }
        }
        let Some(ch) = input[i..].chars().next() else {
            break;
        };
        out.push(ch);
        i += ch.len_utf8();
    }
    out
}

fn strip_markdown_prefix(line: &str) -> String {
    let trimmed = line.trim_start();
    let bytes = trimmed.as_bytes();
    let mut drop_bytes = 0usize;

    while drop_bytes < bytes.len() && bytes[drop_bytes] == b'>' {
        drop_bytes += 1;
        while drop_bytes < bytes.len() && bytes[drop_bytes] == b' ' {
            drop_bytes += 1;
        }
    }

    let trimmed = &trimmed[drop_bytes..];
    let bytes = trimmed.as_bytes();
    let mut idx = 0usize;
    while idx < bytes.len() && bytes[idx] == b'#' {
        idx += 1;
    }
    if idx > 0 && idx < bytes.len() && bytes[idx] == b' ' {
        return trimmed[idx + 1..].to_string();
    }

    for prefix in [
        "- [ ] ", "- [x] ", "- [X] ", "* [ ] ", "* [x] ", "* [X] ", "- ", "* ", "+ ",
    ] {
        if let Some(rest) = trimmed.strip_prefix(prefix) {
            return rest.to_string();
        }
    }

    let mut digits = 0usize;
    for ch in trimmed.chars() {
        if ch.is_ascii_digit() {
            digits += ch.len_utf8();
        } else {
            break;
        }
    }
    if digits > 0 {
        let rest = &trimmed[digits..];
        if let Some(stripped) = rest.strip_prefix(". ").or_else(|| rest.strip_prefix(") ")) {
            return stripped.to_string();
        }
    }

    trimmed.to_string()
}

fn looks_like_markdown_document(input: &str) -> bool {
    input.contains("```")
        || input.contains("](")
        || input.lines().any(|line| {
            let trimmed = line.trim_start();
            trimmed.starts_with('#')
                || trimmed.starts_with('>')
                || trimmed.starts_with("- ")
                || trimmed.starts_with("* ")
                || trimmed.starts_with("+ ")
                || trimmed.starts_with("![")
                || trimmed.starts_with("- [")
                || trimmed.starts_with("* [")
                || trimmed.chars().take_while(|ch| ch.is_ascii_digit()).count() > 0
                    && (trimmed.contains(". ") || trimmed.contains(") "))
        })
}

fn collapse_blank_lines(lines: impl IntoIterator<Item = String>, max_blank_lines: usize) -> String {
    let mut output = Vec::new();
    let mut blank = 0usize;
    for line in lines {
        if line.trim().is_empty() {
            blank += 1;
            if blank <= max_blank_lines {
                output.push(String::new());
            }
        } else {
            blank = 0;
            output.push(line);
        }
    }
    output.join("\n").trim().to_string()
}

fn clean_markdown_document(input: &str) -> String {
    let normalized = input.replace("\r\n", "\n").replace('\r', "\n");
    let mut in_code_fence = false;
    let mut cleaned = Vec::new();
    for raw_line in normalized.lines() {
        let trimmed = raw_line.trim();
        if trimmed.starts_with("```") || trimmed.starts_with("~~~") {
            in_code_fence = !in_code_fence;
            continue;
        }
        let mut line = if in_code_fence {
            raw_line.trim_end().to_string()
        } else {
            strip_markdown_prefix(raw_line)
        };
        line = strip_markdown_links_and_images(&line);
        for marker in ["**", "__", "~~", "`"] {
            line = line.replace(marker, "");
        }
        cleaned.push(line.trim_end().to_string());
    }
    collapse_blank_lines(cleaned, 1)
}

pub(super) fn ai_clean_text(input: &str) -> String {
    let normalized = strip_invisible_text_chars(input)
        .replace("\r\n", "\n")
        .replace('\r', "\n");
    let cleaned = if looks_like_markdown_document(&normalized) {
        clean_markdown_document(&normalized)
    } else {
        let lines = normalized
            .lines()
            .map(|line| line.trim_end().to_string())
            .collect::<Vec<_>>();
        collapse_blank_lines(lines, 1)
    };
    cleaned.trim().to_string()
}

pub(super) unsafe fn maybe_ai_clean_text(state: &AppState, input: &str) -> String {
    if !state.settings.ai_clean_enabled {
        return input.to_string();
    }
    let shift_down = hotkey::shift_pressed();
    if shift_down {
        return input.to_string();
    }
    if input.matches('\n').count() >= 4
        || input.contains("```")
        || input.chars().count() >= 280
        || looks_like_markdown_document(input)
    {
        ai_clean_text(input)
    } else {
        input.to_string()
    }
}

fn export_dir() -> PathBuf {
    let dir = data_dir().join("exports");
    let _ = fs::create_dir_all(&dir);
    dir
}

fn sanitize_export_name(name: &str, fallback: &str) -> String {
    let mut s = name
        .chars()
        .take(40)
        .map(|ch| {
            if matches!(ch, '\\' | '/' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '_'
            } else {
                ch
            }
        })
        .collect::<String>()
        .trim()
        .to_string();
    if s.is_empty() {
        s = fallback.to_string();
    }
    s
}

pub(super) fn materialize_item_as_file(item: &ClipItem) -> Option<PathBuf> {
    let base = export_dir();
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let suffix = if item.id > 0 {
        item.id.to_string()
    } else {
        ts.to_string()
    };
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => {
            let name = sanitize_export_name(&item.preview, "text");
            let path = base.join(format!("{}_{}_{}.txt", name, ts, suffix));
            let text = item.text.as_deref().unwrap_or(&item.preview);
            fs::write(&path, text).ok()?;
            Some(path)
        }
        ClipKind::Image => {
            let name = sanitize_export_name(&item.preview, "image");
            let path = base.join(format!("{}_{}_{}.png", name, ts, suffix));
            if let Some(existing) = save_image_item(item) {
                if existing != path {
                    fs::copy(existing, &path).ok()?;
                }
            } else {
                return None;
            }
            Some(path)
        }
        ClipKind::Files => item
            .file_paths
            .as_ref()
            .and_then(|v| v.first())
            .map(PathBuf::from),
    }
}

fn drag_export_paths_for_item(item: &ClipItem) -> Vec<PathBuf> {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase | ClipKind::Image => {
            materialize_item_as_file(item).into_iter().collect()
        }
        ClipKind::Files => Vec::new(),
    }
}

pub(crate) fn is_supported_ocr_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| {
            matches!(
                ext.to_ascii_lowercase().as_str(),
                "png" | "jpg" | "jpeg" | "bmp" | "gif" | "tif" | "tiff" | "webp"
            )
        })
        .unwrap_or(false)
}

pub(crate) struct OcrImageInput {
    pub(crate) path: PathBuf,
    pub(crate) delete_after: bool,
}

pub(crate) fn write_image_bytes_to_ocr_temp_path(
    bytes: &[u8],
    width: u32,
    height: u32,
) -> Option<PathBuf> {
    let base = std::env::temp_dir().join("zsclip").join("ocr");
    let _ = fs::create_dir_all(&base);
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_millis();
    let out = base.join(format!("ocr_{}.png", ts));
    write_image_bytes_to_path(&out, bytes, width, height)
}

pub(crate) fn image_input_for_ocr(item: &ClipItem) -> Option<OcrImageInput> {
    match item.kind {
        ClipKind::Image => {
            if let Some(path) = item.image_path.as_ref() {
                let src = PathBuf::from(path);
                if src.is_file() && is_supported_ocr_image_path(&src) {
                    return Some(OcrImageInput {
                        path: src,
                        delete_after: false,
                    });
                }
            }
            let (bytes, width, height) = ensure_item_image_bytes(item)?;
            let path = write_image_bytes_to_ocr_temp_path(&bytes, width as u32, height as u32)?;
            Some(OcrImageInput {
                path,
                delete_after: true,
            })
        }
        ClipKind::Files => item.file_paths.as_ref().and_then(|paths| {
            if paths.len() == 1 {
                let path = PathBuf::from(paths[0].clone());
                if path.is_file() && is_supported_ocr_image_path(&path) {
                    Some(OcrImageInput {
                        path,
                        delete_after: false,
                    })
                } else {
                    None
                }
            } else {
                None
            }
        }),
        _ => None,
    }
}

pub(super) unsafe fn spawn_image_ocr_job(hwnd: HWND, settings: AppSettings, item: ClipItem) {
    let hwnd_value = hwnd as isize;
    std::thread::spawn(move || {
        let result = match settings.image_ocr_provider.as_str() {
            "baidu" => image_input_for_ocr(&item)
                .ok_or_else(|| {
                    tr(
                        "当前记录没有可识别的图片文件",
                        "This item does not contain a recognizable image file",
                    )
                    .to_string()
                })
                .and_then(|input| {
                    let result =
                        fs::read(&input.path)
                            .map_err(|e| e.to_string())
                            .and_then(|bytes| {
                                run_baidu_ocr_api(
                                    &settings.image_ocr_cloud_url,
                                    &settings.image_ocr_cloud_token,
                                    &bytes,
                                )
                            });
                    if input.delete_after {
                        let _ = fs::remove_file(&input.path);
                    }
                    result
                }),
            "winocr" => image_input_for_ocr(&item)
                .ok_or_else(|| {
                    tr(
                        "当前记录没有可识别的图片文件",
                        "This item does not contain a recognizable image file",
                    )
                    .to_string()
                })
                .and_then(|input| {
                    let result = run_winocr_dll_ocr(&input.path, &settings.image_ocr_wechat_dir);
                    if input.delete_after {
                        let _ = fs::remove_file(&input.path);
                    }
                    result
                }),
            _ => Err(tr(
                "请先在设置-插件中启用图片 OCR",
                "Please enable Image OCR in Settings > Plugins first",
            )
            .to_string()),
        };

        let payload = match result {
            Ok(text) => TextOperationReadyResult {
                text: Some(text),
                error: None,
            },
            Err(err) => TextOperationReadyResult {
                text: None,
                error: Some(err),
            },
        };
        let _ = unsafe { post_boxed_message(hwnd_value, WM_IMAGE_OCR_READY, 0, Box::new(payload)) };
    });
}

pub(super) unsafe fn spawn_text_translate_text_job(
    hwnd: HWND,
    settings: AppSettings,
    text: String,
) {
    let hwnd_value = hwnd as isize;
    std::thread::spawn(move || {
        let result = match settings.text_translate_provider.as_str() {
            "baidu" => run_baidu_translate_api(
                &settings.text_translate_app_id,
                &settings.text_translate_secret,
                &text,
                &settings.text_translate_target_lang,
            ),
            _ => Err(tr(
                "请先在设置-插件中启用文本翻译",
                "Please enable Text Translate in Settings > Plugins first",
            )
            .to_string()),
        };

        let payload = match result {
            Ok(text) => TextOperationReadyResult {
                text: Some(text),
                error: None,
            },
            Err(err) => TextOperationReadyResult {
                text: None,
                error: Some(err),
            },
        };
        let _ = unsafe {
            post_boxed_message(hwnd_value, WM_TEXT_TRANSLATE_READY, 0, Box::new(payload))
        };
    });
}

pub(super) unsafe fn begin_row_drag_export(
    hwnd: HWND,
    state: &mut AppState,
    visible_idx: i32,
) -> bool {
    if visible_idx < 0 {
        return false;
    }
    let Some(src_idx) = state.visible_src_idx(visible_idx as usize) else {
        return false;
    };
    let Some(item) = state.active_items().get(src_idx).cloned() else {
        return false;
    };
    let Some(item) = state.resolve_item_for_use(&item) else {
        return false;
    };
    let paths = drag_export_paths_for_item(&item);
    if paths.is_empty() {
        return false;
    }
    release_main_pointer(hwnd);
    platform_dragdrop::begin_file_drag(&paths)
}

pub(super) fn clear_hotkey_passthrough_state(state: &mut AppState) {
    state.hotkey_passthrough_active = false;
    state.hotkey_passthrough_target = null_mut();
    state.hotkey_passthrough_focus = null_mut();
    state.hotkey_passthrough_edit = null_mut();
}
