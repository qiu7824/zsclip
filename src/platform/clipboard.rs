use arboard::{Clipboard, ImageData};
use std::{borrow::Cow, mem::size_of};
use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    System::DataExchange::{
        CloseClipboard, EmptyClipboard, EnumClipboardFormats, GetClipboardData,
        GetClipboardFormatNameW, GetClipboardOwner, GetClipboardSequenceNumber,
        IsClipboardFormatAvailable, OpenClipboard, RegisterClipboardFormatW, SetClipboardData,
    },
    UI::Shell::DragQueryFileW,
};

use crate::app_core::ClipboardHost;

use super::memory;

const GMEM_MOVEABLE: u32 = 0x0002;
const GMEM_ZEROINIT: u32 = 0x0040;
const MAX_HTML_FORMAT_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const CF_TEXT: u32 = 1;
pub(crate) const CF_BITMAP: u32 = 2;
pub(crate) const CF_METAFILEPICT: u32 = 3;
pub(crate) const CF_OEMTEXT: u32 = 7;
pub(crate) const CF_DIB: u32 = 8;
pub(crate) const CF_UNICODETEXT: u32 = 13;
pub(crate) const CF_ENHMETAFILE: u32 = 14;
pub(crate) const CF_HDROP: u32 = 15;
pub(crate) const CF_LOCALE: u32 = 16;
pub(crate) const CF_DIBV5: u32 = 17;
const FORMAT_CLIPBOARD_VIEWER_IGNORE: &str = "Clipboard Viewer Ignore";
const FORMAT_EXCLUDE_FROM_MONITOR_PROCESSING: &str = "ExcludeClipboardContentFromMonitorProcessing";
const FORMAT_CAN_INCLUDE_IN_HISTORY: &str = "CanIncludeInClipboardHistory";
const FORMAT_PNG: &str = "PNG";
const FORMAT_IMAGE_PNG: &str = "image/png";
const FORMAT_HTML_FORMAT: &str = "HTML Format";
const FORMAT_UNIFORM_RESOURCE_LOCATOR: &str = "UniformResourceLocator";
const FORMAT_UNIFORM_RESOURCE_LOCATOR_W: &str = "UniformResourceLocatorW";
const FORMAT_TEXT_X_MOZ_URL: &str = "text/x-moz-url";

pub(crate) struct WindowsClipboardHost;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ClipboardNamedFormat {
    pub(crate) format: u32,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(crate) struct ClipboardFormatSnapshot {
    pub(crate) sequence: u32,
    pub(crate) open_failed: bool,
    pub(crate) format_count: usize,
    pub(crate) format_ids: Vec<u32>,
    pub(crate) named_formats: Vec<ClipboardNamedFormat>,
    pub(crate) has_text: bool,
    pub(crate) has_files: bool,
    pub(crate) has_image: bool,
    pub(crate) has_html: bool,
    pub(crate) has_url_payload: bool,
    pub(crate) has_ignore_capture_format: bool,
    pub(crate) has_history_exclusion_format: bool,
    pub(crate) has_any_standard_format: bool,
    pub(crate) has_only_custom_formats: bool,
}

impl ClipboardFormatSnapshot {
    pub(crate) fn has_named_format(&self, target: &str) -> bool {
        self.named_format_id(target).is_some()
    }

    fn named_format_id(&self, target: &str) -> Option<u32> {
        self.named_formats
            .iter()
            .find(|format| format.name.eq_ignore_ascii_case(target))
            .map(|format| format.format)
    }
}

impl ClipboardHost for WindowsClipboardHost {
    fn read_text() -> Option<String> {
        let mut clipboard = Clipboard::new().ok()?;
        clipboard.get_text().ok()
    }

    fn write_text(text: &str) -> bool {
        let Ok(mut clipboard) = Clipboard::new() else {
            return false;
        };
        clipboard.set_text(text.to_string()).is_ok()
    }

    fn read_image_rgba() -> Option<(Vec<u8>, usize, usize)> {
        let mut clipboard = Clipboard::new().ok()?;
        let image = clipboard.get_image().ok()?;
        Some((image.bytes.into_owned(), image.width, image.height))
    }

    fn write_image_rgba(bytes: &[u8], width: usize, height: usize) -> bool {
        let Ok(mut clipboard) = Clipboard::new() else {
            return false;
        };
        clipboard
            .set_image(ImageData {
                width,
                height,
                bytes: Cow::Owned(bytes.to_vec()),
            })
            .is_ok()
    }

    fn read_file_paths() -> Option<Vec<String>> {
        crate::platform::clipboard::file_paths()
    }

    fn write_file_paths(paths: &[String]) -> bool {
        crate::platform::clipboard::set_file_paths(paths)
    }

    fn sequence_number() -> u32 {
        crate::platform::clipboard::sequence_number()
    }

    fn write_text_ignored_by_monitors(text: &str) -> bool {
        crate::platform::clipboard::set_text_ignored_by_monitors(text)
    }

    fn should_ignore_capture_by_named_format() -> bool {
        crate::platform::clipboard::should_ignore_capture_by_named_format()
    }
}

#[repr(C)]
struct DropFiles {
    p_files: u32,
    pt: POINT,
    f_nc: i32,
    f_wide: i32,
}

pub(crate) fn open(owner: HWND) -> bool {
    unsafe { OpenClipboard(owner) != 0 }
}

pub(crate) fn close() {
    unsafe {
        CloseClipboard();
    }
}

pub(crate) fn empty() -> bool {
    unsafe { EmptyClipboard() != 0 }
}

pub(crate) fn enum_next(format: u32) -> u32 {
    unsafe { EnumClipboardFormats(format) }
}

pub(crate) fn format_name(format: u32, buf: &mut [u16]) -> i32 {
    if buf.is_empty() {
        return 0;
    }
    unsafe { GetClipboardFormatNameW(format, buf.as_mut_ptr(), buf.len() as i32) }
}

pub(crate) fn data_handle(format: u32) -> *mut core::ffi::c_void {
    unsafe { GetClipboardData(format) }
}

pub(crate) fn set_data(format: u32, handle: *mut core::ffi::c_void) -> bool {
    unsafe { !SetClipboardData(format, handle).is_null() }
}

fn register_format(name: &str) -> u32 {
    let wide: Vec<u16> = name.encode_utf16().chain([0]).collect();
    unsafe { RegisterClipboardFormatW(wide.as_ptr()) }
}

pub(crate) fn is_format_available(format: u32) -> bool {
    unsafe { IsClipboardFormatAvailable(format) != 0 }
}

pub(crate) fn owner() -> HWND {
    unsafe { GetClipboardOwner() }
}

pub(crate) fn sequence_number() -> u32 {
    unsafe { GetClipboardSequenceNumber() }
}

fn format_name_string(format: u32) -> Option<String> {
    let mut buf = [0u16; 160];
    let len = format_name(format, &mut buf);
    if len <= 0 {
        return None;
    }
    Some(String::from_utf16_lossy(&buf[..len as usize]))
}

fn named_format_is_image(name: &str) -> bool {
    name.eq_ignore_ascii_case(FORMAT_PNG) || name.eq_ignore_ascii_case(FORMAT_IMAGE_PNG)
}

fn named_format_is_html(name: &str) -> bool {
    name.eq_ignore_ascii_case(FORMAT_HTML_FORMAT)
}

fn named_format_is_url_payload(name: &str) -> bool {
    name.eq_ignore_ascii_case(FORMAT_UNIFORM_RESOURCE_LOCATOR_W)
        || name.eq_ignore_ascii_case(FORMAT_UNIFORM_RESOURCE_LOCATOR)
        || name.eq_ignore_ascii_case(FORMAT_TEXT_X_MOZ_URL)
}

fn standard_format_id(format: u32) -> bool {
    matches!(
        format,
        CF_TEXT
            | CF_BITMAP
            | CF_METAFILEPICT
            | CF_OEMTEXT
            | CF_DIB
            | CF_UNICODETEXT
            | CF_ENHMETAFILE
            | CF_HDROP
            | CF_LOCALE
            | CF_DIBV5
    )
}

pub(crate) fn snapshot_formats() -> ClipboardFormatSnapshot {
    let sequence = sequence_number();
    if !open(core::ptr::null_mut()) {
        return ClipboardFormatSnapshot {
            sequence,
            open_failed: true,
            ..Default::default()
        };
    }

    let mut format = 0u32;
    let mut format_ids = Vec::new();
    let mut named_formats = Vec::new();
    loop {
        format = enum_next(format);
        if format == 0 {
            break;
        }
        format_ids.push(format);
        if let Some(name) = format_name_string(format) {
            named_formats.push(ClipboardNamedFormat { format, name });
        }
    }

    let has_text = is_format_available(CF_UNICODETEXT)
        || is_format_available(CF_TEXT)
        || is_format_available(CF_OEMTEXT);
    let has_files = is_format_available(CF_HDROP);
    let has_image = is_format_available(CF_BITMAP)
        || is_format_available(CF_DIB)
        || is_format_available(CF_DIBV5)
        || named_formats
            .iter()
            .any(|format| named_format_is_image(&format.name));
    let has_html = named_formats
        .iter()
        .any(|format| named_format_is_html(&format.name));
    let has_url_payload = named_formats
        .iter()
        .any(|format| named_format_is_url_payload(&format.name));
    let has_ignore_capture_format = named_formats
        .iter()
        .any(|format| named_format_always_ignores_capture(&format.name));
    let has_history_exclusion_format = named_formats
        .iter()
        .any(|format| named_format_ignores_capture_when_false(&format.name));
    let has_any_standard_format = format_ids.iter().any(|format| standard_format_id(*format))
        || has_text
        || has_files
        || has_image
        || has_html
        || has_url_payload;
    let format_count = format_ids.len();
    let has_only_custom_formats = format_count > 0 && !has_any_standard_format;
    let sequence = sequence_number();
    close();

    ClipboardFormatSnapshot {
        sequence,
        open_failed: false,
        format_count,
        format_ids,
        named_formats,
        has_text,
        has_files,
        has_image,
        has_html,
        has_url_payload,
        has_ignore_capture_format,
        has_history_exclusion_format,
        has_any_standard_format,
        has_only_custom_formats,
    }
}

fn hdrop_paths(handle: *mut core::ffi::c_void) -> Vec<String> {
    if handle.is_null() {
        return Vec::new();
    }
    let count = unsafe { DragQueryFileW(handle as _, 0xFFFF_FFFF, core::ptr::null_mut(), 0) };
    let mut paths = Vec::new();
    for i in 0..count {
        let len = unsafe { DragQueryFileW(handle as _, i, core::ptr::null_mut(), 0) };
        if len == 0 {
            continue;
        }
        let mut buf = vec![0u16; len as usize + 1];
        let out = unsafe { DragQueryFileW(handle as _, i, buf.as_mut_ptr(), len + 1) };
        if out > 0 {
            paths.push(String::from_utf16_lossy(&buf[..out as usize]));
        }
    }
    paths
}

pub(crate) fn file_paths() -> Option<Vec<String>> {
    if !open(core::ptr::null_mut()) {
        return None;
    }
    let handle = data_handle(CF_HDROP);
    if handle.is_null() {
        close();
        return None;
    }
    let paths = hdrop_paths(handle);
    close();
    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}

pub(crate) fn set_file_paths(paths: &[String]) -> bool {
    let cleaned = clean_file_paths(paths);
    if cleaned.is_empty() {
        return false;
    }

    let wide_paths: Vec<Vec<u16>> = cleaned
        .iter()
        .map(|path| {
            let mut wide: Vec<u16> = path.encode_utf16().collect();
            wide.push(0);
            wide
        })
        .collect();
    let chars_len: usize = wide_paths.iter().map(|w| w.len()).sum::<usize>() + 1;
    let bytes_len = size_of::<DropFiles>() + chars_len * size_of::<u16>();
    let mem = memory::global_alloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes_len);
    if mem.is_null() {
        return false;
    }
    let locked = memory::global_lock(mem);
    if locked.is_null() {
        memory::global_free(mem);
        return false;
    }

    unsafe {
        let header = locked as *mut DropFiles;
        (*header).p_files = size_of::<DropFiles>() as u32;
        (*header).pt.x = 0;
        (*header).pt.y = 0;
        (*header).f_nc = 0;
        (*header).f_wide = 1;
        let mut cursor = (locked as *mut u8).add(size_of::<DropFiles>()) as *mut u16;
        for path in wide_paths.iter() {
            core::ptr::copy_nonoverlapping(path.as_ptr(), cursor, path.len());
            cursor = cursor.add(path.len());
        }
        *cursor = 0;
    }
    memory::global_unlock(mem);

    if !open(core::ptr::null_mut()) {
        memory::global_free(mem);
        return false;
    }
    let ok = if !empty() {
        false
    } else {
        set_data(CF_HDROP, mem)
    };
    close();
    if !ok {
        memory::global_free(mem);
    }
    ok
}

pub(crate) fn set_text_ignored_by_monitors(text: &str) -> bool {
    let wide: Vec<u16> = text.encode_utf16().chain([0]).collect();
    let bytes_len = wide.len() * size_of::<u16>();
    let mem = memory::global_alloc(GMEM_MOVEABLE | GMEM_ZEROINIT, bytes_len);
    if mem.is_null() {
        return false;
    }
    let locked = memory::global_lock(mem);
    if locked.is_null() {
        memory::global_free(mem);
        return false;
    }
    unsafe {
        core::ptr::copy_nonoverlapping(wide.as_ptr(), locked as *mut u16, wide.len());
    }
    memory::global_unlock(mem);

    if !open(core::ptr::null_mut()) {
        memory::global_free(mem);
        return false;
    }
    let ok = if !empty() {
        false
    } else {
        set_data(CF_UNICODETEXT, mem)
    };
    if ok {
        set_monitor_ignore_formats();
    }
    close();
    if !ok {
        memory::global_free(mem);
    }
    ok
}

fn set_registered_u32_format(name: &str, value: u32) -> bool {
    let format = register_format(name);
    if format == 0 {
        return false;
    }
    let mem = memory::global_alloc(GMEM_MOVEABLE | GMEM_ZEROINIT, size_of::<u32>());
    if mem.is_null() {
        return false;
    }
    let locked = memory::global_lock(mem);
    if locked.is_null() {
        memory::global_free(mem);
        return false;
    }
    unsafe {
        *(locked as *mut u32) = value;
    }
    memory::global_unlock(mem);
    if set_data(format, mem) {
        true
    } else {
        memory::global_free(mem);
        false
    }
}

fn set_monitor_ignore_formats() {
    let _ = set_registered_u32_format(FORMAT_CLIPBOARD_VIEWER_IGNORE, 1);
    let _ = set_registered_u32_format(FORMAT_EXCLUDE_FROM_MONITOR_PROCESSING, 1);
    let _ = set_registered_u32_format(FORMAT_CAN_INCLUDE_IN_HISTORY, 0);
}

fn clean_file_paths(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .map(|path| path.trim())
        .filter(|path| !path.is_empty())
        .map(str::to_string)
        .collect()
}

fn named_format_data_is_false(format: u32) -> bool {
    let handle = data_handle(format);
    if handle.is_null() {
        return false;
    }
    let size = memory::global_size(handle);
    if size == 0 {
        return false;
    }
    let locked = memory::global_lock(handle);
    if locked.is_null() {
        return false;
    }
    let bytes = unsafe { std::slice::from_raw_parts(locked as *const u8, size.min(64)) };
    let is_false = named_format_bytes_are_false(bytes);
    memory::global_unlock(handle);
    is_false
}

fn format_data_bytes(format: u32, max_size: usize) -> Option<Vec<u8>> {
    let handle = data_handle(format);
    if handle.is_null() {
        return None;
    }
    let size = memory::global_size(handle).min(max_size);
    if size == 0 {
        return None;
    }
    let locked = memory::global_lock(handle);
    if locked.is_null() {
        return None;
    }
    let bytes = unsafe { std::slice::from_raw_parts(locked as *const u8, size) }.to_vec();
    memory::global_unlock(handle);
    Some(bytes)
}

pub(crate) fn named_format_bytes_are_false(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    if bytes.len() >= 4 && u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]) == 0 {
        return true;
    }
    if bytes.len() >= 2 && u16::from_le_bytes([bytes[0], bytes[1]]) == 0 {
        return true;
    }
    let ascii = bytes
        .iter()
        .copied()
        .take_while(|byte| *byte != 0)
        .map(char::from)
        .collect::<String>();
    let value = ascii.trim().to_ascii_lowercase();
    value == "0" || value == "false" || value == "no"
}

pub(crate) fn named_format_always_ignores_capture(name: &str) -> bool {
    name.eq_ignore_ascii_case(FORMAT_CLIPBOARD_VIEWER_IGNORE)
        || name.eq_ignore_ascii_case(FORMAT_EXCLUDE_FROM_MONITOR_PROCESSING)
}

pub(crate) fn named_format_ignores_capture_when_false(name: &str) -> bool {
    name.eq_ignore_ascii_case(FORMAT_CAN_INCLUDE_IN_HISTORY)
}

pub(crate) fn should_ignore_capture_by_named_format() -> bool {
    let snapshot = snapshot_formats();
    should_ignore_capture_by_snapshot(&snapshot)
}

pub(crate) fn should_ignore_capture_by_snapshot(snapshot: &ClipboardFormatSnapshot) -> bool {
    if snapshot.has_ignore_capture_format {
        return true;
    }
    if snapshot.has_only_custom_formats || !snapshot.has_history_exclusion_format {
        return false;
    }
    let Some(format) = snapshot.named_format_id(FORMAT_CAN_INCLUDE_IN_HISTORY) else {
        return false;
    };
    if !open(core::ptr::null_mut()) {
        return false;
    }
    let sequence_matches = snapshot.sequence == 0 || sequence_number() == snapshot.sequence;
    let skip = sequence_matches && named_format_data_is_false(format);
    close();
    skip
}

fn decode_text_bytes(bytes: &[u8], utf16: bool) -> Option<String> {
    if bytes.is_empty() {
        return None;
    }
    if utf16 {
        let mut units = Vec::with_capacity(bytes.len() / 2);
        for chunk in bytes.chunks_exact(2) {
            let unit = u16::from_le_bytes([chunk[0], chunk[1]]);
            if unit == 0 {
                break;
            }
            units.push(unit);
        }
        return String::from_utf16(&units).ok();
    }
    let end = bytes
        .iter()
        .position(|byte| *byte == 0)
        .unwrap_or(bytes.len());
    Some(String::from_utf8_lossy(&bytes[..end]).to_string())
}

fn cf_html_header_offset(raw: &str, key: &str) -> Option<usize> {
    raw.lines().find_map(|line| {
        let value = line.strip_prefix(key)?.trim();
        value.parse::<usize>().ok()
    })
}

pub(crate) fn cf_html_extract_fragment(raw: &str) -> Option<String> {
    let bytes = raw.as_bytes();
    if let (Some(start), Some(end)) = (
        cf_html_header_offset(raw, "StartFragment:"),
        cf_html_header_offset(raw, "EndFragment:"),
    ) {
        if start < end && end <= bytes.len() {
            let fragment = String::from_utf8_lossy(&bytes[start..end])
                .trim()
                .to_string();
            if !fragment.is_empty() {
                return Some(fragment);
            }
        }
    }

    let lower = raw.to_ascii_lowercase();
    let start_marker = "<!--startfragment-->";
    let end_marker = "<!--endfragment-->";
    if let (Some(start), Some(end)) = (lower.find(start_marker), lower.find(end_marker)) {
        let start = start + start_marker.len();
        if start < end {
            let fragment = raw[start..end].trim().to_string();
            if !fragment.is_empty() {
                return Some(fragment);
            }
        }
    }

    let trimmed = raw.trim();
    (!trimmed.is_empty()).then(|| trimmed.to_string())
}

pub(crate) fn html_format_payload_from_snapshot(
    snapshot: &ClipboardFormatSnapshot,
) -> Option<String> {
    if !snapshot.has_html {
        return None;
    }
    let format = snapshot.named_format_id(FORMAT_HTML_FORMAT)?;
    if !open(core::ptr::null_mut()) {
        return None;
    }
    if snapshot.sequence != 0 && sequence_number() != snapshot.sequence {
        close();
        return None;
    }
    let text = format_data_bytes(format, MAX_HTML_FORMAT_BYTES)
        .and_then(|bytes| decode_text_bytes(&bytes, false))
        .and_then(|raw| cf_html_extract_fragment(&raw));
    close();
    text
}

pub(crate) fn url_format_payloads_from_snapshot(snapshot: &ClipboardFormatSnapshot) -> Vec<String> {
    if !snapshot.has_url_payload {
        return Vec::new();
    }
    if !open(core::ptr::null_mut()) {
        return Vec::new();
    }
    if snapshot.sequence != 0 && sequence_number() != snapshot.sequence {
        close();
        return Vec::new();
    }
    let mut payloads = Vec::new();
    for named_format in snapshot.named_formats.iter() {
        let utf16 = named_format
            .name
            .eq_ignore_ascii_case(FORMAT_UNIFORM_RESOURCE_LOCATOR_W);
        if !utf16 && !named_format_is_url_payload(&named_format.name) {
            continue;
        }
        let handle = data_handle(named_format.format);
        if handle.is_null() {
            continue;
        }
        let size = memory::global_size(handle).min(64 * 1024);
        let locked = memory::global_lock(handle);
        if locked.is_null() {
            continue;
        }
        if size == 0 {
            memory::global_unlock(handle);
            continue;
        }
        let bytes = unsafe { std::slice::from_raw_parts(locked as *const u8, size) };
        if let Some(text) = decode_text_bytes(bytes, utf16) {
            if !text.trim().is_empty() {
                payloads.push(text);
            }
        }
        memory::global_unlock(handle);
    }
    close();
    payloads
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignore_formats_follow_capture_semantics() {
        assert!(named_format_always_ignores_capture(
            "Clipboard Viewer Ignore"
        ));
        assert!(named_format_always_ignores_capture(
            "ExcludeClipboardContentFromMonitorProcessing"
        ));
        assert!(named_format_ignores_capture_when_false(
            "CanIncludeInClipboardHistory"
        ));
        assert!(!named_format_ignores_capture_when_false(
            "CanUploadToCloudClipboard"
        ));
        assert!(!named_format_always_ignores_capture("HTML Format"));
        assert!(!named_format_ignores_capture_when_false("HTML Format"));
    }

    #[test]
    fn history_boolean_formats_parse_false_values() {
        assert!(!named_format_bytes_are_false(&[]));
        assert!(named_format_bytes_are_false(&0u32.to_le_bytes()));
        assert!(named_format_bytes_are_false(&0u16.to_le_bytes()));
        assert!(named_format_bytes_are_false(b"0\0"));
        assert!(named_format_bytes_are_false(b"false\0"));
        assert!(named_format_bytes_are_false(b" no "));

        assert!(!named_format_bytes_are_false(&1u32.to_le_bytes()));
        assert!(!named_format_bytes_are_false(b"true\0"));
        assert!(!named_format_bytes_are_false(b"yes"));
    }

    #[test]
    fn url_payload_text_decoding_handles_ascii_and_utf16() {
        assert_eq!(
            decode_text_bytes(b"https://example.test/path\0ignored", false).as_deref(),
            Some("https://example.test/path")
        );

        let utf16: Vec<u8> = "https://example.test/中文"
            .encode_utf16()
            .chain([0])
            .flat_map(u16::to_le_bytes)
            .collect();
        assert_eq!(
            decode_text_bytes(&utf16, true).as_deref(),
            Some("https://example.test/中文")
        );
        assert_eq!(decode_text_bytes(&[], false), None);
    }

    #[test]
    fn file_path_clipboard_payload_filters_empty_paths() {
        assert_eq!(
            clean_file_paths(&[
                " C:\\Temp\\a.txt ".to_string(),
                "".to_string(),
                "   ".to_string(),
                "D:\\Docs\\b.txt".to_string(),
            ]),
            vec!["C:\\Temp\\a.txt".to_string(), "D:\\Docs\\b.txt".to_string()]
        );
    }
}
