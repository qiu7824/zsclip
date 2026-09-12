use super::prelude::*;

pub(super) const WM_CLIPBOARD_CAPTURE_READ_READY: u32 =
    windows_sys::Win32::UI::WindowsAndMessaging::WM_APP + 42;
const CLIPBOARD_READ_HELPER_ARG: &str = "--zsclip-read-clipboard-helper";
const CLIPBOARD_READ_HELPER_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
const MAX_CLIPBOARD_HELPER_RESULT_BYTES: u64 = 128 * 1024 * 1024;
const CLIPBOARD_READ_QUEUE_CAPACITY: usize = 16;

#[derive(serde::Serialize, serde::Deserialize)]
enum ClipboardCaptureWirePayload {
    None,
    Files {
        paths: Vec<String>,
    },
    Text {
        normalized: String,
        rich_text_html: Option<String>,
    },
    Image {
        width: usize,
        height: usize,
    },
}

#[derive(serde::Serialize, serde::Deserialize)]
struct ClipboardCaptureWireResult {
    sequence: u32,
    source_app: String,
    payload: ClipboardCaptureWirePayload,
}

enum ClipboardCaptureReadPayload {
    None,
    Files {
        paths: Vec<String>,
    },
    Text {
        normalized: String,
        rich_text_html: Option<String>,
    },
    Image {
        bytes: Vec<u8>,
        width: usize,
        height: usize,
    },
}

struct ClipboardCaptureReadResult {
    sequence: u32,
    source_app: String,
    payload: ClipboardCaptureReadPayload,
}

struct ClipboardCaptureReadRequest {
    hwnd: isize,
    app_data_generation: u64,
    sequence: u32,
    rich_text_clipboard_enabled: bool,
}

struct ClipboardCaptureReadReady {
    app_data_generation: u64,
    result: ClipboardCaptureReadResult,
}

static CLIPBOARD_READ_SENDER: OnceLock<std::sync::mpsc::SyncSender<ClipboardCaptureReadRequest>> =
    OnceLock::new();
static CLIPBOARD_HELPER_TEMP_COUNTER: std::sync::atomic::AtomicU64 =
    std::sync::atomic::AtomicU64::new(1);

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

fn source_app_uses_fragile_delayed_clipboard_rendering(source_app: &str) -> bool {
    let source = source_app.trim().to_ascii_lowercase();
    [
        "cnext",
        "catia",
        "3dexperience",
        "3dexperience launcher",
        "3dswym",
        "enovia",
        "delmia",
        "simulia",
        "netvibes",
    ]
    .iter()
    .any(|name| source.contains(name))
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

fn empty_clipboard_capture_result(sequence: u32, source_app: String) -> ClipboardCaptureReadResult {
    ClipboardCaptureReadResult {
        sequence,
        source_app,
        payload: ClipboardCaptureReadPayload::None,
    }
}

fn read_clipboard_capture_result(
    mut sequence: u32,
    rich_text_enabled: bool,
) -> ClipboardCaptureReadResult {
    if !clipboard_sequence_is_current(sequence) {
        return empty_clipboard_capture_result(sequence, String::new());
    }

    let snapshot = platform_clipboard::snapshot_formats();
    if sequence != 0 && snapshot.sequence != 0 && snapshot.sequence != sequence {
        return empty_clipboard_capture_result(sequence, String::new());
    }
    if sequence == 0 && snapshot.sequence != 0 {
        sequence = snapshot.sequence;
    }

    let source_app = unsafe { clipboard_source_app_name() };
    if snapshot.open_failed || !clipboard_sequence_is_current(sequence) {
        return empty_clipboard_capture_result(sequence, source_app);
    }
    let foreground_app = unsafe { foreground_source_app_name() };
    if source_app_uses_fragile_delayed_clipboard_rendering(&source_app)
        || source_app_uses_fragile_delayed_clipboard_rendering(&foreground_app)
        || snapshot.has_ignore_capture_format
        || snapshot.has_only_custom_formats
        || platform_clipboard::should_ignore_capture_by_snapshot(&snapshot)
        || is_self_clipboard_source_app(&source_app)
    {
        return empty_clipboard_capture_result(sequence, source_app);
    }

    let pixpin_format = snapshot.has_named_format("PixPinData");
    let file_paths = if snapshot.has_files {
        platform_clipboard::WindowsClipboardHost::read_file_paths()
    } else {
        None
    };
    if !clipboard_sequence_is_current(sequence) {
        return empty_clipboard_capture_result(sequence, source_app);
    }

    let windows_screenshot_image_paths = file_paths
        .as_ref()
        .map(|paths| {
            paths_look_like_windows_screen_clip(paths)
                || (source_app_is_windows_screenshot_tool(&source_app)
                    && !paths.is_empty()
                    && paths.iter().all(|path| path_has_image_extension(path)))
        })
        .unwrap_or(false);
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
                return ClipboardCaptureReadResult {
                    sequence,
                    source_app,
                    payload: ClipboardCaptureReadPayload::Image {
                        bytes,
                        width,
                        height,
                    },
                };
            }
            if (windows_screenshot_image_paths
                || source_app_is_windows_screenshot_tool(&source_app))
                && !clipboard_has_image_payload_format(&snapshot)
            {
                return empty_clipboard_capture_result(sequence, source_app);
            }
        }
    }
    if let Some(paths) = file_paths.filter(|_| !file_paths_yield_to_image) {
        return ClipboardCaptureReadResult {
            sequence,
            source_app,
            payload: ClipboardCaptureReadPayload::Files { paths },
        };
    }

    if snapshot.has_text {
        if let Some(text) = platform_clipboard::read_text_for_sequence(sequence) {
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
                    return empty_clipboard_capture_result(sequence, source_app);
                }
                let rich_text_html = if rich_text_enabled {
                    platform_clipboard::html_format_payload_from_snapshot(&snapshot)
                        .filter(|html| !html.trim().is_empty())
                } else {
                    None
                };
                if !clipboard_sequence_is_current(sequence) {
                    return empty_clipboard_capture_result(sequence, source_app);
                }
                return ClipboardCaptureReadResult {
                    sequence,
                    source_app,
                    payload: ClipboardCaptureReadPayload::Text {
                        normalized,
                        rich_text_html,
                    },
                };
            }
        }
    }

    if snapshot.has_image && clipboard_sequence_is_current(sequence) {
        if let Some((bytes, width, height)) = guarded_read_clipboard_image_rgba() {
            if clipboard_sequence_is_current(sequence) {
                if let Some((bytes, width, height)) =
                    normalize_captured_image_rgba(bytes, width, height)
                {
                    return ClipboardCaptureReadResult {
                        sequence,
                        source_app,
                        payload: ClipboardCaptureReadPayload::Image {
                            bytes,
                            width,
                            height,
                        },
                    };
                }
            }
        }
        if clipboard_sequence_is_current(sequence) {
            if let Some((bytes, width, height)) = guarded_read_windows_clipboard_bitmap_rgba() {
                if clipboard_sequence_is_current(sequence) {
                    if let Some((bytes, width, height)) =
                        normalize_captured_image_rgba(bytes, width, height)
                    {
                        return ClipboardCaptureReadResult {
                            sequence,
                            source_app,
                            payload: ClipboardCaptureReadPayload::Image {
                                bytes,
                                width,
                                height,
                            },
                        };
                    }
                }
            }
        }
    }

    empty_clipboard_capture_result(sequence, source_app)
}

fn write_clipboard_helper_result(
    result: ClipboardCaptureReadResult,
    result_path: &Path,
    rgba_path: &Path,
) -> Result<(), String> {
    let payload = match result.payload {
        ClipboardCaptureReadPayload::None => ClipboardCaptureWirePayload::None,
        ClipboardCaptureReadPayload::Files { paths } => {
            ClipboardCaptureWirePayload::Files { paths }
        }
        ClipboardCaptureReadPayload::Text {
            normalized,
            rich_text_html,
        } => ClipboardCaptureWirePayload::Text {
            normalized,
            rich_text_html,
        },
        ClipboardCaptureReadPayload::Image {
            bytes,
            width,
            height,
        } => {
            let expected = width
                .checked_mul(height)
                .and_then(|pixels| pixels.checked_mul(4))
                .ok_or_else(|| "clipboard image dimensions overflow".to_string())?;
            if expected != bytes.len() {
                return Err("clipboard image payload length mismatch".to_string());
            }
            std::fs::write(rgba_path, bytes).map_err(|err| err.to_string())?;
            ClipboardCaptureWirePayload::Image { width, height }
        }
    };
    let wire = ClipboardCaptureWireResult {
        sequence: result.sequence,
        source_app: result.source_app,
        payload,
    };
    let json = serde_json::to_vec(&wire).map_err(|err| err.to_string())?;
    if json.len() as u64 > MAX_CLIPBOARD_HELPER_RESULT_BYTES {
        return Err("clipboard helper result is too large".to_string());
    }
    std::fs::write(result_path, json).map_err(|err| err.to_string())
}

pub(crate) fn maybe_run_clipboard_read_helper_from_args() -> Option<i32> {
    let mut args = std::env::args_os();
    let _ = args.next();
    if args.next().as_deref() != Some(std::ffi::OsStr::new(CLIPBOARD_READ_HELPER_ARG)) {
        return None;
    }
    let Some(sequence) = args
        .next()
        .and_then(|value| value.to_string_lossy().parse::<u32>().ok())
    else {
        return Some(2);
    };
    let Some(rich_text_enabled) =
        args.next()
            .and_then(|value| match value.to_string_lossy().as_ref() {
                "0" => Some(false),
                "1" => Some(true),
                _ => None,
            })
    else {
        return Some(2);
    };
    let Some(result_path) = args.next().map(PathBuf::from) else {
        return Some(2);
    };
    let Some(rgba_path) = args.next().map(PathBuf::from) else {
        return Some(2);
    };

    let result =
        std::panic::catch_unwind(|| read_clipboard_capture_result(sequence, rich_text_enabled))
            .map_err(|_| "clipboard helper panicked".to_string())
            .and_then(|result| write_clipboard_helper_result(result, &result_path, &rgba_path));
    Some(if result.is_ok() { 0 } else { 1 })
}

fn clipboard_helper_temp_paths() -> (PathBuf, PathBuf) {
    let id = CLIPBOARD_HELPER_TEMP_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let base = format!(
        "zsclip-clipboard-{}-{id}",
        platform_process::current_process_id()
    );
    let temp_dir = std::env::temp_dir();
    (
        temp_dir.join(format!("{base}.json")),
        temp_dir.join(format!("{base}.rgba")),
    )
}

fn decode_clipboard_helper_result(
    result_path: &Path,
    rgba_path: &Path,
) -> Option<ClipboardCaptureReadResult> {
    let metadata = std::fs::metadata(result_path).ok()?;
    if metadata.len() == 0 || metadata.len() > MAX_CLIPBOARD_HELPER_RESULT_BYTES {
        return None;
    }
    let wire: ClipboardCaptureWireResult =
        serde_json::from_slice(&std::fs::read(result_path).ok()?).ok()?;
    let payload = match wire.payload {
        ClipboardCaptureWirePayload::None => ClipboardCaptureReadPayload::None,
        ClipboardCaptureWirePayload::Files { paths } => {
            ClipboardCaptureReadPayload::Files { paths }
        }
        ClipboardCaptureWirePayload::Text {
            normalized,
            rich_text_html,
        } => ClipboardCaptureReadPayload::Text {
            normalized,
            rich_text_html,
        },
        ClipboardCaptureWirePayload::Image { width, height } => {
            let expected = width
                .checked_mul(height)
                .and_then(|pixels| pixels.checked_mul(4))?;
            if expected as u64 > MAX_CLIPBOARD_HELPER_RESULT_BYTES {
                return None;
            }
            let metadata = std::fs::metadata(rgba_path).ok()?;
            if metadata.len() != expected as u64 {
                return None;
            }
            ClipboardCaptureReadPayload::Image {
                bytes: std::fs::read(rgba_path).ok()?,
                width,
                height,
            }
        }
    };
    Some(ClipboardCaptureReadResult {
        sequence: wire.sequence,
        source_app: wire.source_app,
        payload,
    })
}

fn run_clipboard_read_helper(
    request: &ClipboardCaptureReadRequest,
) -> Option<ClipboardCaptureReadResult> {
    use std::os::windows::process::CommandExt;

    let executable = std::env::current_exe().ok()?;
    let (result_path, rgba_path) = clipboard_helper_temp_paths();
    let result = (|| {
        let mut child = std::process::Command::new(executable)
            .arg(CLIPBOARD_READ_HELPER_ARG)
            .arg(request.sequence.to_string())
            .arg(if request.rich_text_clipboard_enabled {
                "1"
            } else {
                "0"
            })
            .arg(&result_path)
            .arg(&rgba_path)
            .creation_flags(0x0800_0000)
            .spawn()
            .ok()?;
        let deadline = Instant::now() + CLIPBOARD_READ_HELPER_TIMEOUT;
        loop {
            match child.try_wait() {
                Ok(Some(status)) => {
                    if !status.success() {
                        return None;
                    }
                    return decode_clipboard_helper_result(&result_path, &rgba_path);
                }
                Ok(None) if Instant::now() < deadline => {
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
                Ok(None) | Err(_) => {
                    let _ = child.kill();
                    return None;
                }
            }
        }
    })();
    let _ = std::fs::remove_file(&result_path);
    let _ = std::fs::remove_file(&rgba_path);
    result
}

fn clipboard_read_sender() -> &'static std::sync::mpsc::SyncSender<ClipboardCaptureReadRequest> {
    CLIPBOARD_READ_SENDER.get_or_init(|| {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<ClipboardCaptureReadRequest>(
            CLIPBOARD_READ_QUEUE_CAPACITY,
        );
        let _ = std::thread::Builder::new()
            .name("zsclip-clipboard-read".to_string())
            .spawn(move || {
                while let Ok(request) = receiver.recv() {
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        run_clipboard_read_helper(&request)
                    }))
                    .ok()
                    .flatten();
                    if let Some(result) = result {
                        let ready = ClipboardCaptureReadReady {
                            app_data_generation: request.app_data_generation,
                            result,
                        };
                        unsafe {
                            let _ = post_boxed_message(
                                request.hwnd,
                                WM_CLIPBOARD_CAPTURE_READ_READY,
                                0,
                                Box::new(ready),
                            );
                        }
                    }
                }
            });
        sender
    })
}

struct CapturedItemDbRequest {
    hwnd: isize,
    app_data_generation: u64,
    item: ClipItem,
    signature: String,
    full_dedupe: bool,
    max_items: usize,
}

static CAPTURED_ITEM_DB_SENDER: OnceLock<std::sync::mpsc::SyncSender<CapturedItemDbRequest>> =
    OnceLock::new();

fn process_captured_item_db_request_locked(
    mut item: ClipItem,
    signature: &str,
    full_dedupe: bool,
    max_items: usize,
    removed_ids: &mut Vec<i64>,
) -> CapturedItemDbAction {
    if !signature.is_empty()
        && db_latest_item_signature(0)
            .as_deref()
            .is_some_and(|latest| latest == signature)
    {
        remove_uninserted_image_file(&item);
        return CapturedItemDbAction::Duplicate;
    }

    if full_dedupe && !signature.is_empty() {
        let duplicate_ids = db_find_duplicate_item_ids(0, &item, signature);
        if let Some(existing_id) = duplicate_ids.first().copied() {
            let existing_pinned = db_item_is_pinned(existing_id);
            *removed_ids = if existing_pinned {
                duplicate_ids
                    .iter()
                    .copied()
                    .filter(|id| *id != existing_id && !db_item_is_pinned(*id))
                    .collect()
            } else {
                duplicate_ids.into_iter().skip(1).collect()
            };
            for id in removed_ids.iter().copied() {
                let _ = db_delete_item(id);
            }
            if existing_pinned {
                remove_uninserted_image_file(&item);
                return CapturedItemDbAction::Duplicate;
            }
            match db_promote_item_to_top(existing_id) {
                Ok(new_id) => {
                    remove_uninserted_image_file(&item);
                    return CapturedItemDbAction::Promoted {
                        old_id: existing_id,
                        new_id,
                    };
                }
                Err(_) => {
                    remove_uninserted_image_file(&item);
                    return CapturedItemDbAction::RetryableFailure;
                }
            }
        }
    }

    item.id = db_insert_item(0, &item, Some(signature)).unwrap_or(0);
    if item.id <= 0 {
        remove_uninserted_image_file(&item);
        return CapturedItemDbAction::RetryableFailure;
    }
    if item.created_at.is_empty() {
        item.created_at = now_utc_sqlite();
    }
    db_prune_items(0, max_items);
    CapturedItemDbAction::Inserted { item }
}

fn process_captured_item_db_request(request: CapturedItemDbRequest) -> CapturedItemDbReadyResult {
    let signature = dedupe_signature_for_item(&request.item, &request.signature);
    let cleanup_item = request.item.clone();
    let mut removed_ids = Vec::new();
    let action =
        crate::db_runtime::with_shared_app_data_generation(request.app_data_generation, || {
            process_captured_item_db_request_locked(
                request.item,
                &signature,
                request.full_dedupe,
                request.max_items,
                &mut removed_ids,
            )
        })
        .unwrap_or_else(|| {
            remove_uninserted_image_file(&cleanup_item);
            CapturedItemDbAction::RetryableFailure
        });
    CapturedItemDbReadyResult {
        app_data_generation: request.app_data_generation,
        action,
        removed_ids,
        signature,
    }
}

fn captured_item_db_sender() -> &'static std::sync::mpsc::SyncSender<CapturedItemDbRequest> {
    CAPTURED_ITEM_DB_SENDER.get_or_init(|| {
        let (sender, receiver) = std::sync::mpsc::sync_channel::<CapturedItemDbRequest>(64);
        let _ = std::thread::Builder::new()
            .name("zsclip-capture-db".to_string())
            .spawn(move || {
                while let Ok(request) = receiver.recv() {
                    let hwnd = request.hwnd;
                    let result = process_captured_item_db_request(request);
                    unsafe {
                        let _ = post_boxed_message(
                            hwnd,
                            WM_CAPTURED_ITEM_DB_READY,
                            0,
                            Box::new(result),
                        );
                    }
                }
            });
        sender
    })
}

fn queue_captured_item_add(
    hwnd: HWND,
    state: &AppState,
    item: ClipItem,
    signature: String,
) -> bool {
    let request = CapturedItemDbRequest {
        hwnd: hwnd as isize,
        app_data_generation: state.app_data_generation,
        item,
        signature,
        full_dedupe: state.settings.dedupe_filter_enabled,
        max_items: state.settings.max_items,
    };
    match captured_item_db_sender().try_send(request) {
        Ok(()) => true,
        Err(std::sync::mpsc::TrySendError::Full(request))
        | Err(std::sync::mpsc::TrySendError::Disconnected(request)) => {
            remove_uninserted_image_file(&request.item);
            false
        }
    }
}

pub(super) unsafe fn apply_captured_item_db_ready(hwnd: HWND, payload: CapturedItemDbReadyResult) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    if payload.app_data_generation != state.app_data_generation
        || state.app_data_generation != crate::db_runtime::current_app_data_generation()
    {
        return;
    }
    if !payload.action.completed() {
        return;
    }

    let applied = payload.action.applied();
    let database_changed = applied || !payload.removed_ids.is_empty();
    let anchor = state.current_scroll_anchor();
    for id in &payload.removed_ids {
        state.remove_cached_item(*id);
    }
    state.remove_duplicate_history_items(&payload.removed_ids);

    match payload.action {
        CapturedItemDbAction::Duplicate => {
            if !payload.removed_ids.is_empty() {
                state.reload_state_from_db_preserve_scroll(anchor);
            }
        }
        CapturedItemDbAction::Promoted { old_id, new_id } => {
            state.remove_cached_item(old_id);
            state.remove_cached_item(new_id);
            if state.promote_loaded_item_to_top(old_id, new_id).is_none() {
                reload_state_from_db_persisting(state);
            } else {
                state.refilter();
            }
            if state.tab_index == 0 {
                state.sel_idx = state
                    .records
                    .iter()
                    .position(|item| item.id == new_id)
                    .unwrap_or(0) as i32;
                state.scroll_y = 0;
            }
        }
        CapturedItemDbAction::Inserted { item } => {
            state.cache_full_item(item.clone());
            let summary = clip_item_to_summary(&item);
            let visible_query = state.load_state_for_tab(0).query.clone();
            let inserted_index = if matches!(visible_query, Some(ref query) if query.group_id == 0 && query.search_text.trim().is_empty())
            {
                let index = state.insert_loaded_record_at_sorted_front(summary);
                if state.tab_index == 0 {
                    state.list.apply_visible_len(state.records.len());
                }
                Some(index)
            } else {
                state.invalidate_tab_query(0, state.tab_index == 0);
                None
            };
            if state.settings.max_items > 0 {
                state.invalidate_tab_query(0, state.tab_index == 0);
            }
            if state.tab_index == 0 {
                state.sel_idx = inserted_index.unwrap_or(0) as i32;
            }
            state.refilter();
            maybe_broadcast_lan_clip_item(state, &item, &payload.signature);
        }
        CapturedItemDbAction::RetryableFailure => return,
    }

    if database_changed {
        sync_peer_windows_from_db(state.hwnd);
        refresh_lan_latest_from_db(&state.settings);
    }
    repaint_main_window(hwnd, true);
    play_copy_success_sound_if_enabled(state, applied);
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

fn guarded_read_clipboard_image_rgba() -> Option<(Vec<u8>, usize, usize)> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        platform_clipboard::WindowsClipboardHost::read_image_rgba()
    }))
    .ok()
    .flatten()
}

fn guarded_read_windows_clipboard_bitmap_rgba() -> Option<(Vec<u8>, usize, usize)> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(
        read_windows_clipboard_bitmap_rgba,
    ))
    .ok()
    .flatten()
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
    if !clipboard_sequence_is_current(sequence) {
        return false;
    }
    let expected_generation = state.app_data_generation;
    match crate::db_runtime::with_shared_app_data_generation(expected_generation, || {
        add_captured_image_item_locked(hwnd, state, bytes, width, height, source_app, sequence)
    }) {
        Some(result) => result,
        None => false,
    }
}

unsafe fn add_captured_image_item_locked(
    hwnd: HWND,
    state: &mut AppState,
    bytes: Vec<u8>,
    width: usize,
    height: usize,
    source_app: &str,
    sequence: u32,
) -> bool {
    if !clipboard_sequence_is_current(sequence) {
        return false;
    }
    let sig = image_content_signature(&bytes, width, height);
    if state.consume_recent_programmatic_clipboard_signature(&sig) {
        return true;
    }
    if state.should_skip_transient_duplicate_capture(&sig, source_app, sequence) {
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
    queue_captured_item_add(hwnd, state, candidate, sig)
}

pub(super) fn clipboard_capture_allowed(settings: &AppSettings) -> bool {
    settings.clipboard_capture_enabled
}

fn remember_clipboard_sequence(state: &mut AppState, sequence: u32) {
    if sequence != 0 {
        state.last_clipboard_seq = sequence;
    }
}

pub(super) fn begin_clipboard_sequence_capture(terminal_sequence: &mut u32, sequence: u32) -> bool {
    if sequence == 0 {
        return true;
    }
    if *terminal_sequence == sequence {
        return false;
    }
    *terminal_sequence = sequence;
    true
}

fn clipboard_sequence_is_current(sequence: u32) -> bool {
    sequence == 0 || platform_clipboard::WindowsClipboardHost::sequence_number() == sequence
}

fn queue_clipboard_capture_read(hwnd: HWND, state: &AppState, sequence: u32) -> bool {
    let request = ClipboardCaptureReadRequest {
        hwnd: hwnd as isize,
        app_data_generation: state.app_data_generation,
        sequence,
        rich_text_clipboard_enabled: state.settings.rich_text_clipboard_enabled,
    };
    clipboard_read_sender().try_send(request).is_ok()
}

pub(super) unsafe fn apply_clipboard_capture_read_ready(hwnd: HWND, lparam: LPARAM) {
    if lparam == 0 {
        return;
    }
    let ready = *Box::from_raw(lparam as *mut ClipboardCaptureReadReady);
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    if ready.app_data_generation != state.app_data_generation
        || state.app_data_generation != crate::db_runtime::current_app_data_generation()
        || !clipboard_sequence_is_current(ready.result.sequence)
    {
        return;
    }

    let sequence = ready.result.sequence;
    let source_app = ready.result.source_app;
    match ready.result.payload {
        ClipboardCaptureReadPayload::None => {
            remember_clipboard_sequence(state, sequence);
        }
        ClipboardCaptureReadPayload::Files { paths } => {
            let preview = build_files_preview(&paths);
            let signature = file_paths_signature(&paths);
            if state.consume_recent_programmatic_clipboard_signature(&signature)
                || state.should_skip_transient_duplicate_capture(
                    &signature,
                    source_app.as_str(),
                    sequence,
                )
            {
                return;
            }
            let candidate = ClipItem {
                id: 0,
                kind: ClipKind::Files,
                preview,
                text: Some(paths.join("\n")),
                rich_text_html: None,
                source_app,
                file_paths: Some(paths),
                image_bytes: None,
                image_path: None,
                image_width: 0,
                image_height: 0,
                pinned: false,
                group_id: 0,
                created_at: String::new(),
            };
            let _ = queue_captured_item_add(hwnd, state, candidate, signature);
        }
        ClipboardCaptureReadPayload::Text {
            normalized,
            rich_text_html,
        } => {
            let preview = rich_text_html
                .as_deref()
                .map(|html| build_rich_text_preview(html, &normalized))
                .unwrap_or_else(|| build_preview(&normalized));
            let signature = rich_text_html
                .as_deref()
                .map(|html| rich_text_content_signature(&normalized, html))
                .unwrap_or_else(|| text_content_signature(&normalized));
            if state.consume_recent_programmatic_clipboard_signature(&signature)
                || state.should_skip_transient_duplicate_capture(
                    &signature,
                    source_app.as_str(),
                    sequence,
                )
            {
                return;
            }
            let candidate = ClipItem {
                id: 0,
                kind: ClipKind::Text,
                preview,
                text: Some(normalized),
                rich_text_html,
                source_app,
                file_paths: None,
                image_bytes: None,
                image_path: None,
                image_width: 0,
                image_height: 0,
                pinned: false,
                group_id: 0,
                created_at: String::new(),
            };
            let _ = queue_captured_item_add(hwnd, state, candidate, signature);
        }
        ClipboardCaptureReadPayload::Image {
            bytes,
            width,
            height,
        } => {
            let _ = add_captured_image_item(
                hwnd,
                state,
                bytes,
                width,
                height,
                source_app.as_str(),
                sequence,
            );
        }
    }
}

pub(super) unsafe fn capture_clipboard(hwnd: HWND) {
    let ptr = get_state_ptr(hwnd);
    if ptr.is_null() {
        return;
    }
    let state = &mut *ptr;
    let sequence = platform_clipboard::WindowsClipboardHost::sequence_number();
    if !begin_clipboard_sequence_capture(&mut state.clipboard_terminal_sequence, sequence) {
        return;
    }
    if state.app_data_generation != crate::db_runtime::current_app_data_generation() {
        return;
    }
    if !clipboard_capture_allowed(&state.settings) {
        remember_clipboard_sequence(state, sequence);
        return;
    }
    if state.consume_skip_next_clipboard_update_once(sequence) {
        return;
    }
    if let Some(until) = state.ignore_clipboard_until {
        if Instant::now() < until {
            remember_clipboard_sequence(state, sequence);
            return;
        }
        state.ignore_clipboard_until = None;
    }
    if !queue_clipboard_capture_read(hwnd, state, sequence) {
        remember_clipboard_sequence(state, sequence);
    }
}
pub(super) unsafe fn capture_clipboard_guarded(hwnd: HWND) {
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        capture_clipboard(hwnd);
    }));
}

#[cfg(test)]
mod clipboard_helper_tests {
    use super::*;

    fn remove_helper_test_files(result_path: &Path, rgba_path: &Path) {
        let _ = std::fs::remove_file(result_path);
        let _ = std::fs::remove_file(rgba_path);
    }

    #[test]
    fn clipboard_helper_wire_round_trips_text() {
        let (result_path, rgba_path) = clipboard_helper_temp_paths();
        let result = ClipboardCaptureReadResult {
            sequence: 42,
            source_app: "source.exe".to_string(),
            payload: ClipboardCaptureReadPayload::Text {
                normalized: "clipboard text".to_string(),
                rich_text_html: Some("<b>clipboard text</b>".to_string()),
            },
        };

        write_clipboard_helper_result(result, &result_path, &rgba_path).unwrap();
        let decoded = decode_clipboard_helper_result(&result_path, &rgba_path).unwrap();
        remove_helper_test_files(&result_path, &rgba_path);

        assert_eq!(decoded.sequence, 42);
        assert_eq!(decoded.source_app, "source.exe");
        match decoded.payload {
            ClipboardCaptureReadPayload::Text {
                normalized,
                rich_text_html,
            } => {
                assert_eq!(normalized, "clipboard text");
                assert_eq!(rich_text_html.as_deref(), Some("<b>clipboard text</b>"));
            }
            _ => panic!("expected text clipboard payload"),
        }
    }

    #[test]
    fn clipboard_helper_wire_round_trips_image_sidecar() {
        let (result_path, rgba_path) = clipboard_helper_temp_paths();
        let image_bytes = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let result = ClipboardCaptureReadResult {
            sequence: 43,
            source_app: "source.exe".to_string(),
            payload: ClipboardCaptureReadPayload::Image {
                bytes: image_bytes.clone(),
                width: 2,
                height: 1,
            },
        };

        write_clipboard_helper_result(result, &result_path, &rgba_path).unwrap();
        let decoded = decode_clipboard_helper_result(&result_path, &rgba_path).unwrap();
        remove_helper_test_files(&result_path, &rgba_path);

        match decoded.payload {
            ClipboardCaptureReadPayload::Image {
                bytes,
                width,
                height,
            } => {
                assert_eq!(bytes, image_bytes);
                assert_eq!((width, height), (2, 1));
            }
            _ => panic!("expected image clipboard payload"),
        }
    }
}
