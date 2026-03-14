use std::path::Path;

use windows_sys::Win32::{
    UI::{
        Shell::ShellExecuteW,
        WindowsAndMessaging::{
            CreateIconFromResourceEx, LR_DEFAULTCOLOR, SW_SHOWNORMAL,
        },
    },
};

use crate::app::{AppState, ClipItem, ClipKind, Icons};
use crate::win_system_ui::to_wide;

// ── 图标数据嵌入二进制（无需外部文件）──────────────────────────────────────
static ICO_CLIPBOARD: &[u8] = include_bytes!("../assets/icons/clipboard.ico");
static ICO_SEARCH:    &[u8] = include_bytes!("../assets/icons/search.ico");
static ICO_SETTING:   &[u8] = include_bytes!("../assets/icons/setting.ico");
static ICO_MIN:       &[u8] = include_bytes!("../assets/icons/min.ico");
static ICO_EXIT:      &[u8] = include_bytes!("../assets/icons/exit.ico");
static ICO_TEXT:      &[u8] = include_bytes!("../assets/icons/text.ico");
static ICO_IMAGE:     &[u8] = include_bytes!("../assets/icons/image.ico");
static ICO_FILE:      &[u8] = include_bytes!("../assets/icons/file.ico");
static ICO_FOLD:      &[u8] = include_bytes!("../assets/icons/fold.ico");
static ICO_TOP:       &[u8] = include_bytes!("../assets/icons/top.ico");
static ICO_DEL:       &[u8] = include_bytes!("../assets/icons/del.ico");

pub(crate) unsafe fn open_path_with_shell(path: &str) {
    let op = to_wide("open");
    let wp = to_wide(path);
    ShellExecuteW(std::ptr::null_mut(), op.as_ptr(), wp.as_ptr(), std::ptr::null(), std::ptr::null(), SW_SHOWNORMAL);
}

pub(crate) unsafe fn open_parent_folder(path: &str) {
    let p = Path::new(path);
    if p.is_dir() {
        open_path_with_shell(path);
    } else if let Some(parent) = p.parent() {
        if let Some(s) = parent.to_str() {
            open_path_with_shell(s);
        }
    }
}

pub(crate) fn is_directory_item(item: &ClipItem) -> bool {
    item.file_paths
        .as_ref()
        .and_then(|v| v.first())
        .map(|p| Path::new(p).is_dir())
        .unwrap_or(false)
}

pub(crate) unsafe fn item_icon_handle(state: &mut AppState, item: &ClipItem) -> isize {
    match item.kind {
        ClipKind::Text | ClipKind::Phrase => state.icons.text,
        ClipKind::Image => state.icons.image,
        ClipKind::Files => {
            if item.file_paths.as_ref().and_then(|v| v.first()).map(|p| Path::new(p).is_dir()).unwrap_or(false) {
                state.icons.folder
            } else {
                state.icons.file
            }
        }
    }
}

pub(crate) fn load_icons() -> Icons {
    unsafe {
        Icons {
            app:    load_icon_from_bytes(ICO_CLIPBOARD, 32, 32),
            search: load_icon_from_bytes(ICO_SEARCH,    16, 16),
            setting:load_icon_from_bytes(ICO_SETTING,   16, 16),
            min:    load_icon_from_bytes(ICO_MIN,        16, 16),
            close:  load_icon_from_bytes(ICO_EXIT,      16, 16),
            text:   load_icon_from_bytes(ICO_TEXT,      20, 20),
            image:  load_icon_from_bytes(ICO_IMAGE,     20, 20),
            file:   load_icon_from_bytes(ICO_FILE,      20, 20),
            folder: load_icon_from_bytes(ICO_FOLD,      20, 20),
            pin:    load_icon_from_bytes(ICO_TOP,       16, 16),
            del:    load_icon_from_bytes(ICO_DEL,       16, 16),
        }
    }
}

/// 从 ICO 文件字节流加载指定尺寸的图标句柄。
unsafe fn load_icon_from_bytes(data: &[u8], w: i32, h: i32) -> isize {
    if data.len() < 6 { return 0; }
    let count = u16::from_le_bytes([data[4], data[5]]) as usize;
    // 1st pass: exact size match
    for i in 0..count {
        let base = 6 + i * 16;
        if base + 16 > data.len() { break; }
        let icon_w = data[base] as i32;
        let icon_h = data[base + 1] as i32;
        let icon_w = if icon_w == 0 { 256 } else { icon_w };
        let icon_h = if icon_h == 0 { 256 } else { icon_h };
        if icon_w != w || icon_h != h { continue; }
        if let Some(h) = try_create_icon(data, base, w, h) { return h; }
    }
    // 2nd pass: any size, let system scale
    if count > 0 {
        if let Some(h) = try_create_icon(data, 6, w, h) { return h; }
    }
    0
}

unsafe fn try_create_icon(data: &[u8], base: usize, w: i32, h: i32) -> Option<isize> {
    if base + 16 > data.len() { return None; }
    let size   = u32::from_le_bytes([data[base+8],  data[base+9],  data[base+10], data[base+11]]) as usize;
    let offset = u32::from_le_bytes([data[base+12], data[base+13], data[base+14], data[base+15]]) as usize;
    if offset == 0 || size == 0 || offset + size > data.len() { return None; }
    let slice = &data[offset..offset + size];
    let handle = CreateIconFromResourceEx(
        slice.as_ptr(), slice.len() as u32, 1, 0x00030000, w, h, LR_DEFAULTCOLOR,
    );
    if !handle.is_null() { Some(handle as isize) } else { None }
}
