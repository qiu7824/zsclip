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
const CF_HDROP: u32 = 15;
const CF_UNICODETEXT: u32 = 13;
const FORMAT_CLIPBOARD_VIEWER_IGNORE: &str = "Clipboard Viewer Ignore";
const FORMAT_EXCLUDE_FROM_MONITOR_PROCESSING: &str = "ExcludeClipboardContentFromMonitorProcessing";
const FORMAT_CAN_INCLUDE_IN_HISTORY: &str = "CanIncludeInClipboardHistory";

pub(crate) struct WindowsClipboardHost;

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

pub(crate) fn has_named_format(target: &str) -> bool {
    if !open(core::ptr::null_mut()) {
        return false;
    }
    let mut format = 0u32;
    let mut found = false;
    loop {
        format = enum_next(format);
        if format == 0 {
            break;
        }
        let mut buf = [0u16; 128];
        let len = format_name(format, &mut buf);
        if len > 0 {
            let name = String::from_utf16_lossy(&buf[..len as usize]);
            if name.eq_ignore_ascii_case(target) {
                found = true;
                break;
            }
        }
    }
    close();
    found
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
    if !open(core::ptr::null_mut()) {
        return false;
    }
    let mut format = 0u32;
    let mut skip = false;
    loop {
        format = enum_next(format);
        if format == 0 {
            break;
        }
        let mut buf = [0u16; 160];
        let len = format_name(format, &mut buf);
        if len <= 0 {
            continue;
        }
        let name = String::from_utf16_lossy(&buf[..len as usize]);
        // Clipboard managers should honor the source application's explicit
        // request to keep this clipboard update out of monitoring/history.
        if named_format_always_ignores_capture(&name) {
            skip = true;
            break;
        }
        if named_format_ignores_capture_when_false(&name) && named_format_data_is_false(format) {
            skip = true;
            break;
        }
    }
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

pub(crate) fn url_format_payloads() -> Vec<String> {
    if !open(core::ptr::null_mut()) {
        return Vec::new();
    }
    let mut format = 0u32;
    let mut payloads = Vec::new();
    loop {
        format = enum_next(format);
        if format == 0 {
            break;
        }
        let mut name_buf = [0u16; 160];
        let len = format_name(format, &mut name_buf);
        if len <= 0 {
            continue;
        }
        let name = String::from_utf16_lossy(&name_buf[..len as usize]);
        let utf16 = name.eq_ignore_ascii_case("UniformResourceLocatorW");
        let is_url_format = utf16
            || name.eq_ignore_ascii_case("UniformResourceLocator")
            || name.eq_ignore_ascii_case("text/x-moz-url");
        if !is_url_format {
            continue;
        }
        let handle = data_handle(format);
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
