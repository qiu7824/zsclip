use super::*;
use std::ptr::{null, null_mut};

const CLOUD_WEBDAV_PASS_ENCRYPTED_KEY: &str = "cloud_webdav_pass_encrypted";
const SECRET_STORAGE_FIELDS: [(&str, &str); 5] = [
    ("cloud_webdav_pass", CLOUD_WEBDAV_PASS_ENCRYPTED_KEY),
    ("image_ocr_cloud_url", "image_ocr_cloud_url_encrypted"),
    ("image_ocr_cloud_token", "image_ocr_cloud_token_encrypted"),
    ("text_translate_app_id", "text_translate_app_id_encrypted"),
    ("text_translate_secret", "text_translate_secret_encrypted"),
];
const CRYPTPROTECT_UI_FORBIDDEN: u32 = 0x1;

#[repr(C)]
struct DataBlob {
    cb_data: u32,
    pb_data: *mut u8,
}

#[link(name = "crypt32")]
unsafe extern "system" {
    fn CryptProtectData(
        p_data_in: *const DataBlob,
        sz_data_descr: *const u16,
        p_optional_entropy: *const DataBlob,
        pv_reserved: *mut core::ffi::c_void,
        p_prompt_struct: *mut core::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;
    fn CryptUnprotectData(
        p_data_in: *const DataBlob,
        ppsz_data_descr: *mut *mut u16,
        p_optional_entropy: *const DataBlob,
        pv_reserved: *mut core::ffi::c_void,
        p_prompt_struct: *mut core::ffi::c_void,
        dw_flags: u32,
        p_data_out: *mut DataBlob,
    ) -> i32;
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn LocalFree(hmem: *mut core::ffi::c_void) -> *mut core::ffi::c_void;
}

static DATA_DIR_CACHE: OnceLock<PathBuf> = OnceLock::new();
const AUTOSTART_VALUE_NAME: &str = "ZSClip";
const LEGACY_AUTOSTART_VALUE_NAMES: &[&str] = &["剪贴板", "Clipboard", "筑森剪贴"];

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

pub(super) fn data_dir() -> PathBuf {
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
                    if let Some(secret) = decrypt_secret_from_storage(enc) {
                        obj.insert(plain_key.to_string(), serde_json::Value::String(secret));
                    }
                }
            }
        }
        return serde_json::from_value::<AppSettings>(value).unwrap_or_default();
    }
    AppSettings::default()
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
            } else if let Some(enc) = encrypt_secret_for_storage(&secret) {
                obj.insert(encrypted_key.to_string(), serde_json::Value::String(enc));
            }
        }
    }
    serde_json::to_string_pretty(&value)
}

fn encrypt_secret_for_storage(secret: &str) -> Option<String> {
    if secret.is_empty() {
        return Some(String::new());
    }
    unsafe { protect_bytes(secret.as_bytes()).map(|bytes| hex_encode(&bytes)) }
}

fn decrypt_secret_from_storage(encoded: &str) -> Option<String> {
    if encoded.trim().is_empty() {
        return Some(String::new());
    }
    let raw = hex_decode(encoded)?;
    unsafe { unprotect_bytes(&raw).and_then(|bytes| String::from_utf8(bytes).ok()) }
}

unsafe fn protect_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    let input = DataBlob {
        cb_data: bytes.len() as u32,
        pb_data: bytes.as_ptr() as *mut u8,
    };
    let mut output = DataBlob {
        cb_data: 0,
        pb_data: null_mut(),
    };
    let ok = CryptProtectData(
        &input,
        null(),
        null(),
        null_mut(),
        null_mut(),
        CRYPTPROTECT_UI_FORBIDDEN,
        &mut output,
    );
    if ok == 0 || output.pb_data.is_null() {
        return None;
    }
    let result = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec();
    let _ = LocalFree(output.pb_data as *mut core::ffi::c_void);
    Some(result)
}

unsafe fn unprotect_bytes(bytes: &[u8]) -> Option<Vec<u8>> {
    let input = DataBlob {
        cb_data: bytes.len() as u32,
        pb_data: bytes.as_ptr() as *mut u8,
    };
    let mut output = DataBlob {
        cb_data: 0,
        pb_data: null_mut(),
    };
    let ok = CryptUnprotectData(
        &input,
        null_mut(),
        null(),
        null_mut(),
        null_mut(),
        CRYPTPROTECT_UI_FORBIDDEN,
        &mut output,
    );
    if ok == 0 || output.pb_data.is_null() {
        return None;
    }
    let result = std::slice::from_raw_parts(output.pb_data, output.cb_data as usize).to_vec();
    let _ = LocalFree(output.pb_data as *mut core::ffi::c_void);
    Some(result)
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(HEX[(b >> 4) as usize] as char);
        out.push(HEX[(b & 0x0f) as usize] as char);
    }
    out
}

fn hex_decode(text: &str) -> Option<Vec<u8>> {
    fn nibble(ch: u8) -> Option<u8> {
        match ch {
            b'0'..=b'9' => Some(ch - b'0'),
            b'a'..=b'f' => Some(ch - b'a' + 10),
            b'A'..=b'F' => Some(ch - b'A' + 10),
            _ => None,
        }
    }
    let bytes = text.as_bytes();
    if bytes.len() % 2 != 0 {
        return None;
    }
    let mut out = Vec::with_capacity(bytes.len() / 2);
    let mut idx = 0;
    while idx < bytes.len() {
        let hi = nibble(bytes[idx])?;
        let lo = nibble(bytes[idx + 1])?;
        out.push((hi << 4) | lo);
        idx += 2;
    }
    Some(out)
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
    hwnd: HWND,
    action: CloudSyncAction,
    auto_triggered: bool,
    settings: AppSettings,
) {
    let hwnd_value = hwnd as isize;
    let config = cloud_sync_config_from_settings(&settings);
    let paths = current_cloud_sync_paths();
    std::thread::spawn(move || {
        let result = perform_cloud_sync(action, &config, &paths);
        unsafe {
            let still_alive = hwnd_value != 0 && IsWindow(hwnd_value as HWND) != 0;
            if still_alive {
                if let Ok(mut queue) = cloud_sync_results().lock() {
                    queue.push_back(CloudSyncResult {
                        hwnd: hwnd_value,
                        action,
                        auto_triggered,
                        result,
                    });
                }
                let _ = PostMessageW(hwnd_value as HWND, WM_CLOUD_SYNC_READY, 0, 0);
            }
        }
    });
}

pub(super) fn autostart_command_for_current_exe() -> Option<String> {
    current_exe_path().map(|path| format!("\"{}\"", path.to_string_lossy()))
}

pub(super) fn normalize_run_target(value: &str) -> String {
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
    if let Some(exe) = current_exe_path() {
        normalize_run_target(value) == normalize_run_target(&exe.to_string_lossy())
    } else {
        !value.trim().is_empty()
    }
}

unsafe fn read_run_value(hkey: isize, value_name: &str) -> Option<String> {
    let value_name_wide = to_wide(value_name);
    let mut data_size = 0u32;
    let mut reg_type = 0u32;
    let ret = RegQueryValueExW(
        hkey,
        value_name_wide.as_ptr(),
        null_mut(),
        &mut reg_type,
        null_mut(),
        &mut data_size,
    );
    if ret != 0 || reg_type != REG_SZ || data_size < 2 {
        return None;
    }

    let mut data = vec![0u8; data_size as usize];
    let ret = RegQueryValueExW(
        hkey,
        value_name_wide.as_ptr(),
        null_mut(),
        &mut reg_type,
        data.as_mut_ptr(),
        &mut data_size,
    );
    if ret != 0 || reg_type != REG_SZ {
        return None;
    }

    let wide = std::slice::from_raw_parts(data.as_ptr() as *const u16, (data_size as usize) / 2);
    let value_len = wide.iter().position(|&ch| ch == 0).unwrap_or(wide.len());
    Some(String::from_utf16_lossy(&wide[..value_len]))
}

fn autostart_value_names() -> impl Iterator<Item = &'static str> {
    std::iter::once(AUTOSTART_VALUE_NAME).chain(LEGACY_AUTOSTART_VALUE_NAMES.iter().copied())
}

unsafe fn registered_autostart_value_name_by_path(hkey: isize) -> Option<&'static str> {
    autostart_value_names().find(|name| {
        read_run_value(hkey, name)
            .map(|value| run_target_matches_current_exe(&value))
            .unwrap_or(false)
    })
}

unsafe fn registered_autostart_value_name(hkey: isize) -> Option<&'static str> {
    registered_autostart_value_name_by_path(hkey).or_else(|| {
        autostart_value_names().find(|name| {
            read_run_value(hkey, name)
                .map(|value| !normalize_run_target(&value).is_empty())
                .unwrap_or(false)
        })
    })
}

pub(super) fn is_autostart_enabled() -> bool {
    unsafe {
        let run_key = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let mut hkey: isize = 0;
        if RegOpenKeyExW(
            HKEY_CURRENT_USER_VAL,
            run_key.as_ptr(),
            0,
            KEY_READ_VAL,
            &mut hkey,
        ) != 0
        {
            return false;
        }
        let enabled = registered_autostart_value_name(hkey).is_some();
        RegCloseKey(hkey);
        enabled
    }
}

pub(super) fn apply_autostart(enabled: bool) -> bool {
    unsafe {
        let run_key = to_wide("Software\\Microsoft\\Windows\\CurrentVersion\\Run");
        let mut hkey: isize = 0;
        let flags = KEY_READ_VAL | KEY_SET_VALUE;
        if RegOpenKeyExW(HKEY_CURRENT_USER_VAL, run_key.as_ptr(), 0, flags, &mut hkey) != 0 {
            return false;
        }
        let mut changed = false;
        if enabled {
            if let Some(cmdline) = autostart_command_for_current_exe() {
                let wide = to_wide(&cmdline);
                let bytes = std::slice::from_raw_parts(wide.as_ptr() as *const u8, wide.len() * 2);
                let stable_name = to_wide(AUTOSTART_VALUE_NAME);
                changed = RegSetValueExW(
                    hkey,
                    stable_name.as_ptr(),
                    0,
                    REG_SZ,
                    bytes.as_ptr(),
                    bytes.len() as u32,
                ) == 0;
                if changed {
                    for legacy_name in LEGACY_AUTOSTART_VALUE_NAMES {
                        let legacy_wide = to_wide(legacy_name);
                        let _ = RegDeleteValueW(hkey, legacy_wide.as_ptr());
                    }
                    changed = read_run_value(hkey, AUTOSTART_VALUE_NAME)
                        .map(|value| normalize_run_target(&value) == normalize_run_target(&cmdline))
                        .unwrap_or(false);
                }
            }
        } else {
            changed = false;
            for value_name in autostart_value_names() {
                let value_name_wide = to_wide(value_name);
                if RegDeleteValueW(hkey, value_name_wide.as_ptr()) == 0 {
                    changed = true;
                }
            }
            if !is_autostart_enabled() {
                changed = true;
            }
        }
        RegCloseKey(hkey);
        if enabled {
            changed
        } else {
            changed && !is_autostart_enabled()
        }
    }
}
