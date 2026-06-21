use crate::i18n::tr;
use crate::time_utils::utc_secs_to_local_parts;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::{Read, Write};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::{Component, Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use zip::write::FileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

#[cfg(windows)]
const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;
const WEBDAV_DOWNLOAD_ATTEMPTS: usize = 3;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloudSyncAction {
    SyncNow,
    UploadConfig,
    ApplyRemoteConfig,
    RestoreBackup,
}

#[derive(Clone, Debug)]
pub struct CloudSyncConfig {
    pub webdav_url: String,
    pub webdav_user: String,
    pub webdav_pass: String,
    pub remote_dir: String,
}

#[derive(Clone, Debug)]
pub struct CloudSyncPaths {
    pub data_dir: PathBuf,
    pub settings_file: PathBuf,
    pub db_file: PathBuf,
}

#[derive(Clone, Debug)]
pub struct CloudSyncOutcome {
    pub status_text: String,
    pub reload_settings: bool,
    pub reload_data: bool,
}

#[derive(Clone, Debug)]
struct RemoteLayout {
    settings_url: String,
    manifest_url: String,
    sync_clipboard_url: String,
    sync_file_dir_url: String,
    backup_dir_url: String,
    backup_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CloudSyncManifest {
    version: String,
    updated_at: u64,
    #[serde(default)]
    snapshot_hash: String,
    backup_name: String,
}

const BACKUP_FILE_NAME: &str = "latest.zip";

impl RemoteLayout {
    fn from_config(config: &CloudSyncConfig) -> Result<Self, String> {
        let base = config.webdav_url.trim().trim_end_matches('/');
        if base.is_empty() {
            return Err("请先填写 WebDAV 地址。".to_string());
        }
        let remote_dir = if config.remote_dir.trim().is_empty() {
            "ZSClip"
        } else {
            config.remote_dir.trim()
        };
        let base_url = append_url_path(base, remote_dir);
        let settings_url = append_url_path(&base_url, "settings.json");
        let manifest_url = append_url_path(&base_url, "manifest.json");
        let sync_clipboard_url =
            append_url_path(&base_url, crate::multi_sync::MULTI_SYNC_MANIFEST_FILE_NAME);
        let sync_file_dir_url = append_url_path(&base_url, "file");
        let backup_dir_url = append_url_path(&base_url, "backups");
        let backup_url = append_url_path(&backup_dir_url, BACKUP_FILE_NAME);
        Ok(Self {
            settings_url,
            manifest_url,
            sync_clipboard_url,
            sync_file_dir_url,
            backup_dir_url,
            backup_url,
        })
    }
}

pub fn cloud_sync_interval(label: &str) -> Duration {
    match label.trim() {
        "15分钟" | "15 min" | "15m" | "15min" => Duration::from_secs(15 * 60),
        "30分钟" | "30 min" | "30m" | "30min" => Duration::from_secs(30 * 60),
        "1小时" | "1 hour" | "1h" => Duration::from_secs(60 * 60),
        "6小时" | "6 hours" | "6h" => Duration::from_secs(6 * 60 * 60),
        "12小时" | "12 hours" | "12h" => Duration::from_secs(12 * 60 * 60),
        "24小时" | "24 hours" | "24h" | "1d" => Duration::from_secs(24 * 60 * 60),
        _ => Duration::from_secs(60 * 60),
    }
}

pub fn perform_cloud_sync(
    action: CloudSyncAction,
    config: &CloudSyncConfig,
    paths: &CloudSyncPaths,
) -> Result<CloudSyncOutcome, String> {
    let remote = RemoteLayout::from_config(config)?;
    ensure_remote_layout(config, &remote)?;
    match action {
        CloudSyncAction::SyncNow => sync_snapshot(config, &remote, paths),
        CloudSyncAction::UploadConfig => upload_config(config, &remote, paths),
        CloudSyncAction::ApplyRemoteConfig => apply_remote_config(config, &remote, paths),
        CloudSyncAction::RestoreBackup => restore_remote_backup(config, &remote, paths),
    }
}

fn sync_snapshot(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
    paths: &CloudSyncPaths,
) -> Result<CloudSyncOutcome, String> {
    let imported_light_clip = import_remote_syncclipboard_clip(config, remote)?;
    crate::db_runtime::checkpoint_db().map_err(|err| err.to_string())?;
    let local_stamp = local_state_stamp(paths);
    let local_hash = local_state_hash(paths)?;
    let remote_manifest = download_remote_manifest(config, remote)?;
    if let Some(manifest) = remote_manifest {
        let version_cmp = compare_versions(&manifest.version, env!("CARGO_PKG_VERSION"));
        if !manifest.snapshot_hash.is_empty()
            && manifest.snapshot_hash == local_hash
            && !version_cmp.is_gt()
        {
            return Ok(CloudSyncOutcome {
                status_text: tr(
                    "本地与云端已同步，无需更新。",
                    "Local and cloud data are already in sync.",
                )
                .to_string(),
                reload_settings: false,
                reload_data: false,
            });
        }
        if manifest.updated_at > local_stamp.saturating_add(5) {
            if version_cmp.is_gt() {
                return Err(format!(
                    "{}{}{}",
                    tr("云端备份版本较新（", "Cloud backup version is newer ("),
                    manifest.version,
                    tr(
                        "），请先升级当前程序。",
                        "). Please upgrade this app first."
                    ),
                ));
            }
            let outcome = restore_remote_backup(config, remote, paths)?;
            return Ok(CloudSyncOutcome {
                status_text: format!(
                    "{}{}，{}{}。",
                    tr(
                        "云端较新，已恢复到本地（版本 ",
                        "Cloud copy was newer and has been restored locally (version "
                    ),
                    manifest.version,
                    tr("时间 ", "time "),
                    format_unix_ts(manifest.updated_at)
                ),
                ..outcome
            });
        }
        if local_stamp <= manifest.updated_at.saturating_add(5)
            && version_cmp.is_eq()
            && (manifest.snapshot_hash.is_empty() || manifest.snapshot_hash == local_hash)
        {
            return Ok(CloudSyncOutcome {
                status_text: "本地与云端已同步，无需更新。".to_string(),
                reload_settings: false,
                reload_data: false,
            });
        }
    }

    let stamp = local_stamp.max(unix_now());
    let archive_path = create_snapshot_archive(paths, stamp)?;
    upload_file(config, &archive_path, &remote.backup_url)?;
    upload_file(config, &paths.settings_file, &remote.settings_url)?;
    let manifest = CloudSyncManifest {
        version: env!("CARGO_PKG_VERSION").to_string(),
        updated_at: stamp,
        snapshot_hash: local_hash,
        backup_name: BACKUP_FILE_NAME.to_string(),
    };
    let manifest_path = write_temp_json_file("manifest", &manifest)?;
    upload_file(config, &manifest_path, &remote.manifest_url)?;
    upload_syncclipboard_manifest(config, remote)?;
    let _ = fs::remove_file(&archive_path);
    let _ = fs::remove_file(&manifest_path);
    let imported_note = if imported_light_clip {
        tr(
            " 已导入云端最新轻量清单。",
            " Imported latest cloud lightweight manifest.",
        )
    } else {
        ""
    };
    Ok(CloudSyncOutcome {
        status_text: format!(
            "{}{}{}",
            format!("云同步完成，已上传本地快照（{}）。", format_unix_ts(stamp)),
            imported_note,
            ""
        ),
        reload_settings: false,
        reload_data: imported_light_clip,
    })
}

fn upload_config(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
    paths: &CloudSyncPaths,
) -> Result<CloudSyncOutcome, String> {
    if !paths.settings_file.exists() {
        return Err("本地设置文件不存在，无法上传。".to_string());
    }
    upload_file(config, &paths.settings_file, &remote.settings_url)?;
    Ok(CloudSyncOutcome {
        status_text: "云端配置已上传。".to_string(),
        reload_settings: false,
        reload_data: false,
    })
}

fn apply_remote_config(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
    paths: &CloudSyncPaths,
) -> Result<CloudSyncOutcome, String> {
    let download_path = temp_file_path("settings-download", "json");
    if !download_file(config, &remote.settings_url, &download_path)? {
        return Err("云端没有找到 settings.json。".to_string());
    }
    if let Some(parent) = paths.settings_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    fs::copy(&download_path, &paths.settings_file).map_err(|err| err.to_string())?;
    let _ = fs::remove_file(download_path);
    Ok(CloudSyncOutcome {
        status_text: "已应用云端配置。".to_string(),
        reload_settings: true,
        reload_data: false,
    })
}

fn restore_remote_backup(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
    paths: &CloudSyncPaths,
) -> Result<CloudSyncOutcome, String> {
    if let Some(manifest) = download_remote_manifest(config, remote)? {
        if compare_versions(&manifest.version, env!("CARGO_PKG_VERSION")).is_gt() {
            return Err(format!(
                "{}{}{}",
                tr("云端备份版本较新（", "Cloud backup version is newer ("),
                manifest.version,
                tr(
                    "），请先升级当前程序。",
                    "). Please upgrade this app first."
                ),
            ));
        }
    }
    let download_path = temp_file_path("cloud-backup", "zip");
    if !download_file(config, &remote.backup_url, &download_path)? {
        return Err("云端没有找到可恢复的备份。".to_string());
    }
    let local_backup = create_local_restore_backup(paths)?;
    restore_snapshot_archive(paths, &download_path)?;
    let _ = fs::remove_file(download_path);
    Ok(CloudSyncOutcome {
        status_text: if let Some(path) = local_backup {
            format!(
                "{}{}",
                tr(
                    "已从云端恢复数据备份，本地旧数据已备份到：",
                    "Cloud backup restored. Previous local data was backed up to: "
                ),
                path.to_string_lossy()
            )
        } else {
            "已从云端恢复数据备份。".to_string()
        },
        reload_settings: true,
        reload_data: true,
    })
}

fn ensure_remote_layout(config: &CloudSyncConfig, remote: &RemoteLayout) -> Result<(), String> {
    let base = config.webdav_url.trim().trim_end_matches('/');
    let remote_dir = if config.remote_dir.trim().is_empty() {
        "ZSClip"
    } else {
        config.remote_dir.trim()
    };
    let mut current = base.to_string();
    for part in remote_dir.split('/').filter(|part| !part.trim().is_empty()) {
        current = append_url_path(&current, part);
        webdav_mkcol(config, &current)?;
    }
    webdav_mkcol(config, &remote.backup_dir_url)?;
    webdav_mkcol(config, &remote.sync_file_dir_url)?;
    Ok(())
}

fn upload_syncclipboard_manifest(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
) -> Result<(), String> {
    let manifest = crate::multi_sync::latest_manifest("webdav").map_err(|err| err.to_string())?;
    let data_name = manifest
        .clip
        .as_ref()
        .and_then(|clip| clip.data_name.as_deref())
        .map(|value| value.to_string());
    let manifest_path = write_temp_json_file("SyncClipboard", &manifest)?;
    upload_file(config, &manifest_path, &remote.sync_clipboard_url)?;
    let _ = fs::remove_file(&manifest_path);

    let Some(data_name) = data_name else {
        return Ok(());
    };
    let Some(id) = crate::multi_sync::image_id_from_data_name(&data_name) else {
        return Ok(());
    };
    let Some(bytes) = crate::multi_sync::load_image_png(id).map_err(|err| err.to_string())? else {
        return Ok(());
    };
    let data_path = temp_file_path("syncclipboard-image", "png");
    fs::write(&data_path, bytes).map_err(|err| err.to_string())?;
    let remote_data_url = append_url_path(&remote.sync_file_dir_url, &data_name);
    let upload_result = upload_file(config, &data_path, &remote_data_url);
    let _ = fs::remove_file(&data_path);
    upload_result
}

fn import_remote_syncclipboard_clip(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
) -> Result<bool, String> {
    let temp_path = temp_file_path("SyncClipboard-download", "json");
    let found = download_optional_file(config, &remote.sync_clipboard_url, &temp_path)?;
    if !found {
        return Ok(false);
    }
    let raw = fs::read_to_string(&temp_path).map_err(|err| err.to_string())?;
    let _ = fs::remove_file(&temp_path);
    if raw.trim().is_empty() {
        return Ok(false);
    }
    let manifest = serde_json::from_str::<crate::multi_sync::MultiSyncManifest>(&raw)
        .map_err(|err| err.to_string())?;
    if let Some(outcome) =
        crate::multi_sync::import_remote_text_clip(&manifest).map_err(|err| err.to_string())?
    {
        return Ok(outcome.imported);
    }
    let Some(data_name) = manifest
        .clip
        .as_ref()
        .filter(|clip| clip.kind == "image")
        .and_then(|clip| clip.data_name.as_deref())
    else {
        return Ok(false);
    };
    if crate::multi_sync::image_id_from_data_name(data_name).is_none() {
        return Ok(false);
    }
    let data_url = append_url_path(&remote.sync_file_dir_url, data_name);
    let image_path = temp_file_path("SyncClipboard-image-download", "png");
    let found = download_file(config, &data_url, &image_path)?;
    if !found {
        return Ok(false);
    }
    let png_bytes = fs::read(&image_path).map_err(|err| err.to_string())?;
    let _ = fs::remove_file(&image_path);
    Ok(
        crate::multi_sync::import_remote_image_clip(&manifest, &png_bytes)
            .map_err(|err| err.to_string())?
            .map(|outcome| outcome.imported)
            .unwrap_or(false),
    )
}

fn download_remote_manifest(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
) -> Result<Option<CloudSyncManifest>, String> {
    let temp_path = temp_file_path("manifest-download", "json");
    let found = download_file(config, &remote.manifest_url, &temp_path)?;
    if !found {
        return Ok(None);
    }
    let manifest = fs::read_to_string(&temp_path)
        .map_err(|err| err.to_string())
        .and_then(|raw| {
            serde_json::from_str::<CloudSyncManifest>(&raw).map_err(|err| err.to_string())
        })?;
    let _ = fs::remove_file(temp_path);
    Ok(Some(manifest))
}

fn create_snapshot_archive(paths: &CloudSyncPaths, _stamp: u64) -> Result<PathBuf, String> {
    let staging_root = temp_dir_path("snapshot-staging");
    if staging_root.exists() {
        let _ = fs::remove_dir_all(&staging_root);
    }
    let payload_dir = staging_root.join("payload");
    fs::create_dir_all(&payload_dir).map_err(|err| err.to_string())?;

    if paths.settings_file.exists() {
        fs::copy(&paths.settings_file, payload_dir.join("settings.json"))
            .map_err(|err| err.to_string())?;
    }
    if paths.db_file.exists() {
        fs::copy(&paths.db_file, payload_dir.join("clipboard.db"))
            .map_err(|err| err.to_string())?;
    }
    let images_dir = paths.data_dir.join("images");
    if images_dir.exists() {
        copy_dir_recursive(&images_dir, &payload_dir.join("images"))?;
    }

    let archive_path = temp_unique_path("cloud", "zip");
    compress_archive(&payload_dir, &archive_path)?;
    let _ = fs::remove_dir_all(staging_root);
    Ok(archive_path)
}

fn create_local_restore_backup(paths: &CloudSyncPaths) -> Result<Option<PathBuf>, String> {
    if !local_data_exists(paths) {
        return Ok(None);
    }
    let stamp = local_state_stamp(paths).max(unix_now());
    let temp_archive = create_snapshot_archive(paths, stamp)?;
    let backup_dir = paths.data_dir.join("restore-backups");
    fs::create_dir_all(&backup_dir).map_err(|err| err.to_string())?;
    let final_path = backup_dir.join(format!("before-restore-{}.zip", stamp));
    fs::rename(&temp_archive, &final_path)
        .or_else(|_| {
            fs::copy(&temp_archive, &final_path)
                .map(|_| ())
                .and_then(|_| fs::remove_file(&temp_archive))
        })
        .map_err(|err| err.to_string())?;
    Ok(Some(final_path))
}

fn restore_snapshot_archive(paths: &CloudSyncPaths, archive_path: &Path) -> Result<(), String> {
    let extract_root = temp_dir_path("snapshot-restore");
    if extract_root.exists() {
        let _ = fs::remove_dir_all(&extract_root);
    }
    fs::create_dir_all(&extract_root).map_err(|err| err.to_string())?;
    expand_archive(archive_path, &extract_root)?;
    let payload_dir = extract_root.join("payload");
    let source_dir = if payload_dir.exists() {
        payload_dir
    } else {
        extract_root.clone()
    };

    if let Some(parent) = paths.settings_file.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = paths.db_file.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let settings_src = source_dir.join("settings.json");
    if settings_src.exists() {
        fs::copy(settings_src, &paths.settings_file).map_err(|err| err.to_string())?;
    }

    let db_src = source_dir.join("clipboard.db");
    if db_src.exists() {
        remove_optional_file(&paths.db_file);
        remove_optional_file(&wal_file_for(&paths.db_file));
        remove_optional_file(&shm_file_for(&paths.db_file));
        fs::copy(db_src, &paths.db_file).map_err(|err| err.to_string())?;
    }

    let images_dst = paths.data_dir.join("images");
    if images_dst.exists() {
        let _ = fs::remove_dir_all(&images_dst);
    }
    let images_src = source_dir.join("images");
    if images_src.exists() {
        copy_dir_recursive(&images_src, &images_dst)?;
    }

    let _ = fs::remove_dir_all(extract_root);
    Ok(())
}

fn local_state_stamp(paths: &CloudSyncPaths) -> u64 {
    let mut stamp = 0;
    stamp = stamp.max(file_modified_secs(&paths.settings_file));
    stamp = stamp.max(file_modified_secs(&paths.db_file));
    let images_dir = paths.data_dir.join("images");
    if images_dir.exists() {
        stamp = stamp.max(dir_modified_secs(&images_dir));
    }
    stamp
}

fn local_state_hash(paths: &CloudSyncPaths) -> Result<String, String> {
    let mut hasher = Fnv64::new();
    hash_path_contents(&mut hasher, &paths.settings_file, "settings.json")?;
    hash_path_contents(&mut hasher, &paths.db_file, "clipboard.db")?;
    let images_dir = paths.data_dir.join("images");
    if images_dir.exists() {
        hash_dir_contents(&mut hasher, &images_dir, Path::new("images"))?;
    }
    Ok(format!("{:016x}", hasher.finish()))
}

fn local_data_exists(paths: &CloudSyncPaths) -> bool {
    if paths.settings_file.exists() || paths.db_file.exists() {
        return true;
    }
    let images_dir = paths.data_dir.join("images");
    images_dir.exists()
        && fs::read_dir(images_dir)
            .ok()
            .and_then(|mut entries| entries.next().transpose().ok().flatten())
            .is_some()
}

fn file_modified_secs(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_secs())
        .unwrap_or(0)
}

struct Fnv64 {
    value: u64,
}

impl Fnv64 {
    const OFFSET: u64 = 0xcbf29ce484222325;
    const PRIME: u64 = 0x100000001b3;

    fn new() -> Self {
        Self {
            value: Self::OFFSET,
        }
    }

    fn update(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.value ^= *byte as u64;
            self.value = self.value.wrapping_mul(Self::PRIME);
        }
    }

    fn finish(&self) -> u64 {
        self.value
    }
}

fn hash_path_contents(hasher: &mut Fnv64, path: &Path, label: &str) -> Result<(), String> {
    hasher.update(label.as_bytes());
    if !path.exists() {
        hasher.update(&[0]);
        return Ok(());
    }
    hasher.update(&[1]);
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    hasher.update(&(bytes.len() as u64).to_le_bytes());
    hasher.update(&bytes);
    Ok(())
}

fn hash_dir_contents(hasher: &mut Fnv64, dir: &Path, prefix: &Path) -> Result<(), String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|err| err.to_string())?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_string());
    for entry in entries {
        let path = entry.path();
        let rel = prefix.join(entry.file_name());
        if path.is_dir() {
            hash_dir_contents(hasher, &path, &rel)?;
        } else {
            let rel_s = rel.to_string_lossy();
            hash_path_contents(hasher, &path, &rel_s)?;
        }
    }
    Ok(())
}

fn dir_modified_secs(path: &Path) -> u64 {
    let mut latest = file_modified_secs(path);
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                latest = latest.max(dir_modified_secs(&child));
            } else {
                latest = latest.max(file_modified_secs(&child));
            }
        }
    }
    latest
}

fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst).map_err(|err| err.to_string())?;
    for entry in fs::read_dir(src).map_err(|err| err.to_string())? {
        let entry = entry.map_err(|err| err.to_string())?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

fn compress_archive(source_dir: &Path, archive_path: &Path) -> Result<(), String> {
    if let Some(parent) = archive_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let file = File::create(archive_path).map_err(|err| err.to_string())?;
    let mut writer = ZipWriter::new(file);
    let options = FileOptions::default()
        .compression_method(CompressionMethod::Deflated)
        .unix_permissions(0o644);
    zip_dir_entries(&mut writer, source_dir, source_dir, options)?;
    writer.finish().map_err(|err| err.to_string())?;
    Ok(())
}

fn expand_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir).map_err(|err| err.to_string())?;
    }
    fs::create_dir_all(dest_dir).map_err(|err| err.to_string())?;
    let file = File::open(archive_path).map_err(|err| err.to_string())?;
    let mut archive = ZipArchive::new(file).map_err(|err| err.to_string())?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|err| err.to_string())?;
        let Some(relative_path) = safe_zip_entry_path(entry.name()) else {
            return Err(format!(
                "{}{}",
                tr(
                    "云同步归档包含不安全路径：",
                    "Cloud sync archive contains an unsafe path: "
                ),
                entry.name()
            ));
        };
        let output_path = dest_dir.join(relative_path);
        if entry.is_dir() {
            fs::create_dir_all(&output_path).map_err(|err| err.to_string())?;
            continue;
        }
        if let Some(parent) = output_path.parent() {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let mut output = File::create(&output_path).map_err(|err| err.to_string())?;
        std::io::copy(&mut entry, &mut output).map_err(|err| err.to_string())?;
    }
    Ok(())
}

fn zip_dir_entries(
    writer: &mut ZipWriter<File>,
    root: &Path,
    dir: &Path,
    options: FileOptions,
) -> Result<(), String> {
    let mut entries = fs::read_dir(dir)
        .map_err(|err| err.to_string())?
        .filter_map(|entry| entry.ok())
        .collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name().to_string_lossy().to_string());
    for entry in entries {
        let path = entry.path();
        let rel_name = zip_relative_name(root, &path)?;
        if path.is_dir() {
            writer
                .add_directory(format!("{rel_name}/"), options)
                .map_err(|err| err.to_string())?;
            zip_dir_entries(writer, root, &path, options)?;
        } else {
            writer
                .start_file(rel_name, options)
                .map_err(|err| err.to_string())?;
            let mut input = File::open(&path).map_err(|err| err.to_string())?;
            let mut buffer = Vec::new();
            input
                .read_to_end(&mut buffer)
                .map_err(|err| err.to_string())?;
            writer.write_all(&buffer).map_err(|err| err.to_string())?;
        }
    }
    Ok(())
}

fn zip_relative_name(root: &Path, path: &Path) -> Result<String, String> {
    let relative = path.strip_prefix(root).map_err(|err| err.to_string())?;
    let name = relative
        .components()
        .filter_map(|component| match component {
            Component::Normal(part) => Some(part.to_string_lossy().to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("/");
    if name.is_empty() {
        Err("empty zip entry name".to_string())
    } else {
        Ok(name)
    }
}

fn safe_zip_entry_path(name: &str) -> Option<PathBuf> {
    let mut path = PathBuf::new();
    for component in Path::new(name).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            Component::ParentDir | Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    if path.as_os_str().is_empty() {
        None
    } else {
        Some(path)
    }
}

fn upload_file(
    config: &CloudSyncConfig,
    local_path: &Path,
    remote_url: &str,
) -> Result<(), String> {
    if !local_path.exists() {
        return Err(format!(
            "{}{}",
            tr("本地文件不存在：", "Local file was not found: "),
            local_path.to_string_lossy()
        ));
    }

    let status = run_webdav_curl_status(
        config,
        &[
            "-X".to_string(),
            "PUT".to_string(),
            "-T".to_string(),
            local_path.to_string_lossy().to_string(),
            "-o".to_string(),
            "NUL".to_string(),
            "-w".to_string(),
            "%{http_code}".to_string(),
            remote_url.to_string(),
        ],
    )?;

    match status.as_str() {
        "200" | "201" | "204" => Ok(()),
        _ => Err(format!(
            "{}{}",
            tr(
                "上传失败，HTTP 状态码：",
                "Upload failed with HTTP status: "
            ),
            status
        )),
    }
}

fn download_file(
    config: &CloudSyncConfig,
    remote_url: &str,
    local_path: &Path,
) -> Result<bool, String> {
    download_file_inner(config, remote_url, local_path, false)
}

fn download_optional_file(
    config: &CloudSyncConfig,
    remote_url: &str,
    local_path: &Path,
) -> Result<bool, String> {
    download_file_inner(config, remote_url, local_path, true)
}

fn download_file_inner(
    config: &CloudSyncConfig,
    remote_url: &str,
    local_path: &Path,
    allow_empty_success: bool,
) -> Result<bool, String> {
    if let Some(parent) = local_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let mut empty_success = false;
    for attempt in 0..WEBDAV_DOWNLOAD_ATTEMPTS {
        if attempt > 0 {
            let _ = fs::remove_file(local_path);
            std::thread::sleep(Duration::from_millis(150));
        }

        let status = run_webdav_curl_status(
            config,
            &[
                "-L".to_string(),
                "-o".to_string(),
                local_path.to_string_lossy().to_string(),
                "-w".to_string(),
                "%{http_code}".to_string(),
                remote_url.to_string(),
            ],
        )?;

        match status.as_str() {
            "200" | "206" => {
                let size = fs::metadata(local_path).map(|meta| meta.len()).unwrap_or(0);
                if size > 0 {
                    return Ok(true);
                }
                empty_success = true;
            }
            "404" => {
                let _ = fs::remove_file(local_path);
                return Ok(false);
            }
            _ => {
                return Err(format!(
                    "{}{}",
                    tr(
                        "下载失败，HTTP 状态码：",
                        "Download failed with HTTP status: "
                    ),
                    status
                ));
            }
        }
    }

    if empty_success {
        if allow_empty_success {
            return Ok(true);
        }
        let _ = fs::remove_file(local_path);
        return Err(tr(
            "下载失败，远端返回了空文件。",
            "Download failed because the remote file was empty.",
        )
        .to_string());
    }
    Ok(false)
}

fn webdav_mkcol(config: &CloudSyncConfig, remote_url: &str) -> Result<(), String> {
    let status = run_webdav_curl_status(
        config,
        &[
            "-X".to_string(),
            "MKCOL".to_string(),
            "-o".to_string(),
            "NUL".to_string(),
            "-w".to_string(),
            "%{http_code}".to_string(),
            remote_url.to_string(),
        ],
    )?;

    match status.as_str() {
        "200" | "201" | "204" | "301" | "302" | "405" | "409" => Ok(()),
        _ => Err(format!(
            "{}{}",
            tr(
                "创建云端目录失败，HTTP 状态码：",
                "Failed to create remote directory. HTTP status: "
            ),
            status
        )),
    }
}

fn build_webdav_args(extra: &[String]) -> Vec<String> {
    let mut args = vec![
        "--silent".to_string(),
        "--show-error".to_string(),
        "--connect-timeout".to_string(),
        "15".to_string(),
        "--max-time".to_string(),
        "300".to_string(),
    ];
    args.extend(extra.iter().cloned());
    args
}

fn run_webdav_curl_status(config: &CloudSyncConfig, extra: &[String]) -> Result<String, String> {
    let mut args = build_webdav_args(extra);
    let config_path = if !config.webdav_user.trim().is_empty() || !config.webdav_pass.is_empty() {
        let path = temp_unique_path("webdav_auth", "curl");
        let content = format!(
            "user = {}\n",
            curl_config_quote(&format!(
                "{}:{}",
                config.webdav_user.trim(),
                config.webdav_pass
            ))
        );
        fs::write(&path, content).map_err(|err| err.to_string())?;
        let path_arg = path.to_string_lossy().to_string();
        args.insert(0, path_arg);
        args.insert(0, "--config".to_string());
        Some(path)
    } else {
        None
    };
    let mut result = run_curl_status(&args);
    for _ in 0..5 {
        if !matches!(&result, Err(err) if is_transient_curl_recv_error(err)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(150));
        result = run_curl_status(&args);
    }
    if let Some(path) = config_path {
        let _ = fs::remove_file(path);
    }
    result
}

fn run_curl_status(args: &[String]) -> Result<String, String> {
    let output = hidden_curl()
        .args(args)
        .output()
        .map_err(|err| err.to_string())?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return Err(if stderr.is_empty() {
            tr("curl 执行失败。", "curl execution failed.").to_string()
        } else {
            stderr
        });
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

fn is_transient_curl_recv_error(err: &str) -> bool {
    let lower = err.to_ascii_lowercase();
    lower.contains("curl: (56)")
        || lower.contains("curl: (52)")
        || lower.contains("empty reply from server")
        || lower.contains("recv failure")
        || lower.contains("connection was reset")
        || lower.contains("connection was aborted")
}

fn temp_unique_path(prefix: &str, ext: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!(
        "zsclip_{}_{}_{}.{}",
        prefix,
        std::process::id(),
        ts,
        ext.trim_start_matches('.')
    ))
}

fn curl_config_quote(value: &str) -> String {
    let mut out = String::with_capacity(value.len() + 2);
    out.push('"');
    for ch in value.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            _ => out.push(ch),
        }
    }
    out.push('"');
    out
}

fn hidden_curl() -> Command {
    #[cfg(windows)]
    let mut cmd = Command::new("curl.exe");
    #[cfg(not(windows))]
    let mut cmd = Command::new("curl");
    #[cfg(windows)]
    cmd.creation_flags(CREATE_NO_WINDOW_FLAG);
    cmd
}

fn append_url_path(base: &str, part: &str) -> String {
    let mut url = base.trim_end_matches('/').to_string();
    for segment in part.split('/').filter(|segment| !segment.trim().is_empty()) {
        url.push('/');
        url.push_str(&percent_encode(segment.trim()));
    }
    url
}

fn percent_encode(value: &str) -> String {
    let mut encoded = String::new();
    for byte in value.as_bytes() {
        let keep = matches!(
            *byte,
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~'
        );
        if keep {
            encoded.push(*byte as char);
        } else {
            encoded.push('%');
            encoded.push_str(&format!("{byte:02X}"));
        }
    }
    encoded
}

fn temp_dir_path(prefix: &str) -> PathBuf {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    std::env::temp_dir().join(format!("zsclip-{prefix}-{}-{ts}", std::process::id()))
}

fn temp_file_path(prefix: &str, ext: &str) -> PathBuf {
    temp_unique_path(prefix, ext)
}

fn write_temp_json_file<T: Serialize>(prefix: &str, value: &T) -> Result<PathBuf, String> {
    let path = temp_file_path(prefix, "json");
    let raw = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;
    fs::write(&path, raw).map_err(|err| err.to_string())?;
    Ok(path)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn format_unix_ts(value: u64) -> String {
    if value == 0 {
        return tr("未知时间", "Unknown time").to_owned();
    }
    let (y, m, d, h, min, _) = utc_secs_to_local_parts(value as i64);
    format!("{:04}-{:02}-{:02} {:02}:{:02}", y, m, d, h, min)
}

fn compare_versions(left: &str, right: &str) -> std::cmp::Ordering {
    fn parse(value: &str) -> Vec<u32> {
        value
            .trim()
            .trim_start_matches(['v', 'V'])
            .split('.')
            .map(|part| part.parse::<u32>().ok().unwrap_or(0))
            .collect()
    }
    let mut a = parse(left);
    let mut b = parse(right);
    let max_len = a.len().max(b.len()).max(3);
    a.resize(max_len, 0);
    b.resize(max_len, 0);
    a.cmp(&b)
}

fn wal_file_for(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}-wal", path.to_string_lossy()))
}

fn shm_file_for(path: &Path) -> PathBuf {
    PathBuf::from(format!("{}-shm", path.to_string_lossy()))
}

fn remove_optional_file(path: &Path) {
    if path.exists() {
        let _ = fs::remove_file(path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::{TcpListener, TcpStream};
    use std::sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc, Mutex, MutexGuard, OnceLock,
    };
    use std::thread;
    use std::time::Duration;

    #[derive(Clone, Debug)]
    struct RecordedRequest {
        method: String,
        path: String,
        body: Vec<u8>,
    }

    fn cloud_sync_e2e_guard() -> MutexGuard<'static, ()> {
        static GUARD: OnceLock<Mutex<()>> = OnceLock::new();
        GUARD
            .get_or_init(|| Mutex::new(()))
            .lock()
            .expect("cloud sync e2e test lock poisoned")
    }

    #[test]
    fn bdd_webdav_layout_exposes_syncclipboard_contract() {
        let config = CloudSyncConfig {
            webdav_url: "https://dav.example.com/root/".to_string(),
            webdav_user: String::new(),
            webdav_pass: String::new(),
            remote_dir: "ZS Clip".to_string(),
        };

        let layout = RemoteLayout::from_config(&config).unwrap();

        assert_eq!(
            layout.sync_clipboard_url,
            "https://dav.example.com/root/ZS%20Clip/zsSyncClipboard.json"
        );
        assert_eq!(
            layout.sync_file_dir_url,
            "https://dav.example.com/root/ZS%20Clip/file"
        );
    }

    #[test]
    fn bdd_cloud_sync_interval_accepts_utf8_chinese_and_ascii_aliases() {
        assert_eq!(cloud_sync_interval("15分钟"), Duration::from_secs(15 * 60));
        assert_eq!(cloud_sync_interval("30min"), Duration::from_secs(30 * 60));
        assert_eq!(cloud_sync_interval("1小时"), Duration::from_secs(60 * 60));
        assert_eq!(cloud_sync_interval("6h"), Duration::from_secs(6 * 60 * 60));
        assert_eq!(
            cloud_sync_interval("12 hours"),
            Duration::from_secs(12 * 60 * 60)
        );
        assert_eq!(cloud_sync_interval("1d"), Duration::from_secs(24 * 60 * 60));
        assert_eq!(cloud_sync_interval("bad"), Duration::from_secs(60 * 60));
    }

    #[test]
    fn bdd_sync_now_imports_android_syncclipboard_and_uploads_local_manifest() {
        let _guard = cloud_sync_e2e_guard();
        let data_dir = temp_dir_path("cloud-sync-e2e-test");
        if data_dir.exists() {
            let _ = fs::remove_dir_all(&data_dir);
        }
        fs::create_dir_all(&data_dir).unwrap();
        let settings_file = data_dir.join("settings.json");
        let db_file = data_dir.join("clipboard.db");
        fs::write(&settings_file, "{}").unwrap();

        crate::db_runtime::with_test_db_path(&db_file, || {
            let server = FakeWebDavServer::start();
            let paths = CloudSyncPaths {
                data_dir: data_dir.clone(),
                settings_file: settings_file.clone(),
                db_file: db_file.clone(),
            };
            let config = CloudSyncConfig {
                webdav_url: format!("http://127.0.0.1:{}/root", server.port),
                webdav_user: String::new(),
                webdav_pass: String::new(),
                remote_dir: "ZS Clip".to_string(),
            };

            let outcome = perform_cloud_sync(CloudSyncAction::SyncNow, &config, &paths).unwrap();
            assert!(outcome.reload_data);

            let imported: (String, String, String) = crate::db_runtime::with_db(|conn| {
                conn.query_row(
                    "SELECT text_data, source_app, signature FROM items LIMIT 1",
                    [],
                    |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
                )
            })?;
            assert_eq!(imported.0, "hello from android");
            assert_eq!(imported.1, "WebDAV: Android");
            assert_eq!(
                imported.2,
                "multi:webdav:android:text:android-1:42:md5:a26920b53db734ce40db2d17a2ceb8c3:18"
            );

            let requests = server.requests();
            assert!(requests.iter().any(|req| {
                req.method == "GET" && req.path == "/root/ZS%20Clip/zsSyncClipboard.json"
            }));
            assert!(requests.iter().any(|req| {
                req.method == "PUT" && req.path == "/root/ZS%20Clip/zsSyncClipboard.json"
            }));
            let uploaded_manifest = requests
                .iter()
                .rev()
                .find(|req| {
                    req.method == "PUT" && req.path == "/root/ZS%20Clip/zsSyncClipboard.json"
                })
                .unwrap();
            let uploaded_json: serde_json::Value =
                serde_json::from_slice(&uploaded_manifest.body).unwrap();
            assert_eq!(
                uploaded_json["protocol"],
                crate::multi_sync::MULTI_SYNC_PROTOCOL
            );
            assert_eq!(uploaded_json["clip"]["content"], "hello from android");

            let backup_upload = requests
                .iter()
                .find(|req| req.method == "PUT" && req.path == "/root/ZS%20Clip/backups/latest.zip")
                .unwrap();
            assert_backup_contains_android_text(&backup_upload.body);

            server.stop();
            Ok(())
        })
        .unwrap();
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn bdd_sync_now_uploads_image_data_named_by_syncclipboard_manifest() {
        let _guard = cloud_sync_e2e_guard();
        let data_dir = temp_dir_path("cloud-sync-image-data-test");
        if data_dir.exists() {
            let _ = fs::remove_dir_all(&data_dir);
        }
        fs::create_dir_all(&data_dir).unwrap();
        let settings_file = data_dir.join("settings.json");
        let db_file = data_dir.join("clipboard.db");
        fs::write(&settings_file, "{}").unwrap();

        crate::db_runtime::with_test_db_path(&db_file, || {
            crate::db_runtime::with_db(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, image_data, image_width, image_height, source_app, pinned, group_id)
                     VALUES(0, 'image', 'webdav shot', 'img-sig', ?, 1, 1, 'test', 0, 0)",
                    [vec![255u8, 0, 0, 255]],
                )?;
                Ok(())
            })?;
            let server = FakeWebDavServer::start_without_remote_syncclipboard();
            let paths = CloudSyncPaths {
                data_dir: data_dir.clone(),
                settings_file: settings_file.clone(),
                db_file: db_file.clone(),
            };
            let config = CloudSyncConfig {
                webdav_url: format!("http://127.0.0.1:{}/root", server.port),
                webdav_user: String::new(),
                webdav_pass: String::new(),
                remote_dir: "ZS Clip".to_string(),
            };

            let outcome = perform_cloud_sync(CloudSyncAction::SyncNow, &config, &paths).unwrap();
            assert!(!outcome.reload_data);

            let requests = server.requests();
            let uploaded_manifest = requests
                .iter()
                .rev()
                .find(|req| req.method == "PUT" && req.path == "/root/ZS%20Clip/zsSyncClipboard.json")
                .unwrap();
            let uploaded_json: serde_json::Value =
                serde_json::from_slice(&uploaded_manifest.body).unwrap();
            assert_eq!(uploaded_json["clip"]["type"], "image");
            assert_eq!(uploaded_json["clip"]["dataName"], "zsclip_image_1.png");

            let image_upload = requests
                .iter()
                .find(|req| req.method == "PUT" && req.path == "/root/ZS%20Clip/file/zsclip_image_1.png")
                .unwrap();
            assert!(image_upload.body.starts_with(b"\x89PNG\r\n\x1a\n"));

            server.stop();
            Ok(())
        })
        .unwrap();
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn bdd_sync_now_imports_android_webdav_image_into_windows_history() {
        let _guard = cloud_sync_e2e_guard();
        let data_dir = temp_dir_path("cloud-sync-remote-image-test");
        if data_dir.exists() {
            let _ = fs::remove_dir_all(&data_dir);
        }
        fs::create_dir_all(&data_dir).unwrap();
        let settings_file = data_dir.join("settings.json");
        let db_file = data_dir.join("clipboard.db");
        fs::write(&settings_file, "{}").unwrap();

        crate::db_runtime::with_test_db_path(&db_file, || {
            let server = FakeWebDavServer::start_with_remote_syncclipboard(Some(
                android_image_syncclipboard_json(),
            ));
            let paths = CloudSyncPaths {
                data_dir: data_dir.clone(),
                settings_file: settings_file.clone(),
                db_file: db_file.clone(),
            };
            let config = CloudSyncConfig {
                webdav_url: format!("http://127.0.0.1:{}/root", server.port),
                webdav_user: String::new(),
                webdav_pass: String::new(),
                remote_dir: "ZS Clip".to_string(),
            };

            let outcome = perform_cloud_sync(CloudSyncAction::SyncNow, &config, &paths).unwrap();
            assert!(outcome.reload_data);

            let imported: (String, String, i64, i64, String) =
                crate::db_runtime::with_db(|conn| {
                    conn.query_row(
                        "SELECT kind, preview, image_width, image_height, source_app FROM items LIMIT 1",
                        [],
                        |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
                    )
                })?;
            assert_eq!(imported.0, "image");
            assert_eq!(imported.1, "android shot");
            assert_eq!((imported.2, imported.3), (1, 1));
            assert_eq!(imported.4, "WebDAV: Android");

            let requests = server.requests();
            assert!(requests.iter().any(|req| {
                req.method == "GET" && req.path == "/root/ZS%20Clip/file/zsclip_image_99.png"
            }));
            let uploaded_manifest = requests
                .iter()
                .rev()
                .find(|req| req.method == "PUT" && req.path == "/root/ZS%20Clip/zsSyncClipboard.json")
                .unwrap();
            let uploaded_json: serde_json::Value =
                serde_json::from_slice(&uploaded_manifest.body).unwrap();
            assert_eq!(uploaded_json["clip"]["type"], "image");
            assert_eq!(uploaded_json["clip"]["dataName"], "zsclip_image_1.png");
            assert!(requests.iter().any(|req| {
                req.method == "PUT"
                    && req.path == "/root/ZS%20Clip/file/zsclip_image_1.png"
                    && req.body.starts_with(b"\x89PNG\r\n\x1a\n")
            }));

            server.stop();
            Ok(())
        })
        .unwrap();
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn bdd_sync_now_skips_remote_image_manifest_when_payload_is_missing() {
        let _guard = cloud_sync_e2e_guard();
        let data_dir = temp_dir_path("cloud-sync-missing-remote-image-test");
        if data_dir.exists() {
            let _ = fs::remove_dir_all(&data_dir);
        }
        fs::create_dir_all(&data_dir).unwrap();
        let settings_file = data_dir.join("settings.json");
        let db_file = data_dir.join("clipboard.db");
        fs::write(&settings_file, "{}").unwrap();

        crate::db_runtime::with_test_db_path(&db_file, || {
            let server = FakeWebDavServer::start_with_missing_remote_image(Some(
                android_image_syncclipboard_json(),
            ));
            let paths = CloudSyncPaths {
                data_dir: data_dir.clone(),
                settings_file: settings_file.clone(),
                db_file: db_file.clone(),
            };
            let config = CloudSyncConfig {
                webdav_url: format!("http://127.0.0.1:{}/root", server.port),
                webdav_user: String::new(),
                webdav_pass: String::new(),
                remote_dir: "ZS Clip".to_string(),
            };

            let outcome = perform_cloud_sync(CloudSyncAction::SyncNow, &config, &paths).unwrap();
            assert!(!outcome.reload_data);

            let item_count: i64 = crate::db_runtime::with_db(|conn| {
                conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            })?;
            assert_eq!(item_count, 0);

            let requests = server.requests();
            assert!(requests.iter().any(|req| {
                req.method == "GET" && req.path == "/root/ZS%20Clip/file/zsclip_image_99.png"
            }));

            server.stop();
            Ok(())
        })
        .unwrap();
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn bdd_syncclipboard_download_retries_empty_success_body() {
        let _guard = cloud_sync_e2e_guard();
        let data_dir = temp_dir_path("cloud-sync-empty-syncclipboard-test");
        if data_dir.exists() {
            let _ = fs::remove_dir_all(&data_dir);
        }
        fs::create_dir_all(&data_dir).unwrap();
        let settings_file = data_dir.join("settings.json");
        let db_file = data_dir.join("clipboard.db");
        fs::write(&settings_file, "{}").unwrap();

        crate::db_runtime::with_test_db_path(&db_file, || {
            let server =
                FakeWebDavServer::start_with_empty_syncclipboard_once(android_syncclipboard_json());
            let paths = CloudSyncPaths {
                data_dir: data_dir.clone(),
                settings_file: settings_file.clone(),
                db_file: db_file.clone(),
            };
            let config = CloudSyncConfig {
                webdav_url: format!("http://127.0.0.1:{}/root", server.port),
                webdav_user: String::new(),
                webdav_pass: String::new(),
                remote_dir: "ZS Clip".to_string(),
            };

            let outcome = perform_cloud_sync(CloudSyncAction::SyncNow, &config, &paths).unwrap();
            assert!(outcome.reload_data);

            let preview: String = crate::db_runtime::with_db(|conn| {
                conn.query_row("SELECT preview FROM items LIMIT 1", [], |row| row.get(0))
            })?;
            assert_eq!(preview, "hello from android");

            let requests = server.requests();
            assert!(
                requests
                    .iter()
                    .filter(|req| req.method == "GET"
                        && req.path == "/root/ZS%20Clip/zsSyncClipboard.json")
                    .count()
                    >= 2
            );
            server.stop();
            Ok(())
        })
        .unwrap();
        let _ = fs::remove_dir_all(data_dir);
    }

    #[test]
    fn bdd_sync_now_treats_empty_remote_syncclipboard_as_no_lightweight_record() {
        let _guard = cloud_sync_e2e_guard();
        let data_dir = temp_dir_path("cloud-sync-empty-remote-syncclipboard-test");
        if data_dir.exists() {
            let _ = fs::remove_dir_all(&data_dir);
        }
        fs::create_dir_all(&data_dir).unwrap();
        let settings_file = data_dir.join("settings.json");
        let db_file = data_dir.join("clipboard.db");
        fs::write(&settings_file, "{}").unwrap();

        crate::db_runtime::with_test_db_path(&db_file, || {
            let server = FakeWebDavServer::start_with_remote_syncclipboard(Some(""));
            let paths = CloudSyncPaths {
                data_dir: data_dir.clone(),
                settings_file: settings_file.clone(),
                db_file: db_file.clone(),
            };
            let config = CloudSyncConfig {
                webdav_url: format!("http://127.0.0.1:{}/root", server.port),
                webdav_user: String::new(),
                webdav_pass: String::new(),
                remote_dir: "ZS Clip".to_string(),
            };

            let outcome = perform_cloud_sync(CloudSyncAction::SyncNow, &config, &paths).unwrap();
            assert!(!outcome.reload_data);

            let item_count: i64 = crate::db_runtime::with_db(|conn| {
                conn.query_row("SELECT COUNT(*) FROM items", [], |row| row.get(0))
            })?;
            assert_eq!(item_count, 0);

            let requests = server.requests();
            assert!(requests.iter().any(|req| {
                req.method == "GET" && req.path == "/root/ZS%20Clip/zsSyncClipboard.json"
            }));
            assert!(requests.iter().any(|req| {
                req.method == "PUT" && req.path == "/root/ZS%20Clip/zsSyncClipboard.json"
            }));

            server.stop();
            Ok(())
        })
        .unwrap();
        let _ = fs::remove_dir_all(data_dir);
    }

    struct FakeWebDavServer {
        port: u16,
        running: Arc<AtomicBool>,
        requests: Arc<Mutex<Vec<RecordedRequest>>>,
        handle: Option<thread::JoinHandle<()>>,
    }

    impl FakeWebDavServer {
        fn start() -> Self {
            Self::start_with_remote_syncclipboard(Some(android_syncclipboard_json()))
        }

        fn start_without_remote_syncclipboard() -> Self {
            Self::start_with_remote_syncclipboard(None)
        }

        fn start_with_remote_syncclipboard(remote_syncclipboard: Option<&'static str>) -> Self {
            Self::start_with_remote_syncclipboard_empty_first(remote_syncclipboard, 0)
        }

        fn start_with_missing_remote_image(remote_syncclipboard: Option<&'static str>) -> Self {
            Self::start_with_remote_syncclipboard_options(remote_syncclipboard, 0, false)
        }

        fn start_with_empty_syncclipboard_once(remote_syncclipboard: &'static str) -> Self {
            Self::start_with_remote_syncclipboard_empty_first(Some(remote_syncclipboard), 1)
        }

        fn start_with_remote_syncclipboard_empty_first(
            remote_syncclipboard: Option<&'static str>,
            empty_syncclipboard_count: usize,
        ) -> Self {
            Self::start_with_remote_syncclipboard_options(
                remote_syncclipboard,
                empty_syncclipboard_count,
                true,
            )
        }

        fn start_with_remote_syncclipboard_options(
            remote_syncclipboard: Option<&'static str>,
            empty_syncclipboard_count: usize,
            serve_remote_image: bool,
        ) -> Self {
            let listener = TcpListener::bind("127.0.0.1:0").unwrap();
            listener.set_nonblocking(true).unwrap();
            let port = listener.local_addr().unwrap().port();
            let running = Arc::new(AtomicBool::new(true));
            let requests = Arc::new(Mutex::new(Vec::new()));
            let empty_syncclipboard_count = Arc::new(AtomicUsize::new(empty_syncclipboard_count));
            let running_thread = running.clone();
            let requests_thread = requests.clone();
            let empty_syncclipboard_count_thread = empty_syncclipboard_count.clone();
            let handle = thread::spawn(move || {
                while running_thread.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => handle_request(
                            stream,
                            &requests_thread,
                            remote_syncclipboard,
                            &empty_syncclipboard_count_thread,
                            serve_remote_image,
                        ),
                        Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                            thread::sleep(Duration::from_millis(10));
                        }
                        Err(_) => break,
                    }
                }
            });
            Self {
                port,
                running,
                requests,
                handle: Some(handle),
            }
        }

        fn requests(&self) -> Vec<RecordedRequest> {
            self.requests.lock().unwrap().clone()
        }

        fn stop(mut self) {
            self.running.store(false, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    impl Drop for FakeWebDavServer {
        fn drop(&mut self) {
            self.running.store(false, Ordering::SeqCst);
            let _ = TcpStream::connect(("127.0.0.1", self.port));
            if let Some(handle) = self.handle.take() {
                let _ = handle.join();
            }
        }
    }

    fn handle_request(
        mut stream: TcpStream,
        requests: &Arc<Mutex<Vec<RecordedRequest>>>,
        remote_syncclipboard: Option<&'static str>,
        empty_syncclipboard_count: &Arc<AtomicUsize>,
        serve_remote_image: bool,
    ) {
        let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
        let mut request_line = String::new();
        let mut headers = Vec::new();
        let mut body = Vec::new();
        {
            let mut reader = BufReader::new(&mut stream);
            if reader.read_line(&mut request_line).is_err() || request_line.trim().is_empty() {
                return;
            }
            loop {
                let mut line = String::new();
                if reader.read_line(&mut line).is_err() || line == "\r\n" || line == "\n" {
                    break;
                }
                headers.push(line);
            }
            let content_len = headers
                .iter()
                .find_map(|line| {
                    let (name, value) = line.split_once(':')?;
                    name.eq_ignore_ascii_case("content-length")
                        .then(|| value.trim().parse::<usize>().ok())
                        .flatten()
                })
                .unwrap_or(0);
            let expects_continue = headers.iter().any(|line| {
                line.split_once(':')
                    .map(|(name, value)| {
                        name.eq_ignore_ascii_case("expect")
                            && value.trim().eq_ignore_ascii_case("100-continue")
                    })
                    .unwrap_or(false)
            });
            if expects_continue {
                let _ = reader.get_mut().write_all(b"HTTP/1.1 100 Continue\r\n\r\n");
                let _ = reader.get_mut().flush();
            }
            if content_len > 0 {
                body.resize(content_len, 0);
                let _ = reader.read_exact(&mut body);
            }
        }
        let mut parts = request_line.split_whitespace();
        let method = parts.next().unwrap_or("").to_string();
        let path = parts.next().unwrap_or("").to_string();
        requests.lock().unwrap().push(RecordedRequest {
            method: method.clone(),
            path: path.clone(),
            body: body.clone(),
        });

        let (status, response_body) = if method == "GET" && path.ends_with("/zsSyncClipboard.json")
        {
            if let Some(body) = remote_syncclipboard {
                if empty_syncclipboard_count
                    .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |count| {
                        count.checked_sub(1)
                    })
                    .is_ok()
                {
                    (200, Vec::new())
                } else {
                    (200, body.as_bytes().to_vec())
                }
            } else {
                (404, Vec::new())
            }
        } else if method == "GET"
            && path.ends_with("/file/zsclip_image_99.png")
            && serve_remote_image
        {
            (200, remote_android_image_png())
        } else if method == "GET" && path.ends_with("/manifest.json") {
            (404, Vec::new())
        } else if method == "MKCOL" || method == "PUT" {
            (201, Vec::new())
        } else {
            (404, Vec::new())
        };
        let response = format!(
            "HTTP/1.1 {status} OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
            response_body.len()
        );
        let _ = stream.write_all(response.as_bytes());
        let _ = stream.write_all(&response_body);
        let _ = stream.flush();
        thread::sleep(Duration::from_millis(5));
    }

    fn android_syncclipboard_json() -> &'static str {
        r#"{
          "protocol": "ZSCLIP_MULTI_SYNC_V1",
          "version": 1,
          "transport": "webdav",
          "clip": {
            "id": "android:text:android-1:42",
            "type": "text",
            "hash": "md5:a26920b53db734ce40db2d17a2ceb8c3",
            "preview": "hello from android",
            "content": "hello from android",
            "hasData": false,
            "size": 18,
            "source_app": "Android",
            "created_at": "42"
          }
        }"#
    }

    fn android_image_syncclipboard_json() -> &'static str {
        r#"{
          "protocol": "ZSCLIP_MULTI_SYNC_V1",
          "version": 1,
          "transport": "webdav",
          "clip": {
            "id": "android:image:android-1:99",
            "type": "image",
            "hash": "md5:png",
            "preview": "android shot",
            "content": null,
            "hasData": true,
            "dataName": "zsclip_image_99.png",
            "size": 70,
            "source_app": "Android",
            "created_at": "99"
          }
        }"#
    }

    fn remote_android_image_png() -> Vec<u8> {
        let mut out = Vec::new();
        {
            let mut encoder = png::Encoder::new(&mut out, 1, 1);
            encoder.set_color(png::ColorType::Rgba);
            encoder.set_depth(png::BitDepth::Eight);
            let mut writer = encoder.write_header().unwrap();
            writer.write_image_data(&[255, 0, 0, 255]).unwrap();
        }
        out
    }

    fn assert_backup_contains_android_text(zip_bytes: &[u8]) {
        let archive_path = temp_unique_path("cloud-sync-e2e-backup", "zip");
        fs::write(&archive_path, zip_bytes).unwrap();
        let extract_root = temp_dir_path("cloud-sync-e2e-extract");
        if extract_root.exists() {
            let _ = fs::remove_dir_all(&extract_root);
        }
        expand_archive(&archive_path, &extract_root).unwrap();
        let db_path = {
            let nested = extract_root.join("payload").join("clipboard.db");
            if nested.exists() {
                nested
            } else {
                extract_root.join("clipboard.db")
            }
        };
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let text: String = conn
            .query_row(
                "SELECT text_data FROM items WHERE source_app='WebDAV: Android' LIMIT 1",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(text, "hello from android");
        let _ = fs::remove_file(archive_path);
        let _ = fs::remove_dir_all(extract_root);
    }
}
