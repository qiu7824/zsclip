use crate::app::state::AppSettings;
use crate::app::{data_dir, save_settings, WM_LAN_SYNC_READY};
use crate::db_runtime::with_db;
pub(crate) use crate::lan_sync_core::{
    header_value, http_request, lan_desktop_capabilities, lan_device_can_receive_clip,
    lan_file_content_hasher, lan_file_session_key, lan_hash_string, lan_http_route_for,
    lan_pair_status_response_value, lan_tcp_bind_candidates, make_lan_pair_code, make_lan_token,
    mobile_item_file_path_parts, mobile_item_image_path_id, normalize_lan_capabilities,
    normalize_lan_host, push_lan_file_payload_to_device, query_param, query_param_decoded,
    read_http_request, remember_lan_seen_message_key, safe_lan_file_name, url_encode_component,
    write_http_bytes, write_http_file, write_http_json, DiscoveryPacket, HttpRequest,
    LanClipEnvelope, LanDevice, LanFileMeta, LanHttpRoute, LanHttpRoutePolicy, LanIncomingClip,
    LanPairPrompt, LanPendingPair, LanRuntimeConfig, LanRuntimeEventSink,
    LanRuntimePlatformContext, LanRuntimeSettings, LanServiceLifecyclePlan, LanServiceRuntimeState,
    PairRequestBody, DISCOVERY_INTERVAL_MS, LAN_DISCOVERY_PORT_DEFAULT, LAN_FILE_AUTO_MAX_BYTES,
    LAN_FILE_CHUNK_BYTES, LAN_FILE_MAX_BYTES, LAN_IMAGE_MAX_BYTES, LAN_MAGIC,
    LAN_PAIR_REQUEST_TTL_MS, LAN_PROTOCOL, LAN_TCP_PORT_DEFAULT, MOBILE_IMAGE_LIST_LIMIT,
    MOBILE_ITEM_LIST_LIMIT_DEFAULT, MOBILE_ITEM_LIST_LIMIT_MAX, WPS_TASKPANE_ITEM_LIMIT,
};
use crate::platform::secret_store::{decrypt_secret_from_storage, encrypt_secret_for_storage};
use crate::platform::window as platform_window;
use base64::{engine::general_purpose, Engine as _};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;
use serde_json::json;
use std::collections::{HashMap, HashSet, VecDeque};
use std::fs;
use std::io::Write;
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

const CREATE_NO_WINDOW_FLAG: u32 = 0x0800_0000;

static SERVICE: OnceLock<Mutex<Option<LanServiceHandle>>> = OnceLock::new();
static DISCOVERED: OnceLock<Mutex<Vec<LanDevice>>> = OnceLock::new();
static INCOMING_CLIPS: OnceLock<Mutex<VecDeque<LanIncomingClip>>> = OnceLock::new();
static PAIR_PROMPTS: OnceLock<Mutex<VecDeque<LanPairPrompt>>> = OnceLock::new();
static PENDING_PAIRS: OnceLock<Mutex<Vec<LanPendingPair>>> = OnceLock::new();
static SEEN_MESSAGES: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
static STATUS_TEXT: OnceLock<Mutex<String>> = OnceLock::new();
static LATEST_CLIP: OnceLock<Mutex<Option<LanClipEnvelope>>> = OnceLock::new();
static FILE_SESSIONS: OnceLock<Mutex<HashMap<String, FileSession>>> = OnceLock::new();
static LOCAL_LAN_HOST_CACHE: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static ORIGIN_SEQ: AtomicU64 = AtomicU64::new(1);

struct LanServiceHandle {
    stop: Arc<AtomicBool>,
    state: LanServiceRuntimeState,
    workers: Vec<JoinHandle<()>>,
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

#[derive(Clone, Debug, Serialize)]
struct MobileItemListFile {
    index: usize,
    name: String,
    size: u64,
}

#[derive(Clone, Debug, Serialize)]
struct MobileItemListItem {
    id: i64,
    kind: String,
    preview: String,
    text: String,
    source_app: String,
    created_at: String,
    size: u64,
    width: Option<i64>,
    height: Option<i64>,
    files: Vec<MobileItemListFile>,
}

#[derive(Serialize)]
struct WpsTaskPaneItem {
    id: i64,
    category: String,
    kind: String,
    preview: String,
    text: String,
    source_app: String,
    created_at: String,
    image_url: String,
    image_width: i64,
    image_height: i64,
}

pub(crate) fn ensure_device_identity(settings: &mut AppSettings) -> bool {
    let mut changed = false;
    if settings.lan_device_id.trim().is_empty() {
        settings.lan_device_id = format!("zsclip-{}", make_lan_token(16, now_ms()));
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

fn lan_runtime_settings_from_app(settings: &AppSettings) -> LanRuntimeSettings {
    LanRuntimeSettings {
        lan_sync_enabled: settings.lan_sync_enabled,
        wps_taskpane_enabled: settings.wps_taskpane_enabled,
        device_id: settings.lan_device_id.clone(),
        device_name: settings.lan_device_name.clone(),
        tcp_port: settings.lan_tcp_port,
        udp_port: settings.lan_udp_port,
    }
}

fn windows_lan_runtime_context(event_sink: LanRuntimeEventSink) -> LanRuntimePlatformContext {
    LanRuntimePlatformContext::new(
        data_dir(),
        event_sink,
        encrypt_secret_for_storage,
        decrypt_secret_from_storage,
    )
}

pub(crate) fn next_origin_seq() -> u64 {
    ORIGIN_SEQ.fetch_add(1, Ordering::Relaxed)
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
        if !remember_lan_seen_message_key(&mut seen, key) {
            return false;
        }
    }
    true
}

pub(crate) fn refresh_service(hwnd: HWND, settings: &AppSettings) {
    let runtime_settings = lan_runtime_settings_from_app(settings);
    if !runtime_settings.runtime_enabled() {
        stop_service();
        clear_lan_host_cache();
        release_idle_memory();
        set_status_if_initialized("未启动");
        return;
    }

    let core_config = runtime_settings.core_config();
    if core_config.lan_enabled && core_config.device_id.is_empty() {
        set_status("未生成设备 ID，请保存设置后重试");
        return;
    }
    let mut guard = service_slot().lock().unwrap();
    let should_restart = guard
        .as_ref()
        .map(|handle| handle.state.requires_restart_for(&core_config))
        .unwrap_or(true);
    if !should_restart {
        return;
    }
    if let Some(handle) = guard.take() {
        stop_handle(handle);
        clear_lan_host_cache();
    }
    let config = LanRuntimeConfig::from_core_config(
        windows_lan_runtime_context(LanRuntimeEventSink::platform_main_window(hwnd as isize)),
        core_config,
    );
    let host_refresh_sink = config.platform.event_sink.clone();
    match start_handle(config) {
        Ok(handle) => {
            if handle.state.lan_enabled {
                refresh_lan_host_cache_in_background(host_refresh_sink);
                let firewall_note =
                    ensure_firewall_rules(handle.state.tcp_port, handle.state.udp_port)
                        .err()
                        .map(|err| format!("；防火墙自动放行失败：{err}"));
                set_status(&format!(
                    "已启动：UDP {} / TCP {}{}",
                    handle.state.udp_port,
                    handle.state.tcp_port,
                    firewall_note.unwrap_or_default()
                ));
            } else {
                set_status(&format!(
                    "WPS task pane: http://127.0.0.1:{}/office/wps/taskpane",
                    handle.state.tcp_port
                ));
            }
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
    clear_lan_host_cache();
}

pub(crate) fn release_idle_memory() {
    if SERVICE
        .get()
        .and_then(|slot| slot.lock().ok())
        .and_then(|guard| guard.as_ref().map(|_| ()))
        .is_some()
    {
        return;
    }
    if let Some(slot) = DISCOVERED.get() {
        if let Ok(mut items) = slot.lock() {
            items.clear();
            items.shrink_to_fit();
        }
    }
    if let Some(slot) = INCOMING_CLIPS.get() {
        if let Ok(mut items) = slot.lock() {
            items.clear();
            items.shrink_to_fit();
        }
    }
    if let Some(slot) = PAIR_PROMPTS.get() {
        if let Ok(mut items) = slot.lock() {
            items.clear();
            items.shrink_to_fit();
        }
    }
    if let Some(slot) = PENDING_PAIRS.get() {
        if let Ok(mut items) = slot.lock() {
            items.clear();
            items.shrink_to_fit();
        }
    }
    if let Some(slot) = SEEN_MESSAGES.get() {
        if let Ok(mut items) = slot.lock() {
            items.clear();
            items.shrink_to_fit();
        }
    }
    set_latest_clip(None);
    if let Some(slot) = FILE_SESSIONS.get() {
        if let Ok(mut items) = slot.lock() {
            items.clear();
            items.shrink_to_fit();
        }
    }
}

pub(crate) fn trigger_discovery(settings: &AppSettings) {
    let mut s = settings.clone();
    if ensure_device_identity(&mut s) {
        save_settings(&s);
    }
    let mut runtime_settings = lan_runtime_settings_from_app(&s);
    runtime_settings.lan_sync_enabled = true;
    let config = LanRuntimeConfig::from_core_config(
        windows_lan_runtime_context(LanRuntimeEventSink::None),
        runtime_settings.core_config(),
    );
    send_discovery_once(&config);
}

pub(crate) fn mobile_setup_url(settings: &AppSettings) -> Option<String> {
    mobile_setup_url_for_host(settings, local_lan_host())
}

pub(crate) fn mobile_setup_url_cached(settings: &AppSettings) -> Option<String> {
    mobile_setup_url_for_host(settings, cached_local_lan_host()?)
}

fn mobile_setup_url_for_host(settings: &AppSettings, host: String) -> Option<String> {
    let runtime_settings = lan_runtime_settings_from_app(settings);
    if !runtime_settings.lan_sync_enabled {
        return None;
    }
    let port = service_slot()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|handle| handle.state.tcp_port))
        .unwrap_or(runtime_settings.normalized_tcp_port());
    Some(format!("http://{host}:{port}/mobile/setup"))
}

pub(crate) fn mobile_pair_url(settings: &AppSettings) -> Option<String> {
    mobile_pair_url_for_setup(mobile_setup_url(settings)?)
}

pub(crate) fn mobile_pair_url_cached(settings: &AppSettings) -> Option<String> {
    mobile_pair_url_for_setup(mobile_setup_url_cached(settings)?)
}

fn mobile_pair_url_for_setup(setup_url: String) -> Option<String> {
    let host = setup_url
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://")
        .trim_end_matches("/mobile/setup");
    Some(format!("zsclip://pair?host={}", url_encode_component(host)))
}

pub(crate) fn lan_service_ready() -> bool {
    service_slot()
        .lock()
        .ok()
        .and_then(|guard| guard.as_ref().map(|handle| handle.state.lan_enabled))
        .unwrap_or(false)
}

fn lan_host_cache_slot() -> &'static Mutex<Option<String>> {
    LOCAL_LAN_HOST_CACHE.get_or_init(|| Mutex::new(None))
}

fn clear_lan_host_cache() {
    if let Some(slot) = LOCAL_LAN_HOST_CACHE.get() {
        if let Ok(mut host) = slot.lock() {
            *host = None;
        }
    }
}

fn refresh_lan_host_cache_in_background(event_sink: LanRuntimeEventSink) {
    thread::spawn(move || {
        let host = probe_local_lan_host();
        if let Ok(mut cached) = lan_host_cache_slot().lock() {
            *cached = Some(host);
        }
        post_ready(&event_sink);
    });
}

fn cached_local_lan_host() -> Option<String> {
    lan_host_cache_slot().lock().ok().and_then(|cached| {
        cached
            .as_ref()
            .filter(|host| !host.trim().is_empty())
            .cloned()
    })
}

fn probe_local_lan_host() -> String {
    UdpSocket::bind("0.0.0.0:0")
        .and_then(|sock| {
            let _ = sock.connect("8.8.8.8:80");
            sock.local_addr()
        })
        .ok()
        .and_then(|addr| match addr.ip() {
            IpAddr::V4(ip) if !ip.is_loopback() && !ip.is_unspecified() => Some(ip.to_string()),
            _ => None,
        })
        .unwrap_or_else(|| "127.0.0.1".to_string())
}

fn local_lan_host() -> String {
    if let Ok(cached) = lan_host_cache_slot().lock() {
        if let Some(host) = cached.as_ref().filter(|host| !host.trim().is_empty()) {
            return host.clone();
        }
    }
    let host = probe_local_lan_host();
    if let Ok(mut cached) = lan_host_cache_slot().lock() {
        *cached = Some(host.clone());
    }
    host
}

pub(crate) fn broadcast_clip(settings: &AppSettings, envelope: LanClipEnvelope) {
    let runtime_settings = lan_runtime_settings_from_app(settings);
    if !runtime_settings.lan_sync_enabled {
        return;
    }
    let token_device_id = runtime_settings.device_id;
    let devices = load_devices();
    for device in devices.into_iter().filter(lan_device_can_receive_clip) {
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
    let runtime_settings = lan_runtime_settings_from_app(settings);
    if !runtime_settings.lan_sync_enabled {
        set_status("局域网同步未开启，无法推送文件");
        return;
    }
    let devices: Vec<LanDevice> = trusted_devices()
        .into_iter()
        .filter(lan_device_can_receive_clip)
        .collect();
    if devices.is_empty() {
        set_status("没有可接收推送的信任设备，无法推送文件");
        return;
    }
    let sender_id = runtime_settings.device_id;
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
                .filter(|pair| pair.is_active(now))
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
    pair.mark_accepted();
    upsert_device(pair.to_trusted_device(now_ms()));
    set_status("已允许局域网设备配对");
    true
}

pub(crate) fn reject_pair_request(pair_id: &str) {
    if let Ok(mut pairs) = pending_pair_slot().lock() {
        if let Some(pair) = pairs.iter_mut().find(|pair| pair.prompt.pair_id == pair_id) {
            pair.mark_rejected();
        }
    }
    set_status("已拒绝局域网设备配对");
}

pub(crate) fn start_pair_with_host(hwnd: HWND, settings: AppSettings, host: String) {
    let runtime_settings = lan_runtime_settings_from_app(&settings);
    let host = normalize_lan_host(&host, runtime_settings.normalized_tcp_port());
    if host.is_empty() {
        set_status("请输入局域网设备 IP 或 IP:端口");
        return;
    }
    set_status("正在请求配对...");
    let event_sink = LanRuntimeEventSink::platform_main_window(hwnd as isize);
    thread::spawn(move || {
        let body =
            serde_json::to_vec(&runtime_settings.pair_request_body(lan_desktop_capabilities()))
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
            post_ready(&event_sink);
            return;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&resp) else {
            set_status("配对请求失败：设备返回异常");
            post_ready(&event_sink);
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
            post_ready(&event_sink);
            return;
        }
        set_status("配对请求已发送，请在对方设备的局域网设置页点击允许");
        post_ready(&event_sink);
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
                        .unwrap_or(runtime_settings.normalized_tcp_port());
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
                            capabilities: normalize_lan_capabilities(
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
                        post_ready(&event_sink);
                        return;
                    }
                }
                "rejected" => {
                    set_status("配对已被对方拒绝");
                    post_ready(&event_sink);
                    return;
                }
                _ => {}
            }
        }
        set_status("配对超时");
        post_ready(&event_sink);
    });
}

pub(crate) fn status_summary(settings: &AppSettings) -> String {
    if !lan_runtime_settings_from_app(settings).lan_sync_enabled {
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
        let mode = if lan_device_can_receive_clip(device) {
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
    let lifecycle = LanServiceLifecyclePlan::for_config(&config.core_config());
    let listener = bind_tcp_listener(lifecycle.tcp_port, lifecycle.bind_loopback_only)?;
    let actual_tcp_port = listener.local_addr()?.port();
    listener.set_nonblocking(true)?;
    let mut tcp_config = config.clone();
    tcp_config.tcp_port = actual_tcp_port;

    let mut workers = Vec::new();
    if lifecycle.start_tcp_server {
        let stop = stop.clone();
        let cfg = tcp_config.clone();
        workers.push(thread::spawn(move || tcp_server_loop(listener, cfg, stop)));
    }
    if lifecycle.start_udp_listener {
        let stop = stop.clone();
        let cfg = tcp_config.clone();
        workers.push(thread::spawn(move || udp_discovery_listener(cfg, stop)));
    }
    if lifecycle.start_udp_sender {
        let stop = stop.clone();
        let cfg = tcp_config.clone();
        workers.push(thread::spawn(move || udp_discovery_sender(cfg, stop)));
    }

    Ok(LanServiceHandle {
        stop,
        state: LanServiceRuntimeState::from_core_config(&tcp_config.core_config(), actual_tcp_port),
        workers,
    })
}

fn stop_handle(mut handle: LanServiceHandle) {
    handle.stop.store(true, Ordering::Relaxed);
    let _ = TcpStream::connect(("127.0.0.1", handle.state.tcp_port));
    let lifecycle = LanServiceLifecyclePlan::for_state(&handle.state);
    if lifecycle.wake_udp_on_stop {
        let _ = UdpSocket::bind("0.0.0.0:0").and_then(|sock| {
            sock.set_broadcast(true)?;
            let addr = format!("255.255.255.255:{}", handle.state.udp_port);
            let _ = sock.send_to(b"stop", addr);
            Ok(())
        });
    }
    for worker in handle.workers.drain(..) {
        let _ = worker.join();
    }
}

fn bind_tcp_listener(base: u16, loopback_only: bool) -> std::io::Result<TcpListener> {
    let mut last_err = None;
    for addr in lan_tcp_bind_candidates(base, loopback_only) {
        match TcpListener::bind(addr) {
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
    let exe_key = lan_hash_string(&exe).chars().take(8).collect::<String>();
    ensure_firewall_rule(
        &format!("ZSClip LAN Sync TCP {tcp_port} {exe_key} LocalSubnetV2"),
        "TCP",
        tcp_port,
        &exe,
    )?;
    ensure_firewall_rule(
        &format!("ZSClip LAN Discovery UDP {udp_port} {exe_key} LocalSubnetV2"),
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
        "profile=any",
        "remoteip=localsubnet",
        "enable=yes",
    ])?;
    if output.status.success() || firewall_rule_exists(name) {
        Ok(())
    } else {
        Err(format!(
            "{}；请以管理员身份运行一次，或在 Windows 防火墙中允许 ZSClip 局域子网访问",
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

fn route_available_for_config(config: &LanRuntimeConfig, route: LanHttpRoute) -> bool {
    LanHttpRoutePolicy::for_config(&config.core_config()).route_available_for_route(route)
}

fn route_http_request(stream: &mut TcpStream, req: HttpRequest, config: LanRuntimeConfig) {
    let path = req.path_without_query();
    let route = lan_http_route_for(req.method.as_str(), path);
    if !route_available_for_config(&config, route) {
        let _ = write_http_json(stream, 404, &json!({"error":"not_found"}));
        return;
    }
    match route {
        LanHttpRoute::Info => {
            let _ = write_http_json(
                stream,
                200,
                &json!({
                    "magic": LAN_MAGIC,
                    "protocol": LAN_PROTOCOL,
                    "device_id": config.device_id,
                    "name": config.device_name,
                    "tcp_port": config.tcp_port,
                    "capabilities": lan_desktop_capabilities()
                }),
            );
        }
        LanHttpRoute::PairRequest => handle_pair_request(stream, req, config),
        LanHttpRoute::PairStatus => handle_pair_status(stream, req, config),
        LanHttpRoute::Clip => handle_clip_post(stream, req, config),
        LanHttpRoute::Latest => handle_latest(stream, req),
        LanHttpRoute::WpsTaskpane => handle_wps_taskpane(stream, req),
        LanHttpRoute::WpsItems => handle_wps_taskpane_items(stream, req),
        LanHttpRoute::WpsImage => handle_wps_taskpane_image(stream, req),
        LanHttpRoute::WpsEvents => handle_wps_taskpane_events(stream, req),
        LanHttpRoute::MobileSetup => handle_mobile_setup(stream, req),
        LanHttpRoute::MobileImages => handle_mobile_images(stream, req),
        LanHttpRoute::MobileImage => handle_mobile_image_download(stream, req),
        LanHttpRoute::MobileItems => handle_mobile_items(stream, req),
        LanHttpRoute::MobileItemImage => handle_mobile_item_image(stream, req),
        LanHttpRoute::MobileItemFile => handle_mobile_item_file(stream, req),
        LanHttpRoute::MultiSyncManifest => handle_multi_sync_manifest(stream, req),
        LanHttpRoute::FileStart => handle_file_start(stream, req, config),
        LanHttpRoute::FileChunk => handle_file_chunk(stream, req, config),
        LanHttpRoute::FileFinish => handle_file_finish(stream, req, config),
        LanHttpRoute::MultiSyncFile => handle_multi_sync_file(stream, req),
        LanHttpRoute::NotFound => {
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
    let pair_id = make_lan_token(12, now_ms());
    let code = make_lan_pair_code(&config.device_id, &body.device_id, now_ms());
    let token = make_lan_token(32, now_ms());
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
    let pending = LanPendingPair {
        prompt: prompt.clone(),
        requester_device_id: body.device_id,
        requester_tcp_port: body.tcp_port,
        requester_capabilities: normalize_lan_capabilities(body.capabilities, body.tcp_port),
        token,
        accepted: false,
        rejected: false,
        created_at_ms: now_ms(),
    };
    if let Ok(mut pairs) = pending_pair_slot().lock() {
        pairs.retain(|pair| now_ms().saturating_sub(pair.created_at_ms) < LAN_PAIR_REQUEST_TTL_MS);
        pairs.push(pending);
    }
    if let Ok(mut q) = pair_prompt_slot().lock() {
        q.push_back(prompt);
    }
    set_status("收到配对请求，请在局域网设置页选择请求并点击允许");
    post_ready(&config.platform.event_sink);
    let _ = write_http_json(
        stream,
        200,
        &json!({"pair_id": pair_id, "code": code, "status":"pending"}),
    );
}

fn handle_pair_status(stream: &mut TcpStream, req: HttpRequest, config: LanRuntimeConfig) {
    let pair_id = query_param(&req.path, "id");
    let pairs = pending_pair_slot().lock().unwrap();
    let pair = pairs.iter().find(|pair| pair.prompt.pair_id == pair_id);
    let status = if pair.is_some() { 200 } else { 404 };
    let value =
        lan_pair_status_response_value(pair, &config.core_config(), lan_desktop_capabilities());
    let _ = write_http_json(stream, status, &value);
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
    post_ready(&config.platform.event_sink);
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

fn handle_wps_taskpane(stream: &mut TcpStream, req: HttpRequest) {
    if !office_request_allowed(&req) {
        let _ = write_http_json(stream, 403, &json!({"error":"loopback_required"}));
        return;
    }
    let html = render_wps_taskpane_page();
    let _ = write_http_bytes(
        stream,
        200,
        "text/html; charset=utf-8",
        html.as_bytes(),
        &[],
    );
}

fn handle_wps_taskpane_items(stream: &mut TcpStream, req: HttpRequest) {
    if !office_request_allowed(&req) {
        let _ = write_http_json(stream, 403, &json!({"error":"loopback_required"}));
        return;
    }
    let limit = query_param(&req.path, "limit")
        .parse::<i64>()
        .unwrap_or(WPS_TASKPANE_ITEM_LIMIT)
        .clamp(1, WPS_TASKPANE_ITEM_LIMIT);
    let query = query_param_decoded(&req.path, "q");
    let category = wps_taskpane_category(&query_param(&req.path, "category"));
    let items = load_wps_taskpane_items(limit, &query, category).unwrap_or_default();
    let _ = write_http_json(stream, 200, &json!({ "items": items }));
}

fn handle_wps_taskpane_image(stream: &mut TcpStream, req: HttpRequest) {
    if !office_request_allowed(&req) {
        let _ = write_http_json(stream, 403, &json!({"error":"loopback_required"}));
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
            format!("inline; filename=\"{filename}\""),
        )],
    );
}

fn handle_wps_taskpane_events(stream: &mut TcpStream, req: HttpRequest) {
    if !office_request_allowed(&req) {
        let _ = write_http_json(stream, 403, &json!({"error":"loopback_required"}));
        return;
    }
    let _ = write_wps_taskpane_events(stream);
}

fn handle_mobile_setup(stream: &mut TcpStream, req: HttpRequest) {
    let host = header_value(&req, "host")
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| format!("127.0.0.1:{LAN_TCP_PORT_DEFAULT}"));
    let html = render_mobile_setup_page(&host);
    let _ = write_http_bytes(
        stream,
        200,
        "text/html; charset=utf-8",
        html.as_bytes(),
        &[],
    );
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

fn handle_mobile_items(stream: &mut TcpStream, req: HttpRequest) {
    let Some(device) = authenticated_device(&req) else {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    };
    let limit = query_param(&req.path, "limit")
        .parse::<i64>()
        .unwrap_or(MOBILE_ITEM_LIST_LIMIT_DEFAULT)
        .clamp(1, MOBILE_ITEM_LIST_LIMIT_MAX);
    let items = load_mobile_item_list(limit).unwrap_or_default();
    let _ = write_http_json(
        stream,
        200,
        &json!({
            "items": items,
            "device": {
                "id": device.device_id,
                "name": device.name,
            }
        }),
    );
}

fn handle_mobile_item_image(stream: &mut TcpStream, req: HttpRequest) {
    if authenticated_device(&req).is_none() {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    }
    let Some(id) = mobile_item_image_path_id(req.path_without_query()) else {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_image_id"}));
        return;
    };
    let Ok(Some((png_bytes, preview))) = load_mobile_item_image_png(id) else {
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
            format!("inline; filename=\"{filename}\""),
        )],
    );
}

fn handle_mobile_item_file(stream: &mut TcpStream, req: HttpRequest) {
    if authenticated_device(&req).is_none() {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    }
    let Some((id, index)) = mobile_item_file_path_parts(req.path_without_query()) else {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_file_id"}));
        return;
    };
    let Ok(Some(path)) = load_mobile_item_file_path(id, index) else {
        let _ = write_http_json(stream, 404, &json!({"error":"file_not_found"}));
        return;
    };
    let Ok(meta) = fs::metadata(&path) else {
        let _ = write_http_json(stream, 404, &json!({"error":"file_not_found"}));
        return;
    };
    if !meta.is_file() || meta.len() == 0 || meta.len() > LAN_FILE_MAX_BYTES {
        let _ = write_http_json(stream, 400, &json!({"error":"invalid_file"}));
        return;
    }
    let filename = path
        .file_name()
        .and_then(|name| name.to_str())
        .map(safe_lan_file_name)
        .unwrap_or_else(|| "file.bin".to_string());
    let _ = write_http_file(
        stream,
        200,
        "application/octet-stream",
        &path,
        meta.len(),
        &[(
            "Content-Disposition",
            format!("attachment; filename=\"{filename}\""),
        )],
    );
}

fn handle_multi_sync_manifest(stream: &mut TcpStream, req: HttpRequest) {
    let Some(device) = authenticated_mobile_query(&req) else {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    };
    let manifest = crate::multi_sync::latest_manifest("lan").ok();
    let clip = manifest.as_ref().and_then(|manifest| manifest.clip.clone());
    let _ = write_http_json(
        stream,
        200,
        &json!({
            "protocol": crate::multi_sync::MULTI_SYNC_PROTOCOL,
            "version": crate::multi_sync::MULTI_SYNC_VERSION,
            "transport": "lan",
            "device": {
                "id": device.device_id,
                "name": device.name,
            },
            "clip": clip,
        }),
    );
}

fn handle_multi_sync_file(stream: &mut TcpStream, req: HttpRequest) {
    if authenticated_mobile_query(&req).is_none() {
        let _ = write_http_json(stream, 401, &json!({"error":"unauthorized"}));
        return;
    }
    let path = req.path_without_query();
    let name = path.trim_start_matches("/file/");
    let Some(id) = crate::multi_sync::image_id_from_data_name(name) else {
        let _ = write_http_json(stream, 404, &json!({"error":"file_not_found"}));
        return;
    };
    let Ok(Some((png_bytes, preview))) = load_mobile_image_png(id) else {
        let _ = write_http_json(stream, 404, &json!({"error":"file_not_found"}));
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

fn load_mobile_item_list(limit: i64) -> rusqlite::Result<Vec<MobileItemListItem>> {
    with_db(|conn| {
        let mut stmt = conn.prepare(
            "SELECT id, kind, preview, COALESCE(source_app, ''), \
             CASE WHEN kind='text' THEN COALESCE(text_data, '') ELSE COALESCE(file_paths, '') END, \
             COALESCE(image_path, ''), COALESCE(length(image_data), 0), image_width, image_height, \
             COALESCE(created_at, '') \
             FROM items WHERE category=0 AND kind IN ('text', 'image', 'files') ORDER BY id DESC LIMIT ?",
        )?;
        let rows = stmt.query_map(params![limit.max(1)], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, i64>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, String>(9)?,
            ))
        })?;
        let mut items = Vec::new();
        for row in rows {
            let (
                id,
                kind,
                preview,
                source_app,
                payload_blob,
                image_path,
                image_data_len,
                width,
                height,
                created_at,
            ) = row?;
            if kind == "text" {
                let text = payload_blob;
                let size = text.as_bytes().len() as u64;
                items.push(MobileItemListItem {
                    id,
                    kind,
                    preview,
                    text,
                    source_app,
                    created_at,
                    size,
                    width: None,
                    height: None,
                    files: Vec::new(),
                });
                continue;
            }
            if kind == "image" {
                if let Some(size) = readable_mobile_image_size(&image_path, image_data_len) {
                    items.push(MobileItemListItem {
                        id,
                        kind,
                        preview,
                        text: String::new(),
                        source_app,
                        created_at,
                        size,
                        width: (width > 0).then_some(width),
                        height: (height > 0).then_some(height),
                        files: Vec::new(),
                    });
                }
                continue;
            }
            let files = mobile_file_list_from_blob(&payload_blob);
            if !files.is_empty() {
                let size = files.iter().map(|file| file.size).sum();
                items.push(MobileItemListItem {
                    id,
                    kind,
                    preview,
                    text: String::new(),
                    source_app,
                    created_at,
                    size,
                    width: None,
                    height: None,
                    files,
                });
            }
        }
        Ok(items)
    })
}

fn load_mobile_item_image_png(id: i64) -> rusqlite::Result<Option<(Vec<u8>, String)>> {
    with_db(|conn| {
        conn.query_row(
            "SELECT image_data, COALESCE(image_path, ''), preview \
             FROM items WHERE category=0 AND kind='image' AND id=?",
            params![id],
            |row| {
                Ok((
                    row.get::<_, Option<Vec<u8>>>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            },
        )
        .optional()
        .map(|row| {
            row.and_then(|(image_data, image_path, preview)| {
                if let Some(bytes) = image_data {
                    return Some((bytes, preview));
                }
                let image_path = image_path.trim();
                if image_path.is_empty() {
                    return None;
                }
                fs::read(image_path).ok().map(|bytes| (bytes, preview))
            })
        })
    })
}

fn load_mobile_item_file_path(id: i64, index: usize) -> rusqlite::Result<Option<PathBuf>> {
    with_db(|conn| {
        conn.query_row(
            "SELECT COALESCE(file_paths, text_data, '') \
             FROM items WHERE category=0 AND kind='files' AND id=?",
            params![id],
            |row| row.get::<_, String>(0),
        )
        .optional()
        .map(|blob| {
            blob.and_then(|blob| {
                mobile_paths_from_blob(&blob)
                    .into_iter()
                    .nth(index)
                    .filter(|path| path.is_file())
            })
        })
    })
}

fn mobile_file_list_from_blob(blob: &str) -> Vec<MobileItemListFile> {
    mobile_paths_from_blob(blob)
        .into_iter()
        .enumerate()
        .filter_map(|(index, path)| {
            let meta = fs::metadata(&path).ok()?;
            if !meta.is_file() || meta.len() == 0 || meta.len() > LAN_FILE_MAX_BYTES {
                return None;
            }
            Some(MobileItemListFile {
                index,
                name: path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .map(safe_lan_file_name)
                    .unwrap_or_else(|| "file.bin".to_string()),
                size: meta.len(),
            })
        })
        .collect()
}

fn mobile_paths_from_blob(blob: &str) -> Vec<PathBuf> {
    blob.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(PathBuf::from)
        .collect()
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

fn office_request_allowed(req: &HttpRequest) -> bool {
    req.peer.ip().is_loopback()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum WpsTaskPaneCategory {
    Records,
    Phrases,
}

fn wps_taskpane_category(value: &str) -> WpsTaskPaneCategory {
    if value.trim().eq_ignore_ascii_case("phrases") {
        WpsTaskPaneCategory::Phrases
    } else {
        WpsTaskPaneCategory::Records
    }
}

fn load_wps_taskpane_items(
    limit: i64,
    query: &str,
    category: WpsTaskPaneCategory,
) -> rusqlite::Result<Vec<WpsTaskPaneItem>> {
    let query = query.trim().to_lowercase();
    let category_id = match category {
        WpsTaskPaneCategory::Records => 0,
        WpsTaskPaneCategory::Phrases => 1,
    };
    let category_name = match category {
        WpsTaskPaneCategory::Records => "records",
        WpsTaskPaneCategory::Phrases => "phrases",
    };
    let kind_filter = match category {
        WpsTaskPaneCategory::Records => "('text', 'image')",
        WpsTaskPaneCategory::Phrases => "('text', 'phrase')",
    };
    let sql = format!(
        "SELECT id, kind, COALESCE(preview, ''), COALESCE(text_data, ''), \
         COALESCE(source_app, ''), COALESCE(created_at, ''), \
         COALESCE(image_path, ''), COALESCE(length(image_data), 0), image_width, image_height \
         FROM items WHERE category=? AND kind IN {kind_filter} \
         ORDER BY pinned DESC, id DESC LIMIT ?"
    );
    with_db(|conn| {
        let mut stmt = conn.prepare(&sql)?;
        let rows = stmt.query_map(params![category_id, limit.max(1)], |row| {
            let id = row.get::<_, i64>(0)?;
            let kind = row.get::<_, String>(1)?;
            Ok((
                id,
                kind,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
                row.get::<_, i64>(7)?,
                row.get::<_, i64>(8)?,
                row.get::<_, i64>(9)?,
            ))
        })?;
        let mut items = Vec::new();
        for row in rows {
            let (
                id,
                kind,
                preview,
                text,
                source_app,
                created_at,
                image_path,
                image_data_len,
                image_width,
                image_height,
            ) = row?;
            if kind == "image" && readable_mobile_image_size(&image_path, image_data_len).is_none()
            {
                continue;
            }
            if kind != "image" && text.trim().is_empty() {
                continue;
            }
            if !query.is_empty() {
                let haystack = format!(
                    "{}\n{}\n{}",
                    preview.to_lowercase(),
                    text.to_lowercase(),
                    source_app.to_lowercase()
                );
                if !haystack.contains(&query) {
                    continue;
                }
            }
            items.push(WpsTaskPaneItem {
                id,
                category: category_name.to_string(),
                kind: kind.clone(),
                preview,
                text,
                source_app,
                created_at,
                image_url: if kind == "image" {
                    format!("/office/wps/image?id={id}")
                } else {
                    String::new()
                },
                image_width,
                image_height,
            });
        }
        Ok(items)
    })
}

fn wps_taskpane_fingerprint() -> rusqlite::Result<String> {
    with_db(|conn| {
        let records: (i64, i64, String) = conn.query_row(
            "SELECT COALESCE(MAX(id), 0), COUNT(*), COALESCE(MAX(created_at), '') \
             FROM items WHERE category=0 AND kind IN ('text', 'image')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        let phrases: (i64, i64, String) = conn.query_row(
            "SELECT COALESCE(MAX(id), 0), COUNT(*), COALESCE(MAX(created_at), '') \
             FROM items WHERE category=1 AND kind IN ('text', 'phrase')",
            [],
            |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
        )?;
        Ok(format!(
            "{}:{}:{}|{}:{}:{}",
            records.0, records.1, records.2, phrases.0, phrases.1, phrases.2
        ))
    })
}

fn write_wps_taskpane_events(stream: &mut TcpStream) -> std::io::Result<()> {
    write!(
        stream,
        "HTTP/1.1 200 OK\r\nContent-Type: text/event-stream; charset=utf-8\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\nX-Accel-Buffering: no\r\n\r\n"
    )?;
    stream.write_all(b"event: ready\ndata: {}\n\n")?;
    stream.flush()?;
    let mut last = wps_taskpane_fingerprint().unwrap_or_default();
    for _ in 0..240 {
        thread::sleep(Duration::from_millis(500));
        let next = wps_taskpane_fingerprint().unwrap_or_default();
        if next != last {
            last = next;
            stream.write_all(b"event: changed\ndata: {}\n\n")?;
        } else {
            stream.write_all(b": keepalive\n\n")?;
        }
        if stream.flush().is_err() {
            break;
        }
    }
    Ok(())
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

fn render_wps_taskpane_page() -> &'static str {
    r#"<!doctype html>
<html lang="zh-CN">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>剪贴板</title>
<style>
:root{color-scheme:light dark}
body{margin:0;background:#f5f6f8;color:#202124;font-family:-apple-system,BlinkMacSystemFont,"Segoe UI",sans-serif}
main{padding:12px}
.tabs{display:grid;grid-template-columns:1fr 1fr;gap:6px;margin-bottom:10px}
.tab{border:1px solid #c8d0dc;border-radius:7px;background:#fff;color:#374151;padding:8px 6px;font-size:13px;cursor:pointer}
.tab.active{background:#2563eb;border-color:#2563eb;color:#fff;font-weight:650}
.bar{display:flex;gap:8px;align-items:center;margin-bottom:10px}
input{flex:1;min-width:0;border:1px solid #d7dbe2;border-radius:6px;padding:8px 9px;font-size:13px;background:#fff;color:#202124}
button{border:1px solid #c8d0dc;border-radius:6px;background:#fff;color:#1f2937;padding:8px 10px;font-size:13px;cursor:pointer}
button.primary{background:#2563eb;color:#fff;border-color:#2563eb}
.status{font-size:12px;color:#667085;margin:7px 0 10px;min-height:18px}
.item{background:#fff;border:1px solid #e3e7ee;border-radius:7px;padding:10px;margin:8px 0}
.preview{font-size:13px;font-weight:650;color:#111827;line-height:1.35;word-break:break-word}
.text{font-size:12px;color:#4b5563;line-height:1.45;white-space:pre-wrap;word-break:break-word;max-height:72px;overflow:hidden;margin-top:5px}
.meta{font-size:11px;color:#7b8494;margin-top:7px;display:flex;gap:8px;flex-wrap:wrap}
.actions{display:flex;gap:8px;margin-top:9px}
.imageBox{height:76px;border:1px solid #e5e7eb;border-radius:6px;background:#f8fafc;display:flex;align-items:center;justify-content:center;color:#64748b;font-size:12px;margin-top:7px}
.empty{border:1px dashed #c8d0dc;border-radius:7px;color:#667085;padding:14px;background:#fff;font-size:13px;line-height:1.45}
</style>
</head>
<body>
<main>
<div class="tabs">
  <button class="tab active" data-category="records">复制记录</button>
  <button class="tab" data-category="phrases">常用短语</button>
</div>
<div class="bar">
  <input id="q" type="search" placeholder="搜索剪贴板">
  <button id="refresh" title="刷新">刷新</button>
</div>
<div id="status" class="status">正在加载...</div>
<div id="list"></div>
</main>
<script>
const state = { items: [], category: 'records', polling: 0, eventSource: null };
const IMAGE_ENDPOINT = '/office/wps/image';
const statusEl = document.getElementById('status');
const listEl = document.getElementById('list');
const queryEl = document.getElementById('q');
const refreshEl = document.getElementById('refresh');
const tabEls = Array.from(document.querySelectorAll('.tab'));

function escapeText(value) {
  return String(value || '').replace(/[&<>"']/g, ch => ({
    '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;'
  }[ch]));
}

function wpsApp() {
  if (window.wps && typeof window.wps.WpsApplication === 'function') {
    return window.wps.WpsApplication();
  }
  if (window.Application) {
    return window.Application;
  }
  return null;
}

function insertTextIntoWps(text) {
  const app = wpsApp();
  if (app && app.Selection && typeof app.Selection.TypeText === 'function') {
    app.Selection.TypeText(text);
    return true;
  }
  if (window.Selection && typeof window.Selection.TypeText === 'function') {
    window.Selection.TypeText(text);
    return true;
  }
  return false;
}

function absoluteUrl(path) {
  return new URL(path, window.location.href).href;
}

function insertImageIntoWps(item) {
  const app = wpsApp();
  if (!app || !item.id) return false;
  const imageUrl = absoluteUrl(item.image_url || `${IMAGE_ENDPOINT}?id=${item.id}`);
  try {
    const doc = typeof app.ActiveDocument === 'function' ? app.ActiveDocument() : app.ActiveDocument;
    const selection = app.Selection || window.Selection || null;
    const range = selection && selection.Range ? selection.Range : undefined;
    if (doc && doc.InlineShapes && typeof doc.InlineShapes.AddPicture === 'function') {
      doc.InlineShapes.AddPicture({
        FileName: imageUrl,
        LinkToFile: false,
        SaveWithDocument: true,
        Range: range
      });
      return true;
    }
    if (doc && doc.InlineShapes && doc.InlineShapes.AddPicture) {
      doc.InlineShapes.AddPicture(imageUrl, false, true, range);
      return true;
    }
  } catch (_) {}
  try {
    const selection = app.Selection || window.Selection || null;
    if (selection && selection.InlineShapes && typeof selection.InlineShapes.AddPicture === 'function') {
      selection.InlineShapes.AddPicture(imageUrl, false, true);
      return true;
    }
  } catch (_) {}
  return false;
}

async function copyFallback(text) {
  if (navigator.clipboard && navigator.clipboard.writeText) {
    await navigator.clipboard.writeText(text);
    return true;
  }
  return false;
}

async function useItem(id) {
  const item = state.items.find(entry => entry.id === id);
  if (!item) return;
  if (item.kind === 'image') {
    if (insertImageIntoWps(item)) {
      statusEl.textContent = '图片已插入到 WPS 文档。';
    } else {
      statusEl.textContent = '图片插入失败。';
    }
    return;
  }
  if (insertTextIntoWps(item.text)) {
    statusEl.textContent = '已插入到 WPS 文档。';
    return;
  }
  try {
    if (await copyFallback(item.text)) {
      statusEl.textContent = '无法调用 WPS API，已改为复制文本。';
    } else {
      statusEl.textContent = '无法调用 WPS API，请手动选择并复制文本。';
    }
  } catch (_) {
    statusEl.textContent = '无法调用 WPS API，请手动选择并复制文本。';
  }
}

function render() {
  if (!state.items.length) {
    const text = state.category === 'phrases'
      ? '暂无常用短语。'
      : '暂无复制记录。请先在 Windows 复制文本或图片。';
    listEl.innerHTML = `<div class="empty">${text}</div>`;
    return;
  }
  listEl.innerHTML = state.items.map(item => {
    const preview = item.preview || (item.text || '').slice(0, 80) || `Clip ${item.id}`;
    const source = item.source_app ? `<span>${escapeText(item.source_app)}</span>` : '';
    const created = item.created_at ? `<span>${escapeText(item.created_at)}</span>` : '';
    const isImage = item.kind === 'image';
    const body = isImage
      ? `<div class="imageBox">图片 ${item.image_width || ''}${item.image_height ? ` x ${item.image_height}` : ''}</div>`
      : `<div class="text">${escapeText(item.text)}</div>`;
    const action = isImage ? '插入图片' : '插入';
    return `<section class="item">
      <div class="preview">${escapeText(preview)}</div>
      ${body}
      <div class="meta">${source}${created}</div>
      <div class="actions">
        <button class="primary" data-insert="${item.id}">${action}</button>
      </div>
    </section>`;
  }).join('');
  listEl.querySelectorAll('[data-insert]').forEach(button => {
    button.addEventListener('click', () => useItem(Number(button.dataset.insert)));
  });
}

async function loadItems() {
  statusEl.textContent = '正在加载...';
  const params = new URLSearchParams({ limit: '80', category: state.category });
  const q = queryEl.value.trim();
  if (q) params.set('q', q);
  const res = await fetch(`/office/wps/items?${params}`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  const data = await res.json();
  state.items = Array.isArray(data.items) ? data.items : [];
  statusEl.textContent = `${state.items.length} 条${state.category === 'phrases' ? '常用短语' : '复制记录'}`;
  render();
}

function setCategory(category) {
  state.category = category;
  tabEls.forEach(tab => tab.classList.toggle('active', tab.dataset.category === category));
  loadItems().catch(err => statusEl.textContent = `加载失败: ${err.message}`);
}

function startPolling() {
  if (state.polling) return;
  state.polling = window.setInterval(() => {
    loadItems().catch(err => statusEl.textContent = `加载失败: ${err.message}`);
  }, 3000);
}

function startEvents() {
  if (!window.EventSource) {
    startPolling();
    return;
  }
  try {
    const es = new EventSource('/office/wps/events');
    state.eventSource = es;
    es.addEventListener('changed', () => {
      loadItems().catch(err => statusEl.textContent = `加载失败: ${err.message}`);
    });
    es.onerror = () => {
      if (state.eventSource) {
        state.eventSource.close();
        state.eventSource = null;
      }
      startPolling();
    };
  } catch (_) {
    startPolling();
  }
}

refreshEl.addEventListener('click', () => loadItems().catch(err => {
  statusEl.textContent = `加载失败: ${err.message}`;
}));
queryEl.addEventListener('input', () => {
  clearTimeout(window.__zsclipSearchTimer);
  window.__zsclipSearchTimer = setTimeout(() => {
    loadItems().catch(err => statusEl.textContent = `加载失败: ${err.message}`);
  }, 180);
});
tabEls.forEach(tab => tab.addEventListener('click', () => setCategory(tab.dataset.category)));
loadItems().catch(err => {
  statusEl.textContent = `加载失败: ${err.message}`;
  listEl.innerHTML = '<div class="empty">请先启动剪贴板，并在插件页启用 WPS 任务窗格。</div>';
});
startEvents();
</script>
</body>
</html>"#
}

fn render_mobile_setup_page(host: &str) -> String {
    let safe_host = host
        .trim()
        .trim_start_matches("http://")
        .trim_start_matches("https://");
    let pair_link = format!("zsclip://pair?host={}", url_encode_component(safe_host));
    let setup_url = format!("http://{safe_host}/mobile/setup");
    let manifest_hint = format!(
        "http://{safe_host}/{}?device=<device_id>&token=<token>",
        crate::multi_sync::MULTI_SYNC_MANIFEST_FILE_NAME
    );
    let images_hint = format!("http://{safe_host}/mobile/images?device=<device_id>&token=<token>");
    let mut html = String::new();
    html.push_str("<!doctype html><html lang=\"zh-CN\"><head><meta charset=\"utf-8\">");
    html.push_str("<meta name=\"viewport\" content=\"width=device-width,initial-scale=1\">");
    html.push_str("<title>ZSClip 移动端连接</title><style>");
    html.push_str("body{margin:0;background:#f6f7f9;color:#202124;font-family:-apple-system,BlinkMacSystemFont,'Segoe UI',sans-serif}");
    html.push_str("main{max-width:760px;margin:0 auto;padding:18px}h1{font-size:24px;margin:8px 0 10px}.card{background:#fff;border:1px solid #e3e6ea;border-radius:8px;padding:14px;margin:10px 0}.btn{display:inline-block;margin-top:10px;padding:10px 13px;background:#0a84ff;color:#fff;border-radius:7px;text-decoration:none;font-weight:600}.meta{font-family:ui-monospace,SFMono-Regular,Consolas,monospace;font-size:13px;word-break:break-all;color:#3c4043;background:#f1f3f4;border-radius:6px;padding:8px}.muted{color:#69717c;line-height:1.5}.qrgrid{display:grid;grid-template-columns:repeat(auto-fit,minmax(210px,1fr));gap:12px;margin-top:12px}.qrbox{border:1px solid #e6e8ec;border-radius:8px;padding:12px;background:#fbfcfd}.qrbox strong{display:block;margin-bottom:8px}.qr svg{width:168px;height:168px;display:block;background:#fff;border-radius:6px}</style></head><body><main>");
    html.push_str("<h1>ZSClip 移动端连接</h1>");
    html.push_str("<section class=\"card\"><strong>Android</strong><p class=\"muted\">扫描二维码或打开下面链接后，App 会填入 Windows 地址，再按现有流程请求配对；Windows 仍需要点击允许。</p><a class=\"btn\" href=\"");
    html.push_str(&html_escape(&pair_link));
    html.push_str("\">打开 Android 配对</a><p class=\"meta\">");
    html.push_str(&html_escape(&pair_link));
    html.push_str("</p></section>");
    html.push_str("<section class=\"card\"><strong>扫码连接</strong><p class=\"muted\">Android 扫左侧二维码发起配对；iOS 扫右侧二维码打开移动端连接页。</p><div class=\"qrgrid\"><div class=\"qrbox\"><strong>Android 配对</strong><div class=\"qr\">");
    html.push_str(&render_qr_svg(&pair_link));
    html.push_str(
        "</div></div><div class=\"qrbox\"><strong>iOS / 浏览器</strong><div class=\"qr\">",
    );
    html.push_str(&render_qr_svg(&setup_url));
    html.push_str("</div></div></div></section>");
    html.push_str("<section class=\"card\"><strong>iOS 快捷指令</strong><p class=\"muted\">iOS 不需要安装原生 App。先按文档保存 host、device_id、token，再使用图片列表或清单 URL。</p><p class=\"meta\">");
    html.push_str(&html_escape(&setup_url));
    html.push_str("</p></section>");
    html.push_str("<section class=\"card\"><strong>多端同步入口</strong><p class=\"muted\">文本放在清单里，图片通过 dataName 按需下载。</p><p class=\"meta\">");
    html.push_str(&html_escape(&manifest_hint));
    html.push_str("</p><p class=\"meta\">");
    html.push_str(&html_escape(&images_hint));
    html.push_str("</p></section>");
    html.push_str("</main></body></html>");
    html
}

fn render_qr_svg(payload: &str) -> String {
    use qrcodegen::{QrCode, QrCodeEcc};

    let Ok(qr) = QrCode::encode_text(payload, QrCodeEcc::Medium) else {
        return "<span>二维码生成失败</span>".to_string();
    };
    let border = 4;
    let size = qr.size();
    let view = size + border * 2;
    let mut path = String::new();
    for y in 0..size {
        for x in 0..size {
            if qr.get_module(x, y) {
                path.push_str(&format!("M{} {}h1v1h-1z", x + border, y + border));
            }
        }
    }
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {view} {view}\" role=\"img\" aria-label=\"ZSClip QR\"><rect width=\"100%\" height=\"100%\" fill=\"#fff\"/><path fill=\"#111\" d=\"{path}\"/></svg>"
    )
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
    let file_name = safe_lan_file_name(
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
        .join(safe_lan_file_name(&device.device_id))
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
        content_crc: lan_file_content_hasher(total_size),
    };
    let key = lan_file_session_key(&device.device_id, &transfer_id);
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
    let key = lan_file_session_key(&device.device_id, transfer_id);
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
    let key = lan_file_session_key(&device.device_id, transfer_id);
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
    post_ready(&config.platform.event_sink);
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
    let packet = DiscoveryPacket::new(
        &config.core_config(),
        vec![
            "text".to_string(),
            "image".to_string(),
            "latest".to_string(),
            "manual_file".to_string(),
            "receive_clip".to_string(),
        ],
    );
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
        if !packet.uses_current_protocol() {
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
            capabilities: normalize_lan_capabilities(packet.capabilities, packet.tcp_port),
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
        post_ready(&config.platform.event_sink);
    }
}

fn push_one_file(sender_id: &str, device: &LanDevice, path: &PathBuf) -> std::io::Result<()> {
    push_lan_file_payload_to_device(sender_id, device, path, Duration::from_secs(20))
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

fn load_devices() -> Vec<LanDevice> {
    windows_lan_runtime_context(LanRuntimeEventSink::None).load_devices(normalize_lan_capabilities)
}

fn save_devices(devices: &[LanDevice]) {
    let _ = windows_lan_runtime_context(LanRuntimeEventSink::None)
        .save_devices(devices, normalize_lan_capabilities);
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

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn post_ready(event_sink: &LanRuntimeEventSink) {
    let Some(hwnd) = event_sink.raw_platform_main_window_handle() else {
        return;
    };
    platform_window::post_message(hwnd, WM_LAN_SYNC_READY, 0, 0);
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

fn pending_pair_slot() -> &'static Mutex<Vec<LanPendingPair>> {
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

    fn lan_host_cache_test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: std::sync::OnceLock<std::sync::Mutex<()>> = std::sync::OnceLock::new();
        LOCK.get_or_init(|| std::sync::Mutex::new(()))
            .lock()
            .expect("LAN host cache test lock poisoned")
    }

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
        assert!(!lan_device_can_receive_clip(&test_device(
            vec!["text", "latest", "client_only", "pull_only"],
            0,
        )));
        assert!(!lan_device_can_receive_clip(&test_device(
            vec!["pull_only"],
            38473
        )));
        assert!(lan_device_can_receive_clip(&test_device(
            vec!["text", "image", "receive_clip"],
            38473,
        )));
    }

    #[test]
    fn bdd_capabilities_fallback_marks_zero_port_as_pull_only() {
        assert_eq!(
            normalize_lan_capabilities(Vec::new(), 0),
            vec!["client_only".to_string(), "pull_only".to_string()]
        );
        assert!(normalize_lan_capabilities(Vec::new(), 38473)
            .iter()
            .any(|cap| cap == "receive_clip"));
    }

    #[test]
    fn bdd_file_session_key_is_scoped_by_source_device() {
        assert_eq!(
            lan_file_session_key("android-a", "transfer-1"),
            "android-a:transfer-1"
        );
        assert_ne!(
            lan_file_session_key("android-a", "transfer-1"),
            lan_file_session_key("android-b", "transfer-1")
        );
    }

    #[test]
    fn bdd_file_content_crc_is_stable_for_retried_transfer() {
        let mut first = lan_file_content_hasher(3);
        first.update(b"abc");
        let mut second = lan_file_content_hasher(3);
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
    fn bdd_mobile_query_params_support_android_urlencoder_plus_spaces() {
        let path = "/zsSyncClipboard.json?device=android+phone&token=tok%2Ben";

        assert_eq!(query_param_decoded(path, "device"), "android phone");
        assert_eq!(query_param_decoded(path, "token"), "tok+en");
    }

    #[test]
    fn bdd_mobile_item_api_paths_accept_only_ids_and_indices() {
        assert_eq!(
            mobile_item_image_path_id("/v1/mobile/items/42/image"),
            Some(42)
        );
        assert_eq!(
            mobile_item_file_path_parts("/v1/mobile/items/42/file/3"),
            Some((42, 3))
        );
        assert_eq!(mobile_item_image_path_id("/v1/mobile/items/0/image"), None);
        assert_eq!(
            mobile_item_image_path_id("/v1/mobile/items/42/file/0"),
            None
        );
        assert_eq!(
            mobile_item_file_path_parts("/v1/mobile/items/42/file/../secret"),
            None
        );
        assert_eq!(
            mobile_item_file_path_parts("/v1/mobile/items/not-id/file/0"),
            None
        );
    }

    #[test]
    fn bdd_mobile_file_list_uses_database_indices_and_filters_invalid_paths() {
        let dir = std::env::temp_dir().join(format!("zsclip-lan-file-list-{}", now_ms()));
        fs::create_dir_all(&dir).unwrap();
        let first = dir.join("first.txt");
        let empty = dir.join("empty.txt");
        let second = dir.join("second.txt");
        fs::write(&first, b"abc").unwrap();
        fs::write(&empty, b"").unwrap();
        fs::write(&second, b"defg").unwrap();

        let blob = format!(
            "{}\n{}\n{}\n{}\n{}",
            first.display(),
            empty.display(),
            dir.display(),
            dir.join("missing.txt").display(),
            second.display()
        );
        let files = mobile_file_list_from_blob(&blob);

        assert_eq!(files.len(), 2);
        assert_eq!(files[0].index, 0);
        assert_eq!(files[0].name, "first.txt");
        assert_eq!(files[1].index, 4);
        assert_eq!(files[1].name, "second.txt");
        assert_eq!(files[1].size, 4);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bdd_multi_sync_manifest_route_accepts_new_and_legacy_names() {
        assert!(crate::lan_sync_core::is_multi_sync_manifest_path(
            "/zsSyncClipboard.json"
        ));
        assert!(crate::lan_sync_core::is_multi_sync_manifest_path(
            "/SyncClipboard.json"
        ));
        assert!(!crate::lan_sync_core::is_multi_sync_manifest_path(
            "/manifest.json"
        ));
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

    #[test]
    fn bdd_mobile_item_list_returns_text_images_and_files_newest_first() {
        let dir = std::env::temp_dir().join(format!("zsclip-mobile-items-{}", now_ms()));
        fs::create_dir_all(&dir).unwrap();
        let file = dir.join("doc.txt");
        fs::write(&file, b"hello").unwrap();
        let file_blob = file.to_string_lossy().to_string();

        crate::db_runtime::with_test_db(|| {
            with_db(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, file_paths, source_app, pinned, group_id)
                     VALUES(0, 'text', 'ignore text', 't1', 'body', '', 'notepad', 0, 0)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, image_data, image_width, image_height, source_app, pinned, group_id)
                     VALUES(0, 'image', 'screen image', 'i1', ?, 2, 3, 'snip', 0, 0)",
                    params![vec![1u8, 2, 3, 4]],
                )?;
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, file_paths, source_app, pinned, group_id)
                     VALUES(0, 'files', 'one file', 'f1', ?, 'explorer', 0, 0)",
                    params![file_blob],
                )?;
                Ok(())
            })?;

            let items = load_mobile_item_list(10)?;
            assert_eq!(items.len(), 3);
            assert_eq!(items[0].kind, "files");
            assert_eq!(items[0].files.len(), 1);
            assert_eq!(items[0].files[0].name, "doc.txt");
            assert_eq!(items[1].kind, "image");
            assert_eq!(items[1].preview, "screen image");
            assert_eq!(items[2].kind, "text");
            assert_eq!(items[2].preview, "ignore text");
            assert_eq!(items[2].text, "body");
            let file_path = load_mobile_item_file_path(items[0].id, items[0].files[0].index)?
                .expect("file path should resolve");
            assert_eq!(file_path, file);
            Ok(())
        })
        .unwrap();

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn bdd_mobile_setup_page_contains_safe_android_pair_link() {
        let html = render_mobile_setup_page("192.168.1.2:38473");

        assert!(html.contains("ZSClip 移动端连接"));
        assert!(html.contains("zsclip://pair?host=192.168.1.2%3A38473"));
        assert!(html.contains("Android 配对"));
        assert!(html.contains("iOS / 浏览器"));
        assert!(html.contains("iOS 快捷指令"));
        assert!(html.contains("多端同步入口"));
        assert!(html.matches("<svg").count() >= 2);
        assert!(
            html.contains("/zsSyncClipboard.json?device=&lt;device_id&gt;&amp;token=&lt;token&gt;")
        );
        assert!(html.contains("/mobile/images?device=&lt;device_id&gt;&amp;token=&lt;token&gt;"));
        assert!(!html.contains("<script>"));
    }

    #[test]
    fn bdd_mobile_pair_and_setup_urls_feed_settings_qr_codes() {
        let _guard = lan_host_cache_test_guard();
        clear_lan_host_cache();
        let mut settings = AppSettings::default();
        settings.lan_sync_enabled = true;
        settings.lan_tcp_port = 38473;

        let pair = mobile_pair_url(&settings).unwrap();
        let setup = mobile_setup_url(&settings).unwrap();

        assert!(pair.starts_with("zsclip://pair?host="));
        assert!(pair.contains("%3A38473"));
        assert!(setup.ends_with(":38473/mobile/setup"));
    }

    #[test]
    fn bdd_mobile_urls_reuse_cached_lan_host_for_fast_settings_actions() {
        let _guard = lan_host_cache_test_guard();
        clear_lan_host_cache();
        {
            let mut cached = lan_host_cache_slot().lock().unwrap();
            *cached = Some("192.168.66.7".to_string());
        }
        let mut settings = AppSettings::default();
        settings.lan_sync_enabled = true;
        settings.lan_tcp_port = 38473;

        let setup = mobile_setup_url_cached(&settings).unwrap();
        let pair = mobile_pair_url_cached(&settings).unwrap();

        assert_eq!(setup, "http://192.168.66.7:38473/mobile/setup");
        assert_eq!(pair, "zsclip://pair?host=192.168.66.7%3A38473");
        clear_lan_host_cache();
    }

    #[test]
    fn bdd_settings_qr_urls_never_probe_network_when_host_cache_is_empty() {
        let _guard = lan_host_cache_test_guard();
        clear_lan_host_cache();
        let mut settings = AppSettings::default();
        settings.lan_sync_enabled = true;

        assert_eq!(mobile_setup_url_cached(&settings), None);
        assert_eq!(mobile_pair_url_cached(&settings), None);
    }

    #[test]
    fn bdd_firewall_allow_rules_cover_local_subnet_on_every_profile() {
        let source = include_str!("lan_sync.rs");

        assert!(source.contains("LocalSubnetV2"));
        assert!(source.contains("\"profile=any\""));
        assert!(source.contains("\"remoteip=localsubnet\""));
        assert!(!source.contains("\"profile=private,domain\""));
    }

    #[test]
    fn bdd_mobile_setup_qr_svg_encodes_payload_without_script_markup() {
        let svg = render_qr_svg("zsclip://pair?host=192.168.1.2%3A38473");

        assert!(svg.contains("<svg"));
        assert!(svg.contains("<path"));
        assert!(!svg.contains("<script"));
    }

    #[test]
    fn bdd_wps_taskpane_page_uses_local_office_api_and_items_endpoint() {
        let html = render_wps_taskpane_page();

        assert!(html.contains("/office/wps/items"));
        assert!(html.contains("/office/wps/image"));
        assert!(html.contains("/office/wps/events"));
        assert!(html.contains("Selection.TypeText"));
        assert!(html.contains("复制记录"));
        assert!(html.contains("常用短语"));
        assert!(html.contains("搜索剪贴板"));
        assert!(!html.contains("搜索 ZSClip 文本记录"));
    }

    #[test]
    fn bdd_wps_taskpane_http_requires_loopback_peer() {
        let local = HttpRequest {
            method: "GET".to_string(),
            path: "/office/wps/taskpane".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
            peer: "127.0.0.1:50000".parse().unwrap(),
        };
        let remote = HttpRequest {
            method: "GET".to_string(),
            path: "/office/wps/taskpane".to_string(),
            headers: Vec::new(),
            body: Vec::new(),
            peer: "192.168.1.8:50000".parse().unwrap(),
        };

        assert!(office_request_allowed(&local));
        assert!(!office_request_allowed(&remote));
    }

    #[test]
    fn bdd_wps_taskpane_items_split_records_and_phrases() {
        crate::db_runtime::with_test_db(|| {
            with_db(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app, pinned, group_id)
                     VALUES(0, 'text', 'record text', 'r1', 'record body', 'app', 0, 0)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app, pinned, group_id)
                     VALUES(1, 'phrase', 'phrase text', 'p1', 'phrase body', 'app', 0, 0)",
                    [],
                )?;
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, image_data, image_width, image_height, source_app, pinned, group_id)
                     VALUES(0, 'image', 'record image', 'i1', ?, 1, 1, 'app', 0, 0)",
                    params![vec![255u8, 0, 0, 255]],
                )?;
                Ok(())
            })?;

            let records = load_wps_taskpane_items(10, "", WpsTaskPaneCategory::Records)?;
            let phrases = load_wps_taskpane_items(10, "", WpsTaskPaneCategory::Phrases)?;

            assert!(records.iter().any(|item| item.preview == "record text"));
            assert!(records.iter().any(|item| item.kind == "image"));
            assert!(!records.iter().any(|item| item.preview == "phrase text"));
            assert!(phrases.iter().any(|item| item.preview == "phrase text"));
            assert!(!phrases.iter().any(|item| item.preview == "record text"));
            assert!(!phrases.iter().any(|item| item.kind == "image"));
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn bdd_wps_taskpane_fingerprint_changes_when_items_change() {
        crate::db_runtime::with_test_db(|| {
            let before = wps_taskpane_fingerprint()?;
            with_db(|conn| {
                conn.execute(
                    "INSERT INTO items(category, kind, preview, signature, text_data, source_app, pinned, group_id)
                     VALUES(0, 'text', 'record text', 'r-fp', 'record body', 'app', 0, 0)",
                    [],
                )?;
                Ok(())
            })?;
            let after = wps_taskpane_fingerprint()?;
            assert_ne!(before, after);
            Ok(())
        })
        .unwrap();
    }

    #[test]
    fn bdd_wps_routes_are_independent_from_lan_routes() {
        let mut config = LanRuntimeConfig {
            platform: windows_lan_runtime_context(LanRuntimeEventSink::None),
            device_id: String::new(),
            device_name: "ZSClip".to_string(),
            tcp_port: LAN_TCP_PORT_DEFAULT,
            udp_port: LAN_DISCOVERY_PORT_DEFAULT,
            lan_enabled: false,
            wps_taskpane_enabled: true,
        };

        assert!(route_available_for_config(
            &config,
            LanHttpRoute::WpsTaskpane
        ));
        assert!(route_available_for_config(&config, LanHttpRoute::WpsItems));
        assert!(route_available_for_config(&config, LanHttpRoute::WpsImage));
        assert!(route_available_for_config(&config, LanHttpRoute::WpsEvents));
        assert!(!route_available_for_config(&config, LanHttpRoute::Info));

        config.wps_taskpane_enabled = false;
        assert!(!route_available_for_config(
            &config,
            LanHttpRoute::WpsTaskpane
        ));

        config.lan_enabled = true;
        assert!(route_available_for_config(&config, LanHttpRoute::Info));
        assert!(!route_available_for_config(
            &config,
            LanHttpRoute::WpsTaskpane
        ));
    }

    fn file_content_crc_for_test(total_size: u64, bytes: &[u8]) -> u32 {
        let mut hasher = lan_file_content_hasher(total_size);
        hasher.update(bytes);
        hasher.finalize()
    }
}
