//! OS-owned credentials. Native JSON files contain opaque references, never new plaintext secrets.
use std::{fmt, fs, io::{Read, Write}, path::{Path, PathBuf}, sync::{Mutex, OnceLock}};
use sha2::{Digest, Sha256};
use crate::settings_model::{SettingsNativeCollectSubmission, SettingsNativeJsonApplyResult, SettingsNativeJsonFieldUpdate};

const SERVICE: &str = "org.zsclip.credentials.v1";
const PREFIX: &str = "zsclip-secret:v1:";
const SECRET_FIELDS: [&str; 5] = ["cloud_webdav_pass", "image_ocr_cloud_url", "image_ocr_cloud_token", "text_translate_app_id", "text_translate_secret"];
static SAVE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SecretError { Unavailable, Locked, Missing, InvalidReference, LegacyPlaintext, ForeignEncrypted, Verification, InvalidFile, WriteFailed }
impl fmt::Display for SecretError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::Unavailable => crate::i18n::tr("系统安全凭据服务不可用或拒绝访问。请检查 macOS 钥匙串或 Linux Secret Service。", "The system credential service is unavailable or denied access. Check macOS Keychain or Linux Secret Service."),
            Self::Locked => crate::i18n::tr("系统密钥环已锁定，请先解锁后重试保存。", "The system keyring is locked. Unlock it and retry saving."),
            Self::Missing => crate::i18n::tr("当前系统中找不到已保存的凭据，请重新输入。", "The saved credential is missing from this system. Enter it again."),
            Self::InvalidReference => crate::i18n::tr("凭据引用无效，原配置未被覆盖。", "The credential reference is invalid. The original configuration was preserved."),
            Self::LegacyPlaintext => crate::i18n::tr("检测到旧版明文凭据。请打开设置并保存，将其迁移到系统安全存储。", "Legacy plaintext credentials were found. Open Settings and save to migrate them into system credential storage."),
            Self::ForeignEncrypted => crate::i18n::tr("此凭据由其他系统加密，当前系统无法读取。请重新输入。", "This credential was encrypted by another system and cannot be read here. Enter it again."),
            Self::Verification => crate::i18n::tr("系统安全存储回读校验失败，原配置未被覆盖。", "Credential read-back verification failed. The original configuration was preserved."),
            Self::InvalidFile => crate::i18n::tr("现有配置或配对文件不可读取或格式无效，未覆盖原文件。", "An existing settings or pairing file is unreadable or invalid. The original file was preserved."),
            Self::WriteFailed => crate::i18n::tr("无法原子保存配置文件，原文件保持完整。", "The configuration could not be saved atomically. The original file remains intact."),
        };
        f.write_str(message)
    }
}
impl std::error::Error for SecretError {}

trait CredentialBackend {
    fn platform(&self) -> &'static str;
    fn put(&self, account: &str, value: &[u8]) -> Result<(), SecretError>;
    fn get(&self, account: &str) -> Result<Vec<u8>, SecretError>;
}
struct SystemBackend;

#[cfg(target_os = "macos")]
impl CredentialBackend for SystemBackend {
    fn platform(&self) -> &'static str { "keychain" }
    fn put(&self, account: &str, value: &[u8]) -> Result<(), SecretError> {
        security_framework::passwords::set_generic_password(SERVICE, account, value).map_err(|_| SecretError::Unavailable)
    }
    fn get(&self, account: &str) -> Result<Vec<u8>, SecretError> {
        security_framework::passwords::generic_password(security_framework::passwords::PasswordOptions::new_generic_password(SERVICE, account))
            .map_err(|error| if error.code() == -25300 { SecretError::Missing } else { SecretError::Unavailable })
    }
}

#[cfg(target_os = "linux")]
impl CredentialBackend for SystemBackend {
    fn platform(&self) -> &'static str { "secret-service" }
    fn put(&self, account: &str, value: &[u8]) -> Result<(), SecretError> {
        use secret_service::{blocking::SecretService, EncryptionType};
        let service = SecretService::connect(EncryptionType::Dh).map_err(|_| SecretError::Unavailable)?;
        let collection = service.get_default_collection().map_err(|_| SecretError::Unavailable)?;
        if collection.is_locked().map_err(|_| SecretError::Unavailable)? { return Err(SecretError::Locked); }
        collection.create_item("ZSClip credential", std::collections::HashMap::from([("application", SERVICE), ("account", account)]), value, false, "application/octet-stream")
            .map(|_| ()).map_err(|_| SecretError::Unavailable)
    }
    fn get(&self, account: &str) -> Result<Vec<u8>, SecretError> {
        use secret_service::{blocking::SecretService, EncryptionType};
        let service = SecretService::connect(EncryptionType::Dh).map_err(|_| SecretError::Unavailable)?;
        let items = service.search_items(std::collections::HashMap::from([("application", SERVICE), ("account", account)]))
            .map_err(|_| SecretError::Unavailable)?;
        if !items.locked.is_empty() { return Err(SecretError::Locked); }
        if items.unlocked.len() != 1 { return Err(SecretError::Missing); }
        items.unlocked[0].get_secret().map_err(|_| SecretError::Unavailable)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "linux")))]
impl CredentialBackend for SystemBackend {
    fn platform(&self) -> &'static str { "unsupported" }
    fn put(&self, _: &str, _: &[u8]) -> Result<(), SecretError> { Err(SecretError::Unavailable) }
    fn get(&self, _: &str) -> Result<Vec<u8>, SecretError> { Err(SecretError::Unavailable) }
}

fn fresh_id() -> Result<String, SecretError> {
    #[cfg(unix)]
    {
        let mut bytes = [0_u8; 24];
        fs::File::open("/dev/urandom").and_then(|mut file| file.read_exact(&mut bytes)).map_err(|_| SecretError::Unavailable)?;
        Ok(bytes.iter().map(|byte| format!("{byte:02x}")).collect())
    }
    #[cfg(not(unix))]
    {
        // This module is test-only on Windows; production Windows retains its DPAPI implementation.
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
        Ok(format!("{:048x}", NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)))
    }
}

fn account_from_reference<'a>(value: &'a str, backend: &impl CredentialBackend) -> Result<&'a str, SecretError> {
    let value = value.strip_prefix(PREFIX).ok_or(if value.starts_with("zsclip-secret:") {SecretError::InvalidReference}else{SecretError::LegacyPlaintext})?;
    let (platform, account) = value.split_once(':').ok_or(SecretError::InvalidReference)?;
    if platform != backend.platform() { return Err(SecretError::ForeignEncrypted); }
    if account.len() != 48 || !account.bytes().all(|ch| ch.is_ascii_hexdigit()) { return Err(SecretError::InvalidReference); }
    Ok(account)
}

fn store_with(backend: &impl CredentialBackend, secret: &str) -> Result<String, SecretError> {
    if secret.is_empty() { return Ok(String::new()); }
    let account = fresh_id()?;
    store_with_account(backend,secret,&account)
}

fn store_with_account(backend: &impl CredentialBackend, secret: &str, account: &str) -> Result<String, SecretError> {
    backend.put(account, secret.as_bytes())?;
    let check = zeroize::Zeroizing::new(backend.get(account)?);
    if check.as_slice() != secret.as_bytes() { return Err(SecretError::Verification); }
    Ok(format!("{PREFIX}{}:{account}", backend.platform()))
}

fn load_with(backend: &impl CredentialBackend, encoded: &str) -> Result<String, SecretError> {
    if encoded.is_empty() { return Ok(String::new()); }
    let account = account_from_reference(encoded, backend)?;
    String::from_utf8(backend.get(account)?).map_err(|_| SecretError::Verification)
}

pub(crate) fn store(secret: &str) -> Result<String, SecretError> {
    if secret.is_empty() { return Ok(String::new()); }
    // LAN state may be persisted repeatedly. Reuse an already verified opaque reference.
    static REFERENCES: OnceLock<Mutex<std::collections::HashMap<[u8; 32], String>>> = OnceLock::new();
    let cache = REFERENCES.get_or_init(|| Mutex::new(std::collections::HashMap::new()));
    let fingerprint: [u8; 32] = Sha256::digest(secret.as_bytes()).into();
    let cached = cache.lock().unwrap_or_else(|error| error.into_inner()).get(&fingerprint).cloned();
    if let Some(reference) = cached {
        let check = zeroize::Zeroizing::new(load_with(&SystemBackend, &reference)?);
        if check.as_str() == secret { return Ok(reference); }
    }
    let reference = store_with(&SystemBackend, secret)?;
    cache.lock().unwrap_or_else(|error| error.into_inner()).insert(fingerprint, reference.clone());
    Ok(reference)
}
pub(crate) fn load(encoded: &str) -> Result<String, SecretError> { load_with(&SystemBackend, encoded) }

pub(crate) fn sensitive_field(field: &str) -> Option<&'static str> {
    if field == "image_ocr_cloud_url_or_wechat_dir" { return Some("image_ocr_cloud_url"); }
    SECRET_FIELDS.into_iter().find(|candidate| *candidate == field)
}

pub(crate) fn action_error_message(result_name: &str) -> Option<&str> {
    result_name.strip_prefix("zsclip.settings.native_save_failed: ")
        .or_else(||result_name.strip_prefix("zsclip.settings_sync.credential_error: "))
        .or_else(||result_name.strip_prefix("zsclip.settings_sync.webdav.failed."))
        .or_else(||result_name.starts_with("zsclip.settings_sync.").then(||result_name.split_once(".store_failed.").map(|(_,message)|message)).flatten())
        .or_else(||result_name.starts_with("zsclip.settings_sync.").then(||result_name.split_once(".save_failed.").map(|(_,message)|message)).flatten())
}

pub(crate) fn resolve_setting(settings: &serde_json::Value, field: &str) -> Result<String, SecretError> {
    let plain = settings.get(field).and_then(serde_json::Value::as_str).unwrap_or("");
    if !plain.is_empty() { return load(plain); }
    let key = format!("{field}_encrypted");
    let encoded = settings.get(&key).and_then(serde_json::Value::as_str).unwrap_or("");
    if !encoded.is_empty() && !encoded.starts_with(PREFIX) { return Err(SecretError::ForeignEncrypted); }
    load(encoded)
}

pub(crate) fn validate_settings_for_export(path: &Path) -> Result<(), SecretError> {
    let settings=read_json(path)?;
    for field in SECRET_FIELDS.into_iter().chain(["image_ocr_cloud_url_or_wechat_dir"]) {
        match settings.get(field) {
            None | Some(serde_json::Value::Null) => {},
            Some(serde_json::Value::String(value)) if value.is_empty() => {},
            Some(serde_json::Value::String(value)) if value.starts_with(PREFIX) => {account_from_reference(value,&SystemBackend)?;},
            Some(serde_json::Value::String(_)) => return Err(SecretError::LegacyPlaintext),
            _ => return Err(SecretError::InvalidFile),
        }
    }
    for field in SECRET_FIELDS {
        let key=format!("{field}_encrypted");
        match settings.get(&key) {
            None | Some(serde_json::Value::Null) => {},
            Some(serde_json::Value::String(value)) if value.is_empty() || is_windows_dpapi(value) => {},
            Some(serde_json::Value::String(value)) if value.starts_with(PREFIX) => {account_from_reference(value,&SystemBackend)?;},
            Some(serde_json::Value::String(_)) => return Err(SecretError::ForeignEncrypted),
            _ => return Err(SecretError::InvalidFile),
        }
    }
    Ok(())
}

fn is_windows_dpapi(value: &str) -> bool {
    value.get(..40).is_some_and(|prefix|prefix.eq_ignore_ascii_case("01000000d08c9ddf0115d1118c7a00c04fc297eb"))
        && value.len()%2==0 && value.bytes().all(|byte|byte.is_ascii_hexdigit())
}

fn prepare_settings_with(backend: &impl CredentialBackend, existing: serde_json::Value, submission: &SettingsNativeCollectSubmission) -> Result<SettingsNativeJsonApplyResult, SecretError> {
    let mut ordinary = submission.clone();
    ordinary.applied_fields.retain(|field| sensitive_field(field.field_name).is_none());
    let mut applied = crate::settings_model::settings_native_apply_submission_to_json(existing, &ordinary);
    if !applied.rejected_fields.is_empty() { return Ok(applied); }
    for field in SECRET_FIELDS {
        let submitted = submission.applied_fields.iter().find(|entry| sensitive_field(entry.field_name) == Some(field)).map(|entry| entry.value.as_str()).filter(|value| !value.is_empty());
        let old_plain = applied.settings_json.get(field).and_then(serde_json::Value::as_str).filter(|value|!value.is_empty())
            .or_else(||(field=="image_ocr_cloud_url").then(||applied.settings_json.get("image_ocr_cloud_url_or_wechat_dir").and_then(serde_json::Value::as_str)).flatten())
            .unwrap_or("");
        let new_reference = if let Some(secret) = submitted {
            Some(store_with(backend, secret)?)
        } else if !old_plain.is_empty() {
            if old_plain.starts_with("zsclip-secret:") { account_from_reference(old_plain, backend)?; Some(old_plain.to_string()) }
            else { Some(store_with(backend, old_plain)?) }
        } else { None };
        if let Some(reference) = new_reference {
            let encrypted = format!("{field}_encrypted");
            applied.settings_json[field] = serde_json::Value::String(String::new());
            if field=="image_ocr_cloud_url" {applied.settings_json.as_object_mut().unwrap().remove("image_ocr_cloud_url_or_wechat_dir");}
            applied.settings_json[&encrypted] = serde_json::Value::String(reference.clone());
            applied.field_updates.push(SettingsNativeJsonFieldUpdate { field_name: encrypted, value: serde_json::Value::String(reference) });
        }
    }
    Ok(applied)
}

fn read_json(path: &Path) -> Result<serde_json::Value, SecretError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if !metadata.is_file() || metadata.file_type().is_symlink() || metadata.len() > 16 * 1024 * 1024 => return Err(SecretError::InvalidFile),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(serde_json::json!({})),
        Err(_) => return Err(SecretError::InvalidFile),
        _ => {},
    }
    match fs::read(path) {
        Ok(bytes) => serde_json::from_slice(&bytes).map_err(|_| SecretError::InvalidFile),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(serde_json::json!({})),
        Err(_) => Err(SecretError::InvalidFile),
    }
}

pub(crate) fn validate_lan_json_shape(path: &Path, value: &serde_json::Value) -> Result<(), SecretError> {
    match path.file_name().and_then(|name|name.to_str()) {
        Some("lan_pending_pairs.json") => serde_json::from_value::<crate::lan_sync_core::StoredPendingPairBook>(value.clone()).map(|_|()),
        Some("lan_discovered_devices.json") => serde_json::from_value::<crate::lan_sync_core::StoredDiscoveredDeviceBook>(value.clone()).map(|_|()),
        _ => serde_json::from_value::<crate::lan_sync_core::StoredDeviceBook>(value.clone()).map(|_|()),
    }.map_err(|_|SecretError::InvalidFile)
}

fn migrate_lan_value(backend: &impl CredentialBackend, value: &mut serde_json::Value) -> Result<bool, SecretError> {
    let mut changed = false;
    match value {
        serde_json::Value::Object(fields) => {
            for (key, value) in fields {
                if key == "token_encrypted" {
                    if value.is_null() { continue; }
                    let token = value.as_str().ok_or(SecretError::InvalidFile)?;
                    if !token.is_empty() {
                        if token.starts_with("zsclip-secret:") { let _ = zeroize::Zeroizing::new(load_with(backend, token)?); }
                        else if token.starts_with("dpapi:") || token.starts_with("DPAPI:") || is_windows_dpapi(token) { return Err(SecretError::ForeignEncrypted); }
                        else { *value = serde_json::Value::String(store_with(backend, token)?); changed = true; }
                    }
                } else { changed |= migrate_lan_value(backend, value)?; }
            }
        },
        serde_json::Value::Array(items) => for item in items { changed |= migrate_lan_value(backend, item)?; },
        _ => {},
    }
    Ok(changed)
}

pub(crate) fn validate_lan_profile(directory: &Path) -> Result<(), SecretError> {
    fn inspect(value: &serde_json::Value) -> Result<(), SecretError> {
        match value {
            serde_json::Value::Object(fields) => for (key,value) in fields {
                if key == "token_encrypted" && !value.is_null() {
                    let token=value.as_str().ok_or(SecretError::InvalidFile)?;
                    if !token.is_empty() { let _ = zeroize::Zeroizing::new(load(token)?); }
                } else { inspect(value)?; }
            },
            serde_json::Value::Array(values) => for value in values { inspect(value)?; },
            _ => {},
        }
        Ok(())
    }
    for filename in ["lan_devices.json", "lan_pending_pairs.json", "lan_discovered_devices.json"] {
        let path=directory.join(filename);
        if !path.try_exists().map_err(|_|SecretError::InvalidFile)? {continue;}
        let value=read_json(&path)?;
        validate_lan_json_shape(&path,&value)?;
        inspect(&value)?;
    }
    Ok(())
}

pub(crate) fn write_json_atomically(path: &Path, value: &serde_json::Value) -> Result<(), SecretError> {
    let parent = path.parent().ok_or(SecretError::WriteFailed)?;
    fs::create_dir_all(parent).map_err(|_| SecretError::WriteFailed)?;
    let temporary = parent.join(format!(".zsclip-settings-{}.tmp", fresh_id()?));
    let result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)] { use std::os::unix::fs::OpenOptionsExt; options.mode(0o600); }
        let mut file = options.open(&temporary).map_err(|_| SecretError::WriteFailed)?;
        let text = zeroize::Zeroizing::new(serde_json::to_vec_pretty(value).map_err(|_| SecretError::WriteFailed)?);
        file.write_all(&text).and_then(|_| file.sync_all()).map_err(|_| SecretError::WriteFailed)?;
        drop(file);
        fs::rename(&temporary, path).map_err(|_| SecretError::WriteFailed)
    })();
    if result.is_err() { let _ = fs::remove_file(&temporary); }
    result
}

fn save_settings_with(backend: &impl CredentialBackend, path: &Path, submission: &SettingsNativeCollectSubmission) -> Result<SettingsNativeJsonApplyResult, SecretError> {
    let existing = read_json(path)?;
    if !existing.is_object() { return Err(SecretError::InvalidFile); }
    let applied = prepare_settings_with(backend, existing, submission)?;
    if !applied.rejected_fields.is_empty() { return Ok(applied); }
    let mut migrations: Vec<(PathBuf, serde_json::Value)> = Vec::new();
    let parent = path.parent().ok_or(SecretError::InvalidFile)?;
    for filename in ["lan_devices.json", "lan_pending_pairs.json", "lan_discovered_devices.json"] {
        let file = parent.join(filename);
        if !file.try_exists().map_err(|_|SecretError::InvalidFile)? {continue;}
        let mut value = read_json(&file)?;
        validate_lan_json_shape(&file,&value)?;
        if migrate_lan_value(backend, &mut value)? { migrations.push((file, value)); }
    }
    // Complete all credential writes and verification before replacing any existing file.
    for (file, value) in migrations { write_json_atomically(&file, &value)?; }
    if !applied.field_updates.is_empty() { write_json_atomically(path, &applied.settings_json)?; }
    Ok(applied)
}

pub(crate) fn save_settings(path: &Path, submission: &SettingsNativeCollectSubmission) -> Result<SettingsNativeJsonApplyResult, SecretError> {
    let _guard = SAVE_LOCK.lock().unwrap_or_else(|error| error.into_inner());
    crate::db_runtime::with_shared_app_data(||save_settings_with(&SystemBackend, path, submission))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::cell::{Cell, RefCell};

    #[derive(Default)]
    struct MemoryBackend {
        values: RefCell<std::collections::HashMap<String, Vec<u8>>>,
        fail_after: Cell<Option<usize>>,
        writes: Cell<usize>,
    }
    impl CredentialBackend for MemoryBackend {
        fn platform(&self) -> &'static str { "test" }
        fn put(&self, key: &str, value: &[u8]) -> Result<(), SecretError> {
            if self.fail_after.get().is_some_and(|limit|self.writes.get()>=limit) { return Err(SecretError::Unavailable); }
            self.writes.set(self.writes.get()+1);
            self.values.borrow_mut().insert(key.into(),value.to_vec());
            Ok(())
        }
        fn get(&self, key: &str) -> Result<Vec<u8>, SecretError> {
            self.values.borrow().get(key).cloned().ok_or(SecretError::Missing)
        }
    }
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            let path=std::env::temp_dir().join(format!("zsclip-native-secrets-{}-{}",std::process::id(),fresh_id().unwrap()));
            fs::create_dir_all(&path).unwrap(); Self(path)
        }
    }
    impl Drop for Fixture { fn drop(&mut self) { let _=fs::remove_dir_all(&self.0); } }
    fn submission(values: &[(&str,&str)]) -> SettingsNativeCollectSubmission {
        crate::settings_model::settings_native_collect_submission(&values.iter().map(|(key,value)|crate::settings_model::SettingsNativeSubmittedControlValue {
            control_key:(*key).into(),raw_value:(*value).into(),
        }).collect::<Vec<_>>())
    }

    #[test]
    fn opaque_references_round_trip_and_never_accept_plaintext_or_foreign_platforms() {
        let backend=MemoryBackend::default();
        let reference=store_with(&backend," synthetic secret ").unwrap();
        assert!(reference.starts_with("zsclip-secret:v1:test:"));
        assert!(!reference.contains("synthetic"));
        assert_eq!(load_with(&backend,&reference).unwrap()," synthetic secret ");
        assert_eq!(load_with(&backend,"plaintext"),Err(SecretError::LegacyPlaintext));
        assert_eq!(load_with(&backend,"zsclip-secret:v1:keychain:1234"),Err(SecretError::ForeignEncrypted));
        assert_eq!(load_with(&backend,"zsclip-secret:v1:test:../../credential"),Err(SecretError::InvalidReference));
    }

    #[test]
    fn blank_secret_retains_reference_and_new_value_preserves_password_whitespace() {
        let backend=MemoryBackend::default();
        let reference=store_with(&backend,"old synthetic password").unwrap();
        let original=serde_json::json!({"cloud_webdav_pass":"","cloud_webdav_pass_encrypted":reference});
        let blank=prepare_settings_with(&backend,original.clone(),&submission(&[("cloud_webdav_pass","")])).unwrap();
        assert_eq!(blank.settings_json,original);
        let input=submission(&[("cloud_webdav_pass","  changed synthetic password  ")]);
        assert!(!crate::settings_model::settings_native_apply_submission_to_json(original.clone(),&input).rejected_fields.is_empty());
        let changed=prepare_settings_with(&backend,original,&input).unwrap();
        assert_eq!(changed.settings_json["cloud_webdav_pass"],"");
        let stored=changed.settings_json["cloud_webdav_pass_encrypted"].as_str().unwrap();
        assert_eq!(load_with(&backend,stored).unwrap(),"  changed synthetic password  ");
    }

    #[test]
    fn all_sensitive_controls_write_only_verified_references() {
        let backend=MemoryBackend::default();
        let input=submission(&[("cloud_webdav_pass","synthetic-dav"),("ocr_cloud_url","synthetic-ocr-url"),
            ("ocr_cloud_token","synthetic-ocr-token"),("translate_app_id","synthetic-app-id"),("translate_secret","synthetic-translate")]);
        let applied=prepare_settings_with(&backend,serde_json::json!({}),&input).unwrap();
        assert!(applied.rejected_fields.is_empty());
        for (field,expected) in SECRET_FIELDS.into_iter().zip(["synthetic-dav","synthetic-ocr-url","synthetic-ocr-token","synthetic-app-id","synthetic-translate"]) {
            assert_eq!(applied.settings_json[field],"");
            let key=format!("{field}_encrypted");
            let reference=applied.settings_json[&key].as_str().unwrap();
            assert_eq!(load_with(&backend,reference).unwrap(),expected);
            assert!(!serde_json::to_string(&applied.settings_json).unwrap().contains(expected));
        }
        assert!(!applied.settings_json.as_object().unwrap().contains_key("image_ocr_cloud_url_or_wechat_dir"));
    }

    #[test]
    fn explicit_save_migrates_settings_and_lan_tokens_after_secure_readback() {
        let fixture=Fixture::new(); let backend=MemoryBackend::default();
        let settings=fixture.0.join("settings.json"); let lan=fixture.0.join("lan_devices.json");
        fs::write(&settings,br#"{"cloud_webdav_pass":"legacy-password","keep":7}"#).unwrap();
        fs::write(&lan,br#"{"devices":[{"device_id":"synthetic","name":"Synthetic phone","addr":"127.0.0.1","tcp_port":38475,"last_seen_ms":1,"trusted":true,"token_encrypted":"legacy-token"}]}"#).unwrap();
        save_settings_with(&backend,&settings,&submission(&[])).unwrap();
        let saved=read_json(&settings).unwrap(); let saved_lan=read_json(&lan).unwrap();
        assert_eq!(saved["keep"],7); assert_eq!(saved["cloud_webdav_pass"],"");
        assert_eq!(load_with(&backend,saved["cloud_webdav_pass_encrypted"].as_str().unwrap()).unwrap(),"legacy-password");
        assert_eq!(load_with(&backend,saved_lan["devices"][0]["token_encrypted"].as_str().unwrap()).unwrap(),"legacy-token");
        assert!(!fs::read_to_string(&settings).unwrap().contains("legacy-password"));
        assert!(!fs::read_to_string(&lan).unwrap().contains("legacy-token"));
    }

    #[test]
    fn unavailable_backend_or_invalid_file_never_overwrites_original_files() {
        let fixture=Fixture::new(); let backend=MemoryBackend::default();
        let settings=fixture.0.join("settings.json"); let lan=fixture.0.join("lan_devices.json");
        let original=br#"{"cloud_webdav_pass":"legacy-password","max_items":50}"#;
        let old_lan=br#"{"devices":[{"device_id":"first","name":"First","addr":"127.0.0.1","tcp_port":38475,"last_seen_ms":1,"trusted":true,"token_encrypted":"first-token"},{"device_id":"second","name":"Second","addr":"127.0.0.1","tcp_port":38475,"last_seen_ms":1,"trusted":true,"token_encrypted":"second-token"}]}"#;
        fs::write(&settings,original).unwrap(); fs::write(&lan,old_lan).unwrap();
        backend.fail_after.set(Some(2));
        assert!(save_settings_with(&backend,&settings,&submission(&[("max_items","100")])).is_err());
        assert_eq!(fs::read(&settings).unwrap(),original);
        assert_eq!(fs::read(&lan).unwrap(),old_lan);
        fs::write(&settings,b"{broken}").unwrap();
        assert_eq!(save_settings_with(&backend,&settings,&submission(&[])).unwrap_err(),SecretError::InvalidFile);
        assert_eq!(fs::read(&settings).unwrap(),b"{broken}");
    }

    #[test]
    fn windows_dpapi_lan_blob_is_not_migrated_as_a_plaintext_token() {
        let backend=MemoryBackend::default();
        let mut value=serde_json::json!({"devices":[{"token_encrypted":"01000000d08c9ddf0115d1118c7a00c04fc297eb00000000"}]});
        let original=value.clone();
        assert_eq!(migrate_lan_value(&backend,&mut value),Err(SecretError::ForeignEncrypted));
        assert_eq!(value,original); assert_eq!(backend.writes.get(),0);
    }

    #[test]
    fn malformed_lan_books_and_old_plaintext_export_are_rejected_without_writes() {
        let fixture=Fixture::new(); let backend=MemoryBackend::default();
        let settings=fixture.0.join("settings.json"); let lan=fixture.0.join("lan_devices.json");
        fs::write(&settings,br#"{"image_ocr_cloud_token":"synthetic-legacy-token"}"#).unwrap();
        assert_eq!(validate_settings_for_export(&settings),Err(SecretError::LegacyPlaintext));
        fs::write(&lan,br#"{"devices":[{"token_encrypted":"incomplete-record"}]}"#).unwrap();
        let before=fs::read(&settings).unwrap(); let before_lan=fs::read(&lan).unwrap();
        assert_eq!(save_settings_with(&backend,&settings,&submission(&[])).unwrap_err(),SecretError::InvalidFile);
        assert_eq!(fs::read(&settings).unwrap(),before);
        assert_eq!(fs::read(&lan).unwrap(),before_lan);
        fs::write(&settings,br#"{"image_ocr_cloud_token":"","image_ocr_cloud_token_encrypted":"01000000d08c9ddf0115d1118c7a00c04fc297eb00000000"}"#).unwrap();
        assert!(validate_settings_for_export(&settings).is_ok());
    }

    #[test]
    fn real_backends_are_called_directly_and_settings_normalizer_stays_closed() {
        let source=include_str!("native_secret_store.rs");
        let production=source.split_once("#[cfg(test)]\nmod tests").or_else(||source.split_once("#[cfg(test)]\r\nmod tests")).unwrap().0;
        assert!(production.contains("security_framework::passwords::set_generic_password"));
        assert!(production.contains("SecretService::connect(EncryptionType::Dh)"));
        assert!(!production.contains("std::process::Command"));
        for app_source in [include_str!("macos_app.rs"),include_str!("linux_app.rs")] {
            assert!(app_source.contains("native_secret_store::save_settings(&path, submission)"));
            assert!(app_source.contains("native_secret_store::resolve_setting(settings_json, \"cloud_webdav_pass\")?"));
            assert!(app_source.contains("native_secret_store::store(secret)"));
            assert!(app_source.contains("native_secret_store::load(encoded)"));
            assert!(!app_source.contains("Some(secret.to_string())"));
            assert!(!app_source.contains("Some(encoded.to_string())"));
        }
        assert!(include_str!("macos_native_host.rs").contains("NSSecureTextField::new(mtm)"));
        assert!(include_str!("linux_native_host.rs").contains("entry.set_visibility(false)"));
    }

    #[cfg(any(target_os="linux",target_os="macos"))]
    struct SystemCredentialCleanup { account: String }
    #[cfg(any(target_os="linux",target_os="macos"))]
    impl SystemCredentialCleanup {
        fn new() -> Self {
            let account=fresh_id().unwrap();
            if let Some(path)=std::env::var_os("ZSCLIP_NATIVE_SECRET_TEST_ACCOUNT_FILE") {
                // Non-secret receipt lets the external CI trap remove this exact account after forced termination.
                let mut file=fs::OpenOptions::new().write(true).create_new(true).open(path).expect("Create synthetic credential cleanup receipt");
                file.write_all(account.as_bytes()).unwrap();
            }
            Self {account}
        }
        fn delete(&self) -> Result<(), SecretError> {
            #[cfg(target_os="macos")]
            return security_framework::passwords::delete_generic_password(SERVICE,&self.account)
                .map_err(|error|if error.code()==-25300 {SecretError::Missing}else{SecretError::Unavailable});
            #[cfg(target_os="linux")]
            {
                let service=secret_service::blocking::SecretService::connect(secret_service::EncryptionType::Dh).map_err(|_|SecretError::Unavailable)?;
                let items=service.search_items(std::collections::HashMap::from([("application",SERVICE),("account",self.account.as_str())])).map_err(|_|SecretError::Unavailable)?;
                if !items.locked.is_empty() {return Err(SecretError::Locked);}
                for item in items.unlocked {item.delete().map_err(|_|SecretError::Unavailable)?;}
                Ok(())
            }
        }
    }
    #[cfg(any(target_os="linux",target_os="macos"))]
    impl Drop for SystemCredentialCleanup {
        fn drop(&mut self) {
            if let Err(error)=self.delete() {
                if error!=SecretError::Missing {eprintln!("Synthetic credential cleanup failed: {error}");}
            }
        }
    }

    #[cfg(any(target_os="linux",target_os="macos"))]
    #[test]
    #[ignore = "requires an unlocked OS credential store; creates and removes only one synthetic ZSClip credential"]
    fn actual_system_store_round_trip() {
        let value=format!("ZSClip synthetic credential {}",fresh_id().unwrap());
        let cleanup=SystemCredentialCleanup::new();
        assert_eq!(SystemBackend.get(&cleanup.account).unwrap_err(),SecretError::Missing);
        let reference=store_with_account(&SystemBackend,&value,&cleanup.account).expect("OS credential write and verification");
        assert_eq!(load_with(&SystemBackend,&reference).unwrap(),value);
        cleanup.delete().expect("exact synthetic credential cleanup");
        assert_eq!(load_with(&SystemBackend,&reference).unwrap_err(),SecretError::Missing);
    }

    #[cfg(any(target_os="linux",target_os="macos"))]
    #[test]
    #[ignore = "cleanup helper for the exact random account recorded by the system-store integration test"]
    fn actual_system_store_cleanup_recorded_account() {
        let path=std::env::var_os("ZSCLIP_NATIVE_SECRET_TEST_ACCOUNT_FILE").expect("synthetic credential receipt");
        let account=fs::read_to_string(path).unwrap();
        assert_eq!(account.len(),48);
        assert!(account.bytes().all(|byte|byte.is_ascii_hexdigit()));
        let cleanup=SystemCredentialCleanup {account};
        match cleanup.delete() {Ok(())|Err(SecretError::Missing)=>{},Err(error)=>panic!("Synthetic credential cleanup: {error}")}
        assert_eq!(SystemBackend.get(&cleanup.account).unwrap_err(),SecretError::Missing);
    }

    #[cfg(target_os="linux")]
    fn isolated_secret_service_root() -> PathBuf {
        let root=PathBuf::from(std::env::var_os("ZSCLIP_NATIVE_SECRET_TEST_ROOT").expect("An isolated dbus/keyring fixture is required"));
        let root=fs::canonicalize(root).unwrap();
        assert!(root.file_name().unwrap().to_string_lossy().starts_with("zsclip-secret-service-ci-"));
        assert_eq!(fs::read(root.join("isolated-provider.marker")).unwrap(),b"ZSClip synthetic Secret Service fixture v1\n");
        for (variable,child) in [("XDG_DATA_HOME","data"),("XDG_CONFIG_HOME","config"),("XDG_CACHE_HOME","cache"),("XDG_RUNTIME_DIR","runtime")] {
            assert_eq!(fs::canonicalize(std::env::var_os(variable).expect("isolated XDG directory")).unwrap(),root.join(child));
        }
        assert!(std::env::var("DBUS_SESSION_BUS_ADDRESS").is_ok_and(|address|!address.is_empty()));
        root
    }

    #[cfg(target_os="linux")]
    struct IsolatedKeyringUnlock { control: PathBuf, password: zeroize::Zeroizing<String> }
    #[cfg(target_os="linux")]
    impl IsolatedKeyringUnlock {
        fn unlock(&self) -> Result<(), String> {
            use std::process::{Command, Stdio};
            let mut child=Command::new("gnome-keyring-daemon").args(["--unlock","--components=secrets","--control-directory"])
                .arg(&self.control).stdin(Stdio::piped()).stdout(Stdio::null()).stderr(Stdio::null()).spawn().map_err(|_|"Unable to start isolated provider unlock")?;
            let write=child.stdin.take().ok_or("Missing isolated unlock pipe")?.write_all(self.password.as_bytes());
            if write.is_err() {let _=child.kill();let _=child.wait();return Err("Unable to write isolated unlock pipe".into());}
            let status=child.wait().map_err(|_|"Isolated provider unlock did not complete")?;
            if status.success() {Ok(())} else {Err("Isolated provider unlock failed".into())}
        }
    }
    #[cfg(target_os="linux")]
    impl Drop for IsolatedKeyringUnlock {
        fn drop(&mut self) {if let Err(error)=self.unlock() {eprintln!("{error}");}}
    }

    #[cfg(target_os="linux")]
    #[test]
    #[ignore = "requires the isolated dbus-run-session GNOME Keyring fixture; locks only its private collection"]
    fn actual_isolated_secret_service_locked_store_rejects_reads_and_writes() {
        let root=isolated_secret_service_root();
        let password=zeroize::Zeroizing::new(std::env::var("ZSCLIP_NATIVE_SECRET_TEST_PASSWORD").expect("synthetic fixture password"));
        assert!(!password.is_empty());
        let cleanup=SystemCredentialCleanup::new();
        let reference=store_with_account(&SystemBackend,"synthetic locked value",&cleanup.account).unwrap();
        // Drop order unlocks the private collection before deleting the exact synthetic item.
        let unlock=IsolatedKeyringUnlock {control:root.join("runtime/keyring"),password};
        let service=secret_service::blocking::SecretService::connect(secret_service::EncryptionType::Dh).unwrap();
        let collection=service.get_default_collection().unwrap();
        assert!(!collection.is_locked().unwrap());
        collection.lock().unwrap();
        assert!(collection.is_locked().unwrap());
        assert_eq!(load_with(&SystemBackend,&reference).unwrap_err(),SecretError::Locked);
        assert_eq!(store_with(&SystemBackend,"synthetic rejected write").unwrap_err(),SecretError::Locked);
        unlock.unlock().unwrap();
        assert!(!collection.is_locked().unwrap());
        assert_eq!(load_with(&SystemBackend,&reference).unwrap(),"synthetic locked value");
        cleanup.delete().unwrap();
        assert_eq!(load_with(&SystemBackend,&reference).unwrap_err(),SecretError::Missing);
    }

    #[cfg(target_os="linux")]
    #[test]
    #[ignore = "requires an isolated fixture with DBUS_SESSION_BUS_ADDRESS pointing to its nonexistent socket"]
    fn actual_isolated_secret_service_missing_backend_does_not_fall_back_to_plaintext() {
        let root=isolated_secret_service_root();
        assert_eq!(std::env::var("DBUS_SESSION_BUS_ADDRESS").unwrap(),format!("unix:path={}",root.join("runtime/absent-bus").display()));
        assert!(!root.join("runtime/absent-bus").exists());
        assert_eq!(store("synthetic offline credential").unwrap_err(),SecretError::Unavailable);
        let reference=format!("{PREFIX}secret-service:{}",fresh_id().unwrap());
        assert_eq!(load(&reference).unwrap_err(),SecretError::Unavailable);
    }
}
