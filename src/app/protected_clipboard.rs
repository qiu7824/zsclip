use super::prelude::*;

pub(super) fn notify_protected_capture(hwnd: HWND) {
    static LAST_NOTICE: OnceLock<Mutex<Option<Instant>>> = OnceLock::new();
    let Ok(mut last) = LAST_NOTICE.get_or_init(|| Mutex::new(None)).lock() else { return; };
    let now = Instant::now();
    if last.is_some_and(|when| now.duration_since(when) < std::time::Duration::from_secs(5)) {
        return;
    }
    *last = Some(now);
    drop(last);
    crate::platform::tray_icon::notify(hwnd, TRAY_UID, "ZSClip", "密码与密钥已自动隐藏，可在对应分组中使用。");
}

/// The encrypted vault has committed before this function is called.
pub(super) unsafe fn protected_entry_saved(parent: HWND, value: &str) {
    let cleanup = crate::db_runtime::with_db(|conn| {
        crate::db_runtime::purge_protected_items(conn)?;
        let (busy, _, _): (i32, i32, i32) = conn.query_row(
            "PRAGMA wal_checkpoint(TRUNCATE)", [], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))?;
        if busy != 0 { return Err(rusqlite::Error::InvalidQuery); }
        Ok(())
    });
    discard_pending_protected_content();
    let mut windows = window_host_hwnds().to_vec();
    if !parent.is_null() && !windows.contains(&parent) { windows.push(parent); }
    for hwnd in windows {
        clear_main_hover_state(hwnd);
        let state = get_state_ptr(hwnd);
        if state.is_null() { continue; }
        let state = &mut *state;
        state.records.clear();
        state.phrases.clear();
        vv_popup_hide(hwnd, state);
        state.vv_popup_items.clear();
        state.clear_payload_cache();
        // A removed entry must be capturable on the very next external copy.
        // Keep the current clipboard sequence/markers protected until that copy.
        state.last_capture_signature.clear();
        state.last_capture_source_app.clear();
        state.last_capture_at = None;
        state.recent_capture_signatures.clear();
        state.recent_programmatic_clipboard_signature.clear();
        state.recent_programmatic_clipboard_until = None;
        state.ignore_clipboard_until = None;
        state.clear_selection();
        state.invalidate_all_queries();
        reload_state_from_db_persisting(state);
        refresh_lan_latest_from_db(&state.settings);
        repaint_main_window(hwnd, true);
    }
    sync_peer_windows_from_db(parent);
    let sequence = platform_clipboard::sequence_number();
    if let Some(current) = platform_clipboard::read_text_for_sequence(sequence) {
        if crate::db_runtime::text_is_protected(&current) && platform_clipboard::sequence_number() == sequence {
            let _ = platform_clipboard::set_protected_text(parent, &current);
        }
    }
    if let Err(error) = cleanup {
        platform_dialog::WindowsDialogHost::new().show_message(parent, "密码与密钥",
            &format!("密码与密钥已更新，普通记录保护已生效，但旧记录或数据库日志尚未清理完成：{error}。请关闭其他占用窗口后重试。"),
            NativeDialogLevel::Error);
    }
    let _ = value;
}
