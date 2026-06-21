use crate::platform::registry::{self, RegistryKey};

const AUTOSTART_VALUE_NAME: &str = "ZSClip";
const LEGACY_AUTOSTART_VALUE_NAMES: &[&str] = &["剪贴板", "Clipboard", "筑森剪贴"];

pub(crate) fn is_enabled() -> bool {
    let Ok(key) = RegistryKey::open_current_user(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        registry::KEY_READ,
    ) else {
        return false;
    };
    registered_autostart_value_name(&key).is_some()
}

pub(crate) fn apply(enabled: bool) -> bool {
    let Ok(key) = RegistryKey::open_current_user(
        "Software\\Microsoft\\Windows\\CurrentVersion\\Run",
        registry::KEY_READ | registry::KEY_SET_VALUE,
    ) else {
        return false;
    };
    let changed = if enabled {
        apply_enabled(&key)
    } else {
        apply_disabled(&key)
    };
    if enabled {
        changed
    } else {
        changed && !is_enabled()
    }
}

fn apply_enabled(key: &RegistryKey) -> bool {
    let Some(cmdline) = autostart_command_for_current_exe() else {
        return false;
    };
    if key.set_string(AUTOSTART_VALUE_NAME, &cmdline).is_err() {
        return false;
    }
    for legacy_name in LEGACY_AUTOSTART_VALUE_NAMES {
        let _ = key.delete_value(legacy_name);
    }
    read_run_value(key, AUTOSTART_VALUE_NAME)
        .map(|value| normalize_run_target(&value) == normalize_run_target(&cmdline))
        .unwrap_or(false)
}

fn apply_disabled(key: &RegistryKey) -> bool {
    let mut changed = false;
    for value_name in autostart_value_names() {
        if key.delete_value(value_name).is_ok() {
            changed = true;
        }
    }
    if !is_enabled() {
        changed = true;
    }
    changed
}

fn autostart_command_for_current_exe() -> Option<String> {
    std::env::current_exe()
        .ok()
        .map(|path| format!("\"{}\"", path.to_string_lossy()))
}

fn normalize_run_target(value: &str) -> String {
    let trimmed = value.trim();
    let target = if let Some(rest) = trimmed.strip_prefix('"') {
        rest.split('"').next().unwrap_or(rest)
    } else {
        trimmed.split_whitespace().next().unwrap_or("")
    };
    target
        .trim_matches('"')
        .replace('/', "\\")
        .trim()
        .to_ascii_lowercase()
}

fn run_target_matches_current_exe(value: &str) -> bool {
    if let Ok(exe) = std::env::current_exe() {
        normalize_run_target(value) == normalize_run_target(&exe.to_string_lossy())
    } else {
        !value.trim().is_empty()
    }
}

fn read_run_value(key: &RegistryKey, value_name: &str) -> Option<String> {
    key.query_string(value_name).ok().flatten()
}

fn autostart_value_names() -> impl Iterator<Item = &'static str> {
    std::iter::once(AUTOSTART_VALUE_NAME).chain(LEGACY_AUTOSTART_VALUE_NAMES.iter().copied())
}

fn registered_autostart_value_name_by_path(key: &RegistryKey) -> Option<&'static str> {
    autostart_value_names().find(|name| {
        read_run_value(key, name)
            .map(|value| run_target_matches_current_exe(&value))
            .unwrap_or(false)
    })
}

fn registered_autostart_value_name(key: &RegistryKey) -> Option<&'static str> {
    registered_autostart_value_name_by_path(key).or_else(|| {
        autostart_value_names().find(|name| {
            read_run_value(key, name)
                .map(|value| !normalize_run_target(&value).is_empty())
                .unwrap_or(false)
        })
    })
}

#[cfg(test)]
mod tests {
    use super::normalize_run_target;

    #[test]
    fn run_target_normalization_handles_quoted_paths_and_args() {
        assert_eq!(
            normalize_run_target(r#""C:/Apps/ZSClip/zsclip.exe" --silent"#),
            r"c:\apps\zsclip\zsclip.exe"
        );
        assert_eq!(
            normalize_run_target(r#" C:\Apps\ZSClip\zsclip.exe --silent "#),
            r"c:\apps\zsclip\zsclip.exe"
        );
    }
}
