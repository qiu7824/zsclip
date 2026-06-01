use crate::app::state::AppSettings;
use crate::app::{
    data_dir, decrypt_secret_from_storage, encrypt_secret_for_storage, save_settings,
    WM_LAN_SYNC_READY,
};
use crate::db_runtime::with_db;
use base64::{engine::general_purpose, Engine as _};
use rusqlite::{params, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::{Read, Write};
use std::net::{IpAddr, SocketAddr, TcpListener, TcpStream, UdpSocket};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock};
use std::thread::{self, JoinHandle};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::PostMessageW;

pub(crate) const LAN_DISCOVERY_PORT_DEFAULT: u16 = 38472;
pub(crate) const LAN_TCP_PORT_DEFAULT: u16 = 38473;
pub(crate) const LAN_IMAGE_MAX_BYTES: usize = 10 * 1024 * 1024;
pub(crate) const LAN_FILE_AUTO_MAX_BYTES: u64 = 50 * 1024 * 1024;

const LAN_MAGIC: &str = "ZSCLIP_LAN_V1";
const LAN_PROTOCOL: u32 = 1;
const HTTP_MAX_BODY: usize = 12 * 1024 * 1024;
const DISCOVERY_INTERVAL_MS: u64 = 5000;
const LAN_FILE_MAX_BYTES: u64 = 1024 * 1024 * 1024;
const LAN_FILE_CHUNK_BYTES: usize = 512 * 1024;
const CREATE_NO_WINDOW_FLAG: u32 = 0x0800_0000;
const MOBILE_IMAGE_LIST_LIMIT: i64 = 50;

static SERVICE: OnceLock<Mutex<Option<LanServiceHandle>>> = OnceLock::new();
static DISCOVERED: OnceLock<Mutex<Vec<LanDevice>>> = OnceLock::new();
static INCOMING_CLIPS: OnceLock<Mutex<VecDeque<LanIncomingClip>>> = OnceLock::new();
static PAIR_PROMPTS: OnceLock<Mutex<VecDeque<LanPairPrompt>>> = OnceLock::new();
static PENDING_PAIRS: OnceLock<Mutex<Vec<PendingPair>>> = OnceLock::new();
static SEEN_MESSAGES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static STATUS_TEXT: OnceLock<Mutex<String>> = OnceLock::new();
static LATEST_CLIP: OnceLock<Mutex<Option<LanClipEnvelope>>> = OnceLock::new();
static FILE_SESSIONS: OnceLock<Mutex<HashMap<String, FileSession>>> = OnceLock::new();
static ORIGIN_SEQ: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LanDevice {
    pub(crate) device_id: String,
    pub(crate) name: String,
    pub(crate) addr: String,
    pub(crate) tcp_port: u16,
    pub(crate) token: String,
    pub(crate) last_seen_ms: u64,
    pub(crate) trusted: bool,
    #[serde(default)]
    pub(crate) capabilities: Vec<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LanFileMeta {
    pub(crate) name: String,
    pub(crate) size: u64,
    pub(crate) relative_path: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub(crate) struct LanClipEnvelope {
    pub(crate) message_id: String,
    pub(crate) origin_device_id: String,
    pub(crate) origin_seq: u64,
    pub(crate) kind: String,
    pub(crate) hash: String,
    pub(crate) created_at_ms: u64,
    pub(crate) preview: String,
    pub(crate) text: Option<String>,
    pub(crate) image_png_base64: Option<String>,
    #[serde(default)]
    pub(crate) file_meta: Vec<LanFileMeta>,
}

#[derive(Clone, Debug)]
pub(crate) struct LanIncomingClip {
    pub(crate) envelope: LanClipEnvelope,
    pub(crate) source_device_name: String,
}

#[derive(Clone, Debug)]
pub(crate) struct LanPairPrompt {
    pub(crate) pair_id: String,
    pub(crate) code: String,
    pub(crate) device_name: String,
    pub(crate) addr: String,
    pub(crate) created_at_ms: u64,
}

struct LanServiceHandle {
    stop: Arc<AtomicBool>,
    device_id: String,
    tcp_port: u16,
    udp_port: u16,
    workers: Vec<JoinHandle<()>>,
}

#[derive(Clone)]
struct LanRuntimeConfig {
    hwnd: isize,
    device_id: String,
    device_name: String,
    tcp_port: u16,
    udp_port: u16,
}

#[derive(Clone, Serialize, Deserialize)]
struct DiscoveryPacket {
    magic: String,
    protocol: u32,
    device_id: String,
    name: String,
    tcp_port: u16,
    capabilities: Vec<String>,
}

#[derive(Clone, Serialize, Deserialize)]
struct PairRequestBody {
    device_id: String,
    name: String,
    tcp_port: u16,
    #[serde(default)]
    capabilities: Vec<String>,
}

#[derive(Clone)]
struct PendingPair {
    prompt: LanPairPrompt,
    requester_device_id: String,
    requester_tcp_port: u16,
    requester_capabilities: Vec<String>,
    token: String,
    accepted: bool,
    rejected: bool,
    created_at_ms: u64,
}

#[derive(Serialize, Deserialize, Default)]
struct StoredDeviceBook {
    devices: Vec<StoredLanDevice>,
}

#[derive(Serialize, Deserialize)]
struct StoredLanDevice {
    device_id: String,
    name: String,
    addr: String,
    tcp_port: u16,
    token_encrypted: String,
    last_seen_ms: u64,
    trusted: bool,
    #[serde(default)]
    capabilities: Vec<String>,
}

struct HttpRequest {
    method: String,
    path: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
    peer: SocketAddr,
}

struct FileSession {
    source_device_id: String,
    source_device_name: String,
    transfer_id: String,
    final_path: PathBuf,
    part_path: PathBuf,
    relative_path: String,
    file_name: String,
    total_size: u64,
    received: u64,
    content_crc: crc32fast::Hasher,
}

struct MobileImageListItem {
    id: i64,
    preview: String,
    source_app: String,
    created_at: String,
    size: u64,
    width: i64,
    height: i64,
}

pub(crate) fn ensure_device_identity(settings: &mut AppSettings) -> bool {
    let mut changed = false;
    if settings.lan_device_id.trim().is_empty() {
        settings.lan_device_id = format!("zsclip-{}", make_token(16));
        changed = true;
    }
    if settings.lan_device_name.trim().is_empty() {
        let hostname = std::env::var("COMPUTERNAME")
            .ok()
            .filter(|s| !s.trim().is_empty())
            .unwrap_or_else(|| "Windows".to_string());
        settings.lan_device_name = format!("ZSClip-{hostname}");
        changed = true;
    }
    if settings.lan_tcp_port == 0 {
        settings.lan_tcp_port = LAN_TCP_PORT_DEFAULT;
        changed = true;
    }
    if settings.lan_udp_port == 0 {
        settings.lan_udp_port = LAN_DISCOVERY_PORT_DEFAULT;
        changed = true;
    }
    changed
}

pub(crate) fn next_origin_seq() -> u64 {
    ORIGIN_SEQ.fetch_add(1, Ordering::Relaxed)
}

pub(crate) fn now_ms_public() -> u64 {
    now_ms()
}

fn windows_capabilities() -> Vec<String> {
    ["text", "image", "latest", "manual_file", "receive_clip"]
        .iter()
        .map(|value| value.to_string())
        .collect()
}

fn normalize_capabilities(mut capabilities: Vec<String>, tcp_port: u16) -> Vec<String> {
    capabilities = capabilities
        .into_iter()
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .collect();
    capabilities.sort_unstable();
    capabilities.dedup();
    if !capabilities.is_empty() {
        return capabilities;
    }
    if tcp_port == 0 {
        vec!["client_only".to_string(), "pull_only".to_string()]
    } else {
        windows_capabilities()
    }
}

fn device_can_receive_clip(device: &LanDevice) -> bool {
    device.trusted
        && device.tcp_port > 0
        && !device.capabilities.iter().any(|cap| {
            cap.eq_ignore_ascii_case("client_only") || cap.eq_ignore_ascii_case("pull_only")
        })
}

pub(crate) fn set_latest_clip(clip: Option<LanClipEnvelope>) {
    if let Some(clip) = clip {
        if let Ok(mut latest) = latest_clip_slot().lock() {
            *latest = Some(clip);
        }
    } else if let Some(slot) = LATEST_CLIP.get() {
        if let Ok(mut latest) = slot.lock() {
            *latest = None;
        }
    }
}

fn remember_seen_message_key(key: String) -> bool {
    if let Ok(mut seen) = seen_slot().lock() {
        if seen.len() > 4096 {
            seen.clear();
        }
        if !seen.insert(key) {
            return false;
        }
    }
    true
}

pub(crate) fn refresh_service(hwnd: HWND, settings: &AppSettings) {
    if !settings.lan_sync_enabled {
        stop_service();
        set_status_if_initialized("未启动");
        return;
    }

    let device_id = settings.lan_device_id.trim().to_string();
    if device_id.is_empty() {
        set_status("未生成设备 ID，请保存设置后重试");
        return;
    }
    let desired_tcp = settings.lan_tcp_port.max(1);
    let desired_udp = settings.lan_udp_port.max(1);
    let mut guard = service_slot().lock().unwrap();
    let should_restart = guard
        .as_ref()
        .map(|handle| {
            handle.device_id != device_id
                || handle.tcp_port != desired_tcp
                || handle.udp_port != desired_udp
        })
        .unwrap_or(true);
    if !should_restart {
        return;
    }
    if let Some(handle) = guard.take() {
        stop_handle(handle);
    }
    let config = LanRuntimeConfig {
        hwnd: hwnd as isize,
        device_id,
        device_name: settings.lan_device_name.clone(),
        tcp_port: desired_tcp,
        udp_port: desired_udp,
    };
    match start_handle(config) {
        Ok(handle) => {
            let firewall_note = ensure_firewall_rules(handle.tcp_port, handle.udp_port)
                .err()
                .map(|err| format!("；防火墙自动放行失败：{err}"));
            set_status(&format!(
                "已启动：UDP {} / TCP {}{}",
                handle.udp_port,
                handle.tcp_port,
                firewall_note.unwrap_or_default()
            ));
            *guard = Some(handle);
        }
        Err(err) => {
            set_status(&format!("启动失败：{err}"));
        }
    }
}

pub(crate) fn stop_service() {
    if let Some(slot) = SERVICE.get() {
        let mut guard = slot.lock().unwrap();
        if let Some(handle) = guard.take() {
            stop_handle(handle);
        }
    }
}

pub(crate) fn trigger_discovery(settings: &AppSettings) {
    let mut s = settings.clone();
    if ensure_device_identity(&mut s) {
        save_settings(&s);
    }
    send_discovery_once(&LanRuntimeConfig {
        hwnd: 0,
        device_id: s.lan_device_id,
        device_name: s.lan_device_name,
        tcp_port: s.lan_tcp_port,
        udp_port: s.lan_udp_port,
    });
}

pub(crate) fn broadcast_clip(settings: &AppSettings, envelope: LanClipEnvelope) {
    if !settings.lan_sync_enabled {
        return;
    }
    let token_device_id = settings.lan_device_id.clone();
    let devices = load_devices();
    for device in devices.into_iter().filter(device_can_receive_clip) {
        if device.addr.trim().is_empty() || device.token.trim().is_empty() {
            continue;
        }
        let env = envelope.clone();
        let sender_id = token_device_id.clone();
        thread::spawn(move || {
            let addr = format!("{}:{}", device.addr, device.tcp_port);
            let body = serde_json::to_vec(&env).unwrap_or_default();
            let _ = http_request(
                "POST",
                &addr,
                "/v1/clip",
                &[
                    ("Content-Type", "application/json"),
                    ("X-ZSClip-Device", &sender_id),
                    ("X-ZSClip-Token", &device.token),
                ],
                Some(&body),
                Duration::from_secs(5),
            );
        });
    }
}

pub(crate) fn trusted_devices() -> Vec<LanDevice> {
    load_devices()
        .into_iter()
        .filter(|device| device.trusted)
        .collect()
}

pub(crate) fn discovered_devices() -> Vec<LanDevice> {
    let mut devices = discovered_slot()
        .lock()
        .map(|devices| devices.clone())
        .unwrap_or_default();
    devices.sort_by(|a, b| {
        b.trusted
            .cmp(&a.trusted)
            .then_with(|| b.last_seen_ms.cmp(&a.last_seen_ms))
            .then_with(|| a.name.cmp(&b.name))
    });
    devices
}

pub(crate) fn push_files_to_trusted(settings: &AppSettings, paths: Vec<String>) {
    push_files_to_trusted_inner(settings, paths, LAN_FILE_MAX_BYTES, false);
}

pub(crate) fn push_small_files_to_trusted(settings: &AppSettings, paths: Vec<String>) {
    push_files_to_trusted_inner(settings, paths, LAN_FILE_AUTO_MAX_BYTES, true);
}

fn push_files_to_trusted_inner(settings: &AppSettings, paths: Vec<String>, limit: u64, auto: bool) {
    if !settings.lan_sync_enabled {
        set_status("局域网同步未开启，无法推送文件");
        return;
    }
    let devices: Vec<LanDevice> = trusted_devices()
        .into_iter()
        .filter(device_can_receive_clip)
        .collect();
    if devices.is_empty() {
        set_status("没有可接收推送的信任设备，无法推送文件");
        return;
    }
    let sender_id = settings.lan_device_id.clone();
    let mut total_size = 0u64;
    let mut skipped = 0usize;
    let mut files = Vec::new();
    for path in paths.into_iter().map(PathBuf::from) {
        let Ok(metadata) = fs::metadata(&path) else {
            skipped += 1;
            continue;
        };
        if !path.is_file() || metadata.len() == 0 {
            skipped += 1;
            continue;
        }
        if metadata.len() > limit || total_size.saturating_add(metadata.len()) > limit {
            skipped += 1;
            continue;
        }
        total_size += metadata.len();
        files.push(path);
    }
    if files.is_empty() {
        set_status(if auto {
            "小文件自动同步跳过：目录或超限文件不会自动同步"
        } else {
            "没有可推送的文件，目录和超大文件会被跳过"
        });
        return;
    }
    if auto {
        set_status(&format!(
            "正在自动同步 {} 个小文件{}",
            files.len(),
            if skipped > 0 {
                "，部分项目已跳过"
            } else {
                ""
            }
        ));
    } else {
        set_status("正在后台推送局域网文件...");
    }
    thread::spawn(move || {
        let mut ok_count = 0usize;
        for device in devices {
            for path in &files {
                if push_one_file(&sender_id, &device, path).is_ok() {
                    ok_count += 1;
                }
            }
        }
        set_status(&format!(
            "{}完成：{} 个任务成功",
            if auto {
                "小文件自动同步"
            } else {
                "文件推送"
            },
            ok_count
        ));
    });
}

pub(crate) fn drain_incoming_clips() -> Vec<LanIncomingClip> {
    incoming_slot()
        .lock()
        .map(|mut q| q.drain(..).collect())
        .unwrap_or_default()
}

pub(crate) fn drain_pair_prompts() -> Vec<LanPairPrompt> {
    pair_prompt_slot()
        .lock()
        .map(|mut q| q.drain(..).collect())
        .unwrap_or_default()
}

pub(crate) fn pending_pair_requests() -> Vec<LanPairPrompt> {
    let now = now_ms();
    pending_pair_slot()
        .lock()
        .map(|pairs| {
            pairs
                .iter()
                .filter(|pair| {
                    !pair.accepted
                        && !pair.rejected
                        && now.saturating_sub(pair.created_at_ms) < 10 * 60 * 1000
                })
                .map(|pair| pair.prompt.clone())
                .collect()
        })
        .unwrap_or_default()
}

pub(crate) fn accept_pair_request(pair_id: &str) -> bool {
    let mut pairs = pending_pair_slot().lock().unwrap();
    let Some(pair) = pairs.iter_mut().find(|pair| pair.prompt.pair_id == pair_id) else {
        return false;
    };
    pair.accepted = true;
    pair.rejected = false;
    let device = LanDevice {
        device_id: pair.requester_device_id.clone(),
        name: pair.prompt.device_name.clone(),
        addr: pair.prompt.addr.clone(),
        tcp_port: pair.requester_tcp_port,
        token: pair.token.clone(),
        last_seen_ms: now_ms(),
        trusted: true,
        capabilities: normalize_capabilities(
            pair.requester_capabilities.clone(),
            pair.requester_tcp_port,
        ),
    };
    upsert_device(device);
    set_status("已允许局域网设备配对");
    true
}

pub(crate) fn reject_pair_request(pair_id: &str) {
    if let Ok(mut pairs) = pending_pair_slot().lock() {
        if let Some(pair) = pairs.iter_mut().find(|pair| pair.prompt.pair_id == pair_id) {
            pair.rejected = true;
            pair.accepted = false;
        }
    }
    set_status("已拒绝局域网设备配对");
}

pub(crate) fn start_pair_with_host(hwnd: HWND, settings: AppSettings, host: String) {
    let host = normalize_host(&host, settings.lan_tcp_port);
    if host.is_empty() {
        set_status("请输入局域网设备 IP 或 IP:端口");
        return;
    }
    set_status("正在请求配对...");
    let hwnd_value = hwnd as isize;
    thread::spawn(move || {
        let body = serde_json::to_vec(&PairRequestBody {
            device_id: settings.lan_device_id.clone(),
            name: settings.lan_device_name.clone(),
            tcp_port: settings.lan_tcp_port,
            capabilities: windows_capabilities(),
        })
        .unwrap_or_default();
        let Ok(resp) = http_request(
            "POST",
            &host,
            "/v1/pair/request",
            &[("Content-Type", "application/json")],
            Some(&body),
            Duration::from_secs(5),
        ) else {
            set_status("配对请求失败：无法连接设备");
            post_ready(hwnd_value);
            return;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&resp) else {
            set_status("配对请求失败：设备返回异常");
            post_ready(hwnd_value);
            return;
        };
        let pair_id = value
            .get("pair_id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let code = value
            .get("code")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        if pair_id.is_empty() || code.is_empty() {
            set_status("配对请求失败：对方没有返回配对会话");
            post_ready(hwnd_value);
            return;
        }
        set_status("配对请求已发送，请在对方设备的局域网设置页点击允许");
        post_ready(hwnd_value);
        for _ in 0..90 {
            thread::sleep(Duration::from_secs(1));
            let path = format!("/v1/pair/status?id={pair_id}");
            let Ok(resp) = http_request("GET", &host, &path, &[], None, Duration::from_secs(5))
            else {
                continue;
            };
            let Ok(status) = serde_json::from_slice::<serde_json::Value>(&resp) else {
                continue;
            };
            match status.get("status").and_then(|v| v.as_str()).unwrap_or("") {
                "accepted" => {
                    let token = status
                        .get("token")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let device_id = status
                        .get("device_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default()
                        .to_string();
                    let name = status
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("ZSClip")
                        .to_string();
                    let tcp_port = status
                        .get("tcp_port")
                        .and_then(|v| v.as_u64())
                        .map(|v| v as u16)
                        .unwrap_or(settings.lan_tcp_port);
                    if !token.is_empty() && !device_id.is_empty() {
                        let addr = host.split(':').next().unwrap_or("").to_string();
                        upsert_device(LanDevice {
                            device_id,
                            name,
                            addr,
                            tcp_port,
                            token,
                            last_seen_ms: now_ms(),
                            trusted: true,
                            capabilities: normalize_capabilities(
                                status
                                    .get("capabilities")
                                    .and_then(|v| v.as_array())
                                    .map(|items| {
                                        items
                                            .iter()
                                            .filter_map(|item| item.as_str())
                                            .map(|item| item.to_string())
                                            .collect::<Vec<_>>()
                                    })
                                    .unwrap_or_default(),
                                tcp_port,
                            ),
                        });
                        set_status("配对成功，已保存信任设备");
                        post_ready(hwnd_value);
                        return;
                    }
                }
                "rejected" => {
                    set_status("配对已被对方拒绝");
                    post_ready(hwnd_value);
                    return;
                }
                _ => {}
            }
        }
        set_status("配对超时");
        post_ready(hwnd_value);
    });
}

pub(crate) fn status_summary(settings: &AppSettings) -> String {
    if !settings.lan_sync_enabled {
        return "局域网同步：关闭".to_string();
    }
    status_slot()
        .lock()
        .map(|s| {
            if s.trim().is_empty() {
                "局域网同步：已开启".to_string()
            } else {
                format!("局域网同步：{}", s.as_str())
            }
        })
        .unwrap_or_else(|_| "局域网同步：状态异常".to_string())
}

pub(crate) fn trusted_summary() -> String {
    let devices = load_devices();
    if devices.is_empty() {
        return "信任设备：暂无。可输入 IP 手动连接，或等待自动发现后配对。".to_string();
    }
    let mut lines = vec!["信任设备：".to_string()];
    for device in devices.iter().filter(|d| d.trusted).take(8) {
        let is_mobile_client = device.capabilities.iter().any(|cap| {
            cap.eq_ignore_ascii_case("client_only") || cap.eq_ignore_ascii_case("pull_only")
        });
        let mode = if device_can_receive_clip(device) {
            "自动同步"
        } else if is_mobile_client {
            "手机客户端：可推送/可拉取"
        } else {
            "仅拉取/客户端"
        };
        let endpoint = if device.tcp_port > 0 {
            format!("{}:{}", device.addr, device.tcp_port)
        } else {
            device.addr.clone()
        };
        lines.push(format!("{}  {}  {}", device.name, endpoint, mode));
    }
    lines.join("\r\n")
}

fn start_handle(config: LanRuntimeConfig) -> std::io::Result<LanServiceHandle> {
    let stop = Arc::new(AtomicBool::new(false));
    let listener = bind_tcp_listener(config.tcp_port)?;
    let actual_tcp_port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;
    let mut tcp_config = config.clone();
    tcp_config.tcp_port = actual_tcp_port;

    let mut workers = Vec::new();
    {
        let stop = stop.clone();
        let cfg = tcp_config.clone();
        workers.push(thread::spawn(move || tcp_server_loop(listener, cfg, stop)));
    }
    {
        let stop = stop.clone();
        let cfg = tcp_config.clone();
        workers.push(thread::spawn(move || udp_discovery_listener(cfg, stop)));
    }
    {
        let stop = stop.clone();
        let cfg = tcp_config.clone();
        workers.push(thread::spawn(move || udp_discovery_sender(cfg, stop)));
    }

    Ok(LanServiceHandle {
        stop,
        device_id: tcp_config.device_id,
        tcp_port: actual_tcp_port,
        udp_port: tcp_config.udp_port,
        workers,
    })
}

fn stop_handle(mut handle: LanServiceHandle) {
    handle.stop.store(true, Ordering::Relaxed);
    let _ = TcpStream::connect(("127.0.0.1", handle.tcp_port));
    let _ = UdpSocket::bind("0.0.0.0:0").and_then(|sock| {
        sock.set_broadcast(true)?;
        let addr = format!("255.255.255.255:{}", handle.udp_port);
        let _ = sock.send_to(b"stop", addr);
        Ok(())
    });
    for worker in handle.workers.drain(..) {
        let _ = worker.join();
    }
}

fn bind_tcp_listener(base: u16) -> std::io::Result<TcpListener> {
    let mut last_err = None;
    for offset in 0..20 {
        let port = base.saturating_add(offset);
        match TcpListener::bind(("0.0.0.0", port)) {
            Ok(listener) => return Ok(listener),
            Err(err) => last_err = Some(err),
        }
    }
    Err(last_err.unwrap_or_else(|| std::io::Error::from(std::io::ErrorKind::AddrInUse)))
}

fn ensure_firewall_rules(tcp_port: u16, udp_port: u16) -> Result<(), String> {
    let exe = std::env::current_exe()
        .map_err(|err| format!("读取程序路径失败 {err}"))?
        .to_string_lossy()
        .to_string();
    let exe_key = hash_string(&exe).chars().take(8).collect::<String>();
    ensure_firewall_rule(
        &format!("ZSClip LAN Sync TCP {tcp_port} {exe_key}"),
        "TCP",
        tcp_port,
        &exe,
    )?;
    ensure_firewall_rule(
        &format!("ZSClip LAN Discovery UDP {udp_port} {exe_key}"),
        "UDP",
        udp_port,
        &exe,
    )?;
    Ok(())
}

fn ensure_firewall_rule(
    name: &str,
    protocol: &str,
    local_port: u16,
    exe: &str,
) -> Result<(), String> {
    if firewall_rule_exists(name) {
        return Ok(());
    }
    let port = local_port.to_string();
    let output = run_netsh(&[
        "advfirewall",
        "firewall",
        "add",
        "rule",
        &format!("name={name}"),
        "dir=in",
        "action=allow",
        &format!("program={exe}"),
        &format!("protocol={protocol}"),
        &format!("localport={port}"),
        "profile=private,domain",
        "enable=yes",
    ])?;
    if output.status.success() || firewall_rule_exists(name) {
        Ok(())
    } else {
        Err(format!(
            "{}；请以管理员身份运行一次，或在 Windows 防火墙中允许 ZSClip 专用网络访问",
            command_output_summary(&output)
        ))
    }
}

fn firewall_rule_exists(name: &str) -> bool {
    run_netsh(&[
        "advfirewall",
        "firewall",
        "show",
        "rule",
        &format!("name={name}"),
    ])
    .map(|output| {
        let text = command_output_summary(&output);
        output.status.success()
            && !text.trim().is_empty()
            && !text.to_ascii_lowercase().contains("no rules match")
            && !text.contains("没有与指定条件匹配的规则")
            && !text.contains("找不到")
    })
    .unwrap_or(false)
}

fn run_netsh(args: &[&str]) -> Result<std::process::Output, String> {
    let mut command = Command::new("netsh");
    command
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        command.creation_flags(CREATE_NO_WINDOW_FLAG);
    }
    command
        .output()
        .map_err(|err| format!("无法执行 netsh：{err}"))
}

fn command_output_summary(output: &std::process::Output) -> String {
    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    if !output.stderr.is_empty() {
        if !text.trim().is_empty() {
            text.push(' ');
        }
        text.push_str(&String::from_utf8_lossy(&output.stderr));
    }
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() > 160 {
        compact.chars().take(160).collect()
    } else if compact.is_empty() {
        format!("netsh 退出码 {:?}", output.status.code())
    } else {
        compact
    }
}

fn tcp_server_loop(listener: TcpListener, config: LanRuntimeConfig, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        match listener.accept() {
            Ok((stream, peer)) => {
                let cfg = config.clone();
                thread::spawn(move || handle_http_stream(stream, peer, cfg));
            }
            Err(err) if err.kind() == std::io::ErrorKind::WouldBlock => {
                thread::sleep(Duration::from_millis(80));
            }
            Err(_) => thread::sleep(Duration::from_millis(200)),
        }
    }
}

fn handle_http_stream(mut stream: TcpStream, peer: SocketAddr, config: LanRuntimeConfig) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(8)));
    let _ = stream.set_write_timeout(Some(Duration::from_secs(8)));
    match read_http_request(&mut stream, peer) {
        Ok(req) => route_http_request(&mut stream, req, config),
        Err(_) => {
            let _ = write_http_json(&mut stream, 400, &json!({"error":"bad_request"}));
        }
    }
}

fn route_http_request(stream: &mut TcpStream, req: HttpRequest, config: LanRuntimeConfig) {
    let path = req.path.split('?').next().unwrap_or(req.path.as_str());
    match (req.method.as_str(), path) {
        ("GET", "/v1/info") => {
            let _ = write_http_json(
                stream,
                200,
                &json!({
                    "magic": LAN_MAGIC,
                    "protocol": LAN_PROTOCOL,
                    "device_id": config.device_id,
                    "name": config.device_name,
                    "tcp_port": config.tcp_port,
                    "capabilities": windows_capabilities()
                }),
            );
        }
        ("POST", "/v1/pair/request") => handle_pair_request(stream, req, config),
        ("GET", "/v1/pair/status") => handle_pair_status(stream, req, config),
        ("POST", "/v1/clip") => handle_clip_post(stream, req, config),
        ("GET", "/v1/latest") => handle_latest(stream, req),
        ("GET", "/mobile/images") => handle_mobile_images(stream, req),
        ("GET", "/mobile/image") => handle_mobile_image_download(stream, req),
        ("POST", "/v1/file/start") => handle_file_start(stream, req, config),
        ("POST", "/v1/file/chunk") => handle_file_chunk(stream, req, config),
        ("POST", "/v1/file/finish") => handle_file_finish(stream, req, config),
        _ => {
            let _ = write_http_json(stream, 404, &json!({"error":"not_found"}));
        }
    }
}

fn handle_pair_request(stream: &mut TcpStream, req: HttpRequest, config: LanRuntimeConfig) {
    let Ok(body) = serde_json::from_slice::<PairRequestBody>(&req.body) else {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_pair_request"}));
        return;
    };
    if body.device_id.trim().is_empty() || body.device_id == config.device_id {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_device"}));
        return;
    }
    let pair_id = make_token(12);
    let code = make_pair_code(&config.device_id, &body.device_id);
    let token = make_token(32);
    let addr = match req.peer.ip() {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => ip.to_string(),
    };
    let prompt = LanPairPrompt {
        pair_id: pair_id.clone(),
        code: code.clone(),
        device_name: body.name.clone(),
        addr,
        created_at_ms: now_ms(),
    };
    let pending = PendingPair {
        prompt: prompt.clone(),
        requester_device_id: body.device_id,
        requester_tcp_port: body.tcp_port,
        requester_capabilities: normalize_capabilities(body.capabilities, body.tcp_port),
        token,
        accepted: false,
        rejected: false,
        created_at_ms: now_ms(),
    };
    if let Ok(mut pairs) = pending_pair_slot().lock() {
        pairs.retain(|pair| now_ms().saturating_sub(pair.created_at_ms) < 10 * 60 * 1000);
        pairs.push(pending);
    }
    if let Ok(mut q) = pair_prompt_slot().lock() {
        q.push_back(prompt);
    }
    set_status("收到配对请求，请在局域网设置页选择请求并点击允许");
    post_ready(config.hwnd);
    let _ = write_http_json(
        stream,
        200,
        &json!({"pair_id": pair_id, "code": code, "status":"pending"}),
    );
}

fn handle_pair_status(stream: &mut TcpStream, req: HttpRequest, config: LanRuntimeConfig) {
    let pair_id = query_param(&req.path, "id");
    let pairs = pending_pair_slot().lock().unwrap();
    let Some(pair) = pairs.iter().find(|pair| pair.prompt.pair_id == pair_id) else {
        let _ = write_http_json(stream, 404, &json!({"status":"missing"}));
        return;
    };
    if pair.rejected {
        let _ = write_http_json(stream, 200, &json!({"status":"rejected"}));
        return;
    }
    if pair.accepted {
        let _ = write_http_json(
            stream,
            200,
            &json!({
                "status":"accepted",
                "token": pair.token,
                "device_id": config.device_id,
                "name": config.device_name,
                "tcp_port": config.tcp_port,
                "capabilities": windows_capabilities()
            }),
        );
        return;
    }
    let _ = write_http_json(stream, 200, &json!({"status":"pending"}));
}

fn handle_clip_post(stream: &mut TcpStream, req: HttpRequest, config: LanRuntimeConfig) {
    let Some(device) = authenticated_device(&req) else {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    };
    let Ok(envelope) = serde_json::from_slice::<LanClipEnvelope>(&req.body) else {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_clip"}));
        return;
    };
    if envelope.origin_device_id == config.device_id {
        let _ = write_http_json(stream, 200, &json!({"ok":true,"ignored":"self"}));
        return;
    }
    if envelope.kind == "image" {
        let Some(image) = &envelope.image_png_base64 else {
            let _ = write_http_json(stream, 400, &json!({"error":"missing_image"}));
            return;
        };
        if image.len() > LAN_IMAGE_MAX_BYTES * 2 {
            let _ = write_http_json(stream, 413, &json!({"error":"image_too_large"}));
            return;
        }
    }
    let dedupe_key = format!(
        "{}:{}:{}",
        envelope.origin_device_id, envelope.origin_seq, envelope.hash
    );
    if !remember_seen_message_key(dedupe_key) {
        let _ = write_http_json(stream, 200, &json!({"ok":true,"duplicate":true}));
        return;
    }
    if let Ok(mut q) = incoming_slot().lock() {
        q.push_back(LanIncomingClip {
            envelope,
            source_device_name: device.name.clone(),
        });
    }
    let mut updated = device;
    updated.addr = req.peer.ip().to_string();
    updated.last_seen_ms = now_ms();
    upsert_device(updated);
    post_ready(config.hwnd);
    let _ = write_http_json(stream, 200, &json!({"ok":true}));
}

fn handle_latest(stream: &mut TcpStream, req: HttpRequest) {
    if authenticated_device(&req).is_none() {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    }
    let latest = latest_clip_slot()
        .lock()
        .ok()
        .and_then(|latest| latest.clone());
    let _ = write_http_json(stream, 200, &json!({"clip": latest}));
}

fn handle_mobile_images(stream: &mut TcpStream, req: HttpRequest) {
    let Some(device) = authenticated_mobile_query(&req) else {
        let _ = write_http_bytes(
            stream,
            401,
            "text/html; charset=utf-8",
            unauthorized_mobile_html().as_bytes(),
            &[],
        );
        return;
    };
    let items = load_mobile_image_list(MOBILE_IMAGE_LIST_LIMIT).unwrap_or_default();
    let html = render_mobile_images_page(&device, &query_param(&req.path, "token"), &items);
    let _ = write_http_bytes(
        stream,
        200,
        "text/html; charset=utf-8",
        html.as_bytes(),
        &[],
    );
}

fn handle_mobile_image_download(stream: &mut TcpStream, req: HttpRequest) {
    if authenticated_mobile_query(&req).is_none() {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    }
    let id = query_param(&req.path, "id").parse::<i64>().unwrap_or(0);
    if id <= 0 {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_image_id"}));
        return;
    }
    let Ok(Some((png_bytes, preview))) = load_mobile_image_png(id) else {
        let _ = write_http_json(stream, 404, &json!({"error":"image_not_found"}));
        return;
    };
    if png_bytes.len() > LAN_IMAGE_MAX_BYTES {
        let _ = write_http_json(stream, 413, &json!({"error":"image_too_large"}));
        return;
    }
    let filename = mobile_image_download_name(id, &preview);
    let _ = write_http_bytes(
        stream,
        200,
        "image/png",
        &png_bytes,
        &[(
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        )],
    );
}

fn authenticated_mobile_query(req: &HttpRequest) -> Option<LanDevice> {
    let id = query_param_decoded(&req.path, "device");
    let token = query_param_decoded(&req.path, "token");
    if id.trim().is_empty() || token.trim().is_empty() {
        return None;
    }
    load_devices()
        .into_iter()
        .find(|device| device.trusted && device.device_id == id && device.token == token)
}

fn load_mobile_image_list(limit: i64) -> rusqlite::Result<Vec<MobileImageListItem>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, preview, COALESCE(source_app, ''), COALESCE(image_path, ''), \
             COALESCE(length(image_data), 0), image_width, image_height, \
             COALESCE(created_at, '') \
             FROM items WHERE category=0 AND kind='image' ORDER BY id DESC LIMIT ?",
        )?;
        let rows = stmt.query_map(params![limit.max(1)], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
                row.get::<_, i64>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, String>(7)?,
            ))
        })?;
        let mut items = Vec::new();
        for row in rows {
            let (id, preview, source_app, image_path, image_data_len, width, height, created_at) =
                row?;
            let size = readable_mobile_image_size(&image_path, image_data_len);
            if let Some(size) = size {
                items.push(MobileImageListItem {
                    id,
                    preview,
                    source_app,
                    created_at,
                    size,
                    width,
                    height,
                });
            }
        }
        Ok(items)
    })
}

fn readable_mobile_image_size(image_path: &str, image_data_len: i64) -> Option<u64> {
    let image_path = image_path.trim();
    if !image_path.is_empty() {
        if let Ok(meta) = fs::metadata(image_path) {
            if meta.is_file() && meta.len() > 0 && meta.len() <= LAN_IMAGE_MAX_BYTES as u64 {
                return Some(meta.len());
            }
        }
    }
    if image_data_len > 0 {
        return Some(image_data_len as u64);
    }
    None
}

fn load_mobile_image_png(id: i64) -> rusqlite::Result<Option<(Vec<u8>, String)>> {
    let row = with_db(|conn| {
        conn.query_row(
            "SELECT COALESCE(preview, ''), image_data, COALESCE(image_path, ''), \
             image_width, image_height \
             FROM items WHERE category=0 AND kind='image' AND id=?",
            params![id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<Vec<u8>>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, i64>(3)?,
                    row.get::<_, i64>(4)?,
                ))
            },
        )
        .optional()
    })?;
    let Some((preview, image_data, image_path, width, height)) = row else {
        return Ok(None);
    };
    if !image_path.trim().is_empty() {
        if let Ok(bytes) = fs::read(image_path.trim()) {
            if bytes.len() <= LAN_IMAGE_MAX_BYTES && png_dimensions_from_bytes(&bytes).is_some() {
                return Ok(Some((bytes, preview)));
            }
        }
    }
    let Some(bytes) = image_data else {
        return Ok(None);
    };
    let Some(encoded) = encode_rgba_png_bytes(&bytes, width as u32, height as u32) else {
        return Ok(None);
    };
    if encoded.len() > LAN_IMAGE_MAX_BYTES {
        return Ok(None);
    }
    Ok(Some((encoded, preview)))
}

fn render_mobile_images_page(
    device: &LanDevice,
    token: &str,
    items: &[MobileImageListItem],
) -> String {
    let mut html = String::new();
    html.push_str("<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    html.push_str("<title>ZSClip 图片下载</title><style>");
    html.push_str("body{margin:0;background:#f6f7f9;color:#202124;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif}");
    html.push_str("main{max-width:760px;margin:0 auto;padding:18px}h1{font-size:24px;margin:8px 0 4px}p{color:#5f6368;margin:0 0 16px;line-height:1.5}");
    html.push_str(".item{background:#fff;border:1px solid #e3e6ea;border-radius:8px;padding:14px;margin:10px 0}.name{font-weight:650;word-break:break-word}.meta{font-size:13px;color:#69717c;margin-top:7px;line-height:1.5}.btn{display:inline-block;margin-top:12px;padding:9px 13px;background:#0a84ff;color:#fff;border-radius:7px;text-decoration:none;font-weight:600}.empty{background:#fff;border:1px dashed #c8ccd2;border-radius:8px;padding:18px;color:#69717c}</style></head><body><main>");
    html.push_str("<h1>ZSClip 图片下载</h1><p>已配对设备：");
    html.push_str(&html_escape(&device.name));
    html.push_str("。这里列出最近可下载的图片记录，不会自动加载原图。</p>");
    if items.is_empty() {
        html.push_str("<div class=\"empty\">暂无可下载图片。</div>");
    } else {
        for item in items {
            let preview = if item.preview.trim().is_empty() {
                format!("图片 {}", item.id)
            } else {
                item.preview.clone()
            };
            let href = format!(
                "/mobile/image?id={}&device={}&token={}",
                item.id,
                url_encode_component(&device.device_id),
                url_encode_component(token)
            );
            html.push_str("<section class=\"item\"><div class=\"name\">");
            html.push_str(&html_escape(&preview));
            html.push_str("</div><div class=\"meta\">");
            if !item.created_at.trim().is_empty() {
                html.push_str("时间：");
                html.push_str(&html_escape(&item.created_at));
                html.push_str("<br>");
            }
            let source = item.source_app.trim();
            if !source.is_empty() {
                html.push_str("来源：");
                html.push_str(&html_escape(source));
                html.push_str("<br>");
            }
            if item.width > 0 && item.height > 0 {
                html.push_str(&format!("尺寸：{} x {}<br>", item.width, item.height));
            }
            html.push_str("大小：");
            html.push_str(&format_file_size(item.size));
            html.push_str("</div><a class=\"btn\" href=\"");
            html.push_str(&href);
            html.push_str("\">下载 PNG</a></section>");
        }
    }
    html.push_str("</main></body></html>");
    html
}

fn unauthorized_mobile_html() -> String {
    "<!doctype html><meta charset=\"utf-8\"><meta name=\"viewport\" content=\"width=device-width,initial-scale=1\"><title>未授权</title><body style=\"font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif;padding:24px;background:#f6f7f9;color:#202124\"><h1>未授权</h1><p>请先完成 ZSClip 局域网配对，再从手机端打开图片下载页。</p></body>".to_string()
}

fn mobile_image_download_name(id: i64, preview: &str) -> String {
    let mut name = preview
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '-' || ch == '_' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    name = name.trim_matches('_').chars().take(40).collect();
    if name.is_empty() {
        name = format!("zsclip_image_{id}");
    }
    if !name.ends_with(".png") {
        name.push_str(".png");
    }
    name
}

fn encode_rgba_png_bytes(bytes: &[u8], width: u32, height: u32) -> Option<Vec<u8>> {
    if width == 0 || height == 0 || bytes.len() != width as usize * height as usize * 4 {
        return None;
    }
    let mut out = Vec::new();
    {
        let mut encoder = png::Encoder::new(&mut out, width, height);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        let mut writer = encoder.write_header().ok()?;
        writer.write_image_data(bytes).ok()?;
    }
    Some(out)
}

fn png_dimensions_from_bytes(bytes: &[u8]) -> Option<(usize, usize)> {
    let cursor = std::io::Cursor::new(bytes);
    let decoder = png::Decoder::new(cursor);
    let reader = decoder.read_info().ok()?;
    let info = reader.info();
    Some((info.width as usize, info.height as usize))
}

fn file_session_key(source_device_id: &str, transfer_id: &str) -> String {
    format!("{}:{}", source_device_id.trim(), transfer_id.trim())
}

fn file_content_hasher(total_size: u64) -> crc32fast::Hasher {
    let mut hasher = crc32fast::Hasher::new();
    hasher.update(&(b"file".len() as u64).to_le_bytes());
    hasher.update(b"file");
    hasher.update(&total_size.to_le_bytes());
    hasher
}

fn handle_file_start(stream: &mut TcpStream, req: HttpRequest, _config: LanRuntimeConfig) {
    let Some(device) = authenticated_device(&req) else {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&req.body) else {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_file_start"}));
        return;
    };
    let transfer_id = value
        .get("transfer_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let file_name = safe_file_name(
        value
            .get("file_name")
            .and_then(|v| v.as_str())
            .unwrap_or("file.bin"),
    );
    let total_size = value
        .get("total_size")
        .and_then(|v| v.as_u64())
        .unwrap_or(0);
    if transfer_id.is_empty() || total_size == 0 || total_size > LAN_FILE_MAX_BYTES {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_file_meta"}));
        return;
    }
    let rel_dir = PathBuf::from("lan_received")
        .join(safe_file_name(&device.device_id))
        .join((now_ms() / 86_400_000).to_string());
    let dir = data_dir().join(&rel_dir);
    if fs::create_dir_all(&dir).is_err() {
        let _ = write_http_json(stream, 500, &json!({"error":"create_dir_failed"}));
        return;
    }
    let final_name = format!("{}_{}", now_ms(), file_name);
    let final_path = dir.join(&final_name);
    let part_path = final_path.with_extension("part");
    let relative_path = rel_dir
        .join(&final_name)
        .to_string_lossy()
        .replace('\\', "/");
    let session = FileSession {
        source_device_id: device.device_id.clone(),
        source_device_name: device.name.clone(),
        transfer_id: transfer_id.clone(),
        final_path,
        part_path,
        relative_path,
        file_name,
        total_size,
        received: 0,
        content_crc: file_content_hasher(total_size),
    };
    let key = file_session_key(&device.device_id, &transfer_id);
    if let Ok(mut sessions) = file_session_slot().lock() {
        if let Some(old) = sessions.insert(key, session) {
            let _ = fs::remove_file(old.part_path);
        }
    }
    let _ = write_http_json(stream, 200, &json!({"ok":true}));
}

fn handle_file_chunk(stream: &mut TcpStream, req: HttpRequest, _config: LanRuntimeConfig) {
    let Some(device) = authenticated_device(&req) else {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&req.body) else {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_file_chunk"}));
        return;
    };
    let transfer_id = value
        .get("transfer_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let offset = value.get("offset").and_then(|v| v.as_u64()).unwrap_or(0);
    let data = value
        .get("data_base64")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let Ok(bytes) = general_purpose::STANDARD.decode(data) else {
        let _ = write_http_json(stream, 400, &json!({"error":"bad_chunk_encoding"}));
        return;
    };
    if bytes.is_empty() || bytes.len() > LAN_FILE_CHUNK_BYTES * 2 {
        let _ = write_http_json(stream, 413, &json!({"error":"chunk_too_large"}));
        return;
    }
    let key = file_session_key(&device.device_id, transfer_id);
    let mut sessions = file_session_slot().lock().unwrap();
    let Some(session) = sessions.get_mut(&key) else {
        let _ = write_http_json(stream, 404, &json!({"error":"missing_session"}));
        return;
    };
    if session.received != offset {
        let _ = write_http_json(
            stream,
            409,
            &json!({"error":"offset_mismatch","expected":session.received}),
        );
        return;
    }
    let write_result = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&session.part_path)
        .and_then(|mut file| file.write_all(&bytes));
    if write_result.is_err() {
        let _ = write_http_json(stream, 500, &json!({"error":"write_failed"}));
        return;
    }
    session.received += bytes.len() as u64;
    session.content_crc.update(&bytes);
    let _ = write_http_json(stream, 200, &json!({"ok":true,"received":session.received}));
}

fn handle_file_finish(stream: &mut TcpStream, req: HttpRequest, config: LanRuntimeConfig) {
    let Some(device) = authenticated_device(&req) else {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&req.body) else {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_file_finish"}));
        return;
    };
    let transfer_id = value
        .get("transfer_id")
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let key = file_session_key(&device.device_id, transfer_id);
    let Some(session) = file_session_slot().lock().unwrap().remove(&key) else {
        let _ = write_http_json(stream, 404, &json!({"error":"missing_session"}));
        return;
    };
    if session.received != session.total_size {
        let _ = fs::remove_file(&session.part_path);
        let _ = write_http_json(stream, 409, &json!({"error":"incomplete_file"}));
        return;
    }
    if fs::rename(&session.part_path, &session.final_path).is_err() {
        let _ = write_http_json(stream, 500, &json!({"error":"finish_failed"}));
        return;
    }
    let content_signature = format!("crc:{:08x}", session.content_crc.finalize());
    let seen_key = format!(
        "{}:{}:{}",
        session.source_device_id, session.transfer_id, content_signature
    );
    if !remember_seen_message_key(seen_key) {
        let _ = fs::remove_file(&session.final_path);
        let _ = write_http_json(stream, 200, &json!({"ok":true,"duplicate":true}));
        return;
    }
    let envelope = LanClipEnvelope {
        message_id: format!("file-{}-{}", session.source_device_id, session.transfer_id),
        origin_device_id: session.source_device_id.clone(),
        origin_seq: now_ms(),
        kind: "files".to_string(),
        hash: content_signature,
        created_at_ms: now_ms(),
        preview: session.file_name.clone(),
        text: None,
        image_png_base64: None,
        file_meta: vec![LanFileMeta {
            name: session.file_name,
            size: session.total_size,
            relative_path: session.relative_path,
        }],
    };
    if let Ok(mut q) = incoming_slot().lock() {
        q.push_back(LanIncomingClip {
            envelope,
            source_device_name: session.source_device_name,
        });
    }
    post_ready(config.hwnd);
    let _ = write_http_json(stream, 200, &json!({"ok":true}));
}

fn authenticated_device(req: &HttpRequest) -> Option<LanDevice> {
    let id = header_value(req, "x-zsclip-device")?;
    let token = header_value(req, "x-zsclip-token")?;
    load_devices()
        .into_iter()
        .find(|device| device.trusted && device.device_id == id && device.token == token)
}

fn udp_discovery_sender(config: LanRuntimeConfig, stop: Arc<AtomicBool>) {
    while !stop.load(Ordering::Relaxed) {
        send_discovery_once(&config);
        let mut waited = 0;
        while waited < DISCOVERY_INTERVAL_MS && !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(250));
            waited += 250;
        }
    }
}

fn send_discovery_once(config: &LanRuntimeConfig) {
    let Ok(sock) = UdpSocket::bind("0.0.0.0:0") else {
        return;
    };
    let _ = sock.set_broadcast(true);
    let packet = DiscoveryPacket {
        magic: LAN_MAGIC.to_string(),
        protocol: LAN_PROTOCOL,
        device_id: config.device_id.clone(),
        name: config.device_name.clone(),
        tcp_port: config.tcp_port,
        capabilities: vec![
            "text".to_string(),
            "image".to_string(),
            "latest".to_string(),
            "manual_file".to_string(),
            "receive_clip".to_string(),
        ],
    };
    let body = serde_json::to_vec(&packet).unwrap_or_default();
    let _ = sock.send_to(
        &body,
        format!("255.255.255.255:{}", config.udp_port).as_str(),
    );
}

fn udp_discovery_listener(config: LanRuntimeConfig, stop: Arc<AtomicBool>) {
    let Ok(sock) = UdpSocket::bind(("0.0.0.0", config.udp_port)) else {
        set_status("UDP 发现端口绑定失败，仍可手动 IP 配对");
        return;
    };
    let _ = sock.set_read_timeout(Some(Duration::from_millis(800)));
    let mut buf = [0u8; 4096];
    while !stop.load(Ordering::Relaxed) {
        let Ok((len, peer)) = sock.recv_from(&mut buf) else {
            continue;
        };
        let Ok(packet) = serde_json::from_slice::<DiscoveryPacket>(&buf[..len]) else {
            continue;
        };
        if packet.magic != LAN_MAGIC || packet.protocol != LAN_PROTOCOL {
            continue;
        }
        if packet.device_id == config.device_id {
            continue;
        }
        let mut device = LanDevice {
            device_id: packet.device_id,
            name: packet.name,
            addr: peer.ip().to_string(),
            tcp_port: packet.tcp_port,
            token: String::new(),
            last_seen_ms: now_ms(),
            trusted: false,
            capabilities: normalize_capabilities(packet.capabilities, packet.tcp_port),
        };
        if let Some(trusted) = load_devices()
            .into_iter()
            .find(|saved| saved.device_id == device.device_id && saved.trusted)
        {
            device.token = trusted.token;
            device.trusted = true;
            upsert_device(device.clone());
        }
        upsert_discovered(device);
        post_ready(config.hwnd);
    }
}

fn read_http_request(stream: &mut TcpStream, peer: SocketAddr) -> std::io::Result<HttpRequest> {
    let mut buf = Vec::with_capacity(8192);
    let mut tmp = [0u8; 4096];
    let mut header_end = None;
    while header_end.is_none() && buf.len() < 64 * 1024 {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        buf.extend_from_slice(&tmp[..n]);
        header_end = find_header_end(&buf);
    }
    let Some(header_end) = header_end else {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    };
    let header_text = String::from_utf8_lossy(&buf[..header_end]);
    let mut lines = header_text.lines();
    let request_line = lines.next().unwrap_or_default();
    let mut parts = request_line.split_whitespace();
    let method = parts.next().unwrap_or_default().to_string();
    let path = parts.next().unwrap_or("/").to_string();
    let mut headers = Vec::new();
    let mut content_len = 0usize;
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim().to_string();
        let value = value.trim().to_string();
        if name.eq_ignore_ascii_case("content-length") {
            content_len = value.parse::<usize>().unwrap_or(0);
        }
        headers.push((name, value));
    }
    if content_len > HTTP_MAX_BODY {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidData));
    }
    let body_start = header_end + 4;
    let mut body = buf.get(body_start..).unwrap_or_default().to_vec();
    while body.len() < content_len {
        let n = stream.read(&mut tmp)?;
        if n == 0 {
            break;
        }
        body.extend_from_slice(&tmp[..n]);
    }
    body.truncate(content_len);
    Ok(HttpRequest {
        method,
        path,
        headers,
        body,
        peer,
    })
}

fn write_http_json(
    stream: &mut TcpStream,
    status: u16,
    value: &serde_json::Value,
) -> std::io::Result<()> {
    let body = serde_json::to_vec(value).unwrap_or_else(|_| b"{}".to_vec());
    let reason = match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        413 => "Payload Too Large",
        501 => "Not Implemented",
        _ => "OK",
    };
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: application/json; charset=utf-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
        status,
        reason,
        body.len()
    )?;
    stream.write_all(&body)
}

fn write_http_bytes(
    stream: &mut TcpStream,
    status: u16,
    content_type: &str,
    body: &[u8],
    extra_headers: &[(&str, String)],
) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 {} {}\r\nContent-Type: {}\r\nContent-Length: {}\r\nConnection: close\r\n",
        status,
        http_reason(status),
        content_type,
        body.len()
    )?;
    for (name, value) in extra_headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(body)
}

fn http_reason(status: u16) -> &'static str {
    match status {
        200 => "OK",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        413 => "Payload Too Large",
        501 => "Not Implemented",
        _ => "OK",
    }
}

fn http_request(
    method: &str,
    host: &str,
    path: &str,
    headers: &[(&str, &str)],
    body: Option<&[u8]>,
    timeout: Duration,
) -> std::io::Result<Vec<u8>> {
    let mut stream = TcpStream::connect(host)?;
    stream.set_read_timeout(Some(timeout))?;
    stream.set_write_timeout(Some(timeout))?;
    let body = body.unwrap_or_default();
    write!(
        stream,
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\nContent-Length: {}\r\n",
        method,
        path,
        host,
        body.len()
    )?;
    for (name, value) in headers {
        write!(stream, "{name}: {value}\r\n")?;
    }
    stream.write_all(b"\r\n")?;
    stream.write_all(body)?;
    let mut resp = Vec::new();
    stream.read_to_end(&mut resp)?;
    let Some(header_end) = find_header_end(&resp) else {
        return Ok(resp);
    };
    Ok(resp.get(header_end + 4..).unwrap_or_default().to_vec())
}

fn push_one_file(sender_id: &str, device: &LanDevice, path: &PathBuf) -> std::io::Result<()> {
    let metadata = fs::metadata(path)?;
    let total_size = metadata.len();
    if total_size == 0 || total_size > LAN_FILE_MAX_BYTES {
        return Err(std::io::Error::from(std::io::ErrorKind::InvalidInput));
    }
    let file_name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("file.bin")
        .to_string();
    let transfer_id = make_token(12);
    let addr = format!("{}:{}", device.addr, device.tcp_port);
    let auth_headers = [
        ("Content-Type", "application/json"),
        ("X-ZSClip-Device", sender_id),
        ("X-ZSClip-Token", device.token.as_str()),
    ];
    let start = json!({
        "transfer_id": &transfer_id,
        "file_name": &file_name,
        "total_size": total_size
    });
    let start_body = serde_json::to_vec(&start).unwrap_or_default();
    http_request(
        "POST",
        &addr,
        "/v1/file/start",
        &auth_headers,
        Some(&start_body),
        Duration::from_secs(10),
    )?;
    let mut file = fs::File::open(path)?;
    let mut offset = 0u64;
    let mut buf = vec![0u8; LAN_FILE_CHUNK_BYTES];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        let chunk = json!({
            "transfer_id": &transfer_id,
            "offset": offset,
            "data_base64": general_purpose::STANDARD.encode(&buf[..n])
        });
        let body = serde_json::to_vec(&chunk).unwrap_or_default();
        http_request(
            "POST",
            &addr,
            "/v1/file/chunk",
            &auth_headers,
            Some(&body),
            Duration::from_secs(20),
        )?;
        offset += n as u64;
    }
    let finish = json!({"transfer_id": &transfer_id});
    let finish_body = serde_json::to_vec(&finish).unwrap_or_default();
    http_request(
        "POST",
        &addr,
        "/v1/file/finish",
        &auth_headers,
        Some(&finish_body),
        Duration::from_secs(10),
    )?;
    Ok(())
}

fn find_header_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn header_value<'a>(req: &'a HttpRequest, name: &str) -> Option<&'a str> {
    req.headers
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(name))
        .map(|(_, v)| v.as_str())
}

fn query_param(path: &str, key: &str) -> String {
    let Some((_, query)) = path.split_once('?') else {
        return String::new();
    };
    for part in query.split('&') {
        let Some((k, v)) = part.split_once('=') else {
            continue;
        };
        if k == key {
            return v.to_string();
        }
    }
    String::new()
}

fn query_param_decoded(path: &str, key: &str) -> String {
    percent_decode(&query_param(path, key))
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0usize;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let (Some(hi), Some(lo)) = (hex_value(bytes[i + 1]), hex_value(bytes[i + 2])) {
                out.push((hi << 4) | lo);
                i += 3;
                continue;
            }
        }
        out.push(if bytes[i] == b'+' { b' ' } else { bytes[i] });
        i += 1;
    }
    String::from_utf8(out).unwrap_or_default()
}

fn hex_value(ch: u8) -> Option<u8> {
    match ch {
        b'0'..=b'9' => Some(ch - b'0'),
        b'a'..=b'f' => Some(ch - b'a' + 10),
        b'A'..=b'F' => Some(ch - b'A' + 10),
        _ => None,
    }
}

fn url_encode_component(value: &str) -> String {
    let mut out = String::new();
    for byte in value.as_bytes() {
        match *byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(*byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn html_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for ch in value.chars() {
        match ch {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            _ => out.push(ch),
        }
    }
    out
}

fn format_file_size(bytes: u64) -> String {
    if bytes >= 1024 * 1024 {
        format!("{:.1} MB", bytes as f64 / 1024.0 / 1024.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

fn normalize_host(raw: &str, default_port: u16) -> String {
    let mut s = raw.trim().to_string();
    if let Some(rest) = s.strip_prefix("http://") {
        s = rest.to_string();
    }
    if let Some(rest) = s.strip_prefix("https://") {
        s = rest.to_string();
    }
    if let Some((host, _path)) = s.split_once('/') {
        s = host.to_string();
    }
    if s.is_empty() {
        return String::new();
    }
    if s.contains(':') {
        s
    } else {
        format!("{s}:{default_port}")
    }
}

fn load_devices() -> Vec<LanDevice> {
    let text = fs::read_to_string(device_book_path()).unwrap_or_default();
    let Ok(book) = serde_json::from_str::<StoredDeviceBook>(&text) else {
        return Vec::new();
    };
    book.devices
        .into_iter()
        .filter_map(|stored| {
            let token = decrypt_secret_from_storage(&stored.token_encrypted)?;
            Some(LanDevice {
                device_id: stored.device_id,
                name: stored.name,
                addr: stored.addr,
                tcp_port: stored.tcp_port,
                token,
                last_seen_ms: stored.last_seen_ms,
                trusted: stored.trusted,
                capabilities: normalize_capabilities(stored.capabilities, stored.tcp_port),
            })
        })
        .collect()
}

fn save_devices(devices: &[LanDevice]) {
    let _ = fs::create_dir_all(data_dir());
    let stored = StoredDeviceBook {
        devices: devices
            .iter()
            .filter(|device| device.trusted)
            .filter_map(|device| {
                let token_encrypted = encrypt_secret_for_storage(&device.token)?;
                Some(StoredLanDevice {
                    device_id: device.device_id.clone(),
                    name: device.name.clone(),
                    addr: device.addr.clone(),
                    tcp_port: device.tcp_port,
                    token_encrypted,
                    last_seen_ms: device.last_seen_ms,
                    trusted: device.trusted,
                    capabilities: normalize_capabilities(
                        device.capabilities.clone(),
                        device.tcp_port,
                    ),
                })
            })
            .collect(),
    };
    if let Ok(text) = serde_json::to_string_pretty(&stored) {
        let _ = fs::write(device_book_path(), text);
    }
}

fn upsert_device(device: LanDevice) {
    let mut devices = load_devices();
    if let Some(existing) = devices
        .iter_mut()
        .find(|existing| existing.device_id == device.device_id)
    {
        *existing = device;
    } else {
        devices.push(device);
    }
    save_devices(&devices);
}

fn upsert_discovered(device: LanDevice) {
    if let Ok(mut devices) = discovered_slot().lock() {
        if let Some(existing) = devices
            .iter_mut()
            .find(|existing| existing.device_id == device.device_id)
        {
            *existing = device;
        } else {
            devices.push(device);
        }
        devices.retain(|device| now_ms().saturating_sub(device.last_seen_ms) < 5 * 60 * 1000);
    }
}

fn device_book_path() -> PathBuf {
    data_dir().join("lan_devices.json")
}

fn safe_file_name(name: &str) -> String {
    let cleaned: String = name
        .chars()
        .map(|ch| match ch {
            '<' | '>' | ':' | '"' | '/' | '\\' | '|' | '?' | '*' => '_',
            ch if ch.is_control() => '_',
            ch => ch,
        })
        .collect();
    let trimmed = cleaned.trim().trim_matches('.').trim();
    if trimmed.is_empty() {
        "file.bin".to_string()
    } else {
        trimmed.chars().take(120).collect()
    }
}

fn hash_string(text: &str) -> String {
    format!("{:x}", md5::compute(text.as_bytes()))
}

fn make_pair_code(a: &str, b: &str) -> String {
    let seed = format!("{}:{}:{}", a, b, now_ms() / 60_000);
    let digest = md5::compute(seed.as_bytes());
    let value = u32::from_le_bytes([digest[0], digest[1], digest[2], digest[3]]) % 1_000_000;
    format!("{value:06}")
}

fn make_token(bytes: usize) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let mut out = String::new();
    while out.len() < bytes * 2 {
        let seed = format!(
            "{}:{}:{}",
            now_ms(),
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let digest = md5::compute(seed.as_bytes());
        for byte in digest.0 {
            out.push_str(&format!("{byte:02x}"));
            if out.len() >= bytes * 2 {
                break;
            }
        }
    }
    out
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn post_ready(hwnd: isize) {
    if hwnd == 0 {
        return;
    }
    unsafe {
        let _ = PostMessageW(hwnd as HWND, WM_LAN_SYNC_READY, 0, 0);
    }
}

fn set_status(text: &str) {
    if let Ok(mut status) = status_slot().lock() {
        *status = text.to_string();
    }
}

fn set_status_if_initialized(text: &str) {
    if let Some(slot) = STATUS_TEXT.get() {
        if let Ok(mut status) = slot.lock() {
            *status = text.to_string();
        }
    }
}

fn service_slot() -> &'static Mutex<Option<LanServiceHandle>> {
    SERVICE.get_or_init(|| Mutex::new(None))
}

fn discovered_slot() -> &'static Mutex<Vec<LanDevice>> {
    DISCOVERED.get_or_init(|| Mutex::new(Vec::new()))
}

fn incoming_slot() -> &'static Mutex<VecDeque<LanIncomingClip>> {
    INCOMING_CLIPS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn pair_prompt_slot() -> &'static Mutex<VecDeque<LanPairPrompt>> {
    PAIR_PROMPTS.get_or_init(|| Mutex::new(VecDeque::new()))
}

fn pending_pair_slot() -> &'static Mutex<Vec<PendingPair>> {
    PENDING_PAIRS.get_or_init(|| Mutex::new(Vec::new()))
}

fn seen_slot() -> &'static Mutex<HashSet<String>> {
    SEEN_MESSAGES.get_or_init(|| Mutex::new(HashSet::new()))
}

fn status_slot() -> &'static Mutex<String> {
    STATUS_TEXT.get_or_init(|| Mutex::new(String::new()))
}

fn latest_clip_slot() -> &'static Mutex<Option<LanClipEnvelope>> {
    LATEST_CLIP.get_or_init(|| Mutex::new(None))
}

fn file_session_slot() -> &'static Mutex<HashMap<String, FileSession>> {
    FILE_SESSIONS.get_or_init(|| Mutex::new(HashMap::new()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_device(capabilities: Vec<&str>, tcp_port: u16) -> LanDevice {
        LanDevice {
            device_id: "dev".to_string(),
            name: "Device".to_string(),
            addr: "127.0.0.1".to_string(),
            tcp_port,
            token: "token".to_string(),
            last_seen_ms: 0,
            trusted: true,
            capabilities: capabilities
                .into_iter()
                .map(|cap| cap.to_string())
                .collect(),
        }
    }

    #[test]
    fn bdd_pull_only_android_device_is_not_a_push_target() {
        // Given Android declares itself as client_only/pull_only.
        // When Windows chooses LAN push targets.
        // Then the Android device is skipped.
        assert!(!device_can_receive_clip(&test_device(
            vec!["text", "latest", "client_only", "pull_only"],
            0,
        )));
        assert!(!device_can_receive_clip(&test_device(
            vec!["pull_only"],
            38473
        )));
        assert!(device_can_receive_clip(&test_device(
            vec!["text", "image", "receive_clip"],
            38473,
        )));
    }

    #[test]
    fn bdd_capabilities_fallback_marks_zero_port_as_pull_only() {
        assert_eq!(
            normalize_capabilities(Vec::new(), 0),
            vec!["client_only".to_string(), "pull_only".to_string()]
        );
        assert!(normalize_capabilities(Vec::new(), 38473)
            .iter()
            .any(|cap| cap == "receive_clip"));
    }

    #[test]
    fn bdd_file_session_key_is_scoped_by_source_device() {
        assert_eq!(
            file_session_key("android-a", "transfer-1"),
            "android-a:transfer-1"
        );
        assert_ne!(
            file_session_key("android-a", "transfer-1"),
            file_session_key("android-b", "transfer-1")
        );
    }

    #[test]
    fn bdd_file_content_crc_is_stable_for_retried_transfer() {
        let mut first = file_content_hasher(3);
        first.update(b"abc");
        let mut second = file_content_hasher(3);
        second.update(b"abc");

        assert_eq!(first.finalize(), second.finalize());
        assert_eq!(
            format!("{:08x}", file_content_crc_for_test(3, b"abc")),
            "478ce2c3"
        );
    }

    #[test]
    fn bdd_seen_message_key_blocks_duplicate_retries() {
        let key = format!("test-device:42:crc:{:08x}", now_ms());
        assert!(remember_seen_message_key(key.clone()));
        assert!(!remember_seen_message_key(key));
    }

    #[test]
    fn bdd_mobile_image_page_escapes_text_and_uses_download_links() {
        let device = test_device(vec!["client_only"], 0);
        let items = vec![MobileImageListItem {
            id: 7,
            preview: "<script>bad</script>".to_string(),
            source_app: "微信 & 浏览器".to_string(),
            created_at: "2026-05-13 10:00:00".to_string(),
            size: 1536,
            width: 32,
            height: 24,
        }];

        let html = render_mobile_images_page(&device, "tok en", &items);

        assert!(html.contains("&lt;script&gt;bad&lt;/script&gt;"));
        assert!(html.contains("微信 &amp; 浏览器"));
        assert!(html.contains("/mobile/image?id=7"));
        assert!(html.contains("token=tok%20en"));
        assert!(!html.contains("<script>bad</script>"));
    }

    #[test]
    fn bdd_mobile_query_params_support_percent_encoded_ios_device_names() {
        let path = "/mobile/images?device=ios-%E6%89%8B%E6%9C%BA&token=a%2Bb";

        assert_eq!(query_param_decoded(path, "device"), "ios-手机");
        assert_eq!(query_param_decoded(path, "token"), "a+b");
    }

    #[test]
    fn bdd_mobile_download_name_is_safe_png() {
        assert_eq!(
            mobile_image_download_name(3, "截图: 合同/报价"),
            "zsclip_image_3.png"
        );
        assert_eq!(mobile_image_download_name(4, "image_01"), "image_01.png");
    }

    #[test]
    fn bdd_mobile_image_png_encoder_rejects_bad_dimensions() {
        assert!(encode_rgba_png_bytes(&[0, 0, 0, 255], 1, 1).is_some());
        assert!(encode_rgba_png_bytes(&[0, 0, 0], 1, 1).is_none());
    }

    fn file_content_crc_for_test(total_size: u64, bytes: &[u8]) -> u32 {
        let mut hasher = file_content_hasher(total_size);
        hasher.update(bytes);
        hasher.finalize()
    }
}
