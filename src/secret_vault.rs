//! Local encrypted password/key storage, separate from clipboard history and sync.
use aes_gcm::{
    aead::{Aead, Payload},
    Aes256Gcm, KeyInit, Nonce,
};
use argon2::{Algorithm, Argon2, Params, Version};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use hmac::{Hmac, Mac};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::{
    collections::HashSet,
    ffi::c_void,
    fs,
    io::{Read, Write},
    os::windows::{ffi::OsStrExt, fs::OpenOptionsExt},
    path::{Path, PathBuf},
    ptr::null_mut,
    sync::{Mutex, OnceLock},
    time::SystemTime,
};
use zeroize::{Zeroize, Zeroizing};

const MAX_FILE: u64 = 16 * 1024 * 1024;
const MAX_SECRET: usize = 65536;
const INVALID: &str = "密码库无法读取，请检查文件和当前 Windows 用户；文本记录已暂停以保护敏感内容";
static WRITES: Mutex<()> = Mutex::new(());
static MATCHER: OnceLock<Mutex<Option<Matcher>>> = OnceLock::new();

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub(crate) enum VaultKind {
    Password,
    Key,
}

#[derive(Clone)]
pub(crate) struct EntrySummary {
    pub(crate) id: String,
    pub(crate) kind: VaultKind,
    pub(crate) label: String,
    pub(crate) notes: String,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    kind: VaultKind,
    label: String,
    notes: String,
    value: String,
}
impl Drop for Entry {
    fn drop(&mut self) {
        self.value.zeroize();
        self.label.zeroize();
        self.notes.zeroize();
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Document {
    format: u32,
    id: String,
    revision: u64,
    password: bool,
    key_dpapi: String,
    salt: String,
    nonce: String,
    ciphertext: String,
    filter_key_dpapi: String,
    blocked: Vec<String>,
    mac: String,
}

pub(crate) struct VaultSession {
    path: PathBuf,
    document: Document,
    key: Zeroizing<[u8; 32]>,
    filter_key: Zeroizing<[u8; 32]>,
    entries: Vec<Entry>,
}

#[derive(PartialEq, Eq)]
enum FileStamp {
    Missing,
    Present(Option<SystemTime>, u64),
    Unavailable,
}
#[derive(PartialEq, Eq)]
struct StoreStamp {
    document: FileStamp,
    presence: FileStamp,
}
struct Matcher {
    path: PathBuf,
    stamp: StoreStamp,
    key: Option<Zeroizing<[u8; 32]>>,
    blocked: HashSet<String>,
    failed: bool,
}

/// A verified, immutable exclusion set for destructive cleanup of known secrets only.
pub(crate) struct ExclusionSnapshot {
    key: Option<Zeroizing<[u8; 32]>>,
    tags: HashSet<String>,
}

impl ExclusionSnapshot {
    pub(crate) fn matches(&self, text: &str) -> bool {
        !text.is_empty()
            && self
                .key
                .as_ref()
                .is_some_and(|key| self.tags.contains(&fingerprint(key.as_ref(), text)))
    }
}

impl Drop for ExclusionSnapshot {
    fn drop(&mut self) {
        for mut tag in self.tags.drain() {
            tag.zeroize();
        }
    }
}

fn path() -> PathBuf {
    crate::app::data_dir().join("protected").join("vault.json")
}
fn error<T>(_: T) -> String {
    INVALID.to_string()
}
fn file_stamp(file: &Path) -> FileStamp {
    match fs::metadata(file) {
        Ok(metadata) => FileStamp::Present(metadata.modified().ok(), metadata.len()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => FileStamp::Missing,
        Err(_) => FileStamp::Unavailable,
    }
}
fn invalidate_matcher() {
    if let Some(cache) = MATCHER.get() {
        if let Ok(mut cache) = cache.lock() {
            *cache = None;
        }
    }
}
// The persistent lock file is never deleted; the OS releases its exclusive handle on exit.
fn lock_store(file: &Path) -> Result<fs::File, String> {
    let dir = file.parent().ok_or(INVALID)?;
    fs::create_dir_all(dir).map_err(|_| "无法创建密码库目录")?;
    fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .share_mode(0)
        .open(file.with_extension("lock"))
        .map_err(|e| match e.raw_os_error() {
            Some(32 | 33) => "密码库正被其他窗口使用，请稍后重试".into(),
            _ => "无法锁定密码库文件，请检查目录权限".into(),
        })
}
fn random<const N: usize>() -> Result<[u8; N], String> {
    let mut out = [0; N];
    if unsafe { BCryptGenRandom(null_mut(), out.as_mut_ptr(), N as u32, 2) } < 0 {
        return Err("系统随机数生成失败".into());
    }
    Ok(out)
}
fn random_id() -> Result<String, String> {
    Ok(random::<16>()?.iter().map(|b| format!("{b:02x}")).collect())
}
fn protect(key: &[u8; 32]) -> Result<String, String> {
    let encoded = Zeroizing::new(STANDARD.encode(key));
    crate::platform::secret_store::encrypt_secret_for_storage(&encoded)
        .ok_or_else(|| "无法安全保存密码库密钥".into())
}
fn unprotect(value: &str) -> Result<Zeroizing<[u8; 32]>, String> {
    let encoded = Zeroizing::new(
        crate::platform::secret_store::decrypt_secret_from_storage(value).ok_or(INVALID)?,
    );
    let bytes = Zeroizing::new(STANDARD.decode(encoded.as_bytes()).map_err(error)?);
    Ok(Zeroizing::new(bytes.as_slice().try_into().map_err(error)?))
}
fn password_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; 32]>, String> {
    if password.is_empty() || password.len() > 1024 || password.contains('\0') {
        return Err("管理密码不能为空，且不能超过 1024 字节".into());
    }
    let mut key = Zeroizing::new([0; 32]);
    let params = Params::new(19456, 2, 1, Some(32)).map_err(error)?;
    Argon2::new(Algorithm::Argon2id, Version::V0x13, params)
        .hash_password_into(password.as_bytes(), salt, key.as_mut())
        .map_err(error)?;
    Ok(key)
}
fn canonical(value: &str) -> Zeroizing<String> {
    Zeroizing::new(
        value
            .replace("\r\n", "\n")
            .replace('\r', "\n")
            .trim_matches('\0')
            .trim()
            .to_string(),
    )
}
fn fingerprint(key: &[u8], value: &str) -> String {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).expect("HMAC accepts all key lengths");
    mac.update(b"ZSClip secret exclusion v1\0");
    mac.update(canonical(value).as_bytes());
    STANDARD.encode(mac.finalize().into_bytes())
}
fn active_exclusions(entries: &[Entry], key: &[u8]) -> Vec<String> {
    let mut tags: Vec<String> = entries
        .iter()
        .map(|entry| fingerprint(key, &entry.value))
        .collect();
    tags.sort_unstable();
    tags.dedup();
    tags
}
fn authenticated_bytes(doc: &Document) -> Result<Vec<u8>, String> {
    serde_json::to_vec(&(
        doc.format,
        &doc.id,
        doc.revision,
        doc.password,
        &doc.key_dpapi,
        &doc.salt,
        &doc.nonce,
        &doc.ciphertext,
        &doc.filter_key_dpapi,
        &doc.blocked,
    ))
    .map_err(error)
}
fn sign_document(doc: &mut Document, key: &[u8; 32]) -> Result<(), String> {
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key).map_err(error)?;
    mac.update(b"ZSClip secret metadata v1\0");
    mac.update(&authenticated_bytes(doc)?);
    doc.mac = STANDARD.encode(mac.finalize().into_bytes());
    Ok(())
}
fn read_document(file: &Path) -> Result<Option<(Document, Zeroizing<[u8; 32]>)>, String> {
    let handle = match fs::File::open(file) {
        Ok(handle) => handle,
        Err(e)
            if e.kind() == std::io::ErrorKind::NotFound
                && file_stamp(&file.with_extension("presence")) == FileStamp::Missing =>
        {
            return Ok(None)
        }
        Err(_) => return Err(INVALID.into()),
    };
    let mut bytes = Vec::new();
    handle
        .take(MAX_FILE + 1)
        .read_to_end(&mut bytes)
        .map_err(error)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err(INVALID.into());
    }
    let doc: Document = serde_json::from_slice(&bytes).map_err(error)?;
    if doc.format != 1
        || doc.id.len() != 32
        || doc.blocked.len() > 20000
        || doc.blocked.iter().any(|s| s.len() != 44)
        || doc.filter_key_dpapi.len() > 8192
        || doc.nonce.len() > 32
        || doc.salt.len() > 64
        || doc.key_dpapi.len() > 8192
    {
        return Err(INVALID.into());
    }
    let key = unprotect(&doc.filter_key_dpapi)?;
    let mut mac = <Hmac<Sha256> as Mac>::new_from_slice(key.as_ref()).map_err(error)?;
    mac.update(b"ZSClip secret metadata v1\0");
    mac.update(&authenticated_bytes(&doc)?);
    mac.verify_slice(&STANDARD.decode(&doc.mac).map_err(error)?)
        .map_err(error)?;
    Ok(Some((doc, key)))
}
fn aad(doc: &Document) -> Vec<u8> {
    format!(
        "ZSClip-vault-v1:{}:{}:{}",
        doc.id, doc.revision, doc.password
    )
    .into_bytes()
}
fn encrypt(
    doc: &mut Document,
    entries: &[Entry],
    key: &[u8; 32],
    filter: &[u8; 32],
) -> Result<(), String> {
    let clear = Zeroizing::new(serde_json::to_vec(entries).map_err(error)?);
    if clear.len() > 10 * 1024 * 1024 {
        return Err("密码库容量已达到本地上限，请减少条目或密钥长度".into());
    }
    let nonce = random::<12>()?;
    let cipher = Aes256Gcm::new_from_slice(key).map_err(error)?;
    doc.ciphertext = STANDARD.encode(
        cipher
            .encrypt(
                Nonce::from_slice(&nonce),
                Payload {
                    msg: &clear,
                    aad: &aad(doc),
                },
            )
            .map_err(error)?,
    );
    doc.nonce = STANDARD.encode(nonce);
    sign_document(doc, filter)
}
fn write_document(file: &Path, doc: &Document) -> Result<(), String> {
    let result = write_document_locked(file, doc, replace_document);
    // Failed initialization can create/remove a presence marker, so invalidate on every path.
    invalidate_matcher();
    result
}
fn replace_document(from: &Path, to: &Path) -> Result<(), String> {
    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
    if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
        return Err("无法替换密码库文件".into());
    }
    Ok(())
}
// Callers hold both WRITES and lock_store throughout read-current/encrypt/replace.
fn write_document_locked(
    file: &Path,
    doc: &Document,
    replace: impl FnOnce(&Path, &Path) -> Result<(), String>,
) -> Result<(), String> {
    let bytes = serde_json::to_vec(doc).map_err(error)?;
    if bytes.len() as u64 > MAX_FILE {
        return Err("密码库文件超过本地大小上限".into());
    }
    let dir = file.parent().ok_or(INVALID)?;
    fs::create_dir_all(dir).map_err(|_| "无法创建密码库目录")?;
    let presence = file.with_extension("presence");
    let initially_missing = file_stamp(file) == FileStamp::Missing;
    let mut created_presence = false;
    let mut temporary = None;
    let result = (|| -> Result<(), String> {
        // Finish every fallible marker operation before replacing the encrypted document.
        match fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&presence)
        {
            Ok(mut marker) => {
                created_presence = true;
                marker
                    .write_all(b"ZSClip protected storage v1")
                    .and_then(|_| marker.sync_all())
                    .map_err(|_| "无法保存密码库保护标记")?;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
                if !fs::metadata(&presence).map_err(error)?.is_file() {
                    return Err(INVALID.into());
                }
            }
            Err(_) => return Err("无法保存密码库保护标记".into()),
        }
        let tmp = dir.join(format!(".vault-{}.tmp", random_id()?));
        let mut out = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)
            .map_err(|_| "无法创建密码库暂存文件")?;
        temporary = Some(tmp.clone());
        out.write_all(&bytes)
            .and_then(|_| out.sync_all())
            .map_err(|_| "无法保存密码库")?;
        drop(out);
        replace(&tmp, file)
    })();
    if result.is_err() {
        if let Some(tmp) = temporary {
            let _ = fs::remove_file(tmp);
        }
        // Only undo a marker created by this attempt, and only if no vault ever existed.
        if created_presence && initially_missing && file_stamp(file) == FileStamp::Missing {
            let _ = fs::remove_file(&presence);
        }
    }
    result
}

pub(crate) fn has_password() -> Result<bool, String> {
    Ok(read_document(&path())?.is_some_and(|(d, _)| d.password))
}

/// Public metadata only, used to invalidate an unsent clipboard backup after registration.
pub(crate) fn exclusion_revision() -> Result<String, String> {
    Ok(read_document(&path())?
        .map(|(document, _)| format!("{}:{}", document.id, document.revision))
        .unwrap_or_default())
}
pub(crate) fn exclusion_snapshot() -> Result<ExclusionSnapshot, String> {
    exclusion_snapshot_at(&path())
}
fn exclusion_snapshot_at(file: &Path) -> Result<ExclusionSnapshot, String> {
    match read_document(file)? {
        Some((document, key)) => Ok(ExclusionSnapshot {
            key: Some(key),
            tags: document.blocked.into_iter().collect(),
        }),
        None => Ok(ExclusionSnapshot {
            key: None,
            tags: HashSet::new(),
        }),
    }
}
pub(crate) fn unlock(password: Option<&str>) -> Result<VaultSession, String> {
    VaultSession::open(path(), password)
}

impl VaultSession {
    fn open(file: PathBuf, password: Option<&str>) -> Result<Self, String> {
        let _guard = WRITES.lock().map_err(error)?;
        let _file_guard = lock_store(&file)?;
        let (document, key, filter_key, entries) =
            if let Some((mut doc, filter)) = read_document(&file)? {
                let key = if doc.password {
                    let salt = STANDARD.decode(&doc.salt).map_err(error)?;
                    if salt.len() != 32 {
                        return Err(INVALID.into());
                    }
                    password_key(password.ok_or("请输入管理密码")?, &salt)?
                } else {
                    unprotect(&doc.key_dpapi)?
                };
                let nonce = STANDARD.decode(&doc.nonce).map_err(error)?;
                if nonce.len() != 12 {
                    return Err(INVALID.into());
                }
                let cipher = Aes256Gcm::new_from_slice(key.as_ref()).map_err(error)?;
                let encrypted = STANDARD.decode(&doc.ciphertext).map_err(error)?;
                let clear = Zeroizing::new(
                    cipher
                        .decrypt(
                            Nonce::from_slice(&nonce),
                            Payload {
                                msg: &encrypted,
                                aad: &aad(&doc),
                            },
                        )
                        .map_err(|_| "管理密码不正确，或密码库内容已损坏".to_string())?,
                );
                let entries: Vec<Entry> = serde_json::from_slice(&clear).map_err(error)?;
                if entries.len() > 2000
                    || entries.iter().any(|e| {
                        e.value.len() > MAX_SECRET || e.label.len() > 1024 || e.notes.len() > 8192
                    })
                {
                    return Err(INVALID.into());
                }
                // Older stores retained deleted/edited values. Only authenticated, decrypted
                // entries can establish which exclusions remain active.
                let blocked = active_exclusions(&entries, filter.as_ref());
                if doc.blocked != blocked {
                    doc.blocked = blocked;
                    doc.revision = doc.revision.checked_add(1).ok_or(INVALID)?;
                    encrypt(&mut doc, &entries, &key, &filter)?;
                    write_document(&file, &doc)?;
                }
                (doc, key, filter, entries)
            } else {
                let key = Zeroizing::new(random::<32>()?);
                let filter = Zeroizing::new(random::<32>()?);
                let mut doc = Document {
                    format: 1,
                    id: random_id()?,
                    revision: 0,
                    password: false,
                    key_dpapi: protect(&key)?,
                    salt: String::new(),
                    nonce: String::new(),
                    ciphertext: String::new(),
                    filter_key_dpapi: protect(&filter)?,
                    blocked: Vec::new(),
                    mac: String::new(),
                };
                encrypt(&mut doc, &[], &key, &filter)?;
                write_document(&file, &doc)?;
                (doc, key, filter, Vec::new())
            };
        Ok(Self {
            path: file,
            document,
            key,
            filter_key,
            entries,
        })
    }
    pub(crate) fn list(&self, kind: VaultKind) -> Vec<EntrySummary> {
        self.list_all()
            .into_iter()
            .filter(|entry| entry.kind == kind)
            .collect()
    }
    pub(crate) fn list_all(&self) -> Vec<EntrySummary> {
        self.entries
            .iter()
            .map(|e| EntrySummary {
                id: e.id.clone(),
                kind: e.kind,
                label: e.label.clone(),
                notes: e.notes.clone(),
            })
            .collect()
    }
    pub(crate) fn password_required(&self) -> bool {
        self.document.password
    }
    pub(crate) fn upsert_entry(
        &mut self,
        id: Option<&str>,
        label: &str,
        notes: &str,
        value: &str,
    ) -> Result<String, String> {
        // Retain the old serialized field for compatibility, without exposing a category.
        let kind = id
            .and_then(|id| self.entries.iter().find(|entry| entry.id == id))
            .map_or(VaultKind::Password, |entry| entry.kind);
        self.upsert(id, kind, label, notes, value)
    }
    pub(crate) fn secret(&self, id: &str) -> Result<Zeroizing<String>, String> {
        self.entries
            .iter()
            .find(|e| e.id == id)
            .map(|e| Zeroizing::new(e.value.clone()))
            .ok_or_else(|| "该条目不存在".into())
    }
    pub(crate) fn upsert(
        &mut self,
        id: Option<&str>,
        kind: VaultKind,
        label: &str,
        notes: &str,
        value: &str,
    ) -> Result<String, String> {
        if label.trim().is_empty() || label.len() > 1024 || notes.len() > 8192 {
            return Err("请填写名称（最多1024字节），备注最多8192字节".into());
        }
        if value.is_empty() || value.len() > MAX_SECRET || value.contains('\0') {
            return Err("密码或密钥不能为空、不能含空字符，且最多64 KB".into());
        }
        let mut next = self.entries.clone();
        let id = if let Some(id) = id {
            let entry = next.iter_mut().find(|e| e.id == id).ok_or("该条目不存在")?;
            entry.label.zeroize();
            entry.notes.zeroize();
            entry.value.zeroize();
            entry.label = label.trim().into();
            entry.notes = notes.into();
            entry.value = value.into();
            entry.kind = kind;
            id.to_string()
        } else {
            if next.len() >= 2000 {
                return Err("密码库最多保存2000个条目".into());
            }
            let id = random_id()?;
            next.push(Entry {
                id: id.clone(),
                kind,
                label: label.trim().into(),
                notes: notes.into(),
                value: value.into(),
            });
            id
        };
        self.commit(self.document.clone(), next, None)?;
        Ok(id)
    }
    pub(crate) fn remove(&mut self, id: &str) -> Result<(), String> {
        if !self.entries.iter().any(|e| e.id == id) {
            return Err("该条目不存在".into());
        }
        let mut next = self.entries.clone();
        next.retain(|e| e.id != id);
        self.commit(self.document.clone(), next, None)
    }
    pub(crate) fn set_password(&mut self, password: Option<&str>) -> Result<(), String> {
        let mut doc = self.document.clone();
        let key = if let Some(password) = password {
            let salt = random::<32>()?;
            doc.password = true;
            doc.salt = STANDARD.encode(salt);
            doc.key_dpapi.clear();
            password_key(password, &salt)?
        } else {
            let key = Zeroizing::new(random::<32>()?);
            doc.password = false;
            doc.salt.clear();
            doc.key_dpapi = protect(&key)?;
            key
        };
        self.commit(doc, self.entries.clone(), Some(key))
    }
    fn commit(
        &mut self,
        mut doc: Document,
        entries: Vec<Entry>,
        new_key: Option<Zeroizing<[u8; 32]>>,
    ) -> Result<(), String> {
        let _guard = WRITES.lock().map_err(error)?;
        let _file_guard = lock_store(&self.path)?;
        let current = read_document(&self.path)?.ok_or(INVALID)?.0;
        if current.id != self.document.id || current.revision != self.document.revision {
            return Err("密码库已被其他窗口修改，请关闭后重新打开".into());
        }
        doc.blocked = active_exclusions(&entries, self.filter_key.as_ref());
        doc.revision = current.revision.checked_add(1).ok_or(INVALID)?;
        encrypt(
            &mut doc,
            &entries,
            new_key.as_deref().unwrap_or(&self.key),
            &self.filter_key,
        )?;
        write_document(&self.path, &doc)?;
        self.document = doc;
        self.entries = entries;
        if let Some(key) = new_key {
            self.key = key;
        }
        Ok(())
    }
}

/// Returns true on damaged protected storage as well as on known secrets (fail closed).
pub(crate) fn is_protected(text: &str) -> bool {
    is_protected_at(path(), text)
}
fn is_protected_at(file: PathBuf, text: &str) -> bool {
    if text.is_empty() {
        return false;
    }
    let stamp = StoreStamp {
        document: file_stamp(&file),
        presence: file_stamp(&file.with_extension("presence")),
    };
    let Ok(mut cached) = MATCHER.get_or_init(|| Mutex::new(None)).lock() else {
        return true;
    };
    if cached
        .as_ref()
        .is_none_or(|m| m.path != file || m.stamp != stamp)
    {
        let matcher = match read_document(&file) {
            Ok(Some((doc, key))) => Matcher {
                path: file,
                stamp,
                key: Some(key),
                blocked: doc.blocked.into_iter().collect(),
                failed: false,
            },
            Ok(None) => Matcher {
                path: file,
                stamp,
                key: None,
                blocked: HashSet::new(),
                failed: false,
            },
            Err(_) => Matcher {
                path: file,
                stamp,
                key: None,
                blocked: HashSet::new(),
                failed: true,
            },
        };
        *cached = Some(matcher);
    }
    let matcher = cached.as_ref().unwrap();
    matcher.failed
        || matcher
            .key
            .as_ref()
            .is_some_and(|key| matcher.blocked.contains(&fingerprint(key.as_ref(), text)))
}

#[link(name = "bcrypt")]
unsafe extern "system" {
    fn BCryptGenRandom(provider: *mut c_void, output: *mut u8, len: u32, flags: u32) -> i32;
}
#[link(name = "kernel32")]
unsafe extern "system" {
    fn MoveFileExW(from: *const u16, to: *const u16, flags: u32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    fn test_path() -> PathBuf {
        std::env::temp_dir()
            .join(format!("zsclip-vault-test-{}", random_id().unwrap()))
            .join("vault.json")
    }
    #[test]
    fn secrets_are_encrypted_and_reopen_without_optional_password() {
        let path = test_path();
        let mut session = VaultSession::open(path.clone(), None).unwrap();
        let id = session
            .upsert(
                None,
                VaultKind::Password,
                "测试服务",
                "账号备注",
                " synthetic-password ",
            )
            .unwrap();
        assert_eq!(
            session.secret(&id).unwrap().as_str(),
            " synthetic-password "
        );
        let disk = fs::read_to_string(&path).unwrap();
        assert!(!disk.contains("synthetic-password"));
        assert!(!disk.contains("测试服务"));
        drop(session);
        let session = VaultSession::open(path, None).unwrap();
        assert_eq!(session.list(VaultKind::Password)[0].label, "测试服务");
        assert_eq!(
            session.secret(&id).unwrap().as_str(),
            " synthetic-password "
        );
    }
    #[test]
    fn management_password_is_required_and_can_be_removed_after_unlock() {
        let path = test_path();
        let mut session = VaultSession::open(path.clone(), None).unwrap();
        let id = session
            .upsert(None, VaultKind::Key, "合成密钥", "", "line1\nline2")
            .unwrap();
        session.set_password(Some("test-management-pass")).unwrap();
        drop(session);
        assert!(VaultSession::open(path.clone(), None).is_err());
        assert!(VaultSession::open(path.clone(), Some("wrong")).is_err());
        let mut session = VaultSession::open(path.clone(), Some("test-management-pass")).unwrap();
        assert_eq!(session.secret(&id).unwrap().as_str(), "line1\nline2");
        session.set_password(None).unwrap();
        drop(session);
        assert!(VaultSession::open(path, None).is_ok());
    }
    #[test]
    fn editing_and_deleting_release_only_values_without_remaining_entries() {
        let path = test_path();
        let mut session = VaultSession::open(path.clone(), None).unwrap();
        let id = session
            .upsert(None, VaultKind::Password, "甲", "", "first-secret")
            .unwrap();
        session
            .upsert(
                Some(&id),
                VaultKind::Password,
                "乙",
                "说明",
                "second-secret",
            )
            .unwrap();
        assert!(!is_protected_at(path.clone(), "first-secret"));
        assert!(is_protected_at(path.clone(), "second-secret"));
        session.remove(&id).unwrap();
        drop(session);
        let (doc, _) = read_document(&path).unwrap().unwrap();
        assert!(doc.blocked.is_empty());
        assert!(!is_protected_at(path.clone(), "first-secret"));
        assert!(!is_protected_at(path.clone(), "second-secret"));
        assert!(VaultSession::open(path, None)
            .unwrap()
            .list(VaultKind::Password)
            .is_empty());
    }
    #[test]
    fn duplicate_values_remain_protected_until_the_last_entry_is_removed() {
        let path = test_path();
        let mut session = VaultSession::open(path.clone(), None).unwrap();
        let first = session
            .upsert(None, VaultKind::Password, "first", "", " same\r\nvalue ")
            .unwrap();
        let second = session
            .upsert(None, VaultKind::Key, "second", "", "same\nvalue")
            .unwrap();
        assert_eq!(session.document.blocked.len(), 1);
        session.remove(&first).unwrap();
        assert!(is_protected_at(path.clone(), "same\nvalue"));
        assert_eq!(session.secret(&second).unwrap().as_str(), "same\nvalue");
        session.remove(&second).unwrap();
        assert!(!is_protected_at(path.clone(), "same\nvalue"));
        assert!(!exclusion_snapshot_at(&path).unwrap().matches("same\nvalue"));
    }
    #[test]
    fn unified_list_and_edit_preserve_legacy_entries_and_exact_secret() {
        let path = test_path();
        let mut session = VaultSession::open(path, None).unwrap();
        session
            .upsert(None, VaultKind::Password, "password", "", "first")
            .unwrap();
        let id = session
            .upsert(None, VaultKind::Key, "key", "note", "second")
            .unwrap();
        session
            .upsert_entry(Some(&id), "renamed", "new note", "\n  exact value  \n")
            .unwrap();
        session.upsert_entry(None, "combined", "", "third").unwrap();
        assert_eq!(session.list_all().len(), 3);
        assert_eq!(session.list(VaultKind::Key)[0].label, "renamed");
        assert_eq!(session.secret(&id).unwrap().as_str(), "\n  exact value  \n");
        assert!(!session.password_required());
    }
    #[test]
    fn unlocking_migrates_verified_legacy_exclusions_and_invalidates_old_session() {
        let path = test_path();
        let mut legacy = VaultSession::open(path.clone(), None).unwrap();
        legacy
            .upsert_entry(None, "active", "", "active-secret")
            .unwrap();
        legacy
            .document
            .blocked
            .push(fingerprint(legacy.filter_key.as_ref(), "deleted-secret"));
        sign_document(&mut legacy.document, &legacy.filter_key).unwrap();
        write_document(&path, &legacy.document).unwrap();
        let old_revision = legacy.document.revision;
        assert!(is_protected_at(path.clone(), "deleted-secret"));
        let migrated = VaultSession::open(path.clone(), None).unwrap();
        assert_eq!(migrated.document.revision, old_revision + 1);
        assert!(is_protected_at(path.clone(), "active-secret"));
        assert!(!is_protected_at(path.clone(), "deleted-secret"));
        assert!(legacy
            .upsert_entry(None, "stale", "", "stale-secret")
            .is_err());
        drop(migrated);
        let stable = fs::read(&path).unwrap();
        assert!(VaultSession::open(path.clone(), None).is_ok());
        assert_eq!(stable, fs::read(path).unwrap());
    }
    #[test]
    fn legacy_exclusions_are_not_removed_before_correct_password_unlock() {
        let path = test_path();
        let mut legacy = VaultSession::open(path.clone(), None).unwrap();
        legacy
            .upsert_entry(None, "active", "", "active-secret")
            .unwrap();
        legacy.set_password(Some("synthetic-management")).unwrap();
        legacy
            .document
            .blocked
            .push(fingerprint(legacy.filter_key.as_ref(), "deleted-secret"));
        sign_document(&mut legacy.document, &legacy.filter_key).unwrap();
        write_document(&path, &legacy.document).unwrap();
        let original = fs::read(&path).unwrap();
        drop(legacy);
        assert!(VaultSession::open(path.clone(), None).is_err());
        assert!(VaultSession::open(path.clone(), Some("wrong")).is_err());
        assert_eq!(original, fs::read(&path).unwrap());
        assert!(is_protected_at(path.clone(), "deleted-secret"));
        let session = VaultSession::open(path.clone(), Some("synthetic-management")).unwrap();
        assert!(session.password_required());
        assert!(is_protected_at(path.clone(), "active-secret"));
        assert!(!is_protected_at(path, "deleted-secret"));
    }
    #[test]
    fn failed_legacy_migration_preserves_disk_and_cached_protection() {
        let path = test_path();
        let mut legacy = VaultSession::open(path.clone(), None).unwrap();
        legacy
            .upsert_entry(None, "active", "", "active-secret")
            .unwrap();
        legacy
            .document
            .blocked
            .push(fingerprint(legacy.filter_key.as_ref(), "deleted-secret"));
        sign_document(&mut legacy.document, &legacy.filter_key).unwrap();
        write_document(&path, &legacy.document).unwrap();
        let original = fs::read(&path).unwrap();
        assert!(is_protected_at(path.clone(), "deleted-secret"));
        // Withhold FILE_SHARE_DELETE so replacement fails after verified decryption.
        let handle = fs::OpenOptions::new()
            .read(true)
            .share_mode(1 | 2)
            .open(&path)
            .unwrap();
        assert!(VaultSession::open(path.clone(), None).is_err());
        assert_eq!(original, fs::read(&path).unwrap());
        assert!(is_protected_at(path.clone(), "deleted-secret"));
        assert!(is_protected_at(path.clone(), "active-secret"));
        drop(handle);
        assert!(VaultSession::open(path.clone(), None).is_ok());
        assert!(!is_protected_at(path, "deleted-secret"));
    }
    #[test]
    fn tampering_and_stale_session_cannot_overwrite_current_store() {
        let path = test_path();
        let mut one = VaultSession::open(path.clone(), None).unwrap();
        let mut two = VaultSession::open(path.clone(), None).unwrap();
        one.upsert(None, VaultKind::Key, "first", "", "secret-one")
            .unwrap();
        assert!(two
            .upsert(None, VaultKind::Key, "second", "", "secret-two")
            .is_err());
        let mut doc: Document = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        doc.blocked.clear();
        fs::write(&path, serde_json::to_vec(&doc).unwrap()).unwrap();
        assert!(read_document(&path).is_err());
    }
    #[test]
    fn suppression_normalizes_clipboard_line_endings_without_changing_value() {
        let key = [7; 32];
        assert_eq!(
            fingerprint(&key, " first\r\nsecond "),
            fingerprint(&key, "first\nsecond")
        );
        assert_ne!(
            fingerprint(&key, "first\nsecond"),
            fingerprint(&key, "first second")
        );
    }
    #[test]
    fn missing_vault_with_new_presence_marker_invalidates_empty_cache() {
        let path = test_path();
        assert!(!is_protected_at(path.clone(), "synthetic secret"));
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(
            path.with_extension("presence"),
            b"ZSClip protected storage v1",
        )
        .unwrap();
        assert!(is_protected_at(path.clone(), "synthetic secret"));
        assert!(VaultSession::open(path, None).is_err());
    }
    #[test]
    fn exclusive_file_lock_blocks_open_and_commit_and_releases_without_deletion() {
        let path = test_path();
        let guard = lock_store(&path).unwrap();
        assert!(VaultSession::open(path.clone(), None)
            .err()
            .unwrap()
            .contains("稍后重试"));
        assert!(!path.exists() && !path.with_extension("presence").exists());
        drop(guard);
        let mut session = VaultSession::open(path.clone(), None).unwrap();
        let disk = fs::read(&path).unwrap();
        let guard = lock_store(&path).unwrap();
        assert!(session
            .upsert(None, VaultKind::Key, "synthetic", "", "synthetic-secret")
            .is_err());
        assert!(session.list(VaultKind::Key).is_empty());
        assert_eq!(fs::read(&path).unwrap(), disk);
        drop(guard);
        assert!(path.with_extension("lock").is_file());
        session
            .upsert(None, VaultKind::Key, "synthetic", "", "synthetic-secret")
            .unwrap();
    }
    #[test]
    fn presence_failure_does_not_commit_document_or_session() {
        let path = test_path();
        let mut session = VaultSession::open(path.clone(), None).unwrap();
        let disk = fs::read(&path).unwrap();
        let marker = path.with_extension("presence");
        fs::remove_file(&marker).unwrap();
        fs::create_dir(&marker).unwrap();
        assert!(session
            .upsert(
                None,
                VaultKind::Password,
                "synthetic",
                "",
                "synthetic-secret"
            )
            .is_err());
        assert!(session.list(VaultKind::Password).is_empty());
        assert_eq!(fs::read(&path).unwrap(), disk);
        assert!(marker.is_dir());
        fs::remove_dir(marker).unwrap();
        session
            .upsert(
                None,
                VaultKind::Password,
                "synthetic",
                "",
                "synthetic-secret",
            )
            .unwrap();
    }
    #[test]
    fn first_initialization_failure_only_removes_its_own_new_marker() {
        let seed = VaultSession::open(test_path(), None).unwrap();
        let path = test_path();
        let _guard = WRITES.lock().unwrap();
        let _file_guard = lock_store(&path).unwrap();
        assert!(write_document_locked(&path, &seed.document, |_, _| Err(
            "simulated replace failure".into()
        ))
        .is_err());
        assert!(!path.exists() && !path.with_extension("presence").exists());
        assert!(!fs::read_dir(path.parent().unwrap()).unwrap().any(|e| e
            .unwrap()
            .path()
            .extension()
            .is_some_and(|v| v == "tmp")));
        let marker = path.with_extension("presence");
        fs::write(&marker, b"preexisting protection").unwrap();
        assert!(write_document_locked(&path, &seed.document, |_, _| Err(
            "simulated replace failure".into()
        ))
        .is_err());
        assert_eq!(fs::read(marker).unwrap(), b"preexisting protection");
    }
    #[test]
    fn absent_vault_snapshot_matches_nothing_without_creating_storage() {
        let path = test_path();
        let snapshot = exclusion_snapshot_at(&path).unwrap();
        assert!(!snapshot.matches("synthetic ordinary history"));
        assert!(!snapshot.matches(""));
        assert!(!path.parent().unwrap().exists());
    }
    #[test]
    fn snapshot_matches_only_current_values_while_vault_is_locked() {
        let path = test_path();
        let mut session = VaultSession::open(path.clone(), None).unwrap();
        let id = session
            .upsert(
                None,
                VaultKind::Password,
                "synthetic",
                "",
                "first-secret\r\nline",
            )
            .unwrap();
        session
            .upsert(
                Some(&id),
                VaultKind::Password,
                "synthetic",
                "",
                "second-secret",
            )
            .unwrap();
        session.remove(&id).unwrap();
        session
            .upsert_entry(None, "current", "", "current-secret")
            .unwrap();
        session
            .set_password(Some("synthetic management password"))
            .unwrap();
        drop(session);
        let snapshot = exclusion_snapshot_at(&path).unwrap();
        assert!(!snapshot.matches("first-secret\nline"));
        assert!(!snapshot.matches("second-secret"));
        assert!(snapshot.matches("current-secret"));
        assert!(!snapshot.matches("unrelated ordinary history"));
        assert!(!snapshot.matches(""));
    }
    #[test]
    fn damaged_snapshot_errors_without_changing_previously_verified_snapshot() {
        let path = test_path();
        let mut session = VaultSession::open(path.clone(), None).unwrap();
        session
            .upsert(None, VaultKind::Key, "synthetic", "", "registered-secret")
            .unwrap();
        drop(session);
        let snapshot = exclusion_snapshot_at(&path).unwrap();
        let mut document: Document = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
        document.blocked.clear();
        fs::write(&path, serde_json::to_vec(&document).unwrap()).unwrap();
        assert!(exclusion_snapshot_at(&path).is_err());
        assert!(is_protected_at(path.clone(), "unrelated ordinary history"));
        assert!(snapshot.matches("registered-secret"));
        assert!(!snapshot.matches("unrelated ordinary history"));
        fs::remove_file(&path).unwrap();
        assert!(exclusion_snapshot_at(&path).is_err());
    }
}
