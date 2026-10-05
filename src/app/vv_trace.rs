//! Opt-in input diagnostics. Payload text and clipboard contents are never recorded.
use super::prelude::*;
use std::io::Write;

fn trace_path() -> Option<&'static std::path::Path> {
    static PATH: OnceLock<Option<std::path::PathBuf>> = OnceLock::new();
    PATH.get_or_init(|| {
        std::env::args_os().find_map(|arg| {
            arg.to_str().and_then(|value| value.strip_prefix("--input-trace="))
                .map(std::path::PathBuf::from)
        }).or_else(|| std::env::var_os("ZSCLIP_INPUT_TRACE").map(std::path::PathBuf::from))
            .filter(|path| path.is_absolute())
    }).as_deref()
}

pub(super) fn event(stage: &str, fields: std::fmt::Arguments<'_>) {
    let Some(path) = trace_path() else {return;};
    if let Ok(mut file) = std::fs::OpenOptions::new().create(true).append(true).open(path) {
        let millis = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
            .map(|duration| duration.as_millis()).unwrap_or(0);
        let _ = writeln!(file, "{millis} pid={} {stage} {fields}", std::process::id());
    }
}

pub(super) unsafe fn snapshot(stage: &str, state: &AppState) {
    if trace_path().is_none() {return;}
    let foreground = platform_window::foreground();
    let focus = vv_current_focus(foreground);
    let session = vv_hook_state().try_lock().ok().map(|hook| {
        (hook.session.id, hook.session.phase, hook.session.target, hook.session.focus)
    });
    event(stage, format_args!("state_sid={} visible={} target={:x} focus={:x} foreground={:x} current_focus={:x} guard={:?} session={:?} clipboard_sequence={}",
        state.vv_popup_session_id, state.vv_popup_visible, state.vv_popup_target as usize,
        state.vv_popup_focus as usize, foreground as usize, focus as usize, state.vv_paste_guard, session,
        platform_clipboard::sequence_number()));
}
