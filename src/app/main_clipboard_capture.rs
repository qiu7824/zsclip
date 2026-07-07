use super::prelude::*;

unsafe fn clipboard_source_app_name() -> String {
    let owner = platform_clipboard::owner();
    let identity_host = WindowsWindowIdentityHost::new();
    let owner_root = identity_host.root_handle(owner);
    if !owner_root.is_null() && !is_app_window(owner_root) {
        let name = identity_host.process_name(owner_root);
        if !name.is_empty() {
            return name;
        }
    }
    let foreground = identity_host.foreground_handle();
    if foreground.is_null() || is_app_window(foreground) {
        return String::new();
    }
    identity_host.process_name(foreground)
}

unsafe fn foreground_source_app_name() -> String {
    let identity_host = WindowsWindowIdentityHost::new();
    let foreground = identity_host.foreground_handle();
    if foreground.is_null() || is_app_window(foreground) {
        return String::new();
    }
    identity_host.process_name(foreground)
}

fn is_self_clipboard_source_app(source_app: &str) -> bool {
    let source = source_app.trim().to_ascii_lowercase();
    if source.is_empty() {
        return false;
    }
    if matches!(source.as_str(), "zsclip.exe" | "剪贴板.exe") {
        return true;
    }
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().to_string())
        })
        .map(|name| name.trim().to_ascii_lowercase() == source)
        .unwrap_or(false)
}

pub(super) unsafe fn reset_clipboard_retry(hwnd: HWND, state: &mut AppState) {
    if state.clipboard_retry_timer {
        stop_flagged_timer(
            hwnd,
            ID_TIMER_CLIPBOARD_RETRY,
            &mut state.clipboard_retry_timer,
        );
    }
    state.clipboard_retry_sequence = 0;
    state.clipboard_retry_attempts = 0;
}

pub(super) fn source_app_is_browser(source_app: &str) -> bool {
    let source = source_app.trim().to_ascii_lowercase();
    [
        "chrome",
        "msedge",
        "firefox",
        "brave",
        "vivaldi",
        "opera",
        "iexplore",
        "qqbrowser",
        "catsxp",
        "360se",
        "360chrome",
    ]
    .iter()
    .any(|name| source.contains(name))
}

fn source_app_is_clipboard_proxy(source_app: &str) -> bool {
    let source = source_app.trim().to_ascii_lowercase();
    source.contains("doubao") || source.contains("豆包")
}

fn text_line_is_url(line: &str) -> bool {
    let value = line.trim();
    value.starts_with("http://")
        || value.starts_with("https://")
        || value.starts_with("ftp://")
        || value.starts_with("file://")
}

pub(super) fn browser_download_selection_should_skip(
    source_app: &str,
    foreground_app: &str,
    text: &str,
    url_payloads: &[String],
) -> bool {
    if !source_app_is_browser(source_app)
        && !source_app_is_browser(foreground_app)
        && !source_app_is_clipboard_proxy(source_app)
    {
        return false;
    }
    let text = text.trim();
    if text.is_empty() || text.len() > 4096 {
        return false;
    }
    let lines: Vec<&str> = text
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if lines.is_empty() || lines.iter().all(|line| text_line_is_url(line)) {
        return false;
    }
    let url_payload_present = url_payloads.iter().any(|payload| {
        payload
            .lines()
            .map(str::trim)
            .any(|line| text_line_is_url(line))
    });
    if url_payload_present {
        return true;
    }
    lines.len() > 1
        && lines.iter().any(|line| text_line_is_url(line))
        && lines.iter().any(|line| {
            !text_line_is_url(line)
                && line.len() <= 260
                && !line.contains('\\')
                && !line.contains('/')
        })
}

fn clipboard_has_image_payload_format(
    snapshot: &platform_clipboard::ClipboardFormatSnapshot,
) -> bool {
    snapshot.has_image
}

fn path_has_image_extension(path: &str) -> bool {
    let Some(ext) = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(|value| value.trim().to_ascii_lowercase())
    else {
        return false;
    };
    matches!(
        ext.as_str(),
        "png"
            | "jpg"
            | "jpeg"
            | "bmp"
            | "gif"
            | "webp"
            | "tif"
            | "tiff"
            | "ico"
            | "heic"
            | "heif"
            | "avif"
    )
}

fn path_looks_like_windows_screen_clip(path: &str) -> bool {
    let normalized = path.replace('\\', "/").to_ascii_lowercase();
    (normalized.contains("/packages/") && normalized.contains("/tempstate/screenclip"))
        || normalized.contains("microsoft.screensketch")
        || normalized.contains("/snippingtool/")
}

pub(super) fn paths_look_like_windows_screen_clip(paths: &[String]) -> bool {
    !paths.is_empty()
        && paths.iter().all(|path| path_has_image_extension(path))
        && paths
            .iter()
            .any(|path| path_looks_like_windows_screen_clip(path))
}

fn clipboard_file_paths_should_yield_to_image(
    paths: &[String],
    pixpin_format: bool,
    snapshot: &platform_clipboard::ClipboardFormatSnapshot,
) -> bool {
    if paths.is_empty() {
        return false;
    }
    let all_image_paths = paths.iter().all(|path| path_has_image_extension(path));
    if pixpin_format && all_image_paths {
        return true;
    }
    clipboard_has_image_payload_format(snapshot) && (pixpin_format || all_image_paths)
}

fn normalized_image_payload_from_paths(paths: &[String]) -> Option<(Vec<u8>, usize, usize)> {
    for path in paths {
        if !path_has_image_extension(path) {
            continue;
        }
        let Some((bytes, width, height)) = load_image_bytes_from_path(path) else {
            continue;
        };
        if let Some(normalized) = normalize_captured_image_rgba(bytes, width, height) {
            return Some(normalized);
        }
    }
    None
}

fn clipboard_retry_delay_for_attempt(attempt: u8) -> u32 {
    match attempt {
        0 | 1 => CLIPBOARD_RETRY_DELAY_FAST_MS,
        2 => CLIPBOARD_RETRY_DELAY_MEDIUM_MS,
        _ => CLIPBOARD_RETRY_DELAY_MS,
    }
}

unsafe fn schedule_clipboard_retry_with_limit(
    hwnd: HWND,
    state: &mut AppState,
    sequence: u32,
    max_attempts: u8,
) -> bool {
    if sequence == 0 {
        return false;
    }
    if state.clipboard_retry_sequence != sequence {
        reset_clipboard_retry(hwnd, state);
        state.clipboard_retry_sequence = sequence;
    }
    let limit = max_attempts.max(CLIPBOARD_RETRY_MAX_ATTEMPTS);
    if state.clipboard_retry_attempts >= limit {
        return false;
    }
    state.clipboard_retry_attempts += 1;
    let delay_ms = clipboard_retry_delay_for_attempt(state.clipboard_retry_attempts);
    if state.clipboard_retry_timer {
        timer::start(hwnd, ID_TIMER_CLIPBOARD_RETRY, delay_ms);
    } else {
        start_flagged_timer(
            hwnd,
            ID_TIMER_CLIPBOARD_RETRY,
            delay_ms,
            &mut state.clipboard_retry_timer,
        );
    }
    true
}

fn source_app_prefers_long_clipboard_retry(source_app: &str) -> bool {
    let source = source_app.trim().to_ascii_lowercase();
    source.contains("pixpin") || source_app_is_windows_screenshot_tool(source_app)
}

fn source_app_is_windows_screenshot_tool(source_app: &str) -> bool {
    let source = source_app.trim().to_ascii_lowercase();
    source.contains("snippingtool")
        || source.contains("screenclippinghost")
        || source.contains("snipandsketch")
        || source.contains("screenclip")
}

pub(super) fn normalize_captured_text(raw: &str) -> String {
    let mut normalized = raw
        .replace("\r\n", "\n")
        .replace('\r', "\n")
        .chars()
        .filter(|ch| !matches!(*ch, '\u{200B}' | '\u{200C}' | '\u{200D}' | '\u{FEFF}'))
        .collect::<String>();
    normalized = normalized
        .lines()
        .map(|line| line.trim_end())
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string();
    if normalized.contains('\n') {
        return normalized;
    }
    if normalized.starts_with("http://")
        || normalized.starts_with("https://")
        || normalized.starts_with("www.")
    {
        return normalized
            .split_whitespace()
            .next()
            .unwrap_or(&normalized)
            .to_string();
    }
    normalized
}

pub(super) fn normalize_captured_image_rgba(
    mut bytes: Vec<u8>,
    width: usize,
    height: usize,
) -> Option<(Vec<u8>, usize, usize)> {
    if width == 0 || height == 0 {
        return None;
    }
    let expected = width.checked_mul(height)?.checked_mul(4)?;
    if bytes.len() < expected {
        return None;
    }
    if bytes.len() > expected {
        bytes.truncate(expected);
    }
    normalize_clipboard_rgba_alpha(&mut bytes);
    if width <= MAX_CAPTURE_SIDE
        && height <= MAX_CAPTURE_SIDE
        && width.saturating_mul(height) <= MAX_CAPTURE_PIXELS
    {
        return Some((bytes, width, height));
    }

    let scale_by_pixels = ((MAX_CAPTURE_PIXELS as f64) / ((width as f64) * (height as f64))).sqrt();
    let scale_by_side =
        (MAX_CAPTURE_SIDE as f64 / width as f64).min(MAX_CAPTURE_SIDE as f64 / height as f64);
    let scale = scale_by_pixels.min(scale_by_side).min(1.0);
    let out_w = ((width as f64 * scale).round() as usize).max(1);
    let out_h = ((height as f64 * scale).round() as usize).max(1);
    let mut out = vec![0u8; out_w.checked_mul(out_h)?.checked_mul(4)?];
    for y in 0..out_h {
        let src_y = y * height / out_h;
        for x in 0..out_w {
            let src_x = x * width / out_w;
            let src_idx = (src_y * width + src_x) * 4;
            let dst_idx = (y * out_w + x) * 4;
            out[dst_idx..dst_idx + 4].copy_from_slice(&bytes[src_idx..src_idx + 4]);
        }
    }
    Some((out, out_w, out_h))
}

fn normalize_clipboard_rgba_alpha(bytes: &mut [u8]) {
    if bytes.len() < 4 {
        return;
    }
    let all_zero_alpha = bytes.chunks_exact(4).all(|pixel| pixel[3] == 0);
    if all_zero_alpha {
        for pixel in bytes.chunks_exact_mut(4) {
            pixel[3] = 255;
        }
    }
}

fn read_windows_clipboard_bitmap_rgba() -> Option<(Vec<u8>, usize, usize)> {
    let bitmap_bytes: Vec<u8> =
        clipboard_win::get_clipboard(clipboard_win::formats::Bitmap).ok()?;
    let (declared_width, declared_height) = clipboard_bitmap_dimensions(&bitmap_bytes)?;
    let declared_pixels = declared_width.checked_mul(declared_height)?;
    if declared_width > MAX_CAPTURE_SIDE
        || declared_height > MAX_CAPTURE_SIDE
        || declared_pixels > MAX_CLIPBOARD_BITMAP_DECODE_PIXELS
    {
        return None;
    }
    let image = image::load_from_memory_with_format(&bitmap_bytes, ImageFormat::Bmp).ok()?;
    let rgba = image.to_rgba8();
    let (width, height) = rgba.dimensions();
    Some((rgba.into_raw(), width as usize, height as usize))
}

fn clipboard_bitmap_dimensions(bytes: &[u8]) -> Option<(usize, usize)> {
    if bytes.len() < 16 {
        return None;
    }
    if bytes.starts_with(b"BM") && bytes.len() >= 26 {
        let width = i32::from_le_bytes(bytes.get(18..22)?.try_into().ok()?);
        let height = i32::from_le_bytes(bytes.get(22..26)?.try_into().ok()?);
        return bitmap_dimension_pair(width, height);
    }
    let header_size = u32::from_le_bytes(bytes.get(0..4)?.try_into().ok()?);
    match header_size {
        12 if bytes.len() >= 8 => {
            let width = u16::from_le_bytes(bytes.get(4..6)?.try_into().ok()?) as i32;
            let height = u16::from_le_bytes(bytes.get(6..8)?.try_into().ok()?) as i32;
            bitmap_dimension_pair(width, height)
        }
        40..=124 if bytes.len() >= 12 => {
            let width = i32::from_le_bytes(bytes.get(4..8)?.try_into().ok()?);
            let height = i32::from_le_bytes(bytes.get(8..12)?.try_into().ok()?);
            bitmap_dimension_pair(width, height)
        }
        _ => None,
    }
}

fn bitmap_dimension_pair(width: i32, height: i32) -> Option<(usize, usize)> {
    if width == 0 || height == 0 {
        return None;
    }
    Some((
        width.unsigned_abs() as usize,
        height.unsigned_abs() as usize,
    ))
}

unsafe fn add_captured_image_item(
    hwnd: HWND,
    state: &mut AppState,
    bytes: Vec<u8>,
    width: usize,
    height: usize,
    source_app: &str,
    sequence: u32,
) -> bool {
    let sig = image_content_signature(&bytes, width, height);
    if state.consume_recent_programmatic_clipboard_signature(&sig) {
        reset_clipboard_retry(hwnd, state);
        return true;
    }
    if state.should_skip_transient_duplicate_capture(&sig, source_app, sequence) {
        reset_clipboard_retry(hwnd, state);
        return true;
    }
    let image_path = write_image_bytes_to_output_path(&bytes, width as u32, height as u32);
    let image_bytes = if image_path.is_none() {
        Some(bytes)
    } else {
        None
    };
    let preview = format_local_time_for_image_preview();
    let candidate = ClipItem {
        id: 0,
        kind: ClipKind::Image,
        preview,
        text: None,
        rich_text_html: None,
        source_app: source_app.to_string(),
        file_paths: None,
        image_bytes,
        image_path: image_path.map(|p| p.to_string_lossy().to_string()),
        image_width: width,
        image_height: height,
        pinned: false,
        group_id: 0,
        created_at: String::new(),
    };
    state.add_clip_item(candidate, sig);
    reset_clipboard_retry(hwnd, state);
    repaint_main_window(hwnd, true);
    true
}

pub(super) fn clipboard_capture_allowed(settings: &AppSettings) -> bool {
    settings.clipboard_capture_enabled
}

fn remember_clipboard_sequence(state: &mut AppState, sequence: u32) {
    if sequence != 0 {
        state.last_clipboard_seq = sequence;
    }
}

pub(super) unsafe fn capture_clipboard(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let mut sequence = platform_clipboard::WindowsClipboardHost::sequence_number();
    if !clipboard_capture_allowed(&state.settings) {
        remember_clipboard_sequence(state, sequence);
        reset_clipboard_retry(hwnd, state);
        return;
    }

    let snapshot = platform_clipboard::snapshot_formats();
    if snapshot.sequence != 0 {
        sequence = snapshot.sequence;
    }
    if sequence != 0
        && state.clipboard_retry_sequence != 0
        && state.clipboard_retry_sequence != sequence
    {
        reset_clipboard_retry(hwnd, state);
    }
    if state.consume_skip_next_clipboard_update_once(sequence) {
        reset_clipboard_retry(hwnd, state);
        return;
    }
    if let Some(until) = state.ignore_clipboard_until {
        if Instant::now() < until {
            remember_clipboard_sequence(state, sequence);
            reset_clipboard_retry(hwnd, state);
            return;
        }
        state.ignore_clipboard_until = None;
    }
    if snapshot.open_failed {
        let _ = schedule_clipboard_retry_with_limit(
            hwnd,
            state,
            sequence,
            CLIPBOARD_RETRY_MAX_ATTEMPTS,
        );
        return;
    }
    if snapshot.has_ignore_capture_format
        || snapshot.has_only_custom_formats
        || platform_clipboard::should_ignore_capture_by_snapshot(&snapshot)
    {
        remember_clipboard_sequence(state, sequence);
        reset_clipboard_retry(hwnd, state);
        return;
    }
    let pixpin_format = snapshot.has_named_format("PixPinData");
    let source_app = clipboard_source_app_name();
    let foreground_app = foreground_source_app_name();
    let mut prefer_long_retry = source_app_prefers_long_clipboard_retry(&source_app);
    if is_self_clipboard_source_app(&source_app) {
        remember_clipboard_sequence(state, sequence);
        reset_clipboard_retry(hwnd, state);
        return;
    }

    let file_paths = if snapshot.has_files {
        platform_clipboard::WindowsClipboardHost::read_file_paths()
    } else {
        None
    };
    let windows_screenshot_image_paths = file_paths
        .as_ref()
        .map(|paths| {
            paths_look_like_windows_screen_clip(paths)
                || (source_app_is_windows_screenshot_tool(&source_app)
                    && !paths.is_empty()
                    && paths.iter().all(|path| path_has_image_extension(path)))
        })
        .unwrap_or(false);
    if windows_screenshot_image_paths {
        prefer_long_retry = true;
    }
    let file_paths_yield_to_image = file_paths
        .as_ref()
        .map(|paths| {
            windows_screenshot_image_paths
                || clipboard_file_paths_should_yield_to_image(paths, pixpin_format, &snapshot)
        })
        .unwrap_or(false);
    if file_paths_yield_to_image {
        if let Some(paths) = file_paths.as_ref() {
            if let Some((bytes, width, height)) = normalized_image_payload_from_paths(paths) {
                let _ = add_captured_image_item(
                    hwnd,
                    state,
                    bytes,
                    width,
                    height,
                    source_app.as_str(),
                    sequence,
                );
                return;
            }
            if (windows_screenshot_image_paths
                || source_app_is_windows_screenshot_tool(&source_app))
                && !clipboard_has_image_payload_format(&snapshot)
            {
                let scheduled = schedule_clipboard_retry_with_limit(
                    hwnd,
                    state,
                    sequence,
                    PIXPIN_CLIPBOARD_RETRY_MAX_ATTEMPTS,
                );
                if scheduled {
                    return;
                }
                reset_clipboard_retry(hwnd, state);
                return;
            }
        }
    }
    if let Some(paths) = file_paths.filter(|_| !file_paths_yield_to_image) {
        let preview = build_files_preview(&paths);
        let sig = file_paths_signature(&paths);
        if state.consume_recent_programmatic_clipboard_signature(&sig) {
            reset_clipboard_retry(hwnd, state);
            return;
        }
        if state.should_skip_transient_duplicate_capture(&sig, source_app.as_str(), sequence) {
            reset_clipboard_retry(hwnd, state);
            return;
        }
        let candidate = ClipItem {
            id: 0,
            kind: ClipKind::Files,
            preview,
            text: Some(paths.join("\n")),
            rich_text_html: None,
            source_app: source_app.clone(),
            file_paths: Some(paths),
            image_bytes: None,
            image_path: None,
            image_width: 0,
            image_height: 0,
            pinned: false,
            group_id: 0,
            created_at: String::new(),
        };
        state.add_clip_item(candidate, sig);
        reset_clipboard_retry(hwnd, state);
        repaint_main_window(hwnd, true);
        return;
    }

    if snapshot.has_text {
        if let Some(text) = platform_clipboard::WindowsClipboardHost::read_text() {
            let normalized = normalize_captured_text(&text);
            if !normalized.is_empty() {
                let url_payloads = if source_app_is_browser(&source_app)
                    || source_app_is_browser(&foreground_app)
                    || source_app_is_clipboard_proxy(&source_app)
                {
                    platform_clipboard::url_format_payloads_from_snapshot(&snapshot)
                } else {
                    Vec::new()
                };
                if browser_download_selection_should_skip(
                    &source_app,
                    &foreground_app,
                    &normalized,
                    &url_payloads,
                ) {
                    reset_clipboard_retry(hwnd, state);
                    return;
                }
                let rich_text_html = if state.settings.rich_text_clipboard_enabled {
                    platform_clipboard::html_format_payload_from_snapshot(&snapshot)
                        .filter(|html| !html.trim().is_empty())
                } else {
                    None
                };
                let preview = rich_text_html
                    .as_deref()
                    .map(|html| build_rich_text_preview(html, &normalized))
                    .unwrap_or_else(|| build_preview(&normalized));
                let sig = rich_text_html
                    .as_deref()
                    .map(|html| rich_text_content_signature(&normalized, html))
                    .unwrap_or_else(|| text_content_signature(&normalized));
                if state.consume_recent_programmatic_clipboard_signature(&sig) {
                    reset_clipboard_retry(hwnd, state);
                    return;
                }
                if state.should_skip_transient_duplicate_capture(
                    &sig,
                    source_app.as_str(),
                    sequence,
                ) {
                    reset_clipboard_retry(hwnd, state);
                    return;
                }
                let candidate = ClipItem {
                    id: 0,
                    kind: ClipKind::Text,
                    preview,
                    text: Some(normalized),
                    rich_text_html,
                    source_app: source_app.clone(),
                    file_paths: None,
                    image_bytes: None,
                    image_path: None,
                    image_width: 0,
                    image_height: 0,
                    pinned: false,
                    group_id: 0,
                    created_at: String::new(),
                };
                state.add_clip_item(candidate, sig);
                reset_clipboard_retry(hwnd, state);
                repaint_main_window(hwnd, true);
                return;
            }
        }
    }

    if snapshot.has_image {
        if let Some((bytes, width, height)) =
            platform_clipboard::WindowsClipboardHost::read_image_rgba()
        {
            if let Some((bytes, norm_w, norm_h)) =
                normalize_captured_image_rgba(bytes, width, height)
            {
                let _ = add_captured_image_item(
                    hwnd,
                    state,
                    bytes,
                    norm_w,
                    norm_h,
                    source_app.as_str(),
                    sequence,
                );
                return;
            } else if schedule_clipboard_retry_with_limit(
                hwnd,
                state,
                sequence,
                if prefer_long_retry || pixpin_format {
                    PIXPIN_CLIPBOARD_RETRY_MAX_ATTEMPTS
                } else {
                    CLIPBOARD_RETRY_MAX_ATTEMPTS
                },
            ) {
                return;
            } else {
                reset_clipboard_retry(hwnd, state);
            }
        }

        if let Some((bytes, width, height)) = read_windows_clipboard_bitmap_rgba() {
            if let Some((bytes, norm_w, norm_h)) =
                normalize_captured_image_rgba(bytes, width, height)
            {
                let _ = add_captured_image_item(
                    hwnd,
                    state,
                    bytes,
                    norm_w,
                    norm_h,
                    source_app.as_str(),
                    sequence,
                );
                return;
            }
        }
    }

    if sequence != 0 && snapshot.has_any_standard_format {
        let retry_limit = if prefer_long_retry || pixpin_format {
            PIXPIN_CLIPBOARD_RETRY_MAX_ATTEMPTS
        } else {
            CLIPBOARD_RETRY_MAX_ATTEMPTS
        };
        let _ = schedule_clipboard_retry_with_limit(hwnd, state, sequence, retry_limit);
    }
}

pub(super) unsafe fn capture_clipboard_guarded(hwnd: HWND) {
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        capture_clipboard(hwnd);
    }));
    if result.is_err() {
        let ptr = get_state_ptr(hwnd);
        if !ptr.is_null() {
            reset_clipboard_retry(hwnd, &mut *ptr);
        }
    }
}
