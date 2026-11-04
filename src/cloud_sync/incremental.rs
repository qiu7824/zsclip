use super::*;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::io::{Seek, SeekFrom};

pub(super) const BACKUP_FORMAT: &str = "chunks-v1";
const CHUNK_SIZE: usize = 4 * 1024 * 1024;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ChunkIndex {
    pub files: BTreeMap<String, ChunkFile>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(super) struct ChunkFile {
    size: u64,
    hashes: Vec<String>,
}

// Local receipts are deliberately outside the synchronized payload. They are
// scoped to the endpoint/account so changing a server cannot suppress uploads.
#[derive(Default, Serialize, Deserialize)]
pub(super) struct LocalSyncState {
    endpoint: String,
    pub snapshot_hash: String,
    pub uploaded_clip: String,
    #[serde(default)]
    pub updated_at: u64,
}

impl LocalSyncState {
    pub fn load(config: &CloudSyncConfig, paths: &CloudSyncPaths) -> Self {
        let endpoint = digest(
            &serde_json::to_vec(&[
                config.webdav_url.trim().trim_end_matches('/'),
                config.webdav_user.trim(),
                config.remote_dir.trim(),
            ])
            .expect("string array serialization"),
        );
        fs::read(paths.data_dir.join("cloud-sync-state.json"))
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Self>(&bytes).ok())
            .filter(|state| state.endpoint == endpoint)
            .unwrap_or(Self {
                endpoint,
                ..Self::default()
            })
    }

    pub fn save(&self, paths: &CloudSyncPaths) -> Result<(), String> {
        let temp = write_temp_json_file("cloud-state", self)?;
        let _guard = TempPathGuard::file(temp.clone());
        replace_settings_file_transactionally(&temp, &paths.data_dir.join("cloud-sync-state.json"))
            .map_err(|error| format!("无法保存云同步状态：{error}"))
    }
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

pub(super) fn clip_receipt(
    manifest: &crate::multi_sync::MultiSyncManifest,
) -> Result<String, String> {
    serde_json::to_vec(manifest)
        .map(|bytes| digest(&bytes))
        .map_err(|error| error.to_string())
}

pub(super) fn normalize_snapshot_settings(path: &Path) -> Result<(), String> {
    let mut settings: serde_json::Value =
        serde_json::from_slice(&fs::read(path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
    // Progress/status text is local runtime state, not a settings change.
    if let Some(object) = settings.as_object_mut() {
        object.remove("cloud_last_sync_status");
    }
    fs::write(
        path,
        serde_json::to_vec_pretty(&settings).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())
}

fn payload_files(root: &Path, directory: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    for entry in fs::read_dir(directory).map_err(|error| error.to_string())? {
        let entry = entry.map_err(|error| error.to_string())?;
        let kind = entry.file_type().map_err(|error| error.to_string())?;
        if kind.is_dir() {
            payload_files(root, &entry.path(), files)?;
        } else if kind.is_file() {
            let path = entry.path();
            validate_name(&zip_relative_name(root, &path)?)?;
            files.push(path);
        } else {
            return Err("云备份不支持链接或特殊文件。".into());
        }
    }
    Ok(())
}

fn index_snapshot(root: &Path) -> Result<ChunkIndex, String> {
    let mut paths = Vec::new();
    payload_files(root, root, &mut paths)?;
    paths.sort();
    let mut files = BTreeMap::new();
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    for path in paths {
        let mut input = File::open(&path).map_err(|error| error.to_string())?;
        let size = input.metadata().map_err(|error| error.to_string())?.len();
        let mut remaining = size;
        let mut hashes = Vec::new();
        while remaining > 0 {
            let count = remaining.min(CHUNK_SIZE as u64) as usize;
            input
                .read_exact(&mut buffer[..count])
                .map_err(|error| error.to_string())?;
            hashes.push(digest(&buffer[..count]));
            remaining -= count as u64;
        }
        files.insert(zip_relative_name(root, &path)?, ChunkFile { size, hashes });
    }
    let index = ChunkIndex { files };
    index.validate()?;
    Ok(index)
}

pub(super) fn payload_hash(root: &Path) -> Result<String, String> {
    // Portable '/' paths and ordered entries keep the digest identical on all
    // desktop platforms. Legacy ZIP manifests retain their original hash rule.
    let index = index_snapshot(root)?;
    let bytes = serde_json::to_vec(&index).map_err(|error| error.to_string())?;
    Ok(format!("sha256:{}", digest(&bytes)))
}

fn validate_name(name: &str) -> Result<PathBuf, String> {
    let parts: Vec<_> = name.split('/').collect();
    if parts.iter().any(|part| {
        part.is_empty()
            || *part == "."
            || *part == ".."
            || part.ends_with(['.', ' '])
            || part
                .chars()
                .any(|c| c.is_control() || "\\:<>\"|?*".contains(c))
    }) || !(name == "settings.json"
        || name == "clipboard.db"
        || (parts.len() > 1 && parts[0] == "images"))
    {
        return Err(format!("云备份包含不安全路径：{name}"));
    }
    for part in &parts {
        let stem = part.split('.').next().unwrap_or("").to_ascii_uppercase();
        if ["CON", "PRN", "AUX", "NUL"].contains(&stem.as_str())
            || (stem.len() == 4
                && (stem.starts_with("COM") || stem.starts_with("LPT"))
                && matches!(stem.as_bytes()[3], b'1'..=b'9'))
        {
            return Err(format!("云备份包含保留文件名：{name}"));
        }
    }
    Ok(parts.iter().collect())
}

impl ChunkIndex {
    fn validate(&self) -> Result<(), String> {
        if !self.files.contains_key("settings.json") || !self.files.contains_key("clipboard.db") {
            return Err("云备份缺少设置或数据库。".into());
        }
        let mut names = HashSet::new();
        for (name, file) in &self.files {
            validate_name(name)?;
            if !names.insert(name.to_lowercase()) {
                return Err("云备份包含重名文件。".into());
            }
            let count =
                file.size / CHUNK_SIZE as u64 + u64::from(file.size % CHUNK_SIZE as u64 != 0);
            if count != file.hashes.len() as u64
                || file.hashes.iter().any(|hash| {
                    hash.len() != 64
                        || !hash
                            .bytes()
                            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
                })
            {
                return Err("云备份数据块清单无效。".into());
            }
        }
        for name in &names {
            for (offset, _) in name.match_indices('/') {
                if names.contains(&name[..offset]) {
                    return Err("云备份的文件与目录路径冲突。".into());
                }
            }
        }
        Ok(())
    }

    fn hashes(&self) -> HashSet<&str> {
        self.files
            .values()
            .flat_map(|file| file.hashes.iter().map(String::as_str))
            .collect()
    }
}

fn chunk_url(remote: &RemoteLayout, hash: &str) -> String {
    append_url_path(
        &append_url_path(&remote.backup_dir_url, BACKUP_FORMAT),
        &format!("{hash}.zip"),
    )
}

pub(super) fn upload_snapshot(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
    snapshot: &SnapshotPayload,
    previous: Option<&CloudSyncManifest>,
) -> Result<ChunkIndex, String> {
    let index = index_snapshot(&snapshot.root)?;
    let previous = previous
        .filter(|manifest| manifest.backup_name == BACKUP_FORMAT)
        .and_then(|manifest| manifest.chunks.as_ref());
    if let Some(previous) = previous {
        previous.validate()?;
    }
    let mut available = previous.map(ChunkIndex::hashes).unwrap_or_default();
    webdav_mkcol(
        config,
        &append_url_path(&remote.backup_dir_url, BACKUP_FORMAT),
    )?;
    let temp = temp_unique_path("cloud-chunk", "zip")?;
    let _guard = TempPathGuard::file(temp.clone());
    let mut buffer = vec![0_u8; CHUNK_SIZE];
    for (name, file) in &index.files {
        let mut input = File::open(snapshot.root.join(validate_name(name)?))
            .map_err(|error| error.to_string())?;
        for (position, hash) in file.hashes.iter().enumerate() {
            if available.contains(hash.as_str()) {
                continue;
            }
            let offset = position as u64 * CHUNK_SIZE as u64;
            let count = (file.size - offset).min(CHUNK_SIZE as u64) as usize;
            input
                .seek(SeekFrom::Start(offset))
                .and_then(|_| input.read_exact(&mut buffer[..count]))
                .map_err(|error| error.to_string())?;
            if digest(&buffer[..count]) != *hash {
                return Err("云备份暂存数据发生变化。".into());
            }
            let mut zip = ZipWriter::new(File::create(&temp).map_err(|error| error.to_string())?);
            zip.start_file(
                "chunk",
                FileOptions::default().compression_method(CompressionMethod::Deflated),
            )
            .map_err(|error| error.to_string())?;
            zip.write_all(&buffer[..count])
                .map_err(|error| error.to_string())?;
            zip.finish().map_err(|error| error.to_string())?;
            crate::db_runtime::with_shared_app_data(|| {
                crate::db_runtime::ensure_protected_sync_revision(&snapshot.protection_revision)?;
                upload_file(config, &temp, &chunk_url(remote, hash))
            })?;
            available.insert(hash.as_str());
        }
    }
    Ok(index)
}

pub(super) fn stage_restore(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
    paths: &CloudSyncPaths,
    manifest: &CloudSyncManifest,
) -> Result<StagedRestore, String> {
    let index = manifest.chunks.as_ref().ok_or("云备份缺少数据块清单。")?;
    index.validate()?;
    let root = temp_dir_path("snapshot-restore")?;
    let _guard = TempPathGuard::dir(root.clone());
    let payload = root.join("payload");
    fs::create_dir_all(&payload).map_err(|error| error.to_string())?;
    let temp = root.join("chunk.zip");
    for (name, file) in &index.files {
        let destination = payload.join(validate_name(name)?);
        fs::create_dir_all(destination.parent().unwrap()).map_err(|error| error.to_string())?;
        let mut output = File::create(destination).map_err(|error| error.to_string())?;
        for (position, hash) in file.hashes.iter().enumerate() {
            if !download_file(config, &chunk_url(remote, hash), &temp)? {
                return Err(format!("云备份缺少数据块：{hash}"));
            }
            let mut archive =
                ZipArchive::new(File::open(&temp).map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())?;
            if archive.len() != 1 {
                return Err("云备份数据块格式无效。".into());
            }
            let entry = archive
                .by_name("chunk")
                .map_err(|error| error.to_string())?;
            let expected = (file.size - position as u64 * CHUNK_SIZE as u64).min(CHUNK_SIZE as u64);
            if entry.size() != expected {
                return Err("云备份数据块长度不匹配。".into());
            }
            let mut bytes = Vec::new();
            entry
                .take(CHUNK_SIZE as u64 + 1)
                .read_to_end(&mut bytes)
                .map_err(|error| error.to_string())?;
            if bytes.len() as u64 != expected || digest(&bytes) != *hash {
                return Err("云备份数据块校验失败。".into());
            }
            output
                .write_all(&bytes)
                .map_err(|error| error.to_string())?;
        }
        output.sync_all().map_err(|error| error.to_string())?;
    }
    stage_payload_restore(paths, &root, &payload, Some(&manifest.snapshot_hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chunk_manifest_rejects_paths_sizes_and_hashes_before_restore() {
        let mut index = ChunkIndex {
            files: BTreeMap::from([
                (
                    "settings.json".into(),
                    ChunkFile {
                        size: 2,
                        hashes: vec![digest(b"{}")],
                    },
                ),
                (
                    "clipboard.db".into(),
                    ChunkFile {
                        size: 0,
                        hashes: vec![],
                    },
                ),
            ]),
        };
        index.validate().unwrap();
        for name in [
            "../outside",
            "images/../outside",
            "images\\outside",
            "images/C:outside",
            "images/NUL",
            "images/a.",
            "images/a//b",
            "/images/a",
            "protected/vault.json",
        ] {
            index.files.insert(
                name.into(),
                ChunkFile {
                    size: 0,
                    hashes: vec![],
                },
            );
            assert!(index.validate().is_err(), "accepted {name}");
            index.files.remove(name);
        }
        index.files.get_mut("settings.json").unwrap().size = CHUNK_SIZE as u64 + 1;
        assert!(index.validate().is_err());
        index.files.get_mut("settings.json").unwrap().size = 2;
        index.files.get_mut("settings.json").unwrap().hashes[0] = "../not-a-hash".into();
        assert!(index.validate().is_err());
    }

    #[test]
    fn local_receipts_are_scoped_to_server_account_and_directory() {
        let root = temp_dir_path("incremental-state-test").unwrap();
        let _guard = TempPathGuard::dir(root.clone());
        fs::create_dir_all(&root).unwrap();
        let paths = CloudSyncPaths {
            data_dir: root.clone(),
            settings_file: root.join("settings.json"),
            db_file: root.join("clipboard.db"),
        };
        let config = CloudSyncConfig {
            webdav_url: "https://example.invalid/dav".into(),
            webdav_user: "synthetic-user".into(),
            webdav_pass: String::new(),
            remote_dir: "ZSClip".into(),
        };
        let mut state = LocalSyncState::load(&config, &paths);
        state.snapshot_hash = "snapshot".into();
        state.uploaded_clip = "receipt".into();
        state.save(&paths).unwrap();
        assert_eq!(
            LocalSyncState::load(&config, &paths).uploaded_clip,
            "receipt"
        );
        let changes = [
            CloudSyncConfig {
                webdav_url: "https://another.invalid/dav".into(),
                ..config.clone()
            },
            CloudSyncConfig {
                webdav_user: "another-user".into(),
                ..config.clone()
            },
            CloudSyncConfig {
                remote_dir: "another-directory".into(),
                ..config.clone()
            },
        ];
        for changed in changes {
            assert!(LocalSyncState::load(&changed, &paths)
                .snapshot_hash
                .is_empty());
            assert!(LocalSyncState::load(&changed, &paths)
                .uploaded_clip
                .is_empty());
        }
    }
}
