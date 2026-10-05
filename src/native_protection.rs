//! Read-only protection boundary for native profiles without a Windows DPAPI key.
//! An empty store is usable; an existing opaque store is never treated as empty.

use std::fmt;
use std::fs;
use std::io::Read;
use std::path::Path;

const MAX_REGISTRY_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProtectionError {
    UnreadableDirectory,
    UnrecognizedStorage,
    MissingVault,
    UnreadableVault,
    DamagedVault,
    WindowsEncryptedVault,
}

impl fmt::Display for ProtectionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let message = match self {
            Self::UnreadableDirectory => crate::i18n::tr(
                "无法检查受保护目录，已暂停文本记录、搜索和同步。请检查数据目录权限。",
                "Protected storage cannot be inspected. Text history, search and sync are paused. Check the data directory permissions."),
            Self::UnrecognizedStorage => crate::i18n::tr(
                "受保护目录包含无法识别的存储格式，已暂停文本记录、搜索和同步。",
                "The protected directory contains an unsupported storage format. Text history, search and sync are paused."),
            Self::MissingVault => crate::i18n::tr(
                "检测到密码库保护标记，但密码库文件缺失。已暂停文本记录、搜索和同步，请恢复原始密码库。",
                "A protection marker exists but the vault file is missing. Text history, search and sync are paused. Restore the original vault."),
            Self::UnreadableVault => crate::i18n::tr(
                "受保护存储无法读取，已暂停文本记录、搜索和同步。请检查文件与目录权限。",
                "Protected storage cannot be read. Text history, search and sync are paused. Check the file and directory permissions."),
            Self::DamagedVault => crate::i18n::tr(
                "受保护存储已损坏或格式无效，已暂停文本记录、搜索和同步。请在原设备恢复并检查密码库。",
                "Protected storage is damaged or invalid. Text history, search and sync are paused. Restore and verify the vault on its original device."),
            Self::WindowsEncryptedVault => crate::i18n::tr(
                "此数据目录含 Windows 用户加密的密码库，当前系统无法解密。已暂停文本记录、搜索和同步，请在原 Windows 用户下核验并导出已清理的普通记录。",
                "This profile contains a vault encrypted for a Windows user and cannot be decrypted on this platform. Text history, search and sync are paused. Verify the vault under the original Windows account and export sanitized ordinary history."),
        };
        f.write_str(message)
    }
}

impl std::error::Error for ProtectionError {}

/// Only produced after verifying that no protected registry is present.
pub(crate) struct EmptyProtectionSnapshot {
    revision: String,
}

impl EmptyProtectionSnapshot {
    pub(crate) fn revision(&self) -> &str { &self.revision }
    pub(crate) fn matches(&self, _text: &str) -> bool { false }
}

pub(crate) fn snapshot_at(data_directory: &Path) -> Result<EmptyProtectionSnapshot, ProtectionError> {
    match fs::metadata(data_directory) {
        Ok(metadata) if !metadata.is_dir() => return Err(ProtectionError::UnreadableDirectory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            match fs::symlink_metadata(data_directory) {
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {},
                _ => return Err(ProtectionError::UnreadableDirectory),
            }
            return Ok(EmptyProtectionSnapshot { revision: "native-protection-v1:missing-profile".into() });
        }
        Err(_) => return Err(ProtectionError::UnreadableDirectory),
        _ => {}
    }
    let directory = data_directory.join("protected");
    let metadata = match fs::symlink_metadata(&directory) {
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => metadata,
        Ok(_) => return Err(ProtectionError::UnreadableDirectory),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(EmptyProtectionSnapshot { revision: "native-protection-v1:absent".into() });
        }
        Err(_) => return Err(ProtectionError::UnreadableDirectory),
    };
    let mut vault = false;
    let mut presence = false;
    for entry in fs::read_dir(&directory).map_err(|_| ProtectionError::UnreadableDirectory)? {
        let entry = entry.map_err(|_| ProtectionError::UnreadableDirectory)?;
        let kind = entry.file_type().map_err(|_| ProtectionError::UnreadableDirectory)?;
        if !kind.is_file() || kind.is_symlink() { return Err(ProtectionError::UnrecognizedStorage); }
        match entry.file_name().to_str() {
            Some("vault.json") => vault = true,
            Some("vault.presence") => presence = true,
            Some("vault.lock") => {},
            _ => return Err(ProtectionError::UnrecognizedStorage),
        }
    }
    if !vault {
        if presence { return Err(ProtectionError::MissingVault); }
        return Ok(EmptyProtectionSnapshot {
            revision: format!("native-protection-v1:empty:{:?}:{}", metadata.modified().ok(), metadata.len()),
        });
    }
    let file = fs::File::open(directory.join("vault.json")).map_err(|_| ProtectionError::UnreadableVault)?;
    if file.metadata().map_err(|_| ProtectionError::UnreadableVault)?.len() > MAX_REGISTRY_BYTES {
        return Err(ProtectionError::DamagedVault);
    }
    let mut bytes = Vec::new();
    file.take(MAX_REGISTRY_BYTES + 1).read_to_end(&mut bytes).map_err(|_| ProtectionError::UnreadableVault)?;
    if bytes.len() as u64 > MAX_REGISTRY_BYTES { return Err(ProtectionError::DamagedVault); }
    let value: serde_json::Value = serde_json::from_slice(&bytes).map_err(|_| ProtectionError::DamagedVault)?;
    if value.get("format").and_then(serde_json::Value::as_u64) == Some(1)
        && value.get("filter_key_dpapi").and_then(serde_json::Value::as_str).is_some_and(|value| !value.is_empty())
        && value.get("ciphertext").and_then(serde_json::Value::as_str).is_some_and(|value| !value.is_empty())
    {
        Err(ProtectionError::WindowsEncryptedVault)
    } else {
        Err(ProtectionError::DamagedVault)
    }
}

pub(crate) fn snapshot() -> Result<EmptyProtectionSnapshot, ProtectionError> {
    snapshot_at(&crate::native_paths::data_directory())
}

pub(crate) fn text_is_protected(text: &str) -> bool {
    text_is_protected_at(&crate::native_paths::data_directory(), text)
}

fn text_is_protected_at(directory: &Path, text: &str) -> bool {
    !text.is_empty() && snapshot_at(directory).map(|snapshot| snapshot.matches(text)).unwrap_or(true)
}

pub(crate) fn revision() -> Result<String, ProtectionError> {
    snapshot().map(|snapshot| snapshot.revision)
}

pub(crate) fn validate_profile(directory: &Path) -> Result<(), String> {
    snapshot_at(directory).map(|_| ()).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);
    struct Fixture(std::path::PathBuf);
    impl Fixture {
        fn new() -> Self {
            let root = std::env::temp_dir().join(format!("zsclip-native-protection-{}-{}-{}", std::process::id(),
                std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos(), NEXT.fetch_add(1, Ordering::Relaxed)));
            fs::create_dir_all(&root).unwrap();
            Self(root)
        }
        fn protected(&self) -> std::path::PathBuf {
            let path = self.0.join("protected"); fs::create_dir_all(&path).unwrap(); path
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) { let _ = fs::remove_dir_all(&self.0); }
    }

    #[test]
    fn absent_registry_matches_nothing_and_does_not_create_protected_storage() {
        let fixture = Fixture::new();
        let snapshot = snapshot_at(&fixture.0).unwrap();
        assert!(!snapshot.matches("ordinary synthetic history"));
        assert!(!fixture.0.join("protected").exists());
        let missing = fixture.0.join("not-created");
        assert!(!snapshot_at(&missing).unwrap().matches("ordinary synthetic history"));
        assert!(!missing.exists());
    }

    #[test]
    fn windows_dpapi_registry_is_opaque_and_never_treated_as_unprotected() {
        let fixture = Fixture::new();
        let protected = fixture.protected();
        fs::write(protected.join("vault.json"), br#"{"format":1,"filter_key_dpapi":"opaque-windows-key","ciphertext":"opaque-encrypted-payload"}"#).unwrap();
        assert!(matches!(snapshot_at(&fixture.0), Err(ProtectionError::WindowsEncryptedVault)));
        assert!(text_is_protected_at(&fixture.0, "synthetic protected value"));
        assert!(text_is_protected_at(&fixture.0, "synthetic ordinary value"));
        assert!(!text_is_protected_at(&fixture.0, ""));
        assert!(!validate_profile(&fixture.0).unwrap_err().is_empty());
    }

    #[test]
    fn orphan_presence_and_damaged_or_unknown_files_fail_closed() {
        let fixture = Fixture::new();
        let protected = fixture.protected();
        fs::write(protected.join("vault.presence"), b"ZSClip protected storage v1").unwrap();
        assert!(matches!(snapshot_at(&fixture.0), Err(ProtectionError::MissingVault)));
        fs::write(protected.join("vault.json"), b"{broken}").unwrap();
        assert!(matches!(snapshot_at(&fixture.0), Err(ProtectionError::DamagedVault)));
        fs::write(protected.join("unknown-vault.bin"), b"opaque").unwrap();
        assert!(matches!(snapshot_at(&fixture.0), Err(ProtectionError::UnrecognizedStorage)));
    }

    #[test]
    fn inaccessible_storage_layout_is_not_mistaken_for_absence() {
        let fixture = Fixture::new();
        fs::write(fixture.0.join("protected"), b"not a readable directory").unwrap();
        assert!(matches!(snapshot_at(&fixture.0), Err(ProtectionError::UnreadableDirectory)));
    }

    #[cfg(unix)]
    #[test]
    fn filesystem_access_errors_fail_closed_without_becoming_an_empty_profile() {
        let fixture = Fixture::new();
        let inaccessible = fixture.0.join("loop");
        std::os::unix::fs::symlink("loop", &inaccessible).unwrap();
        // ELOOP is an actual filesystem access failure even for privileged test users.
        assert!(fs::metadata(&inaccessible).is_err());
        assert!(matches!(snapshot_at(&inaccessible), Err(ProtectionError::UnreadableDirectory)));
        assert!(text_is_protected_at(&inaccessible, "synthetic text"));
        let dangling = fixture.0.join("dangling");
        std::os::unix::fs::symlink("not-present", &dangling).unwrap();
        assert!(matches!(snapshot_at(&dangling), Err(ProtectionError::UnreadableDirectory)));
    }

    #[test]
    fn appearance_of_protection_changes_revision_and_blocks_prior_empty_snapshot() {
        let fixture = Fixture::new();
        let before = snapshot_at(&fixture.0).unwrap().revision().to_string();
        let protected = fixture.protected();
        let after = snapshot_at(&fixture.0).unwrap().revision().to_string();
        assert_ne!(before, after);
        fs::write(protected.join("vault.presence"), b"ZSClip protected storage v1").unwrap();
        assert!(snapshot_at(&fixture.0).is_err());
    }
}
