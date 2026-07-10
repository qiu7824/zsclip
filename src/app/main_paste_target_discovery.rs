use super::prelude::*;
use crate::platform::window as platform_window;

fn paste_skip_class_tokens(class_names: &str) -> Vec<String> {
    class_names
        .split(|ch: char| matches!(ch, ',' | '，' | ';' | '；' | '\n' | '\r' | '\t'))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(|item| item.to_ascii_lowercase())
        .collect()
}

unsafe fn paste_window_class_name(hwnd: HWND) -> String {
    platform_window::class_name(hwnd)
}

fn paste_window_class_is_zsclip(class_name: &str) -> bool {
    class_name.eq_ignore_ascii_case(CLASS_NAME) || class_name.eq_ignore_ascii_case(QUICK_CLASS_NAME)
}

pub(super) fn paste_target_skip_classes(settings: &AppSettings) -> &str {
    if settings.paste_target_skip_enabled {
        &settings.paste_target_skip_class_names
    } else {
        ""
    }
}

pub(super) unsafe fn paste_window_class_is_skipped(hwnd: HWND, skip_class_names: &str) -> bool {
    if skip_class_names.trim().is_empty() {
        return false;
    }
    let class_name = paste_window_class_name(hwnd).trim().to_ascii_lowercase();
    if class_name.is_empty() {
        return false;
    }
    paste_skip_class_tokens(skip_class_names)
        .iter()
        .any(|item| item == &class_name)
}

pub(super) unsafe fn paste_window_is_zsclip(hwnd: HWND) -> bool {
    if hwnd.is_null() || is_app_window(hwnd) {
        return !hwnd.is_null();
    }
    let class_name = paste_window_class_name(hwnd);
    paste_window_class_is_zsclip(class_name.trim())
}

pub(super) fn append_unique_skip_class_name(class_names: &str, class_name: &str) -> String {
    let class_name = class_name.trim();
    if class_name.is_empty() {
        return class_names.trim().to_string();
    }
    let mut items: Vec<String> = class_names
        .split(|ch: char| matches!(ch, ',' | '，' | ';' | '；' | '\n' | '\r' | '\t'))
        .map(str::trim)
        .filter(|item| !item.is_empty())
        .map(str::to_string)
        .collect();
    let exists = items
        .iter()
        .any(|item| item.eq_ignore_ascii_case(class_name));
    if !exists {
        items.push(class_name.to_string());
    }
    items.join(", ")
}

pub(super) unsafe fn is_viable_paste_window(
    hwnd: HWND,
    app_hwnd: HWND,
    skip_class_names: &str,
) -> bool {
    if hwnd.is_null() || hwnd == app_hwnd || paste_window_is_zsclip(hwnd) {
        return false;
    }
    if !platform_window::is_visible(hwnd)
        || !platform_window::is_enabled_by_style(hwnd)
        || platform_window::is_minimized(hwnd)
    {
        return false;
    }
    if !platform_window::is_root_window(hwnd) {
        return false;
    }
    let ex_style = platform_window::window_ex_style(hwnd);
    (ex_style & WS_EX_TOOLWINDOW) == 0 && !paste_window_class_is_skipped(hwnd, skip_class_names)
}

unsafe fn paste_window_title_is_ignored(hwnd: HWND) -> bool {
    let title = platform_window::text(hwnd);
    matches!(
        title.trim(),
        "" | "开始" | "dummyLayeredWnd" | "Float" | "屏幕录制" | "RecBackgroundForm"
    )
}

unsafe fn first_viable_paste_window_from(
    wins: &[HWND],
    start: usize,
    app_hwnd: HWND,
    skip_class_names: &str,
) -> HWND {
    for &h in wins.iter().skip(start) {
        if !is_viable_paste_window(h, app_hwnd, skip_class_names) {
            continue;
        }
        if paste_window_title_is_ignored(h) {
            continue;
        }
        return h;
    }
    null_mut()
}

pub(super) unsafe fn find_next_paste_target(app_hwnd: HWND, skip_class_names: &str) -> HWND {
    let wins = platform_window::visible_enabled_top_level_windows();

    let fg = platform_window::foreground();
    let start = wins
        .iter()
        .position(|&h| h == fg)
        .map(|idx| idx + 1)
        .unwrap_or(0);

    first_viable_paste_window_from(&wins, start, app_hwnd, skip_class_names)
}

pub(super) unsafe fn find_next_paste_target_after(
    anchor_hwnd: HWND,
    app_hwnd: HWND,
    skip_class_names: &str,
) -> HWND {
    let wins = platform_window::visible_enabled_top_level_windows();

    let anchor = if anchor_hwnd.is_null() {
        null_mut()
    } else {
        platform_window::root_ancestor(anchor_hwnd)
    };
    let start = wins
        .iter()
        .position(|&h| h == anchor)
        .map(|idx| idx + 1)
        .unwrap_or(0);

    first_viable_paste_window_from(&wins, start, app_hwnd, skip_class_names)
}
