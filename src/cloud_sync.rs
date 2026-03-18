use crate::i18n::tr;
use serde::{Deserialize, Serialize};
use std::fs;
use std::os::windows::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const CREATE_NO_WINDOW_FLAG: u32 = 0x08000000;

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
    backup_dir_url: String,
    backup_url: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct CloudSyncManifest {
    version: String,
    updated_at: u64,
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
        let backup_dir_url = append_url_path(&base_url, "backups");
        let backup_url = append_url_path(&backup_dir_url, BACKUP_FILE_NAME);
        Ok(Self {
            settings_url,
            manifest_url,
            backup_dir_url,
            backup_url,
        })
    }
}

pub fn cloud_sync_interval(label: &str) -> Duration {
    match label.trim() {
        "15分钟" | "15鍒嗛挓" | "15 min" | "15m" | "15min" => Duration::from_secs(15 * 60),
        "30分钟" | "30鍒嗛挓" | "30 min" | "30m" | "30min" => Duration::from_secs(30 * 60),
        "1小时" | "1灏忔椂" | "1 hour" | "1h" => Duration::from_secs(60 * 60),
        "6小时" | "6灏忔椂" | "6 hours" | "6h" => Duration::from_secs(6 * 60 * 60),
        "12小时" | "12灏忔椂" | "12 hours" | "12h" => Duration::from_secs(12 * 60 * 60),
        "24小时" | "24灏忔椂" | "24 hours" | "24h" | "1d" => Duration::from_secs(24 * 60 * 60),
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
    let local_stamp = local_state_stamp(paths);
    let remote_manifest = download_remote_manifest(config, remote)?;
    if let Some(manifest) = remote_manifest {
        if manifest.updated_at > local_stamp.saturating_add(5) {
            let outcome = restore_remote_backup(config, remote, paths)?;
            return Ok(CloudSyncOutcome {
                status_text: format!("云端较新，已恢复到本地（{}）。", manifest.version),
                ..outcome
            });
        }
        if local_stamp <= manifest.updated_at.saturating_add(5) {
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
        backup_name: BACKUP_FILE_NAME.to_string(),
    };
    let manifest_path = write_temp_json_file("manifest", &manifest)?;
    upload_file(config, &manifest_path, &remote.manifest_url)?;
    let _ = fs::remove_file(&archive_path);
    let _ = fs::remove_file(&manifest_path);
    Ok(CloudSyncOutcome {
        status_text: format!("云同步完成，已上传本地快照（{}）。", format_unix_ts(stamp)),
        reload_settings: false,
        reload_data: false,
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
    let download_path = temp_file_path("cloud-backup", "zip");
    if !download_file(config, &remote.backup_url, &download_path)? {
        return Err("云端没有找到可恢复的备份。".to_string());
    }
    restore_snapshot_archive(paths, &download_path)?;
    let _ = fs::remove_file(download_path);
    Ok(CloudSyncOutcome {
        status_text: "已从云端恢复数据备份。".to_string(),
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
    Ok(())
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
        .and_then(|raw| serde_json::from_str::<CloudSyncManifest>(&raw).map_err(|err| err.to_string()))?;
    let _ = fs::remove_file(temp_path);
    Ok(Some(manifest))
}

fn create_snapshot_archive(paths: &CloudSyncPaths, stamp: u64) -> Result<PathBuf, String> {
    let staging_root = temp_dir_path("snapshot-staging");
    if staging_root.exists() {
        let _ = fs::remove_dir_all(&staging_root);
    }
    let payload_dir = staging_root.join("payload");
    fs::create_dir_all(&payload_dir).map_err(|err| err.to_string())?;

    if paths.settings_file.exists() {
        fs::copy(&paths.settings_file, payload_dir.join("settings.json")).map_err(|err| err.to_string())?;
    }
    if paths.db_file.exists() {
        fs::copy(&paths.db_file, payload_dir.join("clipboard.db")).map_err(|err| err.to_string())?;
    }
    let images_dir = paths.data_dir.join("images");
    if images_dir.exists() {
        copy_dir_recursive(&images_dir, &payload_dir.join("images"))?;
    }

    let archive_path = std::env::temp_dir().join(format!("zsclip-cloud-{stamp}.zip"));
    compress_archive(&payload_dir, &archive_path)?;
    let _ = fs::remove_dir_all(staging_root);
    Ok(archive_path)
}

fn restore_snapshot_archive(paths: &CloudSyncPaths, archive_path: &Path) -> Result<(), String> {
    let extract_root = temp_dir_path("snapshot-restore");
    if extract_root.exists() {
        let _ = fs::remove_dir_all(&extract_root);
    }
    fs::create_dir_all(&extract_root).map_err(|err| err.to_string())?;
    expand_archive(archive_path, &extract_root)?;
    let payload_dir = extract_root.join("payload");
    let source_dir = if payload_dir.exists() { payload_dir } else { extract_root.clone() };

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

fn file_modified_secs(path: &Path) -> u64 {
    fs::metadata(path)
        .ok()
        .and_then(|meta| meta.modified().ok())
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|dur| dur.as_secs())
        .unwrap_or(0)
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
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath '{archive}') {{ Remove-Item -LiteralPath '{archive}' -Force }}
Compress-Archive -LiteralPath '{source}' -DestinationPath '{archive}' -Force
"#,
        source = ps_quote(&source_dir.to_string_lossy()),
        archive = ps_quote(&archive_path.to_string_lossy()),
    );
    run_powershell(&script).map(|_| ())
}

fn expand_archive(archive_path: &Path, dest_dir: &Path) -> Result<(), String> {
    let script = format!(
        r#"
$ErrorActionPreference = 'Stop'
if (Test-Path -LiteralPath '{dest}') {{ Remove-Item -LiteralPath '{dest}' -Recurse -Force }}
New-Item -ItemType Directory -Path '{dest}' | Out-Null
Expand-Archive -LiteralPath '{archive}' -DestinationPath '{dest}' -Force
"#,
        archive = ps_quote(&archive_path.to_string_lossy()),
        dest = ps_quote(&dest_dir.to_string_lossy()),
    );
    run_powershell(&script).map(|_| ())
}

fn upload_file(config: &CloudSyncConfig, local_path: &Path, remote_url: &str) -> Result<(), String> {
    if !local_path.exists() {
        return Err(format!(
            "{}{}",
            tr("本地文件不存在：", "Local file was not found: "),
            local_path.to_string_lossy()
        ));
    }

    let status = run_curl_status(build_webdav_args(
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
    ))?;

    match status.as_str() {
        "200" | "201" | "204" => Ok(()),
        _ => Err(format!(
            "{}{}",
            tr("上传失败，HTTP 状态码：", "Upload failed with HTTP status: "),
            status
        )),
    }
}

fn download_file(config: &CloudSyncConfig, remote_url: &str, local_path: &Path) -> Result<bool, String> {
    if let Some(parent) = local_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let status = run_curl_status(build_webdav_args(
        config,
        &[
            "-L".to_string(),
            "-o".to_string(),
            local_path.to_string_lossy().to_string(),
            "-w".to_string(),
            "%{http_code}".to_string(),
            remote_url.to_string(),
        ],
    ))?;

    match status.as_str() {
        "200" | "206" => Ok(true),
        "404" => {
            let _ = fs::remove_file(local_path);
            Ok(false)
        }
        _ => Err(format!(
            "{}{}",
            tr("下载失败，HTTP 状态码：", "Download failed with HTTP status: "),
            status
        )),
    }
}

fn webdav_mkcol(config: &CloudSyncConfig, remote_url: &str) -> Result<(), String> {
    let status = run_curl_status(build_webdav_args(
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
    ))?;

    match status.as_str() {
        "200" | "201" | "204" | "301" | "302" | "405" | "409" => Ok(()),
        _ => Err(format!(
            "{}{}",
            tr("创建云端目录失败，HTTP 状态码：", "Failed to create remote directory. HTTP status: "),
            status
        )),
    }
}

fn build_webdav_args(config: &CloudSyncConfig, extra: &[String]) -> Vec<String> {
    let mut args = vec![
        "--silent".to_string(),
        "--show-error".to_string(),
        "--connect-timeout".to_string(),
        "15".to_string(),
        "--max-time".to_string(),
        "300".to_string(),
    ];

    if !config.webdav_user.trim().is_empty() || !config.webdav_pass.is_empty() {
        args.push("--user".to_string());
        args.push(format!("{}:{}", config.webdav_user.trim(), config.webdav_pass));
    }

    args.extend(extra.iter().cloned());
    args
}

fn run_curl_status(args: Vec<String>) -> Result<String, String> {
    let output = hidden_curl()
        .args(&args)
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

fn run_powershell(script: &str) -> Result<String, String> {
    let encoded = encode_powershell(script);
    let output = hidden_powershell()
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-EncodedCommand", &encoded])
        .output()
        .map_err(|err| err.to_string())?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        if stderr.is_empty() {
            Err("PowerShell 执行失败。".to_string())
        } else {
            Err(stderr)
        }
    }
}

fn encode_powershell(script: &str) -> String {
    let mut bytes = Vec::with_capacity(script.len() * 2);
    for unit in script.encode_utf16() {
        bytes.extend_from_slice(&unit.to_le_bytes());
    }
    base64_encode(&bytes)
}

fn hidden_powershell() -> Command {
    let mut cmd = Command::new("powershell");
    cmd.creation_flags(CREATE_NO_WINDOW_FLAG);
    cmd
}

fn hidden_curl() -> Command {
    let mut cmd = Command::new("curl.exe");
    cmd.creation_flags(CREATE_NO_WINDOW_FLAG);
    cmd
}

fn base64_encode(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    let mut i = 0;
    while i < bytes.len() {
        let b0 = bytes[i];
        let b1 = *bytes.get(i + 1).unwrap_or(&0);
        let b2 = *bytes.get(i + 2).unwrap_or(&0);
        let pad = match bytes.len() - i {
            1 => 2,
            2 => 1,
            _ => 0,
        };
        let triple = ((b0 as u32) << 16) | ((b1 as u32) << 8) | (b2 as u32);
        out.push(TABLE[((triple >> 18) & 0x3f) as usize] as char);
        out.push(TABLE[((triple >> 12) & 0x3f) as usize] as char);
        out.push(if pad >= 2 { '=' } else { TABLE[((triple >> 6) & 0x3f) as usize] as char });
        out.push(if pad >= 1 { '=' } else { TABLE[(triple & 0x3f) as usize] as char });
        i += 3;
    }
    out
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
    std::env::temp_dir().join(format!("zsclip-{prefix}-{}", unix_now()))
}

fn temp_file_path(prefix: &str, ext: &str) -> PathBuf {
    std::env::temp_dir().join(format!("zsclip-{prefix}-{}.{}", unix_now(), ext))
}

fn write_temp_json_file<T: Serialize>(prefix: &str, value: &T) -> Result<PathBuf, String> {
    let path = temp_file_path(prefix, "json");
    let raw = serde_json::to_vec_pretty(value).map_err(|err| err.to_string())?;
    fs::write(&path, raw).map_err(|err| err.to_string())?;
    Ok(path)
}

fn ps_quote(value: &str) -> String {
    value.replace('\'', "''")
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn format_unix_ts(value: u64) -> String {
    format!("{value}")
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

