use super::prelude::*;

const CLOUD_WEBDAV_PASS_ENCRYPTED_KEY: &str = "cloud_webdav_pass_encrypted";
const SECRET_STORAGE_FIELDS: [(&str, &str); 5] = [
    ("cloud_webdav_pass", CLOUD_WEBDAV_PASS_ENCRYPTED_KEY),
    ("image_ocr_cloud_url", "image_ocr_cloud_url_encrypted"),
    ("image_ocr_cloud_token", "image_ocr_cloud_token_encrypted"),
    ("text_translate_app_id", "text_translate_app_id_encrypted"),
    ("text_translate_secret", "text_translate_secret_encrypted"),
];
static DATA_DIR_CACHE: OnceLock<PathBuf> = OnceLock::new();

pub(super) fn current_exe_path() -> Option<PathBuf> {
    std::env::current_exe().ok()
}

pub(super) fn local_app_data_dir() -> Option<PathBuf> {
    std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|path| path.join("ZSClip").join("data"))
}

pub(super) fn preferred_secondary_drive_data_dir() -> Option<PathBuf> {
    let system_drive = std::env::var("SystemDrive")
        .unwrap_or_else(|_| "C:".to_string())
        .trim_end_matches('\\')
        .to_ascii_uppercase();

    for drive in ('D'..='Z').map(|letter| format!("{letter}:")) {
        if drive == system_drive {
            continue;
        }
        let root = PathBuf::from(format!("{drive}\\"));
        if !root.exists() {
            continue;
        }
        let candidate = PathBuf::from(format!("{drive}\\ZSClip\\data"));
        if dir_is_writable(&candidate) {
            return Some(candidate);
        }
    }
    None
}

pub(super) fn dir_is_writable(dir: &Path) -> bool {
    if fs::create_dir_all(dir).is_err() {
        return false;
    }
    let probe = dir.join(".zsclip_write_test");
    match fs::OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(&probe)
    {
        Ok(_) => {
            let _ = fs::remove_file(probe);
            true
        }
        Err(_) => false,
    }
}

pub(crate) fn data_dir() -> PathBuf {
    DATA_DIR_CACHE
        .get_or_init(|| {
            if let Some(exe_dir) =
                current_exe_path().and_then(|path| path.parent().map(|dir| dir.join("data")))
            {
                if dir_is_writable(&exe_dir) {
                    return exe_dir;
                }
            }
            let local = local_app_data_dir().unwrap_or_else(|| PathBuf::from("data"));
            if local.join("clipboard.db").exists()
                || local.join("settings.json").exists()
                || local.join("images").exists()
            {
                let _ = fs::create_dir_all(&local);
                return local;
            }
            if let Some(secondary) = preferred_secondary_drive_data_dir() {
                let _ = fs::create_dir_all(&secondary);
                return secondary;
            }
            let _ = fs::create_dir_all(&local);
            local
        })
        .clone()
}

pub(crate) fn db_file() -> PathBuf {
    data_dir().join("clipboard.db")
}

pub(super) fn settings_file() -> PathBuf {
    data_dir().join("settings.json")
}

pub(super) fn load_settings() -> AppSettings {
    match fs::read_to_string(settings_file()) {
        Ok(text) => load_settings_from_text(&text),
        Err(_) => AppSettings::default(),
    }
}

pub(crate) fn save_settings(settings: &AppSettings) {
    let _ = fs::create_dir_all(data_dir());
    if let Ok(text) = serialize_settings(settings) {
        let _ = fs::write(settings_file(), text);
    }
}

pub(super) fn current_cloud_sync_paths() -> CloudSyncPaths {
    CloudSyncPaths {
        data_dir: data_dir(),
        settings_file: settings_file(),
        db_file: db_file(),
    }
}

fn load_settings_from_text(text: &str) -> AppSettings {
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(text) {
        if let Some(obj) = value.as_object_mut() {
            for (plain_key, encrypted_key) in SECRET_STORAGE_FIELDS {
                if let Some(enc) = obj.get(encrypted_key).and_then(|v| v.as_str()) {
                    if let Some(secret) =
                        crate::platform::secret_store::decrypt_secret_from_storage(enc)
                    {
                        obj.insert(plain_key.to_string(), serde_json::Value::String(secret));
                    }
                }
            }
        }
        return load_settings_from_value(value);
    }
    AppSettings::default()
}

fn load_settings_from_value(value: serde_json::Value) -> AppSettings {
    match serde_json::from_value::<AppSettings>(value.clone()) {
        Ok(settings) => settings,
        Err(_) => serde_json::from_value::<AppSettings>(sanitize_settings_value(value))
            .unwrap_or_default(),
    }
}

fn sanitize_settings_value(mut value: serde_json::Value) -> serde_json::Value {
    let Ok(default_value) = serde_json::to_value(AppSettings::default()) else {
        return value;
    };
    let Some(input) = value.as_object_mut() else {
        return value;
    };
    let Some(defaults) = default_value.as_object() else {
        return value;
    };

    input.retain(|key, incoming| {
        let Some(expected) = defaults.get(key) else {
            return true;
        };
        sanitize_setting_field(incoming, expected)
    });
    value
}

fn sanitize_setting_field(incoming: &mut serde_json::Value, expected: &serde_json::Value) -> bool {
    match expected {
        serde_json::Value::Bool(_) => incoming.is_boolean(),
        serde_json::Value::Number(_) => {
            if incoming.is_number() {
                return true;
            }
            let Some(text) = incoming.as_str().map(str::trim) else {
                return false;
            };
            let Ok(value) = text.parse::<i64>() else {
                return false;
            };
            *incoming = serde_json::Value::Number(value.into());
            true
        }
        serde_json::Value::String(_) => {
            if incoming.is_string() {
                return true;
            }
            if incoming.is_null() {
                *incoming = serde_json::Value::String(String::new());
                return true;
            }
            false
        }
        _ => true,
    }
}

fn serialize_settings(settings: &AppSettings) -> Result<String, serde_json::Error> {
    let mut value = serde_json::to_value(settings)?;
    if let Some(obj) = value.as_object_mut() {
        for (plain_key, encrypted_key) in SECRET_STORAGE_FIELDS {
            let secret = obj
                .get(plain_key)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            obj.insert(
                plain_key.to_string(),
                serde_json::Value::String(String::new()),
            );
            if secret.trim().is_empty() {
                obj.remove(encrypted_key);
            } else if let Some(enc) =
                crate::platform::secret_store::encrypt_secret_for_storage(&secret)
            {
                obj.insert(encrypted_key.to_string(), serde_json::Value::String(enc));
            }
        }
    }
    serde_json::to_string_pretty(&value)
}

pub(super) fn cloud_sync_config_from_settings(settings: &AppSettings) -> CloudSyncConfig {
    CloudSyncConfig {
        webdav_url: settings.cloud_webdav_url.clone(),
        webdav_user: settings.cloud_webdav_user.clone(),
        webdav_pass: settings.cloud_webdav_pass.clone(),
        remote_dir: settings.cloud_remote_dir.clone(),
    }
}

pub(super) fn cloud_sync_action_label(action: CloudSyncAction) -> &'static str {
    match action {
        CloudSyncAction::SyncNow => "云同步",
        CloudSyncAction::UploadConfig => "上传配置",
        CloudSyncAction::ApplyRemoteConfig => "应用云端配置",
        CloudSyncAction::RestoreBackup => "云备份恢复",
    }
}

pub(super) fn cloud_sync_running_text(auto_triggered: bool) -> &'static str {
    if auto_triggered {
        "自动云同步执行中..."
    } else {
        "云同步执行中..."
    }
}

pub(super) fn cloud_sync_should_schedule(settings: &AppSettings) -> bool {
    settings.cloud_sync_enabled && !settings.cloud_webdav_url.trim().is_empty()
}

pub(super) fn schedule_cloud_sync(state: &mut AppState, immediate: bool) {
    state.cloud_sync_next_due = if cloud_sync_should_schedule(&state.settings) {
        Some(if immediate {
            Instant::now()
        } else {
            Instant::now() + cloud_sync_interval(&state.settings.cloud_sync_interval)
        })
    } else {
        None
    };
}

pub(super) fn spawn_cloud_sync_job(
    hwnd: isize,
    ready_msg: u32,
    action: CloudSyncAction,
    auto_triggered: bool,
    settings: AppSettings,
) {
    let hwnd_value = hwnd;
    let config = cloud_sync_config_from_settings(&settings);
    let paths = current_cloud_sync_paths();
    std::thread::spawn(move || {
        let result = perform_cloud_sync(action, &config, &paths);
        if crate::platform::window::is_window_alive(hwnd_value) {
            if let Ok(mut queue) = cloud_sync_results().lock() {
                queue.push_back(CloudSyncResult {
                    hwnd: hwnd_value,
                    action,
                    auto_triggered,
                    result,
                });
            }
            crate::platform::window::post_message(hwnd_value, ready_msg, 0, 0);
        }
    });
}

pub(super) fn is_autostart_enabled() -> bool {
    crate::platform::autostart::is_enabled()
}

pub(super) fn apply_autostart(enabled: bool) -> bool {
    crate::platform::autostart::apply(enabled)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_dedupe_filter_setting_defaults_enabled() {
        let settings = load_settings_from_text(r#"{"hotkey_enabled":false}"#);
        assert!(settings.dedupe_filter_enabled);
    }

    #[test]
    fn explicit_dedupe_filter_false_is_preserved() {
        let settings = load_settings_from_text(r#"{"dedupe_filter_enabled":false}"#);
        assert!(!settings.dedupe_filter_enabled);
    }

    #[test]
    fn clipboard_capture_setting_defaults_on_and_preserves_false() {
        let legacy = load_settings_from_text(r#"{"hotkey_enabled":false}"#);
        assert!(legacy.clipboard_capture_enabled);

        let disabled = load_settings_from_text(r#"{"clipboard_capture_enabled":false}"#);
        assert!(!disabled.clipboard_capture_enabled);
    }

    #[test]
    fn invalid_setting_field_does_not_reset_max_items_to_default() {
        let settings = load_settings_from_text(r#"{"max_items":1000,"lan_tcp_port":"not-a-port"}"#);

        assert_eq!(settings.max_items, 1000);
        assert_eq!(settings.lan_tcp_port, AppSettings::default().lan_tcp_port);
    }

    #[test]
    fn string_max_items_is_migrated_to_number() {
        let settings = load_settings_from_text(r#"{"max_items":"500"}"#);

        assert_eq!(settings.max_items, 500);
    }

    #[test]
    fn saved_max_items_round_trips() {
        let mut settings = AppSettings::default();
        settings.max_items = 3000;
        let text = serialize_settings(&settings).unwrap();
        let loaded = load_settings_from_text(&text);

        assert_eq!(loaded.max_items, 3000);
    }
}
