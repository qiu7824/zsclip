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

pub(super) fn install_data_dir() -> Option<PathBuf> {
    current_exe_path().and_then(|path| path.parent().map(|dir| dir.join("data")))
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
            if let Some(exe_dir) = install_data_dir() {
                if dir_is_writable(&exe_dir) {
                    migrate_legacy_data_dirs_to(&exe_dir);
                    return exe_dir;
                }
            }
            if let Some(secondary) = preferred_secondary_drive_data_dir() {
                let _ = fs::create_dir_all(&secondary);
                return secondary;
            }
            let local = local_app_data_dir().unwrap_or_else(|| PathBuf::from("data"));
            let _ = fs::create_dir_all(&local);
            local
        })
        .clone()
}

fn migrate_legacy_data_dirs_to(target: &Path) {
    if fs::create_dir_all(target).is_err() {
        return;
    }
    for source in legacy_data_dir_candidates(target) {
        if !legacy_data_dir_has_content(&source) {
            continue;
        }
        if same_path(&source, target) {
            continue;
        }
        if migrate_dir_contents(&source, target, target).is_ok() {
            let _ = fs::remove_dir_all(&source);
            remove_empty_legacy_parent_dirs(&source);
        }
    }
}

fn legacy_data_dir_candidates(target: &Path) -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    push_unique_path(&mut dirs, local_app_data_dir());
    for drive in ('D'..='Z').map(|letter| format!("{letter}:")) {
        push_unique_path(
            &mut dirs,
            Some(PathBuf::from(format!("{drive}\\ZSClip\\data"))),
        );
    }
    if !target.ends_with("data") {
        push_unique_path(&mut dirs, Some(PathBuf::from("data")));
    }
    dirs
}

fn push_unique_path(paths: &mut Vec<PathBuf>, candidate: Option<PathBuf>) {
    let Some(candidate) = candidate else {
        return;
    };
    if paths.iter().any(|path| same_path(path, &candidate)) {
        return;
    }
    paths.push(candidate);
}

fn legacy_data_dir_has_content(path: &Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    path.join("clipboard.db").exists()
        || path.join("settings.json").exists()
        || path.join("images").exists()
        || fs::read_dir(path)
            .map(|mut entries| entries.next().is_some())
            .unwrap_or(false)
}

fn migrate_dir_contents(source: &Path, target: &Path, migration_root: &Path) -> io::Result<()> {
    fs::create_dir_all(target)?;
    for entry in fs::read_dir(source)? {
        let entry = entry?;
        let source_path = entry.path();
        let target_path = target.join(entry.file_name());
        if same_path(&source_path, migration_root) {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            migrate_dir_contents(&source_path, &target_path, migration_root)?;
        } else if file_type.is_file() {
            migrate_file(&source_path, &target_path, migration_root)?;
        }
    }
    Ok(())
}

fn migrate_file(source: &Path, target: &Path, migration_root: &Path) -> io::Result<()> {
    if !target.exists() {
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::copy(source, target)?;
        return Ok(());
    }
    if file_contents_match(source, target) {
        return Ok(());
    }
    let backup = legacy_conflict_backup_path(source, migration_root);
    if let Some(parent) = backup.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, backup)?;
    Ok(())
}

fn legacy_conflict_backup_path(source: &Path, migration_root: &Path) -> PathBuf {
    let label = source
        .components()
        .filter_map(|part| match part {
            std::path::Component::Prefix(prefix) => {
                Some(prefix.as_os_str().to_string_lossy().replace(':', ""))
            }
            std::path::Component::Normal(name) => Some(name.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("_");
    migration_root
        .join("legacy_migration")
        .join(if label.is_empty() { "legacy" } else { &label })
}

fn file_contents_match(left: &Path, right: &Path) -> bool {
    let Ok(left_meta) = fs::metadata(left) else {
        return false;
    };
    let Ok(right_meta) = fs::metadata(right) else {
        return false;
    };
    if left_meta.len() != right_meta.len() {
        return false;
    }
    match (fs::read(left), fs::read(right)) {
        (Ok(left_bytes), Ok(right_bytes)) => left_bytes == right_bytes,
        _ => false,
    }
}

fn same_path(left: &Path, right: &Path) -> bool {
    let left_norm = left
        .canonicalize()
        .unwrap_or_else(|_| left.to_path_buf())
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase();
    let right_norm = right
        .canonicalize()
        .unwrap_or_else(|_| right.to_path_buf())
        .to_string_lossy()
        .trim_end_matches(['\\', '/'])
        .to_ascii_lowercase();
    left_norm == right_norm
}

fn remove_empty_legacy_parent_dirs(source: &Path) {
    let Some(parent) = source.parent() else {
        return;
    };
    if parent.file_name().and_then(|name| name.to_str()) == Some("ZSClip") {
        let _ = fs::remove_dir(parent);
    }
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
    let mut merged = settings.clone();
    if let Ok(text) = fs::read_to_string(settings_file()) {
        let persisted = load_settings_from_text(&text);
        merged.sticker_x = persisted.sticker_x;
        merged.sticker_y = persisted.sticker_y;
        merged.sticker_zoom_pct = persisted.sticker_zoom_pct;
    }
    write_settings(&merged);
}

fn write_settings(settings: &AppSettings) {
    let _ = fs::create_dir_all(data_dir());
    if let Ok(text) = serialize_settings(settings) {
        let _ = fs::write(settings_file(), text);
    }
}

pub(crate) fn persist_sticker_layout(x: i32, y: i32, zoom_pct: i32) {
    let mut settings = load_settings();
    settings.sticker_x = x;
    settings.sticker_y = y;
    settings.sticker_zoom_pct = zoom_pct.clamp(20, 400);
    write_settings(&settings);
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

    fn test_runtime_dir(name: &str) -> PathBuf {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("zsclip-runtime-{name}-{ts}"))
    }

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

    #[test]
    fn sticker_layout_defaults_and_round_trips() {
        let defaults = load_settings_from_text("{}");
        assert_eq!((defaults.sticker_x, defaults.sticker_y), (-1, -1));
        assert_eq!(defaults.sticker_zoom_pct, 100);

        let loaded =
            load_settings_from_text(r#"{"sticker_x":320,"sticker_y":180,"sticker_zoom_pct":170}"#);
        assert_eq!((loaded.sticker_x, loaded.sticker_y), (320, 180));
        assert_eq!(loaded.sticker_zoom_pct, 170);
    }

    #[test]
    fn legacy_data_migration_moves_missing_files_to_install_data_dir() {
        let root = test_runtime_dir("move");
        let source = root.join("old").join("ZSClip").join("data");
        let target = root.join("install").join("data");
        fs::create_dir_all(source.join("images")).unwrap();
        fs::write(source.join("settings.json"), "{}").unwrap();
        fs::write(source.join("images").join("clip.png"), b"png").unwrap();

        migrate_dir_contents(&source, &target, &target).unwrap();

        assert_eq!(
            fs::read_to_string(target.join("settings.json")).unwrap(),
            "{}"
        );
        assert_eq!(
            fs::read(target.join("images").join("clip.png")).unwrap(),
            b"png"
        );
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_data_migration_preserves_conflicting_files_under_install_data_dir() {
        let root = test_runtime_dir("conflict");
        let source = root.join("old").join("ZSClip").join("data");
        let target = root.join("install").join("data");
        fs::create_dir_all(&source).unwrap();
        fs::create_dir_all(&target).unwrap();
        fs::write(source.join("clipboard.db"), b"old-db").unwrap();
        fs::write(target.join("clipboard.db"), b"new-db").unwrap();

        migrate_dir_contents(&source, &target, &target).unwrap();

        assert_eq!(fs::read(target.join("clipboard.db")).unwrap(), b"new-db");
        let backups = fs::read_dir(target.join("legacy_migration"))
            .unwrap()
            .filter_map(Result::ok)
            .collect::<Vec<_>>();
        assert_eq!(backups.len(), 1);
        assert_eq!(fs::read(backups[0].path()).unwrap(), b"old-db");
        let _ = fs::remove_dir_all(root);
    }
}
