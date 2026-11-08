use crate::platform::registry::{self, RegistryKey};

const RUN_KEY: &str = "Software\\Microsoft\\Windows\\CurrentVersion\\Run";
const RUN_APPROVAL_KEY: &str =
    "Software\\Microsoft\\Windows\\CurrentVersion\\Explorer\\StartupApproved\\Run";
const AUTOSTART_VALUE_NAME: &str = "ZSClip";
const LEGACY_AUTOSTART_VALUE_NAMES: &[&str] = &["剪贴板", "Clipboard", "筑森剪贴"];

pub(crate) fn is_enabled() -> bool {
    let Ok(executable) = std::env::current_exe() else {
        return false;
    };
    let Ok(run) = RegistryKey::open_current_user(RUN_KEY, registry::KEY_READ) else {
        return false;
    };
    let approval = RegistryKey::open_current_user(RUN_APPROVAL_KEY, registry::KEY_READ).ok();
    let executable = normalize_path(&executable.to_string_lossy());
    autostart_value_names().any(|name| {
        run.query_string(name).ok().flatten().is_some_and(|command| {
            normalize_run_target(&command).as_deref() == Some(executable.as_str())
        }) && !approval.as_ref().is_some_and(|key| entry_is_disabled(key, name))
    })
}

pub(crate) fn apply(enabled: bool) -> bool {
    let access = registry::KEY_READ | registry::KEY_SET_VALUE;
    if enabled {
        if let Ok(run) = RegistryKey::create_current_user(RUN_KEY, access) {
            apply_enabled(&run);
        }
    } else if let Ok(run) = RegistryKey::open_current_user(RUN_KEY, access) {
        for name in autostart_value_names() {
            let _ = run.delete_value(name);
        }
    }
    is_enabled()
}

fn apply_enabled(run: &RegistryKey) {
    let Ok(executable) = std::env::current_exe() else {
        return;
    };
    let command = format!("\"{}\"", executable.display());
    if run.set_string(AUTOSTART_VALUE_NAME, &command).is_err() {
        return;
    }
    // The user explicitly enabled ZSClip. Clear only its previous disable
    // marker so a stale Task Manager setting cannot block the new Run entry.
    if let Ok(approval) = RegistryKey::open_current_user(
        RUN_APPROVAL_KEY, registry::KEY_READ | registry::KEY_SET_VALUE,
    ) {
        if entry_is_disabled(&approval, AUTOSTART_VALUE_NAME) {
            let _ = approval.delete_value(AUTOSTART_VALUE_NAME);
        }
    }
    for legacy_name in LEGACY_AUTOSTART_VALUE_NAMES {
        let _ = run.delete_value(legacy_name);
    }
}

fn entry_is_disabled(key: &RegistryKey, name: &str) -> bool {
    key.query_value(name).ok().flatten().is_some_and(|value| {
        value.value_type == registry::REG_BINARY && is_disabled_approval(&value.bytes)
    })
}

fn is_disabled_approval(bytes: &[u8]) -> bool {
    // Explorer stores a state DWORD followed by a FILETIME.
    bytes.len() == 12 && matches!(bytes[..4], [3 | 7, 0, 0, 0])
}

fn normalize_run_target(value: &str) -> Option<String> {
    let trimmed = value.trim();
    let target = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split_once('"')?.0
    } else {
        trimmed.split_whitespace().next()?
    };
    let normalized = normalize_path(target);
    (!normalized.is_empty()).then_some(normalized)
}

fn normalize_path(path: &str) -> String {
    path.replace('/', "\\").to_lowercase()
}

fn autostart_value_names() -> impl Iterator<Item = &'static str> {
    std::iter::once(AUTOSTART_VALUE_NAME).chain(LEGACY_AUTOSTART_VALUE_NAMES.iter().copied())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn run_target_normalization_handles_quoted_paths_and_args() {
        assert_eq!(
            normalize_run_target(r#""D:/Program Files/ZSClip/剪贴板.exe" --silent"#),
            Some(normalize_path(r"D:\Program Files\ZSClip\剪贴板.exe"))
        );
        assert_eq!(
            normalize_run_target(r#" C:\Apps\ZSClip\zsclip.exe --silent "#).as_deref(),
            Some(r"c:\apps\zsclip\zsclip.exe")
        );
        assert_ne!(
            normalize_run_target(r#""D:\Old ZSClip\剪贴板.exe""#),
            Some(normalize_path(r"D:\Program Files\ZSClip\剪贴板.exe"))
        );
        assert_eq!(normalize_run_target(r#""C:\Apps\zsclip.exe"#), None);
    }

    #[test]
    fn detects_disabled_windows_startup_entries() {
        let mut entry = [0; 12];
        for code in [3, 7] {
            entry[0] = code;
            assert!(is_disabled_approval(&entry));
            assert!(!is_disabled_approval(&entry[..4]));
        }
        for code in [2, 6] {
            entry[0] = code;
            assert!(!is_disabled_approval(&entry));
        }
    }
}
