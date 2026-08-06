use crate::app_version::APP_VERSION;
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

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CloudSyncTempCleanup {
    pub files_removed: usize,
    pub dirs_removed: usize,
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

struct SnapshotArchive {
    path: PathBuf,
    settings_copy: PathBuf,
    content_hash: String,
    state_stamp: u64,
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
    let _ = cleanup_cloud_sync_temp_files();
    let remote = RemoteLayout::from_config(config)?;
    match action {
        CloudSyncAction::SyncNow => sync_snapshot(config, &remote, paths),
        CloudSyncAction::UploadConfig => upload_config(config, &remote, paths),
        CloudSyncAction::ApplyRemoteConfig => apply_remote_config(config, &remote, paths),
        CloudSyncAction::RestoreBackup => restore_remote_backup(config, &remote, paths),
    }
}

pub fn cleanup_cloud_sync_temp_files() -> CloudSyncTempCleanup {
    let mut report = cleanup_cloud_sync_temp_files_in_dir(&cloud_sync_temp_root());
    let old_report = cleanup_cloud_sync_temp_files_in_dir(&std::env::temp_dir());
    report.files_removed += old_report.files_removed;
    report.dirs_removed += old_report.dirs_removed;
    report
}

fn cleanup_cloud_sync_temp_files_in_dir(dir: &Path) -> CloudSyncTempCleanup {
    let mut report = CloudSyncTempCleanup::default();
    let Ok(entries) = fs::read_dir(dir) else {
        return report;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if is_cloud_sync_temp_file_name(name) {
            if fs::remove_file(&path).is_ok() {
                report.files_removed += 1;
            }
        } else if is_cloud_sync_temp_dir_name(name) && fs::remove_dir_all(&path).is_ok() {
            report.dirs_removed += 1;
        }
    }
    report
}

fn is_cloud_sync_temp_file_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    (lower.ends_with(".zip")
        && (lower.starts_with("zsclip-cloud-")
            || lower.starts_with("zsclip_cloud_")
            || lower.starts_with("zsclip_cloud-")
            || lower.starts_with("zsclip_before-restore_")))
        || (lower.ends_with(".json") && lower.starts_with("zsclip_snapshot-settings_"))
}

fn is_cloud_sync_temp_dir_name(name: &str) -> bool {
    let lower = name.to_ascii_lowercase();
    lower.starts_with("zsclip-snapshot-staging-")
        || lower.starts_with("zsclip-snapshot-restore-")
        || lower.starts_with("zsclip-local-restore-backup-staging-")
}

fn sync_snapshot(
    config: &CloudSyncConfig,
    remote: &RemoteLayout,
    paths: &CloudSyncPaths,
) -> Result<CloudSyncOutcome, String> {
    let imported_light_clip = crate::db_runtime::with_shared_app_data(|| {
        import_remote_syncclipboard_clip(config, remote)
    })?;
    let snapshot = crate::db_runtime::with_exclusive_app_data_snapshot(|| {
        crate::db_runtime::with_exclusive_db_snapshot(&paths.db_file, || {
            create_snapshot_archive(paths)
        })
    })?;
    let _archive_guard = TempPathGuard::file(snapshot.path.clone());
    let _settings_guard = TempPathGuard::file(snapshot.settings_copy.clone());
    let local_stamp = snapshot.state_stamp;
    let local_hash = snapshot.content_hash.clone();
    let remote_manifest = download_remote_manifest(config, remote)?;
    if let Some(manifest) = remote_manifest {
        let version_cmp = compare_versions(&manifest.version, APP_VERSION);
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
    ensure_remote_layout(config, remote)?;
    upload_file(config, &snapshot.path, &remote.backup_url)?;
    upload_file(config, &snapshot.settings_copy, &remote.settings_url)?;
    let manifest = CloudSyncManifest {
        version: APP_VERSION.to_string(),
        updated_at: stamp,
        snapshot_hash: local_hash,
        backup_name: BACKUP_FILE_NAME.to_string(),
    };
    let manifest_path = write_temp_json_file("manifest", &manifest)?;
    let _manifest_guard = TempPathGuard::file(manifest_path.clone());
    upload_file(config, &manifest_path, &remote.manifest_url)?;
    upload_syncclipboard_manifest(config, remote)?;
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
    let settings_copy = temp_unique_path("config-upload", "json");
    let _settings_guard = TempPathGuard::file(settings_copy.clone());
    crate::db_runtime::with_exclusive_app_data_snapshot(|| {
        if !paths.settings_file.exists() {
            return Err("本地设置文件不存在，无法上传。".to_string());
        }
        validate_settings_json(&paths.settings_file)?;
        fs::copy(&paths.settings_file, &settings_copy)
            .map_err(|err| format!("无法暂存本地设置：{err}"))?;
        sync_file(&settings_copy).map_err(|err| format!("无法同步本地设置副本：{err}"))?;
        validate_settings_json(&settings_copy)
    })?;
    ensure_remote_layout(config, remote)?;
    upload_file(config, &settings_copy, &remote.settings_url)?;
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
    let _download_guard = TempPathGuard::file(download_path.clone());
    if !download_file(config, &remote.settings_url, &download_path)? {
        return Err("云端没有找到 settings.json。".to_string());
    }
    validate_settings_json(&download_path)?;
    with_app_data_replacement_for_paths(paths, || {
        if let Some(parent) = paths.settings_file.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("无法创建设置目录 {}：{err}", parent.to_string_lossy()))?;
        }
        replace_settings_file_transactionally(&download_path, &paths.settings_file)
    })?;
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
    let manifest = download_remote_manifest(config, remote)?;
    if let Some(manifest) = manifest.as_ref() {
        if compare_versions(&manifest.version, APP_VERSION).is_gt() {
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
        if manifest.backup_name.trim() != BACKUP_FILE_NAME {
            return Err(format!(
                "不支持云端备份文件名：{}（当前仅支持 {BACKUP_FILE_NAME}）。",
                manifest.backup_name
            ));
        }
    }
    let download_path = temp_file_path("cloud-backup", "zip");
    let _download_guard = TempPathGuard::file(download_path.clone());
    if !download_file(config, &remote.backup_url, &download_path)? {
        return Err("云端没有找到可恢复的备份。".to_string());
    }
    let expected_hash = manifest
        .as_ref()
        .map(|manifest| manifest.snapshot_hash.trim())
        .filter(|hash| !hash.is_empty());
    let mut staged_restore = stage_snapshot_restore(paths, &download_path, expected_hash)?;
    let local_backup = with_app_data_replacement_for_paths(paths, || {
        crate::db_runtime::with_exclusive_db_file_replacement(&paths.db_file, || {
            let local_backup = create_local_restore_backup(paths)?;
            if let Err(err) = commit_staged_restore(paths, &mut staged_restore) {
                let recovery_note = local_backup
                    .as_ref()
                    .map(|path| format!("；恢复前本地备份保存在：{}", path.to_string_lossy()))
                    .unwrap_or_default();
                return Err(format!("{err}{recovery_note}"));
            }
            Ok(local_backup)
        })
    })?;
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

fn with_app_data_replacement_for_paths<T, F>(
    paths: &CloudSyncPaths,
    replace: F,
) -> Result<T, String>
where
    F: FnOnce() -> Result<T, String>,
{
    if crate::db_runtime::is_runtime_db_file(&paths.db_file) {
        crate::db_runtime::with_exclusive_app_data_replacement(replace)
    } else {
        crate::db_runtime::with_exclusive_app_data_snapshot(replace)
    }
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

fn validate_settings_json(path: &Path) -> Result<(), String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("无法读取设置文件 {}：{err}", path.to_string_lossy()))?;
    let parsed = serde_json::from_str::<serde_json::Value>(&raw)
        .map_err(|err| format!("设置文件不是有效 JSON：{err}"))?;
    if !parsed.is_object() {
        return Err("设置文件必须是 JSON 对象。".to_string());
    }
    Ok(())
}

fn create_snapshot_archive(paths: &CloudSyncPaths) -> Result<SnapshotArchive, String> {
    let staging_root = temp_dir_path("snapshot-staging");
    if staging_root.exists() {
        let _ = fs::remove_dir_all(&staging_root);
    }
    let staging_guard = TempPathGuard::dir(staging_root.clone());
    let payload_dir = staging_root.join("payload");
    fs::create_dir_all(&payload_dir).map_err(|err| err.to_string())?;

    if !paths.settings_file.is_file() {
        return Err("无法创建云备份：本地缺少 settings.json。".to_string());
    }
    validate_settings_json(&paths.settings_file)?;
    if !paths.db_file.is_file() {
        return Err("无法创建云备份：本地缺少 clipboard.db。".to_string());
    }
    fs::copy(&paths.settings_file, payload_dir.join("settings.json"))
        .map_err(|err| err.to_string())?;
    validate_settings_json(&payload_dir.join("settings.json"))?;
    fs::copy(&paths.db_file, payload_dir.join("clipboard.db")).map_err(|err| err.to_string())?;
    let images_dir = paths.data_dir.join("images");
    if images_dir.exists() {
        copy_dir_recursive(&images_dir, &payload_dir.join("images"))?;
    }

    let archive_path = temp_unique_path("cloud", "zip");
    let payload_paths = CloudSyncPaths {
        data_dir: payload_dir.clone(),
        settings_file: payload_dir.join("settings.json"),
        db_file: payload_dir.join("clipboard.db"),
    };
    let content_hash = local_state_hash(&payload_paths)?;
    let state_stamp = local_state_stamp(paths);
    let archive_guard = TempPathGuard::file(archive_path.clone());
    let settings_copy = temp_unique_path("snapshot-settings", "json");
    let settings_copy_guard = TempPathGuard::file(settings_copy.clone());
    fs::copy(payload_dir.join("settings.json"), &settings_copy)
        .map_err(|err| format!("无法保留云备份设置副本：{err}"))?;
    sync_file(&settings_copy).map_err(|err| format!("无法同步云备份设置副本：{err}"))?;
    compress_archive(&payload_dir, &archive_path)?;
    archive_guard.dismiss();
    settings_copy_guard.dismiss();
    drop(staging_guard);
    Ok(SnapshotArchive {
        path: archive_path,
        settings_copy,
        content_hash,
        state_stamp,
    })
}

fn create_local_backup_archive(paths: &CloudSyncPaths) -> Result<PathBuf, String> {
    let staging_root = temp_dir_path("local-restore-backup-staging");
    let staging_guard = TempPathGuard::dir(staging_root.clone());
    let payload_dir = staging_root.join("payload");
    fs::create_dir_all(&payload_dir).map_err(|err| err.to_string())?;
    if paths.settings_file.is_file() {
        fs::copy(&paths.settings_file, payload_dir.join("settings.json"))
            .map_err(|err| err.to_string())?;
    }
    if paths.db_file.is_file() {
        fs::copy(&paths.db_file, payload_dir.join("clipboard.db"))
            .map_err(|err| err.to_string())?;
    }
    let images_dir = paths.data_dir.join("images");
    if images_dir.is_dir() {
        copy_dir_recursive(&images_dir, &payload_dir.join("images"))?;
    }
    let archive_path = temp_unique_path("before-restore", "zip");
    let archive_guard = TempPathGuard::file(archive_path.clone());
    compress_archive(&payload_dir, &archive_path)?;
    archive_guard.dismiss();
    drop(staging_guard);
    Ok(archive_path)
}

fn create_local_restore_backup(paths: &CloudSyncPaths) -> Result<Option<PathBuf>, String> {
    if !local_data_exists(paths) {
        return Ok(None);
    }
    let stamp = local_state_stamp(paths).max(unix_now());
    let temp_archive = create_local_backup_archive(paths)?;
    let temp_guard = TempPathGuard::file(temp_archive.clone());
    let backup_dir = paths.data_dir.join("restore-backups");
    fs::create_dir_all(&backup_dir).map_err(|err| err.to_string())?;
    let mut final_path = backup_dir.join(format!("before-restore-{stamp}.zip"));
    let mut suffix = 1u32;
    while final_path.exists() {
        final_path = backup_dir.join(format!("before-restore-{stamp}-{suffix}.zip"));
        suffix = suffix.saturating_add(1);
    }
    fs::rename(&temp_archive, &final_path)
        .or_else(|_| {
            fs::copy(&temp_archive, &final_path)
                .map(|_| ())
                .and_then(|_| fs::remove_file(&temp_archive))
        })
        .map_err(|err| err.to_string())?;
    temp_guard.dismiss();
    Ok(Some(final_path))
}

fn stage_snapshot_restore(
    paths: &CloudSyncPaths,
    archive_path: &Path,
    expected_hash: Option<&str>,
) -> Result<StagedRestore, String> {
    let extract_root = temp_dir_path("snapshot-restore");
    if extract_root.exists() {
        let _ = fs::remove_dir_all(&extract_root);
    }
    let extract_guard = TempPathGuard::dir(extract_root.clone());
    fs::create_dir_all(&extract_root).map_err(|err| err.to_string())?;
    expand_archive(archive_path, &extract_root)?;
    let payload_dir = extract_root.join("payload");
    let source_dir = if payload_dir.exists() {
        payload_dir
    } else {
        extract_root.clone()
    };
    if !source_dir.is_dir() {
        return Err("云备份缺少有效的 payload 目录。".to_string());
    }

    let settings_src = source_dir.join("settings.json");
    if !settings_src.is_file() {
        return Err("云备份缺少 settings.json。".to_string());
    }
    validate_settings_json(&settings_src)?;
    let db_src = source_dir.join("clipboard.db");
    if !db_src.is_file() {
        return Err("云备份缺少 clipboard.db。".to_string());
    }
    let images_src = source_dir.join("images");
    if images_src.exists() && !images_src.is_dir() {
        return Err("云备份中的 images 不是目录。".to_string());
    }

    let payload_paths = CloudSyncPaths {
        data_dir: source_dir.clone(),
        settings_file: settings_src.clone(),
        db_file: db_src.clone(),
    };
    let payload_hash = local_state_hash(&payload_paths)?;
    if let Some(expected_hash) = expected_hash {
        if !payload_hash.eq_ignore_ascii_case(expected_hash.trim()) {
            return Err(format!(
                "云备份内容校验失败：清单哈希为 {}，实际为 {payload_hash}。",
                expected_hash.trim()
            ));
        }
    }

    for parent in [paths.settings_file.parent(), paths.db_file.parent()] {
        if let Some(parent) = parent {
            fs::create_dir_all(parent).map_err(|err| {
                format!("无法创建恢复暂存目录 {}：{err}", parent.to_string_lossy())
            })?;
        }
    }
    fs::create_dir_all(&paths.data_dir).map_err(|err| {
        format!(
            "无法创建恢复数据目录 {}：{err}",
            paths.data_dir.to_string_lossy()
        )
    })?;

    let token = restore_token();
    let active_images_dir = paths.data_dir.join("images");
    let settings_staged = restore_side_path(&paths.settings_file, "stage", &token);
    let db_staged = restore_side_path(&paths.db_file, "stage", &token);
    let images_staged = restore_side_path(&active_images_dir, "stage", &token);
    remove_path_if_exists(&settings_staged, RestoreArtifactKind::File)?;
    remove_path_if_exists(&db_staged, RestoreArtifactKind::File)?;
    remove_path_if_exists(&images_staged, RestoreArtifactKind::Dir)?;

    let settings_guard = TempPathGuard::file(settings_staged.clone());
    let db_guard = TempPathGuard::file(db_staged.clone());
    let images_guard = TempPathGuard::dir(images_staged.clone());
    fs::copy(&settings_src, &settings_staged).map_err(|err| format!("无法暂存恢复设置：{err}"))?;
    sync_file(&settings_staged).map_err(|err| format!("无法写入恢复设置暂存文件：{err}"))?;
    validate_settings_json(&settings_staged)?;
    fs::copy(&db_src, &db_staged).map_err(|err| format!("无法暂存恢复数据库：{err}"))?;
    sync_file(&db_staged).map_err(|err| format!("无法写入恢复数据库暂存文件：{err}"))?;
    fs::create_dir_all(&images_staged).map_err(|err| format!("无法创建恢复图片暂存目录：{err}"))?;
    if images_src.is_dir() {
        copy_dir_recursive(&images_src, &images_staged)?;
    }
    crate::db_runtime::prepare_restored_database(&db_staged, &images_staged, &active_images_dir)?;
    sync_file(&db_staged).map_err(|err| format!("无法同步恢复数据库暂存文件：{err}"))?;

    settings_guard.dismiss();
    db_guard.dismiss();
    images_guard.dismiss();
    drop(extract_guard);
    Ok(StagedRestore {
        token,
        settings: settings_staged,
        database: db_staged,
        images: images_staged,
        preserve_materials: false,
    })
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
    let root = cloud_sync_temp_root();
    let _ = fs::create_dir_all(&root);
    root.join(format!(
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
    let root = cloud_sync_temp_root();
    let _ = fs::create_dir_all(&root);
    root.join(format!("zsclip-{prefix}-{}-{ts}", std::process::id()))
}

fn temp_file_path(prefix: &str, ext: &str) -> PathBuf {
    temp_unique_path(prefix, ext)
}

fn cloud_sync_temp_root() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|path| {
            path.parent()
                .map(|dir| dir.join("data").join("temp").join("cloud_sync"))
        })
        .unwrap_or_else(|| std::env::temp_dir().join("zsclip").join("cloud_sync"))
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

fn remove_optional_file(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("无法删除 {}：{err}", path.to_string_lossy())),
    }
}

fn sync_file(path: &Path) -> std::io::Result<()> {
    fs::OpenOptions::new().write(true).open(path)?.sync_all()
}

fn restore_token() -> String {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or(0);
    format!("{}-{nonce}", std::process::id())
}

fn restore_side_path(path: &Path, role: &str, token: &str) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("zsclip-data");
    path.parent()
        .unwrap_or_else(|| Path::new("."))
        .join(format!(".{file_name}.restore-{token}.{role}"))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum RestoreArtifactKind {
    File,
    Dir,
}

fn remove_path_if_exists(path: &Path, kind: RestoreArtifactKind) -> Result<(), String> {
    let result = match kind {
        RestoreArtifactKind::File => fs::remove_file(path),
        RestoreArtifactKind::Dir => fs::remove_dir_all(path),
    };
    match result {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("无法删除 {}：{err}", path.to_string_lossy())),
    }
}

#[derive(Debug)]
struct StagedRestore {
    token: String,
    settings: PathBuf,
    database: PathBuf,
    images: PathBuf,
    preserve_materials: bool,
}

impl StagedRestore {
    fn preserve(&mut self) {
        self.preserve_materials = true;
    }

    fn material_paths(&self, paths: &CloudSyncPaths) -> Vec<PathBuf> {
        let images_destination = paths.data_dir.join("images");
        let destinations = [
            (
                &self.settings,
                &paths.settings_file,
                RestoreArtifactKind::File,
            ),
            (&self.images, &images_destination, RestoreArtifactKind::Dir),
            (&self.database, &paths.db_file, RestoreArtifactKind::File),
        ];
        let mut materials = Vec::new();
        for (staged, destination, _) in destinations {
            materials.push(staged.clone());
            materials.push(restore_side_path(destination, "backup", &self.token));
            materials.push(restore_side_path(destination, "recovery", &self.token));
        }
        materials
    }
}

impl Drop for StagedRestore {
    fn drop(&mut self) {
        if self.preserve_materials {
            return;
        }
        let _ = remove_path_if_exists(&self.settings, RestoreArtifactKind::File);
        let _ = remove_path_if_exists(&self.database, RestoreArtifactKind::File);
        let _ = remove_path_if_exists(&self.images, RestoreArtifactKind::Dir);
    }
}

struct RestoreArtifact {
    label: &'static str,
    kind: RestoreArtifactKind,
    staged: PathBuf,
    destination: PathBuf,
    backup: PathBuf,
    recovery: PathBuf,
    original_existed: bool,
    committed: bool,
}

impl RestoreArtifact {
    fn new(
        label: &'static str,
        kind: RestoreArtifactKind,
        staged: PathBuf,
        destination: PathBuf,
        token: &str,
    ) -> Self {
        let backup = restore_side_path(&destination, "backup", token);
        let recovery = restore_side_path(&destination, "recovery", token);
        Self {
            label,
            kind,
            staged,
            destination,
            backup,
            recovery,
            original_existed: false,
            committed: false,
        }
    }
}

struct RestoreSwapFailure {
    message: String,
    rollback_confirmed: bool,
}

#[cfg(windows)]
fn replace_file_with_backup(
    staged: &Path,
    destination: &Path,
    backup: &Path,
) -> std::io::Result<()> {
    if !destination.exists() {
        return fs::rename(staged, destination);
    }

    use std::os::windows::ffi::OsStrExt;

    #[link(name = "Kernel32")]
    extern "system" {
        fn ReplaceFileW(
            replaced_file_name: *const u16,
            replacement_file_name: *const u16,
            backup_file_name: *const u16,
            replace_flags: u32,
            exclude: *mut std::ffi::c_void,
            reserved: *mut std::ffi::c_void,
        ) -> i32;
    }

    let destination_wide = destination
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let staged_wide = staged
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let backup_wide = backup
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let replaced = unsafe {
        ReplaceFileW(
            destination_wide.as_ptr(),
            staged_wide.as_ptr(),
            backup_wide.as_ptr(),
            0,
            std::ptr::null_mut(),
            std::ptr::null_mut(),
        )
    };
    if replaced == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

fn recover_failed_file_swap(artifact: &mut RestoreArtifact) -> Result<(), String> {
    if artifact.backup.exists() {
        remove_path_if_exists(&artifact.recovery, artifact.kind)?;
        if artifact.destination.exists() {
            fs::rename(&artifact.destination, &artifact.recovery).map_err(|err| {
                format!(
                    "旧数据已在 {}，但无法保全失败后的新文件 {}：{err}",
                    artifact.backup.to_string_lossy(),
                    artifact.destination.to_string_lossy()
                )
            })?;
        }
        if let Err(err) = fs::rename(&artifact.backup, &artifact.destination) {
            if artifact.recovery.exists() && !artifact.destination.exists() {
                let _ = fs::rename(&artifact.recovery, &artifact.destination);
            }
            return Err(format!(
                "无法把旧数据 {} 恢复到 {}：{err}",
                artifact.backup.to_string_lossy(),
                artifact.destination.to_string_lossy()
            ));
        }
        if artifact.recovery.exists() && !artifact.staged.exists() {
            let _ = fs::rename(&artifact.recovery, &artifact.staged);
        }
        return Ok(());
    }

    if artifact.destination.exists() {
        // ReplaceFileW was given an explicit backup path. If no backup was
        // produced and the destination still exists, Windows retained the old
        // destination name and the active file is safe.
        return Ok(());
    }
    Err(format!(
        "无法确认 {} 的旧数据位置；暂存文件：{}，备份文件：{}",
        artifact.label,
        artifact.staged.to_string_lossy(),
        artifact.backup.to_string_lossy()
    ))
}

fn swap_restore_artifact(artifact: &mut RestoreArtifact) -> Result<(), RestoreSwapFailure> {
    if !artifact.staged.exists() {
        return Err(RestoreSwapFailure {
            message: format!(
                "{} 暂存材料不存在：{}",
                artifact.label,
                artifact.staged.display()
            ),
            rollback_confirmed: true,
        });
    }
    if let Err(err) = remove_path_if_exists(&artifact.backup, artifact.kind) {
        return Err(RestoreSwapFailure {
            message: err,
            rollback_confirmed: true,
        });
    }
    if let Err(err) = remove_path_if_exists(&artifact.recovery, artifact.kind) {
        return Err(RestoreSwapFailure {
            message: err,
            rollback_confirmed: true,
        });
    }
    artifact.original_existed = artifact.destination.exists();

    #[cfg(windows)]
    if artifact.kind == RestoreArtifactKind::File && artifact.original_existed {
        if let Err(err) =
            replace_file_with_backup(&artifact.staged, &artifact.destination, &artifact.backup)
        {
            let recovery = recover_failed_file_swap(artifact);
            let rollback_confirmed = recovery.is_ok();
            return Err(RestoreSwapFailure {
                message: match recovery {
                    Ok(()) => format!("替换 {} 失败，旧数据已恢复：{err}", artifact.label),
                    Err(recovery_err) => {
                        format!("替换 {} 失败：{err}；{recovery_err}", artifact.label)
                    }
                },
                rollback_confirmed,
            });
        }
        artifact.committed = true;
        return Ok(());
    }

    if artifact.original_existed {
        if let Err(err) = fs::rename(&artifact.destination, &artifact.backup) {
            return Err(RestoreSwapFailure {
                message: format!("无法备份现有 {}：{err}", artifact.label),
                rollback_confirmed: true,
            });
        }
    }
    if let Err(err) = fs::rename(&artifact.staged, &artifact.destination) {
        let rollback_confirmed = if artifact.original_existed {
            fs::rename(&artifact.backup, &artifact.destination).is_ok()
        } else {
            true
        };
        return Err(RestoreSwapFailure {
            message: format!("无法启用暂存 {}：{err}", artifact.label),
            rollback_confirmed,
        });
    }
    artifact.committed = true;
    Ok(())
}

fn replace_settings_file_transactionally(source: &Path, destination: &Path) -> Result<(), String> {
    let token = restore_token();
    let staged = restore_side_path(destination, "stage", &token);
    remove_path_if_exists(&staged, RestoreArtifactKind::File)?;
    let staged_guard = TempPathGuard::file(staged.clone());
    fs::copy(source, &staged).map_err(|err| format!("无法暂存云端设置：{err}"))?;
    sync_file(&staged).map_err(|err| format!("无法同步云端设置暂存文件：{err}"))?;
    validate_settings_json(&staged)?;

    let mut artifact = RestoreArtifact::new(
        "设置文件",
        RestoreArtifactKind::File,
        staged.clone(),
        destination.to_path_buf(),
        &token,
    );
    match swap_restore_artifact(&mut artifact) {
        Ok(()) => {
            cleanup_restore_artifact_materials(&artifact);
            Ok(())
        }
        Err(failure) if failure.rollback_confirmed => {
            cleanup_restore_artifact_materials(&artifact);
            Err(failure.message)
        }
        Err(failure) => {
            staged_guard.dismiss();
            let materials = [&artifact.staged, &artifact.backup, &artifact.recovery]
                .into_iter()
                .filter(|path| path.exists())
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            Err(if materials.is_empty() {
                failure.message
            } else {
                format!(
                    "{}；已保留设置恢复材料：{}",
                    failure.message,
                    materials.join("，")
                )
            })
        }
    }
}

fn rollback_restore_artifact(artifact: &mut RestoreArtifact) -> Result<(), String> {
    if !artifact.committed {
        return Ok(());
    }
    remove_path_if_exists(&artifact.recovery, artifact.kind)?;
    if artifact.original_existed {
        if !artifact.backup.exists() {
            return Err(format!(
                "{} 的回滚备份不存在：{}",
                artifact.label,
                artifact.backup.display()
            ));
        }
        if artifact.destination.exists() {
            fs::rename(&artifact.destination, &artifact.recovery)
                .map_err(|err| format!("无法保全已提交的 {}：{err}", artifact.label))?;
        }
        if let Err(err) = fs::rename(&artifact.backup, &artifact.destination) {
            if artifact.recovery.exists() && !artifact.destination.exists() {
                let _ = fs::rename(&artifact.recovery, &artifact.destination);
            }
            return Err(format!("无法回滚 {}：{err}", artifact.label));
        }
        if artifact.recovery.exists() && !artifact.staged.exists() {
            let _ = fs::rename(&artifact.recovery, &artifact.staged);
        }
    } else if artifact.destination.exists() {
        let target = if artifact.staged.exists() {
            &artifact.recovery
        } else {
            &artifact.staged
        };
        fs::rename(&artifact.destination, target)
            .map_err(|err| format!("无法回滚新建的 {}：{err}", artifact.label))?;
    }
    artifact.committed = false;
    Ok(())
}

fn cleanup_restore_artifact_materials(artifact: &RestoreArtifact) {
    let _ = remove_path_if_exists(&artifact.backup, artifact.kind);
    let _ = remove_path_if_exists(&artifact.recovery, artifact.kind);
}

fn commit_staged_restore(paths: &CloudSyncPaths, staged: &mut StagedRestore) -> Result<(), String> {
    // The exclusive DB gate has already closed every internal connection and
    // verified a complete WAL checkpoint before this function is called.
    remove_optional_file(&wal_file_for(&paths.db_file))?;
    remove_optional_file(&shm_file_for(&paths.db_file))?;

    let images_destination = paths.data_dir.join("images");
    let mut artifacts = vec![
        RestoreArtifact::new(
            "设置文件",
            RestoreArtifactKind::File,
            staged.settings.clone(),
            paths.settings_file.clone(),
            &staged.token,
        ),
        RestoreArtifact::new(
            "图片目录",
            RestoreArtifactKind::Dir,
            staged.images.clone(),
            images_destination,
            &staged.token,
        ),
        // Commit the database last. Once this succeeds there are no remaining
        // fallible activation steps, so callers never observe a new DB with an
        // error result and stale in-memory data.
        RestoreArtifact::new(
            "数据库",
            RestoreArtifactKind::File,
            staged.database.clone(),
            paths.db_file.clone(),
            &staged.token,
        ),
    ];

    for index in 0..artifacts.len() {
        if let Err(failure) = swap_restore_artifact(&mut artifacts[index]) {
            let mut rollback_errors = Vec::new();
            for rollback_index in (0..index).rev() {
                if let Err(err) = rollback_restore_artifact(&mut artifacts[rollback_index]) {
                    rollback_errors.push(err);
                }
            }
            let rollback_confirmed = failure.rollback_confirmed && rollback_errors.is_empty();
            if rollback_confirmed {
                for artifact in &artifacts {
                    cleanup_restore_artifact_materials(artifact);
                }
                return Err(failure.message);
            }

            staged.preserve();
            let mut message = failure.message;
            if !rollback_errors.is_empty() {
                message.push_str("；回滚失败：");
                message.push_str(&rollback_errors.join("；"));
            }
            let materials = staged
                .material_paths(paths)
                .into_iter()
                .filter(|path| path.exists())
                .map(|path| path.to_string_lossy().to_string())
                .collect::<Vec<_>>();
            if !materials.is_empty() {
                message.push_str("；已保留恢复材料：");
                message.push_str(&materials.join("，"));
            }
            return Err(message);
        }
    }

    for artifact in &artifacts {
        cleanup_restore_artifact_materials(artifact);
    }
    Ok(())
}

struct TempPathGuard {
    path: Option<PathBuf>,
    kind: TempPathKind,
}

#[derive(Clone, Copy)]
enum TempPathKind {
    File,
    Dir,
}

impl TempPathGuard {
    fn file(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            kind: TempPathKind::File,
        }
    }

    fn dir(path: PathBuf) -> Self {
        Self {
            path: Some(path),
            kind: TempPathKind::Dir,
        }
    }

    fn dismiss(mut self) {
        self.path = None;
    }
}

impl Drop for TempPathGuard {
    fn drop(&mut self) {
        let Some(path) = self.path.take() else {
            return;
        };
        match self.kind {
            TempPathKind::File => {
                let _ = fs::remove_file(path);
            }
            TempPathKind::Dir => {
                let _ = fs::remove_dir_all(path);
            }
        }
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

    fn write_restore_test_database_with_image(
        path: &Path,
        preview: &str,
        image_path: Option<&str>,
    ) {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.execute_batch(
            "CREATE TABLE items(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category INTEGER NOT NULL DEFAULT 0,
                kind TEXT NOT NULL DEFAULT 'text',
                preview TEXT NOT NULL,
                signature TEXT NOT NULL DEFAULT '',
                text_data TEXT,
                rich_text_html TEXT,
                source_app TEXT NOT NULL DEFAULT '',
                file_paths TEXT,
                image_data BLOB,
                image_path TEXT,
                image_width INTEGER NOT NULL DEFAULT 0,
                image_height INTEGER NOT NULL DEFAULT 0,
                pinned INTEGER NOT NULL DEFAULT 0,
                group_id INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );
            CREATE TABLE clip_groups(
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                category INTEGER NOT NULL DEFAULT 0,
                name TEXT NOT NULL,
                sort_order INTEGER NOT NULL DEFAULT 0,
                created_at DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP
            );",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO items(kind, preview, image_path) VALUES(?, ?, ?)",
            rusqlite::params![
                if image_path.is_some() {
                    "image"
                } else {
                    "text"
                },
                preview,
                image_path
            ],
        )
        .unwrap();
        conn.execute_batch("PRAGMA wal_checkpoint(TRUNCATE)")
            .unwrap();
    }

    fn write_restore_test_database(path: &Path, preview: &str) {
        write_restore_test_database_with_image(path, preview, None);
    }

    fn restored_test_preview(path: &Path) -> String {
        let conn = rusqlite::Connection::open(path).unwrap();
        conn.query_row("SELECT preview FROM items LIMIT 1", [], |row| row.get(0))
            .unwrap()
    }

    fn restore_test_paths(data_dir: &Path) -> CloudSyncPaths {
        CloudSyncPaths {
            data_dir: data_dir.to_path_buf(),
            settings_file: data_dir.join("settings.json"),
            db_file: data_dir.join("clipboard.db"),
        }
    }

    fn write_restore_test_archive(
        root: &Path,
        settings: Option<&str>,
        database: Option<&Path>,
        images: &[(&str, &[u8])],
    ) -> PathBuf {
        let payload = root.join("remote-payload").join("payload");
        fs::create_dir_all(&payload).unwrap();
        if let Some(settings) = settings {
            fs::write(payload.join("settings.json"), settings).unwrap();
        }
        if let Some(database) = database {
            fs::copy(database, payload.join("clipboard.db")).unwrap();
        }
        for (relative, bytes) in images {
            let path = payload.join("images").join(relative);
            fs::create_dir_all(path.parent().unwrap()).unwrap();
            fs::write(path, bytes).unwrap();
        }
        let archive = root.join("remote-backup.zip");
        compress_archive(&payload, &archive).unwrap();
        archive
    }

    fn write_active_restore_fixture(paths: &CloudSyncPaths) {
        fs::create_dir_all(paths.data_dir.join("images")).unwrap();
        fs::write(&paths.settings_file, r#"{"source":"local"}"#).unwrap();
        write_restore_test_database(&paths.db_file, "local data");
        fs::write(
            paths.data_dir.join("images").join("local.png"),
            b"local image",
        )
        .unwrap();
    }

    #[test]
    fn restore_staging_rejects_missing_database_and_bad_settings_without_touching_active_data() {
        let _guard = cloud_sync_e2e_guard();
        let root = temp_dir_path("restore-required-payload-test");
        let data_dir = root.join("active");
        fs::create_dir_all(&data_dir).unwrap();
        let paths = restore_test_paths(&data_dir);
        write_active_restore_fixture(&paths);

        let missing_db_archive =
            write_restore_test_archive(&root.join("missing-db"), Some(r#"{"ok":true}"#), None, &[]);
        let error = stage_snapshot_restore(&paths, &missing_db_archive, None).unwrap_err();
        assert!(error.contains("clipboard.db"));

        let remote_db = root.join("remote.db");
        write_restore_test_database(&remote_db, "cloud data");
        let bad_settings_archive = write_restore_test_archive(
            &root.join("bad-settings"),
            Some("{not-json"),
            Some(&remote_db),
            &[],
        );
        let error = stage_snapshot_restore(&paths, &bad_settings_archive, None).unwrap_err();
        assert!(error.contains("JSON"));

        assert_eq!(
            fs::read_to_string(&paths.settings_file).unwrap(),
            r#"{"source":"local"}"#
        );
        assert_eq!(restored_test_preview(&paths.db_file), "local data");
        assert_eq!(
            fs::read(paths.data_dir.join("images").join("local.png")).unwrap(),
            b"local image"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restore_staging_rejects_hash_mismatch_before_touching_active_data() {
        let _guard = cloud_sync_e2e_guard();
        let root = temp_dir_path("restore-hash-mismatch-test");
        let data_dir = root.join("active");
        fs::create_dir_all(&data_dir).unwrap();
        let paths = restore_test_paths(&data_dir);
        write_active_restore_fixture(&paths);
        let remote_db = root.join("remote.db");
        write_restore_test_database(&remote_db, "cloud data");
        let archive = write_restore_test_archive(
            &root.join("remote"),
            Some(r#"{"source":"cloud"}"#),
            Some(&remote_db),
            &[],
        );

        let error = stage_snapshot_restore(&paths, &archive, Some("0000000000000000")).unwrap_err();

        assert!(error.contains("哈希"));
        assert_eq!(restored_test_preview(&paths.db_file), "local data");
        assert_eq!(
            fs::read_to_string(&paths.settings_file).unwrap(),
            r#"{"source":"local"}"#
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn staged_restore_migrates_database_remaps_images_and_commits_all_artifacts() {
        let _guard = cloud_sync_e2e_guard();
        let root = temp_dir_path("restore-transaction-success-test");
        let data_dir = root.join("active");
        fs::create_dir_all(&data_dir).unwrap();
        let paths = restore_test_paths(&data_dir);
        write_active_restore_fixture(&paths);
        let remote_db = root.join("remote.db");
        write_restore_test_database_with_image(
            &remote_db,
            "cloud image",
            Some(r#"C:\old-device\data\images\nested\cloud.png"#),
        );
        let archive = write_restore_test_archive(
            &root.join("remote"),
            Some(r#"{"source":"cloud"}"#),
            Some(&remote_db),
            &[("nested/cloud.png", b"cloud image")],
        );
        let mut staged = stage_snapshot_restore(&paths, &archive, None).unwrap();

        let migrated_columns: i64 = rusqlite::Connection::open(&staged.database)
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM pragma_table_info('items') WHERE name='lan_origin_hash'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(migrated_columns, 1);
        crate::db_runtime::with_exclusive_db_file_replacement(&paths.db_file, || {
            commit_staged_restore(&paths, &mut staged)
        })
        .unwrap();

        assert_eq!(
            fs::read_to_string(&paths.settings_file).unwrap(),
            r#"{"source":"cloud"}"#
        );
        assert_eq!(restored_test_preview(&paths.db_file), "cloud image");
        assert_eq!(
            fs::read(paths.data_dir.join("images/nested/cloud.png")).unwrap(),
            b"cloud image"
        );
        assert!(!paths.data_dir.join("images/local.png").exists());
        let restored_image_path: String = rusqlite::Connection::open(&paths.db_file)
            .unwrap()
            .query_row("SELECT image_path FROM items LIMIT 1", [], |row| row.get(0))
            .unwrap();
        assert_eq!(
            PathBuf::from(restored_image_path),
            paths.data_dir.join("images/nested/cloud.png")
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_database_at_commit_rolls_back_settings_and_images() {
        let _guard = cloud_sync_e2e_guard();
        let root = temp_dir_path("restore-transaction-rollback-test");
        let data_dir = root.join("active");
        fs::create_dir_all(&data_dir).unwrap();
        let paths = restore_test_paths(&data_dir);
        write_active_restore_fixture(&paths);
        let remote_db = root.join("remote.db");
        write_restore_test_database(&remote_db, "cloud data");
        let archive = write_restore_test_archive(
            &root.join("remote"),
            Some(r#"{"source":"cloud"}"#),
            Some(&remote_db),
            &[("cloud.png", b"cloud image")],
        );
        let mut staged = stage_snapshot_restore(&paths, &archive, None).unwrap();
        fs::remove_file(&staged.database).unwrap();

        let error = crate::db_runtime::with_exclusive_db_file_replacement(&paths.db_file, || {
            commit_staged_restore(&paths, &mut staged)
        })
        .unwrap_err();

        assert!(error.contains("数据库"));
        assert_eq!(
            fs::read_to_string(&paths.settings_file).unwrap(),
            r#"{"source":"local"}"#
        );
        assert_eq!(restored_test_preview(&paths.db_file), "local data");
        assert_eq!(
            fs::read(paths.data_dir.join("images/local.png")).unwrap(),
            b"local image"
        );
        assert!(!paths.data_dir.join("images/cloud.png").exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn missing_images_stage_rolls_back_settings_without_touching_active_data() {
        let _guard = cloud_sync_e2e_guard();
        let root = temp_dir_path("restore-images-stage-rollback-test");
        let data_dir = root.join("active");
        fs::create_dir_all(&data_dir).unwrap();
        let paths = restore_test_paths(&data_dir);
        write_active_restore_fixture(&paths);
        let remote_db = root.join("remote.db");
        write_restore_test_database(&remote_db, "cloud data");
        let archive = write_restore_test_archive(
            &root.join("remote"),
            Some(r#"{"source":"cloud"}"#),
            Some(&remote_db),
            &[("cloud.png", b"cloud image")],
        );
        let mut staged = stage_snapshot_restore(&paths, &archive, None).unwrap();
        fs::remove_dir_all(&staged.images).unwrap();

        let error = crate::db_runtime::with_exclusive_db_file_replacement(&paths.db_file, || {
            commit_staged_restore(&paths, &mut staged)
        })
        .unwrap_err();

        assert!(error.contains("图片目录"));
        assert_eq!(
            fs::read_to_string(&paths.settings_file).unwrap(),
            r#"{"source":"local"}"#
        );
        assert_eq!(restored_test_preview(&paths.db_file), "local data");
        assert_eq!(
            fs::read(paths.data_dir.join("images/local.png")).unwrap(),
            b"local image"
        );
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn read_only_webdav_applies_valid_remote_settings_without_mkcol_or_put() {
        let _guard = cloud_sync_e2e_guard();
        let data_dir = temp_dir_path("read-only-settings-apply-test");
        fs::create_dir_all(&data_dir).unwrap();
        let paths = restore_test_paths(&data_dir);
        fs::write(&paths.settings_file, r#"{"source":"local"}"#).unwrap();
        let server = FakeWebDavServer::start_read_only_settings(r#"{"source":"cloud"}"#);
        let config = CloudSyncConfig {
            webdav_url: format!("http://127.0.0.1:{}/root", server.port),
            webdav_user: String::new(),
            webdav_pass: String::new(),
            remote_dir: "ZS Clip".to_string(),
        };

        let outcome =
            perform_cloud_sync(CloudSyncAction::ApplyRemoteConfig, &config, &paths).unwrap();

        assert!(outcome.reload_settings);
        assert_eq!(
            fs::read_to_string(&paths.settings_file).unwrap(),
            r#"{"source":"cloud"}"#
        );
        let requests = server.requests();
        assert!(requests.iter().any(|request| {
            request.method == "GET" && request.path.ends_with("/settings.json")
        }));
        assert!(!requests
            .iter()
            .any(|request| request.method == "MKCOL" || request.method == "PUT"));
        server.stop();
        fs::remove_dir_all(data_dir).unwrap();
    }

    #[test]
    fn invalid_remote_settings_leave_active_settings_unchanged() {
        let _guard = cloud_sync_e2e_guard();
        let data_dir = temp_dir_path("invalid-settings-apply-test");
        fs::create_dir_all(&data_dir).unwrap();
        let paths = restore_test_paths(&data_dir);
        fs::write(&paths.settings_file, r#"{"source":"local"}"#).unwrap();
        let server = FakeWebDavServer::start_read_only_settings("{not-json");
        let config = CloudSyncConfig {
            webdav_url: format!("http://127.0.0.1:{}/root", server.port),
            webdav_user: String::new(),
            webdav_pass: String::new(),
            remote_dir: "ZS Clip".to_string(),
        };

        let error =
            perform_cloud_sync(CloudSyncAction::ApplyRemoteConfig, &config, &paths).unwrap_err();

        assert!(error.contains("JSON"));
        assert_eq!(
            fs::read_to_string(&paths.settings_file).unwrap(),
            r#"{"source":"local"}"#
        );
        let requests = server.requests();
        assert!(!requests
            .iter()
            .any(|request| request.method == "MKCOL" || request.method == "PUT"));
        server.stop();
        fs::remove_dir_all(data_dir).unwrap();
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
    fn bdd_cleanup_removes_legacy_and_current_cloud_temp_archives() {
        let nonce = unix_now();
        let dir = temp_dir_path("cloud-cleanup-test");
        fs::create_dir_all(&dir).unwrap();
        let legacy = dir.join(format!("zsclip-cloud-{nonce}.zip"));
        let current = dir.join(format!("zsclip_cloud_{}_{}.zip", std::process::id(), nonce));
        let download = dir.join(format!(
            "zsclip_cloud-backup_{}_{}.zip",
            std::process::id(),
            nonce
        ));
        let unrelated = dir.join(format!("zsclip-unrelated-{nonce}.zip"));
        fs::write(&legacy, b"old").unwrap();
        fs::write(&current, b"new").unwrap();
        fs::write(&download, b"download").unwrap();
        fs::write(&unrelated, b"keep").unwrap();

        let cleanup = cleanup_cloud_sync_temp_files_in_dir(&dir);

        assert!(cleanup.files_removed >= 3);
        assert!(!legacy.exists());
        assert!(!current.exists());
        assert!(!download.exists());
        assert!(unrelated.exists());
        let _ = fs::remove_dir_all(dir);
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

        fn start_read_only_settings(remote_settings: &'static str) -> Self {
            Self::start_with_remote_syncclipboard_options(
                None,
                0,
                false,
                Some(remote_settings),
                false,
            )
        }

        fn start_with_missing_remote_image(remote_syncclipboard: Option<&'static str>) -> Self {
            Self::start_with_remote_syncclipboard_options(
                remote_syncclipboard,
                0,
                false,
                None,
                true,
            )
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
                None,
                true,
            )
        }

        fn start_with_remote_syncclipboard_options(
            remote_syncclipboard: Option<&'static str>,
            empty_syncclipboard_count: usize,
            serve_remote_image: bool,
            remote_settings: Option<&'static str>,
            allow_writes: bool,
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
                            remote_settings,
                            allow_writes,
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
        remote_settings: Option<&'static str>,
        allow_writes: bool,
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
        } else if method == "GET" && path.ends_with("/settings.json") {
            remote_settings
                .map(|body| (200, body.as_bytes().to_vec()))
                .unwrap_or_else(|| (404, Vec::new()))
        } else if method == "MKCOL" || method == "PUT" {
            if allow_writes {
                (201, Vec::new())
            } else {
                (403, Vec::new())
            }
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
