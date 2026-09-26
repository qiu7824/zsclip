//! Short-lived, user-initiated account handoff from a paired input method.
use crate::qq_cloud::CloudAccount;
use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, KeyInit, Nonce,
};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{
    ffi::c_void,
    fs,
    io::{Read, Write},
    os::windows::ffi::OsStrExt,
    ptr::{null, null_mut},
    sync::{Mutex, OnceLock},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

const FORMAT: &str = "zsclip-qq-cloud-auth-v1";
const MAX_BODY: usize = 32 * 1024;
const WINDOW_MS: u64 = 300_000;
const INVALID: &str = "授权内容无效，请在电脑重新发起连接";
static STATE: OnceLock<Mutex<Option<AuthorizationSession>>> = OnceLock::new();
static STORE_LOCK: Mutex<()> = Mutex::new(());
static LAST_ACCEPTED: Mutex<Option<String>> = Mutex::new(None);

pub(crate) struct AuthorizationWindow {
    pub(crate) code: String,
    pub(crate) request_id: String,
}

#[derive(Serialize, Deserialize)]
struct StoredAccount {
    format: u32,
    encrypted_account: String,
}

struct Pending {
    id: String,
    expires: u64,
    deadline: Instant,
    key: RsaKey,
    public: String,
    fingerprint: String,
    peer: Option<String>,
}

enum AuthorizationSession {
    Pending(Pending),
    Candidate(Candidate),
}

impl AuthorizationSession {
    fn expired(&self) -> bool {
        match self {
            Self::Pending(pending) => pending.expired(),
            Self::Candidate(candidate) => candidate.expired(),
        }
    }
}

struct Candidate {
    id: String,
    expires: u64,
    deadline: Instant,
    confirmation_code: String,
    account: CloudAccount,
}

impl Candidate {
    fn expired(&self) -> bool {
        self.expires <= now_ms() || Instant::now() >= self.deadline
    }

    fn check_confirmation(&self, id: &str, code: &str) -> Result<(), String> {
        if self.expired() || self.id != id || self.confirmation_code != code {
            return Err("授权确认码不一致或已过期，请重新发起连接".into());
        }
        Ok(())
    }
}

impl Drop for Candidate {
    fn drop(&mut self) {
        // The unconfirmed credential is held only in memory and cleared on
        // confirmation, cancellation, replacement, or an expired-window check.
        unsafe {
            self.account.sgid.as_bytes_mut().fill(0);
            self.account.device_id.as_bytes_mut().fill(0);
            self.account.version.as_bytes_mut().fill(0);
        }
    }
}

fn state() -> &'static Mutex<Option<AuthorizationSession>> {
    STATE.get_or_init(|| Mutex::new(None))
}
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}
fn lock_error<T>(_: T) -> String {
    "QQ 云账号正在使用，请稍后重试".into()
}

impl Pending {
    fn new() -> Result<Self, String> {
        let key = RsaKey::new()?;
        let der = key.public_der()?;
        let fingerprint = Sha256::digest(&der)[..6]
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect();
        let mut id = [0u8; 16];
        random(&mut id)?;
        Ok(Self {
            id: id.iter().map(|b| format!("{b:02x}")).collect(),
            expires: now_ms() + WINDOW_MS,
            deadline: Instant::now() + Duration::from_millis(WINDOW_MS),
            key,
            public: STANDARD.encode(der),
            fingerprint,
            peer: None,
        })
    }

    fn expired(&self) -> bool {
        self.expires <= now_ms() || Instant::now() >= self.deadline
    }

    fn descriptor(&self) -> Value {
        json!({"format":"zsclip-qq-cloud-request-v1", "request_id":self.id,
            "public_key":self.public,"fingerprint":self.fingerprint,"expires_at_ms":self.expires})
    }

    fn decrypt(&self, peer: &str, body: &Value) -> Result<(CloudAccount, String), String> {
        if self.expired()
            || body["format"].as_str() != Some(FORMAT)
            || body["request_id"].as_str() != Some(self.id.as_str())
        {
            return Err(INVALID.into());
        }
        let decode = |name: &str| -> Result<Vec<u8>, String> {
            STANDARD
                .decode(body[name].as_str().ok_or(INVALID)?)
                .map_err(|_| INVALID.into())
        };
        let wrapped = decode("wrapped_key")?;
        let nonce = decode("nonce")?;
        let ciphertext = decode("ciphertext")?;
        if wrapped.len() != 256
            || nonce.len() != 12
            || ciphertext.len() < 16
            || ciphertext.len() > 16384
        {
            return Err(INVALID.into());
        }
        let mut secret = self.key.decrypt(&wrapped)?;
        if secret.len() != 32 {
            secret.fill(0);
            return Err(INVALID.into());
        }
        let cipher = Aes256Gcm::new_from_slice(&secret).map_err(|_| INVALID.to_string())?;
        secret.fill(0);
        let aad = format!("{FORMAT}:{}:{peer}", self.id);
        let mut plaintext = cipher
            .decrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &ciphertext,
                    aad: aad.as_bytes(),
                },
            )
            .map_err(|_| INVALID.to_string())?;
        let result =
            serde_json::from_slice::<CloudAccount>(&plaintext).map_err(|_| INVALID.to_string());
        plaintext.fill(0);
        let account = result?;
        account.validate()?;
        let mut digest = Sha256::new();
        digest.update(&wrapped);
        digest.update(&nonce);
        digest.update(&ciphertext);
        let confirmation_code = digest.finalize()[..6]
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect();
        Ok((account, confirmation_code))
    }
}

pub(crate) fn begin_authorization() -> Result<AuthorizationWindow, String> {
    let pending = Pending::new()?;
    let code = pending
        .fingerprint
        .as_bytes()
        .chunks(4)
        .map(|s| String::from_utf8_lossy(s).to_uppercase())
        .collect::<Vec<_>>()
        .join(" ");
    let request_id = pending.id.clone();
    let mut slot = state().lock().map_err(lock_error)?;
    *slot = Some(AuthorizationSession::Pending(pending));
    *LAST_ACCEPTED.lock().map_err(lock_error)? = None;
    Ok(AuthorizationWindow { code, request_id })
}

pub(crate) fn authorization_completed(id: &str) -> bool {
    LAST_ACCEPTED
        .lock()
        .map(|v| v.as_deref() == Some(id))
        .unwrap_or(false)
}

pub(crate) fn cancel_authorization() {
    if let Ok(mut pending) = state().lock() {
        *pending = None;
    }
}

pub(crate) fn authorization_descriptor(peer: &str) -> Result<Value, String> {
    let mut slot = state().lock().map_err(lock_error)?;
    if slot.as_ref().is_some_and(AuthorizationSession::expired) {
        *slot = None;
    }
    let pending = match slot.as_mut() {
        Some(AuthorizationSession::Pending(pending)) => pending,
        Some(AuthorizationSession::Candidate(_)) => {
            return Err("手机授权已提交，请在电脑核对确认码".into())
        }
        None => return Err("请先在电脑右键菜单中选择“连接 QQ 云剪贴板”".into()),
    };
    if pending.peer.as_deref().is_some_and(|id| id != peer) {
        return Err("本次授权已由另一台手机领取，请重新发起连接".into());
    }
    pending.peer = Some(peer.to_string());
    Ok(pending.descriptor())
}

pub(crate) fn accept_authorization(peer: &str, body: &[u8]) -> Result<String, String> {
    if body.len() > MAX_BODY {
        return Err(INVALID.into());
    }
    let envelope: Value = serde_json::from_slice(body).map_err(|_| INVALID.to_string())?;
    let mut slot = state().lock().map_err(lock_error)?;
    let current = match slot.as_ref() {
        Some(AuthorizationSession::Pending(pending)) => pending,
        _ => return Err("授权窗口已关闭，请在电脑重新发起连接".into()),
    };
    if current.id != envelope["request_id"].as_str().unwrap_or_default()
        || current.peer.as_deref() != Some(peer)
    {
        return Err(INVALID.into());
    }
    // Each key accepts only one attempt, limiting decryption-oracle exposure and replays.
    let pending = match slot.take() {
        Some(AuthorizationSession::Pending(pending)) => pending,
        _ => return Err(INVALID.into()),
    };
    let (account, confirmation_code) = pending.decrypt(peer, &envelope)?;
    *slot = Some(AuthorizationSession::Candidate(Candidate {
        id: pending.id,
        expires: pending.expires,
        deadline: pending.deadline,
        confirmation_code: confirmation_code.clone(),
        account,
    }));
    Ok(confirmation_code)
}

pub(crate) fn pending_confirmation(request_id: &str) -> Result<String, String> {
    let mut slot = state().lock().map_err(lock_error)?;
    if slot.as_ref().is_some_and(AuthorizationSession::expired) {
        *slot = None;
    }
    match slot.as_ref() {
        Some(AuthorizationSession::Candidate(candidate)) if candidate.id == request_id => {
            Ok(candidate.confirmation_code.clone())
        }
        _ => Err("尚未收到有效的手机授权，或授权窗口已过期，请重新发起连接".into()),
    }
}

/// Called only after the user compares the phone's locally calculated envelope
/// code with the desktop code. Receiving an authenticated LAN POST alone must
/// never replace the stored cloud account.
pub(crate) fn confirm_authorization(
    request_id: &str,
    confirmation_code: &str,
) -> Result<(), String> {
    let mut slot = state().lock().map_err(lock_error)?;
    match slot.as_ref() {
        Some(AuthorizationSession::Candidate(candidate)) if candidate.id == request_id => {}
        _ => return Err("没有可确认的手机授权，请重新发起连接".into()),
    }
    // A matching request gets one confirmation attempt. Wrong codes, expiry and
    // storage failures require a new handoff and cannot silently retry a key.
    let candidate = match slot.take() {
        Some(AuthorizationSession::Candidate(candidate)) => candidate,
        _ => return Err(INVALID.into()),
    };
    candidate.check_confirmation(request_id, confirmation_code)?;
    save_account(&candidate.account)?;
    *LAST_ACCEPTED.lock().map_err(lock_error)? = Some(candidate.id.clone());
    Ok(())
}

fn account_path() -> std::path::PathBuf {
    crate::app::data_dir().join("qq_cloud_account.json")
}

pub(crate) fn load_account() -> Result<CloudAccount, String> {
    let _guard = STORE_LOCK.lock().map_err(lock_error)?;
    let file =
        fs::File::open(account_path()).map_err(|_| "请先连接 QQ 云剪贴板账号".to_string())?;
    let mut bytes = Vec::new();
    file.take(65537)
        .read_to_end(&mut bytes)
        .map_err(|_| "无法读取账号，请重新授权".to_string())?;
    if bytes.len() > 65536 {
        return Err("账号文件无效，请重新授权".into());
    }
    let stored: StoredAccount =
        serde_json::from_slice(&bytes).map_err(|_| "账号文件无效，请重新授权".to_string())?;
    if stored.format != 1 {
        return Err("账号文件版本不支持，请重新授权".into());
    }
    let mut clear =
        crate::platform::secret_store::decrypt_secret_from_storage(&stored.encrypted_account)
            .ok_or("无法解密账号，请在当前 Windows 用户下重新授权")?;
    let account = serde_json::from_str::<CloudAccount>(&clear)
        .map_err(|_| "账号文件无效，请重新授权".to_string());
    // Keep credential text out of diagnostics and release the temporary copy promptly.
    unsafe {
        clear.as_bytes_mut().fill(0);
    }
    let account = account?;
    account.validate()?;
    Ok(account)
}

fn save_account(account: &CloudAccount) -> Result<(), String> {
    let _guard = STORE_LOCK.lock().map_err(lock_error)?;
    let mut clear = serde_json::to_string(account).map_err(|_| INVALID.to_string())?;
    let encrypted = crate::platform::secret_store::encrypt_secret_for_storage(&clear);
    unsafe {
        clear.as_bytes_mut().fill(0);
    }
    let stored = StoredAccount {
        format: 1,
        encrypted_account: encrypted.ok_or("无法安全保存账号")?,
    };
    let path = account_path();
    let parent = path.parent().ok_or("账号保存目录无效")?;
    fs::create_dir_all(parent).map_err(|_| "无法创建账号保存目录")?;
    let mut salt = [0u8; 8];
    random(&mut salt)?;
    let suffix: String = salt.iter().map(|b| format!("{b:02x}")).collect();
    let temp = parent.join(format!(".qq_cloud_account.{suffix}.tmp"));
    // Only a file successfully created by this attempt may be cleaned up.
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp)
        .map_err(|_| "无法创建账号文件")?;
    let result = (|| -> Result<(), String> {
        let data = serde_json::to_vec(&stored).map_err(|_| "账号序列化失败")?;
        file.write_all(&data)
            .and_then(|_| file.sync_all())
            .map_err(|_| "无法保存账号文件")?;
        drop(file);
        let from: Vec<u16> = temp.as_os_str().encode_wide().chain([0]).collect();
        let to: Vec<u16> = path.as_os_str().encode_wide().chain([0]).collect();
        if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 0x1 | 0x8) } == 0 {
            return Err("无法替换账号文件".into());
        }
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

pub(crate) fn disconnect() -> Result<(), String> {
    // Same lock order as confirmation: session, persistent store, completion.
    let mut pending = state().lock().map_err(lock_error)?;
    *pending = None;
    let _guard = STORE_LOCK.lock().map_err(lock_error)?;
    *LAST_ACCEPTED.lock().map_err(lock_error)? = None;
    match fs::remove_file(account_path()) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(_) => Err("无法移除当前电脑的账号授权".into()),
    }
}

fn wide(value: &str) -> Vec<u16> {
    value.encode_utf16().chain(Some(0)).collect()
}
fn random(bytes: &mut [u8]) -> Result<(), String> {
    if unsafe { BCryptGenRandom(null_mut(), bytes.as_mut_ptr(), bytes.len() as u32, 2) } < 0 {
        Err("系统随机数生成失败".into())
    } else {
        Ok(())
    }
}

struct RsaKey {
    provider: *mut c_void,
    key: *mut c_void,
}
// CNG handles are owned by this object; access is serialized through STATE.
unsafe impl Send for RsaKey {}
impl Drop for RsaKey {
    fn drop(&mut self) {
        unsafe {
            if !self.key.is_null() {
                BCryptDestroyKey(self.key);
            }
            if !self.provider.is_null() {
                BCryptCloseAlgorithmProvider(self.provider, 0);
            }
        }
    }
}
impl RsaKey {
    fn new() -> Result<Self, String> {
        let mut value = Self {
            provider: null_mut(),
            key: null_mut(),
        };
        let algorithm = wide("RSA");
        unsafe {
            if BCryptOpenAlgorithmProvider(&mut value.provider, algorithm.as_ptr(), null(), 0) < 0
                || BCryptGenerateKeyPair(value.provider, &mut value.key, 2048, 0) < 0
                || BCryptFinalizeKeyPair(value.key, 0) < 0
            {
                return Err("无法创建安全授权密钥".into());
            }
        }
        Ok(value)
    }
    fn public_der(&self) -> Result<Vec<u8>, String> {
        let kind = wide("RSAPUBLICBLOB");
        let mut size = 0;
        unsafe {
            if BCryptExportKey(
                self.key,
                null_mut(),
                kind.as_ptr(),
                null_mut(),
                0,
                &mut size,
                0,
            ) < 0
            {
                return Err(INVALID.into());
            }
        }
        let mut blob = vec![0u8; size as usize];
        unsafe {
            if BCryptExportKey(
                self.key,
                null_mut(),
                kind.as_ptr(),
                blob.as_mut_ptr(),
                size,
                &mut size,
                0,
            ) < 0
            {
                return Err(INVALID.into());
            }
        }
        if blob.len() < 24 {
            return Err(INVALID.into());
        }
        let word = |at| u32::from_le_bytes(blob[at..at + 4].try_into().unwrap());
        let exponent = word(8) as usize;
        let modulus = word(12) as usize;
        if word(0) != 0x31415352
            || word(4) != 2048
            || exponent > 8
            || modulus != 256
            || 24 + exponent + modulus != blob.len()
        {
            return Err(INVALID.into());
        }
        let mut content = integer(&blob[24 + exponent..]);
        content.extend(integer(&blob[24..24 + exponent]));
        let key = der(0x30, &content);
        let algorithm = [
            0x30, 0x0d, 0x06, 0x09, 0x2a, 0x86, 0x48, 0x86, 0xf7, 0x0d, 0x01, 0x01, 0x01, 0x05,
            0x00,
        ];
        let mut bits = vec![0];
        bits.extend(key);
        let mut spki = algorithm.to_vec();
        spki.extend(der(3, &bits));
        Ok(der(0x30, &spki))
    }
    fn decrypt(&self, ciphertext: &[u8]) -> Result<Vec<u8>, String> {
        let digest = wide("SHA256");
        let padding = OaepPadding {
            algorithm: digest.as_ptr(),
            label: null_mut(),
            label_size: 0,
        };
        let mut result = vec![0u8; 256];
        let mut size = 0;
        let code = unsafe {
            BCryptDecrypt(
                self.key,
                ciphertext.as_ptr(),
                ciphertext.len() as u32,
                &padding as *const _ as *const c_void,
                null_mut(),
                0,
                result.as_mut_ptr(),
                result.len() as u32,
                &mut size,
                4,
            )
        };
        if code < 0 || size > 256 {
            result.fill(0);
            return Err(INVALID.into());
        }
        result.truncate(size as usize);
        Ok(result)
    }
}

fn der(tag: u8, value: &[u8]) -> Vec<u8> {
    let mut result = vec![tag];
    if value.len() < 128 {
        result.push(value.len() as u8);
    } else if value.len() <= 255 {
        result.extend([0x81, value.len() as u8]);
    } else {
        result.extend([0x82, (value.len() >> 8) as u8, value.len() as u8]);
    }
    result.extend(value);
    result
}
fn integer(value: &[u8]) -> Vec<u8> {
    let mut bytes = Vec::new();
    if value.first().is_some_and(|b| b & 0x80 != 0) {
        bytes.push(0);
    }
    bytes.extend(value);
    der(2, &bytes)
}
#[repr(C)]
struct OaepPadding {
    algorithm: *const u16,
    label: *mut u8,
    label_size: u32,
}
#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptOpenAlgorithmProvider(
        result: *mut *mut c_void,
        algorithm: *const u16,
        implementation: *const u16,
        flags: u32,
    ) -> i32;
    fn BCryptCloseAlgorithmProvider(handle: *mut c_void, flags: u32) -> i32;
    fn BCryptGenerateKeyPair(
        provider: *mut c_void,
        key: *mut *mut c_void,
        length: u32,
        flags: u32,
    ) -> i32;
    fn BCryptFinalizeKeyPair(key: *mut c_void, flags: u32) -> i32;
    fn BCryptDestroyKey(key: *mut c_void) -> i32;
    fn BCryptExportKey(
        key: *mut c_void,
        export: *mut c_void,
        kind: *const u16,
        output: *mut u8,
        len: u32,
        result: *mut u32,
        flags: u32,
    ) -> i32;
    fn BCryptDecrypt(
        key: *mut c_void,
        input: *const u8,
        len: u32,
        padding: *const c_void,
        iv: *mut u8,
        iv_len: u32,
        output: *mut u8,
        output_len: u32,
        result: *mut u32,
        flags: u32,
    ) -> i32;
    fn BCryptGenRandom(provider: *mut c_void, output: *mut u8, len: u32, flags: u32) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn authorization_key_is_fresh_and_expiring() {
        let one = Pending::new().unwrap();
        let two = Pending::new().unwrap();
        assert_ne!(one.public, two.public);
        assert_ne!(one.id, two.id);
        assert_eq!(one.fingerprint.len(), 12);
        assert!(one.expires > now_ms());
        assert_eq!(STANDARD.decode(one.public).unwrap()[0], 0x30);
        assert_eq!(two.descriptor()["format"], "zsclip-qq-cloud-request-v1");
    }
    #[test]
    fn wrong_request_and_invalid_ciphertext_are_rejected() {
        let pending = Pending::new().unwrap();
        assert!(pending
            .decrypt("device", &json!({"format":FORMAT,"request_id":"other"}))
            .is_err());
        assert!(pending.decrypt("device", &json!({"format":FORMAT,"request_id":pending.id,"wrapped_key":"bad","nonce":"bad","ciphertext":"bad"})).is_err());
    }

    #[test]
    fn monotonic_deadline_expires_even_when_wall_clock_expiry_is_future() {
        let mut pending = Pending::new().unwrap();
        pending.expires = now_ms() + WINDOW_MS * 5;
        pending.deadline = Instant::now();
        assert!(pending.expired());
        assert!(pending
            .decrypt("device", &json!({"format":FORMAT,"request_id":pending.id}))
            .is_err());
    }

    #[test]
    fn staged_candidate_requires_matching_code_and_original_deadline() {
        let mut candidate = Candidate {
            id: "fixture-request".into(),
            expires: now_ms() + WINDOW_MS,
            deadline: Instant::now() + Duration::from_millis(WINDOW_MS),
            confirmation_code: "0123456789ab".into(),
            account: CloudAccount {
                sgid: "fixture-account".into(),
                device_id: "fixture-device".into(),
                version: "8.7.15.6291".into(),
            },
        };
        assert!(candidate
            .check_confirmation("fixture-request", "0123456789ab")
            .is_ok());
        assert!(candidate
            .check_confirmation("other-request", "0123456789ab")
            .is_err());
        assert!(candidate
            .check_confirmation("fixture-request", "000000000000")
            .is_err());
        candidate.deadline = Instant::now();
        assert!(candidate
            .check_confirmation("fixture-request", "0123456789ab")
            .is_err());
        candidate.deadline = Instant::now() + Duration::from_millis(WINDOW_MS);
        candidate.expires = now_ms();
        assert!(candidate
            .check_confirmation("fixture-request", "0123456789ab")
            .is_err());
    }
}
